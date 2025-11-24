use crate::errors::{Result, WalletError};
use reqwest::StatusCode;
use serde::Deserialize;

/// Lightweight RPC client for wallet flows.
#[derive(Clone, Debug)]
pub struct WalletRpcClient {
    client: reqwest::Client,
    base_url: String,
}

impl WalletRpcClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    fn endpoint(&self, path: &str) -> String {
        format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }

    /// Fetch account state by hex-encoded address.
    pub async fn fetch_account(&self, address_hex: &str) -> Result<Option<AccountState>> {
        let url = self.endpoint(&format!("account/{address_hex}"));
        let response = self.client.get(url).send().await?;

        match response.status() {
            StatusCode::OK => {
                let dto = response.json::<AccountResponse>().await?;
                Ok(Some(AccountState::from(dto)?))
            }
            StatusCode::NOT_FOUND => Ok(None),
            status => Err(WalletError::RpcError(format!(
                "account lookup failed (status {})",
                status
            ))),
        }
    }

    /// Submit a payment transaction to `/tx/payment`.
    pub async fn submit_payment(&self, payload: &serde_json::Value) -> Result<serde_json::Value> {
        let url = self.endpoint("tx/payment");
        let response = self.client.post(url).json(payload).send().await?;
        let status = response.status();
        let body = response
            .json::<serde_json::Value>()
            .await
            .unwrap_or_else(|_| serde_json::Value::Null);

        if status.is_success() {
            Ok(body)
        } else {
            Err(WalletError::RpcError(format!(
                "payment rejected (status {}): {}",
                status,
                serde_json::to_string_pretty(&body).unwrap_or_else(|_| body.to_string())
            )))
        }
    }
}

/// Parsed account information.
#[derive(Debug, Clone)]
pub struct AccountState {
    pub address_hex: String,
    pub balance_atomic: u128,
    pub nonce: u64,
}

impl AccountState {
    fn from(dto: AccountResponse) -> Result<Self> {
        let balance_atomic = dto
            .balance_atomic
            .replace('_', "")
            .parse::<u128>()
            .map_err(|err| WalletError::RpcError(format!("invalid balance format: {err}")))?;
        Ok(Self {
            address_hex: dto.address,
            balance_atomic,
            nonce: dto.nonce,
        })
    }
}

#[derive(Debug, Deserialize)]
struct AccountResponse {
    pub address: String,
    pub balance_atomic: String,
    pub nonce: u64,
}
