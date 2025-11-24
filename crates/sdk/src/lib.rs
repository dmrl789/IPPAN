mod error;

pub use crate::error::SdkError;
use ippan_types::Amount;
use reqwest::{Client, Response};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::time::Duration;
use url::Url;

/// Convenience HTTP client for interacting with IPPAN nodes or gateway APIs.
#[derive(Clone)]
pub struct IppanClient {
    base_url: Url,
    http: Client,
}

impl IppanClient {
    /// Create a new client with the provided base URL (e.g. `http://localhost:8081/api/`).
    pub fn new(base_url: impl AsRef<str>) -> Result<Self, SdkError> {
        Self::with_http_client(
            base_url,
            Client::builder().timeout(Duration::from_secs(10)).build()?,
        )
    }

    /// Use an existing reqwest client (useful for custom TLS or middleware).
    pub fn with_http_client(base_url: impl AsRef<str>, http: Client) -> Result<Self, SdkError> {
        let mut url = Url::parse(base_url.as_ref())
            .map_err(|_| SdkError::InvalidBaseUrl(base_url.as_ref().to_string()))?;
        if !url.path().ends_with('/') {
            let mut path = url.path().trim_end_matches('/').to_owned();
            path.push('/');
            url.set_path(&path);
        }
        Ok(Self {
            base_url: url,
            http,
        })
    }

    /// Expose the underlying base URL.
    pub fn base_url(&self) -> &Url {
        &self.base_url
    }

    /// Fetch account information plus the most recent payments/transactions.
    pub async fn get_account(&self, address_hex: &str) -> Result<AccountInfo, SdkError> {
        let path = format!("account/{address_hex}");
        self.get_json::<AccountResponse>(&path).await?.try_into()
    }

    /// Fetch a block by hash or height.
    pub async fn get_block(&self, id: &str) -> Result<BlockInfo, SdkError> {
        let path = format!("block/{id}");
        self.get_json::<BlockResponse>(&path).await?.try_into()
    }

    /// Fetch a transaction by hash.
    pub async fn get_transaction(&self, hash: &str) -> Result<TransactionInfo, SdkError> {
        let path = format!("tx/{hash}");
        self.get_json::<TransactionView>(&path).await?.try_into()
    }

    /// Fetch the latest network time/hash timer info.
    pub async fn get_time(&self) -> Result<TimeInfo, SdkError> {
        self.get_json::<TimeResponse>("time").await?.try_into()
    }

    /// Submit a signed payment to the RPC endpoint.
    pub async fn submit_payment(
        &self,
        request: PaymentRequest,
    ) -> Result<PaymentReceipt, SdkError> {
        let payload = OutgoingPaymentRequest::from(request);
        self.post_json::<_, PaymentResponse>("tx/payment", &payload)
            .await?
            .try_into()
    }

    async fn get_json<T>(&self, path: &str) -> Result<T, SdkError>
    where
        T: DeserializeOwned,
    {
        let url = self.base_url.join(path)?;
        let response = self.http.get(url).send().await?;
        Self::map_response(response).await
    }

    async fn post_json<B, T>(&self, path: &str, body: &B) -> Result<T, SdkError>
    where
        B: serde::Serialize,
        T: DeserializeOwned,
    {
        let url = self.base_url.join(path)?;
        let response = self.http.post(url).json(body).send().await?;
        Self::map_response(response).await
    }

    async fn map_response<T>(response: Response) -> Result<T, SdkError>
    where
        T: DeserializeOwned,
    {
        if !response.status().is_success() {
            return Err(Self::map_api_error(response).await);
        }
        Ok(response.json::<T>().await?)
    }

    async fn map_api_error(response: Response) -> SdkError {
        let status = response.status().as_u16();
        let bytes = response.bytes().await.unwrap_or_default();
        if let Ok(api_error) = serde_json::from_slice::<ApiErrorResponse>(&bytes) {
            return SdkError::server_error(
                status,
                api_error.code.unwrap_or_else(|| "unknown".into()),
                api_error.message.unwrap_or_else(|| "request failed".into()),
            );
        }
        let text = String::from_utf8_lossy(&bytes).to_string();
        SdkError::server_error(status, "http_error", text)
    }
}

/// Request payload passed to `/tx/payment`.
#[derive(Debug, Clone)]
pub struct PaymentRequest {
    pub from: String,
    pub to: String,
    pub amount_atomic: u128,
    pub fee_atomic: Option<u128>,
    pub nonce: Option<u64>,
    pub memo: Option<String>,
    pub signing_key: String,
}

impl PaymentRequest {
    pub fn new(
        from: impl Into<String>,
        to: impl Into<String>,
        amount_atomic: u128,
        signing_key: impl Into<String>,
    ) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            amount_atomic,
            fee_atomic: None,
            nonce: None,
            memo: None,
            signing_key: signing_key.into(),
        }
    }

    pub fn with_fee(mut self, fee_atomic: impl Into<Option<u128>>) -> Self {
        self.fee_atomic = fee_atomic.into();
        self
    }

    pub fn with_nonce(mut self, nonce: impl Into<Option<u64>>) -> Self {
        self.nonce = nonce.into();
        self
    }

    pub fn with_memo(mut self, memo: impl Into<Option<String>>) -> Self {
        self.memo = memo.into();
        self
    }
}

#[derive(Debug, Clone)]
pub struct AccountInfo {
    pub address: String,
    pub balance: Amount,
    pub nonce: u64,
    pub recent_transactions: Vec<TransactionSummary>,
    pub recent_payments: Vec<PaymentSummary>,
}

#[derive(Debug, Clone)]
pub struct TransactionSummary {
    pub hash: String,
    pub from: String,
    pub to: String,
    pub amount: Amount,
    pub fee: Amount,
    pub nonce: u64,
    pub timestamp: u64,
    pub hash_timer: String,
    pub memo: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PaymentSummary {
    pub hash: String,
    pub direction: PaymentDirection,
    pub amount: Amount,
    pub fee: Amount,
    pub total_cost: Option<Amount>,
    pub status: PaymentStatus,
    pub timestamp: u64,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum PaymentDirection {
    Incoming,
    Outgoing,
    SelfTransfer,
}

#[derive(Debug, Clone, Copy)]
pub enum PaymentStatus {
    AcceptedToMempool,
    Finalized,
}

#[derive(Debug, Clone)]
pub struct BlockInfo {
    pub id: String,
    pub round: u64,
    pub height: Option<u64>,
    pub creator: String,
    pub hash_timer: String,
    pub timestamp: u64,
    pub parent_ids: Vec<String>,
    pub tx_hashes: Vec<String>,
    pub transactions: Vec<TransactionSummary>,
    pub fee_summary: Option<FeeSummary>,
}

#[derive(Debug, Clone)]
pub struct FeeSummary {
    pub round: u64,
    pub total_fees: Amount,
    pub treasury_fees: Amount,
    pub applied_payments: u64,
    pub rejected_payments: u64,
}

#[derive(Debug, Clone)]
pub struct PaymentReceipt {
    pub tx_hash: String,
    pub status: PaymentStatus,
    pub from: String,
    pub to: String,
    pub nonce: u64,
    pub amount: Amount,
    pub fee: Amount,
    pub timestamp: u64,
    pub hash_timer: String,
    pub memo: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TransactionInfo {
    pub summary: TransactionSummary,
    pub status: PaymentStatus,
}

#[derive(Debug, Clone)]
pub struct TimeInfo {
    pub timestamp: u64,
    pub time_us: u64,
}

#[derive(Debug, Deserialize)]
struct ApiErrorResponse {
    code: Option<String>,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct AccountResponse {
    address: String,
    balance_atomic: String,
    nonce: u64,
    #[serde(default)]
    recent_transactions: Vec<TransactionView>,
    #[serde(default)]
    recent_payments: Vec<PaymentView>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
struct TransactionView {
    hash: String,
    from: String,
    to: String,
    amount_atomic: String,
    fee_atomic: String,
    nonce: u64,
    timestamp: u64,
    hash_timer: String,
    #[serde(default)]
    memo: Option<String>,
    status: PaymentStatusView,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct PaymentView {
    hash: String,
    amount_atomic: String,
    fee_atomic: String,
    #[serde(default)]
    total_cost_atomic: Option<String>,
    timestamp: u64,
    #[serde(default)]
    memo: Option<String>,
    direction: PaymentDirectionView,
    status: PaymentStatusView,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct BlockResponse {
    block: BlockView,
    #[serde(default)]
    fee_summary: Option<FeeSummaryView>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct BlockView {
    id: String,
    round: u64,
    #[serde(default)]
    height: Option<u64>,
    creator: String,
    hash_timer: String,
    timestamp: u64,
    parent_ids: Vec<String>,
    transaction_hashes: Vec<String>,
    #[serde(default)]
    transactions: Vec<TransactionView>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct FeeSummaryView {
    round: u64,
    total_fees_atomic: String,
    treasury_fees_atomic: String,
    applied_payments: u64,
    rejected_payments: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct PaymentResponse {
    tx_hash: String,
    status: PaymentStatusView,
    from: String,
    to: String,
    nonce: u64,
    amount_atomic: String,
    fee_atomic: String,
    timestamp: u64,
    hash_timer: String,
    #[serde(default)]
    memo: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct TimeResponse {
    timestamp: u64,
    time_us: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum PaymentDirectionView {
    Incoming,
    Outgoing,
    SelfTransfer,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
enum PaymentStatusView {
    AcceptedToMempool,
    Finalized,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
struct OutgoingPaymentRequest {
    from: String,
    to: String,
    amount: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    fee: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    nonce: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    memo: Option<String>,
    signing_key: String,
}

impl From<PaymentRequest> for OutgoingPaymentRequest {
    fn from(req: PaymentRequest) -> Self {
        Self {
            from: req.from,
            to: req.to,
            amount: req.amount_atomic.to_string(),
            fee: req.fee_atomic.map(|f| f.to_string()),
            nonce: req.nonce,
            memo: req.memo,
            signing_key: req.signing_key,
        }
    }
}

// --- TryFrom impls ---

impl TryFrom<AccountResponse> for AccountInfo {
    type Error = SdkError;

    fn try_from(value: AccountResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            address: value.address,
            balance: parse_amount(&value.balance_atomic)?,
            nonce: value.nonce,
            recent_transactions: value
                .recent_transactions
                .into_iter()
                .map(TransactionSummary::try_from)
                .collect::<Result<_, _>>()?,
            recent_payments: value
                .recent_payments
                .into_iter()
                .map(PaymentSummary::try_from)
                .collect::<Result<_, _>>()?,
        })
    }
}

impl TryFrom<TransactionView> for TransactionSummary {
    type Error = SdkError;

    fn try_from(value: TransactionView) -> Result<Self, Self::Error> {
        Ok(Self {
            hash: value.hash,
            from: value.from,
            to: value.to,
            amount: parse_amount(&value.amount_atomic)?,
            fee: parse_amount(&value.fee_atomic)?,
            nonce: value.nonce,
            timestamp: value.timestamp,
            hash_timer: value.hash_timer,
            memo: value.memo,
        })
    }
}

impl TryFrom<PaymentView> for PaymentSummary {
    type Error = SdkError;

    fn try_from(value: PaymentView) -> Result<Self, Self::Error> {
        Ok(Self {
            hash: value.hash,
            direction: value.direction.into(),
            amount: parse_amount(&value.amount_atomic)?,
            fee: parse_amount(&value.fee_atomic)?,
            total_cost: match value.total_cost_atomic {
                Some(ref c) => Some(parse_amount(c)?),
                None => None,
            },
            status: value.status.into(),
            timestamp: value.timestamp,
            memo: value.memo,
        })
    }
}

impl TryFrom<BlockResponse> for BlockInfo {
    type Error = SdkError;

    fn try_from(value: BlockResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value.block.id,
            round: value.block.round,
            height: value.block.height,
            creator: value.block.creator,
            hash_timer: value.block.hash_timer,
            timestamp: value.block.timestamp,
            parent_ids: value.block.parent_ids,
            tx_hashes: value.block.transaction_hashes.clone(),
            transactions: value
                .block
                .transactions
                .into_iter()
                .map(TransactionSummary::try_from)
                .collect::<Result<_, _>>()?,
            fee_summary: value.fee_summary.map(FeeSummary::try_from).transpose()?,
        })
    }
}

impl TryFrom<FeeSummaryView> for FeeSummary {
    type Error = SdkError;

    fn try_from(value: FeeSummaryView) -> Result<Self, Self::Error> {
        Ok(Self {
            round: value.round,
            total_fees: parse_amount(&value.total_fees_atomic)?,
            treasury_fees: parse_amount(&value.treasury_fees_atomic)?,
            applied_payments: value.applied_payments,
            rejected_payments: value.rejected_payments,
        })
    }
}

impl TryFrom<PaymentResponse> for PaymentReceipt {
    type Error = SdkError;

    fn try_from(value: PaymentResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            tx_hash: value.tx_hash,
            status: value.status.into(),
            from: value.from,
            to: value.to,
            nonce: value.nonce,
            amount: parse_amount(&value.amount_atomic)?,
            fee: parse_amount(&value.fee_atomic)?,
            timestamp: value.timestamp,
            hash_timer: value.hash_timer,
            memo: value.memo,
        })
    }
}

impl TryFrom<TransactionView> for TransactionInfo {
    type Error = SdkError;

    fn try_from(value: TransactionView) -> Result<Self, Self::Error> {
        Ok(Self {
            summary: TransactionSummary::try_from(value.clone())?,
            status: value.status.into(),
        })
    }
}

impl TryFrom<TimeResponse> for TimeInfo {
    type Error = SdkError;

    fn try_from(value: TimeResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            timestamp: value.timestamp,
            time_us: value.time_us,
        })
    }
}

impl From<PaymentDirectionView> for PaymentDirection {
    fn from(value: PaymentDirectionView) -> Self {
        match value {
            PaymentDirectionView::Incoming => PaymentDirection::Incoming,
            PaymentDirectionView::Outgoing => PaymentDirection::Outgoing,
            PaymentDirectionView::SelfTransfer => PaymentDirection::SelfTransfer,
        }
    }
}

impl From<PaymentStatusView> for PaymentStatus {
    fn from(value: PaymentStatusView) -> Self {
        match value {
            PaymentStatusView::AcceptedToMempool => PaymentStatus::AcceptedToMempool,
            PaymentStatusView::Finalized => PaymentStatus::Finalized,
        }
    }
}

fn parse_amount(raw: &str) -> Result<Amount, SdkError> {
    let atomic = raw
        .replace('_', "")
        .parse::<u128>()
        .map_err(|err| SdkError::parse_error(format!("invalid amount `{raw}`: {err}")))?;
    Ok(Amount::from_atomic(atomic))
}
