use std::collections::BTreeMap;
use std::convert::Infallible;
use std::fmt;
use std::future::Future;
use std::net::SocketAddr;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::task::{Context as TaskContext, Poll};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context as AnyhowContext, Result};
use axum::async_trait;
use axum::body::Body;
use axum::error_handling::HandleErrorLayer;
use axum::extract::rejection::JsonRejection;
use axum::extract::FromRequest;
use axum::extract::{ConnectInfo, Path as AxumPath, Query, State};
use axum::http::header::{HeaderValue, CONTENT_TYPE};
use axum::http::Request;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use ed25519_dalek::{Signer, SigningKey};
#[cfg(test)]
use http_body_util::BodyExt;
use ippan_consensus::PoAConsensus;
use ippan_consensus_dlc::AiConsensusStatus;
use ippan_files::{FileDhtService, FileStorage};
use ippan_l1_fees::FeePolicy;
use ippan_l1_handle_anchors::L1HandleAnchorStorage;
use ippan_l2_handle_registry::{
    dht::HandleDhtService, Handle, HandleMetadata, HandleRegistryError, HandleStatus,
    L2HandleRegistry,
};
use ippan_mempool::Mempool;
use ippan_security::{SecurityError, SecurityManager};
use ippan_storage::{Account, Storage};
use ippan_types::address::{decode_address, encode_address};
use ippan_types::health::{HealthStatus, NodeHealth, NodeHealthContext};
use ippan_types::time_service::ippan_time_now;
use ippan_types::{
    Amount, Block, HandleOperation, HandleRegisterOp, L2Commit, L2ExitRecord, L2Network,
    RoundFinalizationRecord, Transaction, TransactionVisibility,
};
use metrics_exporter_prometheus::PrometheusHandle;
use serde::de::{self, DeserializeOwned, Deserializer, Visitor};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::sync::{mpsc, Mutex};
use tower::limit::ConcurrencyLimitLayer;
use tower::timeout::TimeoutLayer;
use tower::BoxError;
use tower::Layer;
use tower::Service;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing::{debug, error, info, warn};

use hex::encode as hex_encode;

use crate::{
    files::{handle_get_file, handle_publish_file},
    HttpP2PNetwork, NetworkMessage,
};

const RATE_LIMIT_PER_SECOND: u64 = 200;
const CIRCUIT_BREAKER_FAILURE_THRESHOLD: usize = 5;
const CIRCUIT_BREAKER_OPEN_SECS: u64 = 30;
const MAX_MEMO_BYTES: usize = 256;
const DEFAULT_PAYMENT_HISTORY_LIMIT: usize = 25;
const MAX_PAYMENT_HISTORY_LIMIT: usize = 200;
const PAYMENT_ENDPOINT: &str = "/tx/payment";
const HANDLE_REGISTER_ENDPOINT: &str = "/handle/register";
const HANDLE_LOOKUP_ENDPOINT: &str = "/handle/:handle";
const MAX_BODY_BYTES: usize = 64 * 1024; // 64 KiB default when security manager not configured
const REQUEST_TIMEOUT_SECS: u64 = 10;
const MAX_CONCURRENT_REQUESTS: usize = 128;

/// Layer 2 configuration
#[derive(Clone, Debug, Serialize)]
pub struct L2Config {
    pub max_commit_size: usize,
    pub min_epoch_gap_ms: u64,
    pub challenge_window_ms: u64,
    pub da_mode: String,
    pub max_l2_count: usize,
}

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<dyn Storage + Send + Sync>,
    pub start_time: Instant,
    pub peer_count: Arc<AtomicUsize>,
    pub p2p_network: Option<Arc<HttpP2PNetwork>>,
    pub tx_sender: Option<mpsc::UnboundedSender<Transaction>>,
    pub node_id: String,
    pub consensus_mode: String,
    pub consensus: Option<ConsensusHandle>,
    pub ai_status: Option<AiStatusHandle>,
    pub l2_config: L2Config,
    pub mempool: Arc<Mempool>,
    pub unified_ui_dist: Option<PathBuf>,
    pub req_count: Arc<AtomicUsize>,
    pub security: Option<Arc<SecurityManager>>,
    pub metrics: Option<PrometheusHandle>,
    pub file_storage: Option<Arc<dyn FileStorage>>,
    pub file_dht: Option<Arc<dyn FileDhtService>>,
    pub dht_file_mode: String,
    pub dev_mode: bool,
    pub handle_registry: Arc<L2HandleRegistry>,
    pub handle_anchors: Arc<L1HandleAnchorStorage>,
    pub handle_dht: Option<Arc<dyn HandleDhtService>>,
    pub dht_handle_mode: String,
}

type AiStatusFuture = Pin<Box<dyn Future<Output = AiConsensusStatus> + Send>>;

/// Handle for retrieving AI status snapshots from consensus or stubs.
#[derive(Clone)]
pub struct AiStatusHandle {
    getter: Arc<dyn Fn() -> AiStatusFuture + Send + Sync>,
}

impl AiStatusHandle {
    pub fn new<F, Fut>(factory: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = AiConsensusStatus> + Send + 'static,
    {
        Self {
            getter: Arc::new(move || {
                let future = factory();
                Box::pin(future) as AiStatusFuture
            }),
        }
    }

    pub fn from_static(status: AiConsensusStatus) -> Self {
        Self::new(move || {
            let snapshot = status.clone();
            async move { snapshot }
        })
    }

    pub async fn snapshot(&self) -> AiConsensusStatus {
        (self.getter)().await
    }
}

/// Consensus handle abstraction
#[derive(Clone)]
pub struct ConsensusHandle {
    consensus: Arc<Mutex<PoAConsensus>>,
    tx_sender: mpsc::UnboundedSender<Transaction>,
    #[allow(dead_code)]
    mempool: Arc<Mempool>,
}

impl ConsensusHandle {
    pub fn new(
        consensus: Arc<Mutex<PoAConsensus>>,
        tx_sender: mpsc::UnboundedSender<Transaction>,
        mempool: Arc<Mempool>,
    ) -> Self {
        Self {
            consensus,
            tx_sender,
            mempool,
        }
    }

    pub async fn snapshot(&self) -> Result<ConsensusStateView> {
        let guard = self.consensus.lock().await;
        let state = guard.get_state();
        let validators: Vec<String> = guard
            .config
            .validators
            .iter()
            .map(|v| hex::encode(v.id))
            .collect();
        Ok(ConsensusStateView {
            round: state.current_round,
            validators,
        })
    }

    pub fn submit_transaction(&self, tx: Transaction) -> Result<()> {
        self.tx_sender
            .send(tx)
            .map_err(|err| anyhow!("failed to enqueue transaction: {err}"))
    }
}

/// Simplified consensus state view
#[derive(Clone, Debug, Serialize)]
pub struct ConsensusStateView {
    pub round: u64,
    pub validators: Vec<String>,
}

#[derive(Debug, Serialize)]
struct AiStatus {
    enabled: bool,
    using_stub: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    model_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    model_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    consensus_mode: Option<String>,
}

impl AiStatus {
    fn disabled() -> Self {
        Self {
            enabled: false,
            using_stub: false,
            model_hash: None,
            model_version: None,
            consensus_mode: None,
        }
    }
}

impl From<AiConsensusStatus> for AiStatus {
    fn from(status: AiConsensusStatus) -> Self {
        Self {
            enabled: status.enabled,
            using_stub: status.using_stub,
            model_hash: status.model_hash,
            model_version: status.model_version,
            consensus_mode: None,
        }
    }
}

/// Transaction lookup response payload used by JSON responses.
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct BlockResponse {
    block: BlockView,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    fee_summary: Option<RoundFeeSummaryView>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct BlockView {
    id: String,
    round: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    height: Option<u64>,
    creator: String,
    hash_timer: String,
    timestamp: u64,
    parent_ids: Vec<String>,
    transaction_hashes: Vec<String>,
    tx_count: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    transactions: Vec<TransactionView>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct RoundFeeSummaryView {
    round: u64,
    total_fees_atomic: String,
    treasury_fees_atomic: String,
    applied_payments: u64,
    rejected_payments: u64,
}

impl RoundFeeSummaryView {
    fn from_record(record: &RoundFinalizationRecord) -> Option<Self> {
        let total = record.total_fees_atomic?;
        let treasury = record.treasury_fees_atomic.unwrap_or(0);
        Some(Self {
            round: record.round,
            total_fees_atomic: format_atomic(total),
            treasury_fees_atomic: format_atomic(treasury),
            applied_payments: record.applied_payments.unwrap_or(0),
            rejected_payments: record.rejected_payments.unwrap_or(0),
        })
    }
}

impl TransactionView {
    fn from_transaction(tx: &Transaction, status: TransactionStatus) -> Self {
        let fee_required = FeePolicy::default().required_fee(tx);
        Self {
            hash: hex_encode(tx.hash()),
            from: encode_address(&tx.from),
            to: encode_address(&tx.to),
            amount_atomic: format_atomic(tx.amount.atomic()),
            fee_atomic: format_fee(fee_required),
            nonce: tx.nonce,
            timestamp: tx.timestamp.0,
            hash_timer: tx.hashtimer.to_hex(),
            status,
            visibility: tx.visibility,
            memo: tx.topics.first().cloned(),
            handle_operation: tx.handle_op.clone(),
        }
    }
}

impl BlockView {
    fn from_block(block: &Block, height: Option<u64>) -> Self {
        let transaction_hashes = block
            .transactions
            .iter()
            .map(|tx| hex_encode(tx.hash()))
            .collect::<Vec<_>>();
        let transactions = block
            .transactions
            .iter()
            .map(|tx| TransactionView::from_transaction(tx, TransactionStatus::Finalized))
            .collect::<Vec<_>>();
        let timestamp = block.header.hashtimer.timestamp_us.max(0) as u64;
        Self {
            id: hex_encode(block.header.id),
            round: block.header.round,
            height,
            creator: hex_encode(block.header.creator),
            hash_timer: block.header.hashtimer.to_hex(),
            timestamp,
            parent_ids: block.header.parent_ids.iter().map(hex_encode).collect(),
            transaction_hashes,
            tx_count: block.transactions.len(),
            transactions,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum TransactionStatus {
    #[allow(dead_code)]
    AcceptedToMempool,
    Finalized,
}

#[derive(Debug, Serialize)]
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
    status: TransactionStatus,
    visibility: TransactionVisibility,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    memo: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    handle_operation: Option<HandleOperation>,
}

/// Account lookup response payload with recent transaction history.
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct AccountResponse {
    address: String,
    balance_atomic: String,
    nonce: u64,
    recent_transactions: Vec<TransactionView>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    recent_payments: Vec<PaymentView>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PaymentRequest {
    from: String,
    to: String,
    #[serde(deserialize_with = "deserialize_u128_from_any")]
    amount: u128,
    #[serde(default, deserialize_with = "deserialize_option_u128_from_any")]
    fee: Option<u128>,
    #[serde(default)]
    nonce: Option<u64>,
    #[serde(default)]
    memo: Option<String>,
    #[serde(default)]
    signing_key: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DevFundRequest {
    address: String,
    amount: u64,
    #[serde(default)]
    nonce: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct PaymentResponse {
    tx_hash: String,
    status: PaymentStatus,
    from: String,
    to: String,
    nonce: u64,
    amount_atomic: String,
    fee_atomic: String,
    timestamp: u64,
    hash_timer: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    memo: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum PaymentStatus {
    AcceptedToMempool,
    Finalized,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct PaymentView {
    hash: String,
    from: String,
    to: String,
    direction: PaymentDirection,
    amount_atomic: String,
    fee_atomic: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    total_cost_atomic: Option<String>,
    nonce: u64,
    timestamp: u64,
    hash_timer: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    memo: Option<String>,
    status: PaymentStatus,
}

#[derive(Debug, Serialize)]
struct DevFundResponse {
    address_hex: String,
    address_base58: String,
    balance: u64,
    nonce: u64,
    created: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HandleRegisterRequest {
    handle: String,
    owner: String,
    #[serde(default)]
    metadata: BTreeMap<String, String>,
    #[serde(default)]
    expires_at: Option<u64>,
    #[serde(default, deserialize_with = "deserialize_option_u128_from_any")]
    fee: Option<u128>,
    #[serde(default)]
    nonce: Option<u64>,
    signing_key: String,
}

#[derive(Debug, Serialize)]
struct HandleRegisterResponse {
    tx_hash: String,
    handle: String,
    owner: String,
    nonce: u64,
    fee_atomic: String,
    expires_at: Option<u64>,
    metadata: BTreeMap<String, String>,
    status: HandleSubmissionStatus,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum HandleSubmissionStatus {
    AcceptedToMempool,
}

#[derive(Debug, Serialize)]
struct HandleInfoResponse {
    handle: String,
    owner: String,
    status: String,
    expires_at: Option<u64>,
    metadata: BTreeMap<String, String>,
    created_at: u64,
    updated_at: u64,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
enum PaymentDirection {
    Incoming,
    Outgoing,
    SelfTransfer,
}

#[derive(Debug, Default, Deserialize)]
struct PaymentHistoryQuery {
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug)]
pub struct ValidatedJson<T>(pub T);

impl<T> Deref for ValidatedJson<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[async_trait]
impl<S, T> FromRequest<S, Body> for ValidatedJson<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<ApiError>);

    async fn from_request(req: Request<Body>, state: &S) -> Result<Self, Self::Rejection> {
        match Json::<T>::from_request(req, state).await {
            Ok(Json(value)) => Ok(Self(value)),
            Err(rejection) => {
                let (status, code, message) = map_json_rejection(&rejection);
                Err((status, Json(ApiError::new(code, message))))
            }
        }
    }
}

fn map_json_rejection(rejection: &JsonRejection) -> (StatusCode, &'static str, String) {
    match rejection {
        JsonRejection::JsonDataError(err) => {
            (StatusCode::BAD_REQUEST, "invalid_json", err.to_string())
        }
        JsonRejection::JsonSyntaxError(err) => {
            (StatusCode::BAD_REQUEST, "invalid_json", err.to_string())
        }
        JsonRejection::MissingJsonContentType(_) => (
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "unsupported_media_type",
            "missing or invalid content-type; expected application/json".to_string(),
        ),
        JsonRejection::BytesRejection(err) => {
            (StatusCode::BAD_REQUEST, "invalid_body", err.to_string())
        }
        _ => (
            StatusCode::BAD_REQUEST,
            "invalid_json",
            rejection.to_string(),
        ),
    }
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    code: &'static str,
    message: String,
}

impl ApiError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

async fn handle_service_error(err: BoxError) -> (StatusCode, Json<ApiError>) {
    if err.is::<tower::timeout::error::Elapsed>() {
        return (
            StatusCode::REQUEST_TIMEOUT,
            Json(ApiError::new("timeout", "request timed out")),
        );
    }

    let message = err.to_string();
    let message_lower = message.to_lowercase();

    if message_lower.contains("body") && message_lower.contains("limit") {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(ApiError::new(
                "body_too_large",
                "request body exceeds configured limit",
            )),
        );
    }

    if message_lower.contains("failed to buffer the request body") {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(ApiError::new(
                "body_too_large",
                "request body exceeds configured limit",
            )),
        );
    }

    if message_lower.contains("concurrency") || message_lower.contains("capacity") {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(ApiError::new(
                "too_many_requests",
                "server is handling too many requests; please retry",
            )),
        );
    }

    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ApiError::new("internal_error", "unexpected internal error")),
    )
}

async fn handle_not_found() -> (StatusCode, Json<ApiError>) {
    (
        StatusCode::NOT_FOUND,
        Json(ApiError::new(
            "not_found",
            "requested resource was not found",
        )),
    )
}

#[derive(Debug, Error)]
enum PaymentError {
    #[error("signing key is required to submit a payment")]
    MissingSigningKey,
    #[error("invalid signing key: {0}")]
    InvalidSigningKey(String),
    #[error("invalid account address: {0}")]
    InvalidAddress(String),
    #[error("memo exceeds {0} bytes")]
    MemoTooLong(usize),
    #[error("amount must be greater than zero")]
    ZeroAmount,
    #[error("account not found")]
    AccountNotFound,
    #[error("failed to derive nonce: {0}")]
    NonceLookupFailed(String),
    #[error("required fee {required} exceeds provided limit {provided}")]
    FeeTooLow { required: u64, provided: u128 },
    #[error("consensus not active")]
    ConsensusUnavailable,
    #[error("transaction signing failed: {0}")]
    SigningFailed(String),
    #[error("transaction submission failed: {0}")]
    SubmissionFailed(String),
}

#[derive(Debug, Error)]
enum HandleRegistrationError {
    #[error("handle must include @ prefix and suffix (e.g. @user.ipn)")]
    InvalidHandleFormat,
    #[error("invalid owner address: {0}")]
    InvalidOwner(String),
    #[error("missing signing key")]
    MissingSigningKey,
    #[error("invalid signing key: {0}")]
    InvalidSigningKey(String),
    #[error("account not found")]
    AccountNotFound,
    #[error("failed to derive nonce")]
    NonceDerivationFailed,
    #[error("fee too low (required {required}, provided {provided})")]
    FeeTooLow { required: u64, provided: u128 },
    #[error("failed to build handle transaction: {0}")]
    BuildFailure(String),
    #[error("consensus not active")]
    ConsensusUnavailable,
    #[error("failed to submit handle transaction: {0}")]
    SubmissionFailed(String),
}

impl HandleRegistrationError {
    fn status_and_code(&self) -> (StatusCode, &'static str) {
        match self {
            HandleRegistrationError::InvalidHandleFormat => {
                (StatusCode::BAD_REQUEST, "invalid_handle")
            }
            HandleRegistrationError::InvalidOwner(_) => (StatusCode::BAD_REQUEST, "invalid_owner"),
            HandleRegistrationError::MissingSigningKey => {
                (StatusCode::BAD_REQUEST, "missing_signing_key")
            }
            HandleRegistrationError::InvalidSigningKey(_) => {
                (StatusCode::BAD_REQUEST, "invalid_signing_key")
            }
            HandleRegistrationError::AccountNotFound => {
                (StatusCode::BAD_REQUEST, "account_not_found")
            }
            HandleRegistrationError::NonceDerivationFailed => {
                (StatusCode::INTERNAL_SERVER_ERROR, "nonce_error")
            }
            HandleRegistrationError::FeeTooLow { .. } => (StatusCode::BAD_REQUEST, "fee_too_low"),
            HandleRegistrationError::BuildFailure(_) => (StatusCode::BAD_REQUEST, "build_failure"),
            HandleRegistrationError::ConsensusUnavailable => {
                (StatusCode::SERVICE_UNAVAILABLE, "consensus_unavailable")
            }
            HandleRegistrationError::SubmissionFailed(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "submission_failed")
            }
        }
    }
}

impl PaymentError {
    fn status_and_code(&self) -> (StatusCode, &'static str) {
        match self {
            PaymentError::MissingSigningKey => (StatusCode::BAD_REQUEST, "missing_signing_key"),
            PaymentError::InvalidSigningKey(_) => (StatusCode::BAD_REQUEST, "invalid_signing_key"),
            PaymentError::InvalidAddress(_) => (StatusCode::BAD_REQUEST, "invalid_address"),
            PaymentError::MemoTooLong(_) => (StatusCode::BAD_REQUEST, "memo_too_long"),
            PaymentError::ZeroAmount => (StatusCode::BAD_REQUEST, "invalid_amount"),
            PaymentError::AccountNotFound => (StatusCode::NOT_FOUND, "account_not_found"),
            PaymentError::NonceLookupFailed(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "nonce_lookup_failed")
            }
            PaymentError::FeeTooLow { .. } => (StatusCode::BAD_REQUEST, "fee_too_low"),
            PaymentError::ConsensusUnavailable => {
                (StatusCode::SERVICE_UNAVAILABLE, "consensus_unavailable")
            }
            PaymentError::SigningFailed(_) => (StatusCode::BAD_REQUEST, "signing_failed"),
            PaymentError::SubmissionFailed(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "submission_failed")
            }
        }
    }
}

struct BuiltPayment {
    transaction: Transaction,
    amount_atomic: u128,
    fee_atomic: u64,
    memo: Option<String>,
    from: [u8; 32],
    to: [u8; 32],
}

struct BuiltHandleRegistration {
    transaction: Transaction,
    handle: String,
    owner: [u8; 32],
    metadata: BTreeMap<String, String>,
    fee_atomic: u64,
    nonce: u64,
    expires_at: Option<u64>,
}

fn deserialize_u128_from_any<'de, D>(deserializer: D) -> Result<u128, D::Error>
where
    D: Deserializer<'de>,
{
    struct U128Visitor;

    impl<'de> Visitor<'de> for U128Visitor {
        type Value = u128;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an integer encoded as a number or string")
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value as u128)
        }

        fn visit_u128<E>(self, value: u128) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value)
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            value
                .trim()
                .parse::<u128>()
                .map_err(|err| de::Error::custom(format!("invalid integer: {err}")))
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            self.visit_str(&value)
        }
    }

    deserializer.deserialize_any(U128Visitor)
}

fn deserialize_option_u128_from_any<'de, D>(deserializer: D) -> Result<Option<u128>, D::Error>
where
    D: Deserializer<'de>,
{
    struct OptionVisitor;

    impl<'de> Visitor<'de> for OptionVisitor {
        type Value = Option<u128>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an optional integer encoded as a number or string")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserialize_u128_from_any(deserializer).map(Some)
        }
    }

    deserializer.deserialize_option(OptionVisitor)
}

fn format_atomic(value: u128) -> String {
    value.to_string()
}

fn format_fee(value: u64) -> String {
    format_atomic(value as u128)
}

fn clamp_history_limit(limit: Option<usize>) -> usize {
    let requested = limit.unwrap_or(DEFAULT_PAYMENT_HISTORY_LIMIT);
    requested.clamp(1, MAX_PAYMENT_HISTORY_LIMIT)
}

fn parse_signing_key_hex(raw: &str) -> Result<[u8; 32], PaymentError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(PaymentError::MissingSigningKey);
    }
    let normalized = trimmed.strip_prefix("0x").unwrap_or(trimmed);
    let bytes =
        hex::decode(normalized).map_err(|err| PaymentError::InvalidSigningKey(err.to_string()))?;
    if bytes.len() != 32 {
        return Err(PaymentError::InvalidSigningKey(format!(
            "expected 32-byte key, got {} bytes",
            bytes.len()
        )));
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes);
    Ok(key)
}

fn normalize_memo(memo: Option<String>) -> Result<Option<String>, PaymentError> {
    if let Some(raw) = memo {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }
        if trimmed.len() > MAX_MEMO_BYTES {
            return Err(PaymentError::MemoTooLong(MAX_MEMO_BYTES));
        }
        return Ok(Some(trimmed.to_string()));
    }
    Ok(None)
}

fn normalize_handle_input(handle: &str) -> Result<String, HandleRegistrationError> {
    let trimmed = handle.trim();
    if trimmed.is_empty() {
        return Err(HandleRegistrationError::InvalidHandleFormat);
    }
    let normalized = if trimmed.starts_with('@') {
        trimmed.to_string()
    } else {
        format!("@{trimmed}")
    };
    let candidate = Handle::new(normalized.clone());
    if !candidate.is_valid() {
        return Err(HandleRegistrationError::InvalidHandleFormat);
    }
    Ok(candidate.as_str().to_string())
}

fn normalize_handle_query(raw: &str) -> Result<Handle, HandleRegistrationError> {
    let normalized = normalize_handle_input(raw)?;
    Ok(Handle::new(normalized))
}

fn parse_handle_signing_key(raw: &str) -> Result<SigningKey, HandleRegistrationError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(HandleRegistrationError::MissingSigningKey);
    }
    let normalized = trimmed.strip_prefix("0x").unwrap_or(trimmed);
    let bytes = hex::decode(normalized)
        .map_err(|err| HandleRegistrationError::InvalidSigningKey(err.to_string()))?;
    if bytes.len() != 32 {
        return Err(HandleRegistrationError::InvalidSigningKey(format!(
            "expected 32-byte key, got {} bytes",
            bytes.len()
        )));
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes);
    Ok(SigningKey::from_bytes(&key))
}

fn derive_handle_nonce(
    state: &Arc<AppState>,
    owner: &[u8; 32],
    requested: Option<u64>,
) -> Result<u64, HandleRegistrationError> {
    if let Some(nonce) = requested {
        return Ok(nonce);
    }
    match state.storage.get_account(owner) {
        Ok(Some(account)) => Ok(account.nonce.saturating_add(1)),
        Ok(None) => Err(HandleRegistrationError::AccountNotFound),
        Err(_) => Err(HandleRegistrationError::NonceDerivationFailed),
    }
}

fn sort_metadata(metadata: BTreeMap<String, String>) -> BTreeMap<String, String> {
    metadata
        .into_iter()
        .filter_map(|(key, value)| {
            let trimmed_key = key.trim();
            if trimmed_key.is_empty() {
                return None;
            }
            let trimmed_value = value.trim();
            Some((trimmed_key.to_string(), trimmed_value.to_string()))
        })
        .collect()
}

fn sign_handle_registration_payload(
    signing_key: &SigningKey,
    handle: &str,
    owner: &[u8; 32],
    expires_at: Option<u64>,
) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(b"IPPAN_HANDLE_REGISTRATION");
    payload.extend_from_slice(handle.as_bytes());
    payload.extend_from_slice(owner);
    if let Some(exp) = expires_at {
        payload.extend_from_slice(&exp.to_le_bytes());
    }
    let digest = Sha256::digest(&payload);
    signing_key.sign(&digest).to_bytes().to_vec()
}

fn build_handle_registration_transaction(
    state: &Arc<AppState>,
    request: HandleRegisterRequest,
) -> Result<BuiltHandleRegistration, HandleRegistrationError> {
    let handle = normalize_handle_input(&request.handle)?;
    let owner_bytes = decode_address(&request.owner)
        .map_err(|err| HandleRegistrationError::InvalidOwner(err.to_string()))?;
    let signing_key = parse_handle_signing_key(&request.signing_key)?;
    if signing_key.verifying_key().to_bytes() != owner_bytes {
        return Err(HandleRegistrationError::InvalidOwner(
            "owner does not match signing key".to_string(),
        ));
    }

    let nonce = derive_handle_nonce(state, &owner_bytes, request.nonce)?;
    let metadata = sort_metadata(request.metadata);
    let signature =
        sign_handle_registration_payload(&signing_key, &handle, &owner_bytes, request.expires_at);

    let mut tx = Transaction::new(owner_bytes, [0u8; 32], Amount::zero(), nonce);
    let operation = HandleOperation::Register(HandleRegisterOp {
        handle: handle.clone(),
        owner: owner_bytes,
        metadata: metadata.clone(),
        expires_at: request.expires_at,
        signature,
    });
    tx.set_handle_operation(operation);
    let signing_bytes = signing_key.to_bytes();
    tx.sign(&signing_bytes)
        .map_err(|err| HandleRegistrationError::BuildFailure(err.to_string()))?;

    let policy = FeePolicy::default();
    let fee_atomic = policy.required_fee(&tx);
    if let Err(err) = policy.enforce_fee_limit(request.fee, fee_atomic) {
        if let ippan_l1_fees::FeePolicyError::FeeBelowMinimum { required, provided } = err {
            return Err(HandleRegistrationError::FeeTooLow { required, provided });
        }
        return Err(HandleRegistrationError::BuildFailure(err.to_string()));
    }

    Ok(BuiltHandleRegistration {
        transaction: tx,
        handle,
        owner: owner_bytes,
        metadata,
        fee_atomic,
        nonce,
        expires_at: request.expires_at,
    })
}

async fn handle_error_response(
    state: &Arc<AppState>,
    addr: &SocketAddr,
    endpoint: &str,
    err: HandleRegistrationError,
) -> (StatusCode, Json<ApiError>) {
    let (status, code) = err.status_and_code();
    let message = err.to_string();
    record_security_failure(state, addr, endpoint, &message).await;
    (status, Json(ApiError::new(code, message)))
}

fn handle_registration_response_from_built(
    built: &BuiltHandleRegistration,
) -> HandleRegisterResponse {
    HandleRegisterResponse {
        tx_hash: hex::encode(built.transaction.hash()),
        handle: built.handle.clone(),
        owner: encode_address(&built.owner),
        nonce: built.nonce,
        fee_atomic: format_fee(built.fee_atomic),
        expires_at: built.expires_at,
        metadata: built.metadata.clone(),
        status: HandleSubmissionStatus::AcceptedToMempool,
    }
}

fn format_handle_status(status: &HandleStatus) -> &'static str {
    match status {
        HandleStatus::Active => "active",
        HandleStatus::Suspended => "suspended",
        HandleStatus::Expired => "expired",
        HandleStatus::Transferred => "transferred",
    }
}

fn handle_info_from_metadata(handle: &Handle, metadata: HandleMetadata) -> HandleInfoResponse {
    let owner = encode_address(metadata.owner.as_bytes());
    let metadata_map = metadata
        .metadata
        .into_iter()
        .collect::<BTreeMap<String, String>>();
    HandleInfoResponse {
        handle: handle.as_str().to_string(),
        owner,
        status: format_handle_status(&metadata.status).to_string(),
        expires_at: if metadata.expires_at == 0 {
            None
        } else {
            Some(metadata.expires_at)
        },
        metadata: metadata_map,
        created_at: metadata.created_at,
        updated_at: metadata.updated_at,
    }
}

fn map_handle_lookup_error(err: HandleRegistryError) -> (StatusCode, &'static str, String) {
    match err {
        HandleRegistryError::HandleNotFound { .. } => {
            (StatusCode::NOT_FOUND, "handle_not_found", err.to_string())
        }
        HandleRegistryError::HandleExpired { .. } => {
            (StatusCode::NOT_FOUND, "handle_expired", err.to_string())
        }
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "handle_registry_error",
            err.to_string(),
        ),
    }
}

impl PaymentView {
    fn from_transaction(
        tx: &Transaction,
        perspective: Option<&[u8; 32]>,
        status: PaymentStatus,
    ) -> Self {
        let hash = hex_encode(tx.hash());
        let from = encode_address(&tx.from);
        let to = encode_address(&tx.to);
        let memo = tx.topics.first().cloned();
        let direction = perspective
            .map(|addr| PaymentDirection::from_perspective(tx, addr))
            .unwrap_or(PaymentDirection::Outgoing);
        let fee_required = FeePolicy::default().required_fee(tx);
        let fee_atomic = format_fee(fee_required);
        let fee_required_u128 = fee_required as u128;
        let total_cost_atomic = match direction {
            PaymentDirection::Outgoing | PaymentDirection::SelfTransfer => Some(format_atomic(
                tx.amount.atomic().saturating_add(fee_required_u128),
            )),
            PaymentDirection::Incoming => None,
        };

        Self {
            hash,
            from,
            to,
            direction,
            amount_atomic: format_atomic(tx.amount.atomic()),
            fee_atomic,
            total_cost_atomic,
            nonce: tx.nonce,
            timestamp: tx.timestamp.0,
            hash_timer: tx.hashtimer.to_hex(),
            memo,
            status,
        }
    }
}

impl PaymentDirection {
    fn from_perspective(tx: &Transaction, perspective: &[u8; 32]) -> Self {
        if tx.from == tx.to {
            PaymentDirection::SelfTransfer
        } else if tx.from == *perspective {
            PaymentDirection::Outgoing
        } else if tx.to == *perspective {
            PaymentDirection::Incoming
        } else {
            PaymentDirection::Outgoing
        }
    }
}

/// Optional filter parameters for L2 endpoints.
#[derive(Debug, Default, Deserialize)]
struct L2Filter {
    #[serde(default)]
    l2_id: Option<String>,
}

/// Start the RPC server
pub async fn start_server(state: AppState, addr: &str) -> Result<()> {
    info!("Starting RPC server on {}", addr);
    let shared = Arc::new(state);
    let app = build_router(shared.clone());
    let listener = bind_listener(addr).await?;
    let bound_addr = listener.local_addr()?;
    info!("RPC server listening on {}", bound_addr);
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .context("RPC server terminated unexpectedly")
}

/// Bind to TCP listener
async fn bind_listener(addr: &str) -> Result<tokio::net::TcpListener> {
    let socket_addr: SocketAddr = addr.parse()?;
    tokio::net::TcpListener::bind(socket_addr)
        .await
        .with_context(|| format!("failed to bind RPC listener on {socket_addr}"))
}

/// Build router and endpoints
fn build_router(state: Arc<AppState>) -> Router {
    let max_body_bytes = configured_body_limit(&state);
    let request_timeout = configured_request_timeout(&state);
    let global_rps = configured_global_rps(&state);
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    let rate_limiter = RateLimiterLayer::new(global_rps, Duration::from_secs(1));
    let circuit_breaker = CircuitBreakerLayer::new(
        CIRCUIT_BREAKER_FAILURE_THRESHOLD,
        Duration::from_secs(CIRCUIT_BREAKER_OPEN_SECS),
    );

    let mut router = Router::new()
        .route("/health", get(handle_get_health))
        .route("/status", get(handle_status))
        .route("/time", get(handle_time))
        .route("/version", get(handle_version))
        .route("/metrics", get(handle_metrics))
        .route("/ai/status", get(handle_get_ai_status))
        .route("/tx", post(handle_submit_tx))
        .route("/tx/payment", post(handle_payment_tx))
        .route("/tx/:hash", get(handle_get_transaction))
        .route(HANDLE_REGISTER_ENDPOINT, post(handle_register_handle))
        .route(HANDLE_LOOKUP_ENDPOINT, get(handle_get_handle))
        .route("/files/publish", post(handle_publish_file))
        .route("/files/:id", get(handle_get_file))
        .route("/block/:id", get(handle_get_block))
        .route("/account/:address", get(handle_get_account))
        .route(
            "/account/:address/payments",
            get(handle_get_account_payments),
        )
        .route("/peers", get(handle_get_peers))
        .route("/p2p/peers", get(handle_get_p2p_peers))
        .route("/p2p/blocks", post(handle_p2p_blocks))
        .route("/p2p/transactions", post(handle_p2p_transactions))
        .route("/p2p/peer-info", post(handle_p2p_peer_info))
        .route("/p2p/peer-discovery", post(handle_p2p_peer_discovery))
        .route("/p2p/block-request", post(handle_p2p_block_request))
        .route("/p2p/block-response", post(handle_p2p_block_response))
        .route("/l2/config", get(handle_get_l2_config))
        .route("/l2/networks", get(handle_list_l2_networks))
        .route("/l2/commits", get(handle_list_l2_commits))
        .route("/l2/exits", get(handle_list_l2_exits));

    if state.dev_mode {
        router = router.route("/dev/fund", post(handle_dev_fund));
    }

    if let Some(static_root) = &state.unified_ui_dist {
        if Path::new(static_root).exists() {
            info!("Serving Unified UI from {:?}", static_root);
            let file_service =
                ServeDir::new(static_root).not_found_service(tower::service_fn(|_req| async {
                    Ok::<_, Infallible>(handle_not_found().await.into_response())
                }));
            router = router.fallback_service(file_service);
        } else {
            warn!("Static UI directory {:?} not found", static_root);
            router = router.fallback(handle_not_found);
        }
    } else {
        router = router.fallback(handle_not_found);
    }

    let middleware_stack = ServiceBuilder::new()
        .layer(HandleErrorLayer::new(handle_service_error))
        .layer(ConcurrencyLimitLayer::new(MAX_CONCURRENT_REQUESTS))
        .layer(TimeoutLayer::new(request_timeout))
        .layer(RequestBodyLimitLayer::new(max_body_bytes));

    router
        .layer(middleware_stack)
        .layer(cors)
        .layer(rate_limiter)
        .layer(circuit_breaker)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

fn configured_body_limit(state: &Arc<AppState>) -> usize {
    state
        .security
        .as_ref()
        .map(|manager| manager.max_request_size().max(1))
        .unwrap_or(MAX_BODY_BYTES)
}

fn configured_request_timeout(state: &Arc<AppState>) -> Duration {
    state
        .security
        .as_ref()
        .map(|manager| manager.request_timeout())
        .unwrap_or_else(|| Duration::from_secs(REQUEST_TIMEOUT_SECS))
}

fn configured_global_rps(state: &Arc<AppState>) -> u64 {
    state
        .security
        .as_ref()
        .map(|manager| manager.global_rps())
        .unwrap_or(RATE_LIMIT_PER_SECOND)
}

async fn guard_request(
    state: &Arc<AppState>,
    addr: &SocketAddr,
    endpoint: &str,
) -> Result<(), SecurityError> {
    if let Some(security) = &state.security {
        security.check_request(addr.ip(), endpoint).await?;
    }

    Ok(())
}

async fn record_security_success(state: &Arc<AppState>, addr: &SocketAddr, endpoint: &str) {
    if let Some(security) = &state.security {
        if let Err(err) = security.record_success(addr.ip(), endpoint).await {
            warn!(
                "Failed to record security success for {} from {}: {}",
                endpoint, addr, err
            );
        }
    }
}

async fn record_security_failure(
    state: &Arc<AppState>,
    addr: &SocketAddr,
    endpoint: &str,
    reason: &str,
) {
    if let Some(security) = &state.security {
        if let Err(err) = security
            .record_failed_attempt(addr.ip(), endpoint, reason)
            .await
        {
            warn!(
                "Failed to record security failed attempt for {} from {}: {}",
                endpoint, addr, err
            );
        }

        if let Err(err) = security.record_failure(addr.ip(), endpoint, reason).await {
            warn!(
                "Failed to record security failure for {} from {}: {}",
                endpoint, addr, err
            );
        }
    }
}

fn map_security_error(err: &SecurityError) -> (StatusCode, &'static str) {
    match err {
        SecurityError::IpBlocked => (StatusCode::FORBIDDEN, "IP address blocked"),
        SecurityError::IpNotWhitelisted => (StatusCode::FORBIDDEN, "IP address not permitted"),
        SecurityError::RateLimitExceeded => (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded"),
        SecurityError::CircuitBreakerOpen => (
            StatusCode::SERVICE_UNAVAILABLE,
            "Service temporarily unavailable",
        ),
        SecurityError::ValidationFailed(_) => (StatusCode::BAD_REQUEST, "Invalid request payload"),
        SecurityError::AuditFailed(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, "Security audit failure")
        }
    }
}

async fn deny_request(
    state: &Arc<AppState>,
    addr: &SocketAddr,
    endpoint: &str,
    err: SecurityError,
) -> (StatusCode, &'static str) {
    let reason = err.to_string();
    warn!(
        "Security rejected request {} from {}: {}",
        endpoint, addr, reason
    );
    record_security_failure(state, addr, endpoint, &reason).await;
    map_security_error(&err)
}

// -----------------------------------------------------------------------------
// Handlers
// -----------------------------------------------------------------------------

async fn build_health_snapshot(state: &Arc<AppState>) -> HealthStatus {
    let peer_count = state.peer_count.load(Ordering::Relaxed) as u64;
    let mempool_size = state.mempool.size() as u64;
    let uptime_seconds = state.start_time.elapsed().as_secs();
    let requests_served = state.req_count.load(Ordering::Relaxed) as u64;

    let ai_enabled = if let Some(handle) = &state.ai_status {
        handle.snapshot().await.enabled
    } else {
        false
    };

    let (consensus_healthy, last_consensus_round) = if let Some(consensus) = &state.consensus {
        match consensus.snapshot().await {
            Ok(view) => (true, Some(view.round)),
            Err(err) => {
                warn!("Failed to snapshot consensus state for /health: {}", err);
                (false, None)
            }
        }
    } else {
        (false, None)
    };

    let (mut storage_healthy, last_finalized_round) =
        match state.storage.get_latest_round_finalization() {
            Ok(record) => (true, record.map(|entry| entry.round)),
            Err(err) => {
                warn!("Failed to read latest finalization for /health: {}", err);
                (false, None)
            }
        };

    if let Err(err) = state.storage.get_latest_height() {
        warn!("Failed to read latest height for /health: {}", err);
        storage_healthy = false;
    }

    let context = NodeHealthContext {
        consensus_mode: state.consensus_mode.clone(),
        consensus_healthy,
        ai_enabled,
        dht_file_mode: state.dht_file_mode.clone(),
        dht_handle_mode: state.dht_handle_mode.clone(),
        dht_healthy: state.file_dht.is_some() || state.handle_dht.is_some(),
        rpc_healthy: true,
        storage_healthy,
        last_finalized_round,
        last_consensus_round,
        peer_count,
        mempool_size,
        uptime_seconds,
        requests_served,
        node_id: state.node_id.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        dev_mode: state.dev_mode,
    };

    NodeHealth::snapshot(context)
}

async fn handle_get_health(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<Json<HealthStatus>, (StatusCode, &'static str)> {
    const ENDPOINT: &str = "/health";
    if let Err(err) = guard_request(&state, &addr, ENDPOINT).await {
        return Err(deny_request(&state, &addr, ENDPOINT, err).await);
    }

    let snapshot = build_health_snapshot(&state).await;
    record_security_success(&state, &addr, ENDPOINT).await;
    Ok(Json(snapshot))
}

async fn handle_status(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<Json<serde_json::Value>, (StatusCode, &'static str)> {
    const ENDPOINT: &str = "/status";
    if let Err(err) = guard_request(&state, &addr, ENDPOINT).await {
        return Err(deny_request(&state, &addr, ENDPOINT, err).await);
    }

    let uptime_seconds = state.start_time.elapsed().as_secs();
    let peer_count = state.peer_count.load(Ordering::Relaxed);
    let requests_served = state.req_count.load(Ordering::Relaxed);
    let mempool_size = state.mempool.size();

    let consensus_view = if let Some(consensus) = &state.consensus {
        match consensus.snapshot().await {
            Ok(view) => Some(serde_json::json!({
                "round": view.round,
                "validator_count": view.validators.len(),
                "validators": view.validators,
            })),
            Err(err) => {
                warn!("Failed to snapshot consensus state: {}", err);
                None
            }
        }
    } else {
        None
    };

    record_security_success(&state, &addr, ENDPOINT).await;

    Ok(Json(serde_json::json!({
        "status": "ok",
        "node_id": state.node_id.clone(),
        "version": env!("CARGO_PKG_VERSION"),
        "peer_count": peer_count,
        "uptime_seconds": uptime_seconds,
        "requests_served": requests_served,
        "network_active": state.p2p_network.is_some(),
        "consensus": consensus_view,
        "mempool_size": mempool_size
    })))
}

async fn handle_time(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<Json<serde_json::Value>, (StatusCode, &'static str)> {
    const ENDPOINT: &str = "/time";
    if let Err(err) = guard_request(&state, &addr, ENDPOINT).await {
        return Err(deny_request(&state, &addr, ENDPOINT, err).await);
    }

    let now = ippan_time_now();
    record_security_success(&state, &addr, ENDPOINT).await;
    Ok(Json(
        serde_json::json!({ "timestamp": now, "time_us": now }),
    ))
}

async fn handle_version(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<Json<serde_json::Value>, (StatusCode, &'static str)> {
    const ENDPOINT: &str = "/version";
    if let Err(err) = guard_request(&state, &addr, ENDPOINT).await {
        return Err(deny_request(&state, &addr, ENDPOINT, err).await);
    }

    record_security_success(&state, &addr, ENDPOINT).await;
    Ok(Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "commit": git_commit_hash(),
        "mode": rpc_consensus_mode(&state.consensus_mode)
    })))
}

/// Optional Git commit hash set by CI (GIT_COMMIT_HASH env var) during builds.
fn git_commit_hash() -> &'static str {
    option_env!("GIT_COMMIT_HASH").unwrap_or("unknown")
}

fn rpc_consensus_mode(mode: &str) -> &'static str {
    if mode.eq_ignore_ascii_case("DLC") {
        "DLC"
    } else {
        "PoA"
    }
}

async fn handle_metrics(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<Response, (StatusCode, &'static str)> {
    const ENDPOINT: &str = "/metrics";
    if let Err(err) = guard_request(&state, &addr, ENDPOINT).await {
        return Err(deny_request(&state, &addr, ENDPOINT, err).await);
    }

    if let Some(handle) = &state.metrics {
        let mut response = Response::new(Body::from(handle.render()));
        *response.status_mut() = StatusCode::OK;
        response.headers_mut().insert(
            CONTENT_TYPE,
            HeaderValue::from_static("text/plain; version=0.0.4"),
        );
        record_security_success(&state, &addr, ENDPOINT).await;
        Ok(response)
    } else {
        let mut response = Response::new(Body::from("Prometheus metrics disabled"));
        *response.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
        record_security_success(&state, &addr, ENDPOINT).await;
        Ok(response)
    }
}

async fn handle_get_ai_status(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<Json<AiStatus>, (StatusCode, &'static str)> {
    const ENDPOINT: &str = "/ai/status";
    if let Err(err) = guard_request(&state, &addr, ENDPOINT).await {
        return Err(deny_request(&state, &addr, ENDPOINT, err).await);
    }

    let mut response = if let Some(handle) = &state.ai_status {
        let snapshot = handle.snapshot().await;
        AiStatus::from(snapshot)
    } else {
        AiStatus::disabled()
    };
    response.consensus_mode = Some(state.consensus_mode.clone());
    record_security_success(&state, &addr, ENDPOINT).await;
    Ok(Json(response))
}

async fn handle_submit_tx(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    ValidatedJson(tx): ValidatedJson<Transaction>,
) -> Result<&'static str, (StatusCode, Json<ApiError>)> {
    if let Err(err) = guard_request(&state, &addr, "/tx").await {
        let (status, message) = deny_request(&state, &addr, "/tx", err).await;
        return Err((status, Json(ApiError::new("security_error", message))));
    }

    if let Some(consensus) = &state.consensus {
        if let Err(e) = consensus.submit_transaction(tx.clone()) {
            warn!("Failed to enqueue transaction: {}", e);
            record_security_failure(&state, &addr, "/tx", &e.to_string()).await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("submission_failed", "Failed to submit tx")),
            ));
        }
        record_security_success(&state, &addr, "/tx").await;
        Ok("Transaction accepted")
    } else {
        record_security_failure(&state, &addr, "/tx", "Consensus not active").await;
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiError::new(
                "consensus_unavailable",
                "Consensus not active",
            )),
        ))
    }
}

async fn handle_register_handle(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    ValidatedJson(request): ValidatedJson<HandleRegisterRequest>,
) -> Result<Json<HandleRegisterResponse>, (StatusCode, Json<ApiError>)> {
    if let Err(err) = guard_request(&state, &addr, HANDLE_REGISTER_ENDPOINT).await {
        let (status, message) = deny_request(&state, &addr, HANDLE_REGISTER_ENDPOINT, err).await;
        return Err((status, Json(ApiError::new("security_error", message))));
    }

    let built = match build_handle_registration_transaction(&state, request) {
        Ok(built) => built,
        Err(err) => {
            return Err(handle_error_response(&state, &addr, HANDLE_REGISTER_ENDPOINT, err).await)
        }
    };

    let response = handle_registration_response_from_built(&built);
    let tx_for_consensus = built.transaction.clone();

    if let Some(consensus) = &state.consensus {
        if let Err(err) = consensus.submit_transaction(tx_for_consensus) {
            return Err(handle_error_response(
                &state,
                &addr,
                HANDLE_REGISTER_ENDPOINT,
                HandleRegistrationError::SubmissionFailed(err.to_string()),
            )
            .await);
        }
    } else {
        return Err(handle_error_response(
            &state,
            &addr,
            HANDLE_REGISTER_ENDPOINT,
            HandleRegistrationError::ConsensusUnavailable,
        )
        .await);
    }

    record_security_success(&state, &addr, HANDLE_REGISTER_ENDPOINT).await;
    Ok(Json(response))
}

async fn handle_get_handle(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    AxumPath(raw_handle): AxumPath<String>,
) -> Result<Json<HandleInfoResponse>, (StatusCode, Json<ApiError>)> {
    if let Err(err) = guard_request(&state, &addr, HANDLE_LOOKUP_ENDPOINT).await {
        let (status, message) = deny_request(&state, &addr, HANDLE_LOOKUP_ENDPOINT, err).await;
        return Err((status, Json(ApiError::new("security_error", message))));
    }

    let handle = match normalize_handle_query(&raw_handle) {
        Ok(handle) => handle,
        Err(err) => {
            return Err(handle_error_response(&state, &addr, HANDLE_LOOKUP_ENDPOINT, err).await)
        }
    };

    match state.handle_registry.get_metadata(&handle) {
        Ok(metadata) => {
            let response = handle_info_from_metadata(&handle, metadata);
            record_security_success(&state, &addr, HANDLE_LOOKUP_ENDPOINT).await;
            Ok(Json(response))
        }
        Err(err) => {
            let (status, code, message) = map_handle_lookup_error(err);
            if status.is_server_error() {
                record_security_failure(&state, &addr, HANDLE_LOOKUP_ENDPOINT, &message).await;
            } else {
                record_security_success(&state, &addr, HANDLE_LOOKUP_ENDPOINT).await;
            }
            Err((status, Json(ApiError::new(code, message))))
        }
    }
}

async fn handle_payment_tx(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    ValidatedJson(request): ValidatedJson<PaymentRequest>,
) -> Result<Json<PaymentResponse>, (StatusCode, Json<ApiError>)> {
    if let Err(err) = guard_request(&state, &addr, PAYMENT_ENDPOINT).await {
        let (status, message) = deny_request(&state, &addr, PAYMENT_ENDPOINT, err).await;
        return Err((status, Json(ApiError::new("security_error", message))));
    }

    let built = match build_payment_transaction(&state, request) {
        Ok(tx) => tx,
        Err(err) => return Err(payment_error_response(&state, &addr, err).await),
    };

    let response = payment_response_from_built(&built);
    let tx_for_consensus = built.transaction.clone();

    if let Some(consensus) = &state.consensus {
        if let Err(err) = consensus.submit_transaction(tx_for_consensus) {
            return Err(payment_error_response(
                &state,
                &addr,
                PaymentError::SubmissionFailed(err.to_string()),
            )
            .await);
        }
    } else {
        return Err(
            payment_error_response(&state, &addr, PaymentError::ConsensusUnavailable).await,
        );
    }

    record_security_success(&state, &addr, PAYMENT_ENDPOINT).await;
    Ok(Json(response))
}

async fn handle_dev_fund(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    ValidatedJson(request): ValidatedJson<DevFundRequest>,
) -> Result<Json<DevFundResponse>, (StatusCode, Json<ApiError>)> {
    const ENDPOINT: &str = "/dev/fund";

    if let Err(err) = guard_request(&state, &addr, ENDPOINT).await {
        let (status, message) = deny_request(&state, &addr, ENDPOINT, err).await;
        return Err((status, Json(ApiError::new("security_error", message))));
    }

    if !state.dev_mode {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ApiError::new(
                "dev_mode_disabled",
                "/dev/fund is only available when IPPAN_DEV_MODE=true or --dev is set",
            )),
        ));
    }

    if !addr.ip().is_loopback() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ApiError::new(
                "dev_only",
                "/dev/fund only accepts requests from 127.0.0.1/::1",
            )),
        ));
    }

    let address_bytes = match decode_any_address(&request.address) {
        Ok(bytes) => bytes,
        Err(err) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiError::new("invalid_address", err)),
            ))
        }
    };

    let (mut account, existed) = match state.storage.get_account(&address_bytes) {
        Ok(Some(acc)) => (acc, true),
        Ok(None) => (
            Account {
                address: address_bytes,
                balance: 0,
                nonce: 0,
            },
            false,
        ),
        Err(err) => {
            error!("Failed to load account for /dev/fund: {}", err);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("storage_error", err.to_string())),
            ));
        }
    };

    account.balance = request.amount;
    if let Some(nonce) = request.nonce {
        account.nonce = nonce;
    }

    if let Err(err) = state.storage.update_account(account.clone()) {
        error!("Failed to update account via /dev/fund: {}", err);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::new("storage_error", err.to_string())),
        ));
    }

    record_security_success(&state, &addr, ENDPOINT).await;
    info!(
        "Dev funded account {} with balance {} (nonce {})",
        hex::encode(account.address),
        account.balance,
        account.nonce
    );

    Ok(Json(DevFundResponse {
        address_hex: hex::encode(account.address),
        address_base58: encode_address(&account.address),
        balance: account.balance,
        nonce: account.nonce,
        created: !existed,
    }))
}

fn build_payment_transaction(
    state: &Arc<AppState>,
    request: PaymentRequest,
) -> Result<BuiltPayment, PaymentError> {
    let from_bytes = decode_address(&request.from)
        .map_err(|err| PaymentError::InvalidAddress(format!("from: {err}")))?;
    let to_bytes = decode_address(&request.to)
        .map_err(|err| PaymentError::InvalidAddress(format!("to: {err}")))?;

    if request.amount == 0 {
        return Err(PaymentError::ZeroAmount);
    }

    let memo = normalize_memo(request.memo)?;
    let amount_atomic = request.amount;
    let amount = Amount::from_atomic(amount_atomic);
    let signing_key = request
        .signing_key
        .ok_or(PaymentError::MissingSigningKey)
        .and_then(|key| parse_signing_key_hex(&key))?;

    let nonce = if let Some(nonce) = request.nonce {
        nonce
    } else {
        derive_next_nonce(state, &from_bytes)?
    };

    let mut tx = Transaction::new(from_bytes, to_bytes, amount, nonce);
    if let Some(memo_value) = memo.clone() {
        tx.set_topics(vec![memo_value.clone()]);
    }

    tx.sign(&signing_key).map_err(PaymentError::SigningFailed)?;

    let fee_policy = FeePolicy::default();
    let fee_atomic = fee_policy.required_fee(&tx);
    if let Err(err) = fee_policy.enforce_fee_limit(request.fee, fee_atomic) {
        if let ippan_l1_fees::FeePolicyError::FeeBelowMinimum { required, provided } = err {
            return Err(PaymentError::FeeTooLow { required, provided });
        } else {
            return Err(PaymentError::SubmissionFailed(err.to_string()));
        }
    }

    Ok(BuiltPayment {
        transaction: tx,
        amount_atomic,
        fee_atomic,
        memo,
        from: from_bytes,
        to: to_bytes,
    })
}

fn payment_response_from_built(built: &BuiltPayment) -> PaymentResponse {
    PaymentResponse {
        tx_hash: hex::encode(built.transaction.hash()),
        status: PaymentStatus::AcceptedToMempool,
        from: encode_address(&built.from),
        to: encode_address(&built.to),
        nonce: built.transaction.nonce,
        amount_atomic: format_atomic(built.amount_atomic),
        fee_atomic: format_fee(built.fee_atomic),
        timestamp: built.transaction.timestamp.0,
        hash_timer: built.transaction.hashtimer.to_hex(),
        memo: built.memo.clone(),
    }
}

fn derive_next_nonce(state: &Arc<AppState>, address: &[u8; 32]) -> Result<u64, PaymentError> {
    match state.storage.get_account(address) {
        Ok(Some(account)) => Ok(account.nonce.saturating_add(1)),
        Ok(None) => Err(PaymentError::AccountNotFound),
        Err(err) => Err(PaymentError::NonceLookupFailed(err.to_string())),
    }
}

async fn payment_error_response(
    state: &Arc<AppState>,
    addr: &SocketAddr,
    err: PaymentError,
) -> (StatusCode, Json<ApiError>) {
    let (status, code) = err.status_and_code();
    let message = err.to_string();
    record_security_failure(state, addr, PAYMENT_ENDPOINT, &message).await;
    (status, Json(ApiError::new(code, message)))
}

async fn handle_get_transaction(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    AxumPath(hash): AxumPath<String>,
) -> Result<Json<TransactionView>, (StatusCode, &'static str)> {
    if let Err(err) = guard_request(&state, &addr, "/tx/:hash").await {
        return Err(deny_request(&state, &addr, "/tx/:hash", err).await);
    }

    let hash_bytes = match parse_hex_32(&hash) {
        Ok(bytes) => bytes,
        Err(err) => {
            warn!("Invalid transaction hash from {}: {} ({})", addr, hash, err);
            record_security_failure(&state, &addr, "/tx/:hash", "Invalid transaction hash").await;
            return Err((StatusCode::BAD_REQUEST, "Invalid transaction hash"));
        }
    };

    match state.storage.get_transaction(&hash_bytes) {
        Ok(Some(tx)) => {
            let envelope = TransactionView::from_transaction(&tx, TransactionStatus::Finalized);
            record_security_success(&state, &addr, "/tx/:hash").await;
            Ok(Json(envelope))
        }
        Ok(None) => {
            record_security_success(&state, &addr, "/tx/:hash").await;
            Err((StatusCode::NOT_FOUND, "Transaction not found"))
        }
        Err(err) => {
            error!("Failed fetching transaction {} for {}: {}", hash, addr, err);
            record_security_failure(&state, &addr, "/tx/:hash", &err.to_string()).await;
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to load transaction",
            ))
        }
    }
}

async fn handle_get_block(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<BlockResponse>, (StatusCode, &'static str)> {
    if let Err(err) = guard_request(&state, &addr, "/block/:id").await {
        return Err(deny_request(&state, &addr, "/block/:id", err).await);
    }

    let identifier = match parse_block_identifier(&id) {
        Some(identifier) => identifier,
        None => {
            warn!("Invalid block identifier from {}: {}", addr, id);
            record_security_failure(&state, &addr, "/block/:id", "Invalid block identifier").await;
            return Err((StatusCode::BAD_REQUEST, "Invalid block identifier"));
        }
    };

    let mut height_hint = None;
    let block_result = match identifier {
        BlockIdentifier::Hash(hash) => state.storage.get_block(&hash),
        BlockIdentifier::Height(height) => {
            height_hint = Some(height);
            state.storage.get_block_by_height(height)
        }
    };

    match block_result {
        Ok(Some(block)) => {
            record_security_success(&state, &addr, "/block/:id").await;
            let response = block_response_with_fee_summary(&state.storage, block, height_hint);
            Ok(Json(response))
        }
        Ok(None) => {
            record_security_success(&state, &addr, "/block/:id").await;
            Err((StatusCode::NOT_FOUND, "Block not found"))
        }
        Err(err) => {
            error!("Failed fetching block {} for {}: {}", id, addr, err);
            record_security_failure(&state, &addr, "/block/:id", &err.to_string()).await;
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to load block"))
        }
    }
}

async fn handle_get_account(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    AxumPath(address): AxumPath<String>,
) -> Result<Json<AccountResponse>, (StatusCode, &'static str)> {
    if let Err(err) = guard_request(&state, &addr, "/account/:address").await {
        return Err(deny_request(&state, &addr, "/account/:address", err).await);
    }

    let address_bytes = match parse_hex_32(&address) {
        Ok(bytes) => bytes,
        Err(err) => {
            warn!(
                "Invalid account address from {}: {} ({})",
                addr, address, err
            );
            record_security_failure(
                &state,
                &addr,
                "/account/:address",
                "Invalid account address",
            )
            .await;
            return Err((StatusCode::BAD_REQUEST, "Invalid account address"));
        }
    };

    match state.storage.get_account(&address_bytes) {
        Ok(Some(account)) => match state.storage.get_transactions_by_address(&address_bytes) {
            Ok(transactions) => {
                let response = account_to_response(account, transactions);
                record_security_success(&state, &addr, "/account/:address").await;
                Ok(Json(response))
            }
            Err(err) => {
                error!(
                    "Failed fetching transactions for account {} ({}): {}",
                    address, addr, err
                );
                record_security_failure(&state, &addr, "/account/:address", &err.to_string()).await;
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to load account transactions",
                ))
            }
        },
        Ok(None) => {
            record_security_success(&state, &addr, "/account/:address").await;
            Err((StatusCode::NOT_FOUND, "Account not found"))
        }
        Err(err) => {
            error!("Failed fetching account {} for {}: {}", address, addr, err);
            record_security_failure(&state, &addr, "/account/:address", &err.to_string()).await;
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to load account"))
        }
    }
}

async fn handle_get_account_payments(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    AxumPath(address): AxumPath<String>,
    Query(query): Query<PaymentHistoryQuery>,
) -> Result<Json<Vec<PaymentView>>, (StatusCode, &'static str)> {
    const ENDPOINT: &str = "/account/:address/payments";

    if let Err(err) = guard_request(&state, &addr, ENDPOINT).await {
        return Err(deny_request(&state, &addr, ENDPOINT, err).await);
    }

    let address_bytes = match parse_hex_32(&address) {
        Ok(bytes) => bytes,
        Err(err) => {
            warn!(
                "Invalid account address for payments from {}: {} ({})",
                addr, address, err
            );
            record_security_failure(&state, &addr, ENDPOINT, "Invalid account address").await;
            return Err((StatusCode::BAD_REQUEST, "Invalid account address"));
        }
    };

    let limit = clamp_history_limit(query.limit);

    match state.storage.get_transactions_by_address(&address_bytes) {
        Ok(mut transactions) => {
            transactions.sort_by(|a, b| b.timestamp.0.cmp(&a.timestamp.0));
            transactions.truncate(limit);
            let views = transactions
                .iter()
                .map(|tx| {
                    PaymentView::from_transaction(
                        tx,
                        Some(&address_bytes),
                        PaymentStatus::Finalized,
                    )
                })
                .collect();
            record_security_success(&state, &addr, ENDPOINT).await;
            Ok(Json(views))
        }
        Err(err) => {
            error!(
                "Failed fetching transactions for account payments {} ({}): {}",
                address, addr, err
            );
            record_security_failure(&state, &addr, ENDPOINT, &err.to_string()).await;
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to load account transactions",
            ))
        }
    }
}

async fn handle_get_peers(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<Json<Vec<String>>, (StatusCode, &'static str)> {
    serve_peer_listing(state, addr, "/peers").await
}

async fn handle_get_p2p_peers(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<Json<Vec<String>>, (StatusCode, &'static str)> {
    serve_peer_listing(state, addr, "/p2p/peers").await
}

async fn serve_peer_listing(
    state: Arc<AppState>,
    addr: SocketAddr,
    endpoint: &'static str,
) -> Result<Json<Vec<String>>, (StatusCode, &'static str)> {
    if let Err(err) = guard_request(&state, &addr, endpoint).await {
        return Err(deny_request(&state, &addr, endpoint, err).await);
    }

    let peers = if let Some(net) = &state.p2p_network {
        net.get_peers()
    } else {
        vec![]
    };

    record_security_success(&state, &addr, endpoint).await;
    Ok(Json(peers))
}

// -----------------------------------------------------------------------------
// P2P Handlers
// -----------------------------------------------------------------------------

async fn handle_p2p_blocks(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    ValidatedJson(message): ValidatedJson<NetworkMessage>,
) -> Result<&'static str, (StatusCode, Json<ApiError>)> {
    if let Err(err) = guard_request(&state, &addr, "/p2p/blocks").await {
        let (status, message) = deny_request(&state, &addr, "/p2p/blocks", err).await;
        return Err((status, Json(ApiError::new("security_error", message))));
    }

    let from = resolve_peer_address(&state, &addr, &message);
    forward_to_network(&state, &from, message.clone()).await;

    match message {
        NetworkMessage::Block(block) => match ingest_block_from_peer(&state, &block) {
            Ok(()) => {
                record_security_success(&state, &addr, "/p2p/blocks").await;
                Ok("Block accepted")
            }
            Err(err) => {
                error!("Failed to persist block from {}: {}", from, err);
                record_security_failure(&state, &addr, "/p2p/blocks", &err.to_string()).await;
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiError::new(
                        "block_persist_failed",
                        "Failed to persist block",
                    )),
                ))
            }
        },
        other => {
            warn!(
                "Unexpected payload on /p2p/blocks from {}: {:?}",
                from, other
            );
            let reason = format!("Unexpected payload: {:?}", other);
            record_security_failure(&state, &addr, "/p2p/blocks", &reason).await;
            Err((
                StatusCode::BAD_REQUEST,
                Json(ApiError::new("invalid_message", "Expected block message")),
            ))
        }
    }
}

async fn handle_p2p_block_response(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    ValidatedJson(message): ValidatedJson<NetworkMessage>,
) -> Result<&'static str, (StatusCode, Json<ApiError>)> {
    if let Err(err) = guard_request(&state, &addr, "/p2p/block-response").await {
        let (status, message) = deny_request(&state, &addr, "/p2p/block-response", err).await;
        return Err((status, Json(ApiError::new("security_error", message))));
    }

    let from = resolve_peer_address(&state, &addr, &message);
    forward_to_network(&state, &from, message.clone()).await;

    match message {
        NetworkMessage::BlockResponse(block) => match ingest_block_from_peer(&state, &block) {
            Ok(()) => {
                record_security_success(&state, &addr, "/p2p/block-response").await;
                Ok("Block response accepted")
            }
            Err(err) => {
                error!("Failed to handle block response from {}: {}", from, err);
                record_security_failure(&state, &addr, "/p2p/block-response", &err.to_string())
                    .await;
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiError::new(
                        "block_response_failed",
                        "Failed to handle block response",
                    )),
                ))
            }
        },
        other => {
            warn!(
                "Unexpected payload on /p2p/block-response from {}: {:?}",
                from, other
            );
            let reason = format!("Unexpected payload: {:?}", other);
            record_security_failure(&state, &addr, "/p2p/block-response", &reason).await;
            Err((
                StatusCode::BAD_REQUEST,
                Json(ApiError::new(
                    "invalid_message",
                    "Expected block response message",
                )),
            ))
        }
    }
}

async fn handle_p2p_transactions(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    ValidatedJson(message): ValidatedJson<NetworkMessage>,
) -> Result<&'static str, (StatusCode, Json<ApiError>)> {
    if let Err(err) = guard_request(&state, &addr, "/p2p/transactions").await {
        let (status, message) = deny_request(&state, &addr, "/p2p/transactions", err).await;
        return Err((status, Json(ApiError::new("security_error", message))));
    }

    let from = resolve_peer_address(&state, &addr, &message);
    forward_to_network(&state, &from, message.clone()).await;

    match message {
        NetworkMessage::Transaction(tx) => match ingest_transaction_from_peer(&state, &tx) {
            Ok(()) => {
                record_security_success(&state, &addr, "/p2p/transactions").await;
                Ok("Transaction accepted")
            }
            Err(err) => {
                error!("Failed to ingest transaction from {}: {}", from, err);
                record_security_failure(&state, &addr, "/p2p/transactions", &err.to_string()).await;
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiError::new(
                        "transaction_ingest_failed",
                        "Failed to ingest transaction",
                    )),
                ))
            }
        },
        other => {
            warn!(
                "Unexpected payload on /p2p/transactions from {}: {:?}",
                from, other
            );
            let reason = format!("Unexpected payload: {:?}", other);
            record_security_failure(&state, &addr, "/p2p/transactions", &reason).await;
            Err((
                StatusCode::BAD_REQUEST,
                Json(ApiError::new(
                    "invalid_message",
                    "Expected transaction message",
                )),
            ))
        }
    }
}

async fn handle_p2p_peer_info(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    ValidatedJson(message): ValidatedJson<NetworkMessage>,
) -> Result<&'static str, (StatusCode, Json<ApiError>)> {
    if let Err(err) = guard_request(&state, &addr, "/p2p/peer-info").await {
        let (status, message) = deny_request(&state, &addr, "/p2p/peer-info", err).await;
        return Err((status, Json(ApiError::new("security_error", message))));
    }

    let from = resolve_peer_address(&state, &addr, &message);
    forward_to_network(&state, &from, message.clone()).await;

    match message {
        NetworkMessage::PeerInfo { .. } => {
            record_security_success(&state, &addr, "/p2p/peer-info").await;
            Ok("Peer info accepted")
        }
        other => {
            warn!(
                "Unexpected payload on /p2p/peer-info from {}: {:?}",
                from, other
            );
            let reason = format!("Unexpected payload: {:?}", other);
            record_security_failure(&state, &addr, "/p2p/peer-info", &reason).await;
            Err((
                StatusCode::BAD_REQUEST,
                Json(ApiError::new(
                    "invalid_message",
                    "Expected peer info message",
                )),
            ))
        }
    }
}

async fn handle_p2p_peer_discovery(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    ValidatedJson(message): ValidatedJson<NetworkMessage>,
) -> Result<&'static str, (StatusCode, Json<ApiError>)> {
    if let Err(err) = guard_request(&state, &addr, "/p2p/peer-discovery").await {
        let (status, message) = deny_request(&state, &addr, "/p2p/peer-discovery", err).await;
        return Err((status, Json(ApiError::new("security_error", message))));
    }

    let from = resolve_peer_address(&state, &addr, &message);
    forward_to_network(&state, &from, message.clone()).await;

    match message {
        NetworkMessage::PeerDiscovery { .. } => {
            record_security_success(&state, &addr, "/p2p/peer-discovery").await;
            Ok("Peer discovery accepted")
        }
        other => {
            warn!(
                "Unexpected payload on /p2p/peer-discovery from {}: {:?}",
                from, other
            );
            let reason = format!("Unexpected payload: {:?}", other);
            record_security_failure(&state, &addr, "/p2p/peer-discovery", &reason).await;
            Err((
                StatusCode::BAD_REQUEST,
                Json(ApiError::new(
                    "invalid_message",
                    "Expected peer discovery message",
                )),
            ))
        }
    }
}

async fn handle_p2p_block_request(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    ValidatedJson(message): ValidatedJson<NetworkMessage>,
) -> Result<Json<NetworkMessage>, (StatusCode, Json<ApiError>)> {
    if let Err(err) = guard_request(&state, &addr, "/p2p/block-request").await {
        let (status, message) = deny_request(&state, &addr, "/p2p/block-request", err).await;
        return Err((status, Json(ApiError::new("security_error", message))));
    }

    let from = resolve_peer_address(&state, &addr, &message);
    forward_to_network(&state, &from, message.clone()).await;

    match message {
        NetworkMessage::BlockRequest { hash } => match state.storage.get_block(&hash) {
            Ok(Some(block)) => {
                record_security_success(&state, &addr, "/p2p/block-request").await;
                Ok(Json(NetworkMessage::BlockResponse(block)))
            }
            Ok(None) => {
                debug!(
                    "Block request from {} not found: {}",
                    from,
                    hex_encode(hash)
                );
                record_security_success(&state, &addr, "/p2p/block-request").await;
                Err((
                    StatusCode::NOT_FOUND,
                    Json(ApiError::new(
                        "block_not_found",
                        "Requested block not found",
                    )),
                ))
            }
            Err(err) => {
                error!("Failed to serve block request from {}: {}", from, err);
                record_security_failure(&state, &addr, "/p2p/block-request", &err.to_string())
                    .await;
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiError::new(
                        "block_request_failed",
                        "Failed to serve block request",
                    )),
                ))
            }
        },
        other => {
            warn!(
                "Unexpected payload on /p2p/block-request from {}: {:?}",
                from, other
            );
            let reason = format!("Unexpected payload: {:?}", other);
            record_security_failure(&state, &addr, "/p2p/block-request", &reason).await;
            Err((
                StatusCode::BAD_REQUEST,
                Json(ApiError::new(
                    "invalid_message",
                    "Expected block request message",
                )),
            ))
        }
    }
}

async fn forward_to_network(state: &Arc<AppState>, from: &str, message: NetworkMessage) {
    if let Some(net) = &state.p2p_network {
        if let Err(err) = net.process_incoming_message(from, message).await {
            warn!(
                "Failed to process inbound P2P message from {}: {}",
                from, err
            );
        }
    }
}

fn message_announced_address(message: &NetworkMessage) -> Option<String> {
    match message {
        NetworkMessage::PeerInfo { addresses, .. } => addresses
            .iter()
            .find(|addr| !addr.is_empty() && !addr.contains("0.0.0.0"))
            .cloned()
            .or_else(|| addresses.first().cloned()),
        _ => None,
    }
}

fn resolve_peer_address(
    state: &Arc<AppState>,
    socket: &SocketAddr,
    message: &NetworkMessage,
) -> String {
    if let Some(addr) = message_announced_address(message) {
        return addr;
    }

    if let Some(net) = &state.p2p_network {
        let host = socket.ip().to_string();
        if let Some(info) = net
            .get_peer_metadata()
            .into_iter()
            .find(|info| info.address.contains(&host))
        {
            return info.address;
        }
    }

    format!("http://{}:{}", socket.ip(), socket.port())
}

fn ingest_block_from_peer(state: &Arc<AppState>, block: &Block) -> Result<()> {
    state.storage.store_block(block.clone())?;

    for tx in &block.transactions {
        let hash_hex = hex_encode(tx.hash());
        if let Err(err) = state.mempool.remove_transaction(&hash_hex) {
            debug!(
                "Failed to prune transaction {} from mempool after block import: {}",
                hash_hex, err
            );
        }
    }

    Ok(())
}

fn ingest_transaction_from_peer(state: &Arc<AppState>, tx: &Transaction) -> Result<()> {
    state.storage.store_transaction(tx.clone())?;

    match state.mempool.add_transaction(tx.clone()) {
        Ok(true) => {}
        Ok(false) => debug!(
            "Duplicate transaction from peer ignored: {}",
            hex_encode(tx.hash())
        ),
        Err(err) => return Err(err),
    }

    if let Some(consensus) = &state.consensus {
        if let Err(err) = consensus.submit_transaction(tx.clone()) {
            warn!(
                "Consensus rejected transaction {} from peer: {}",
                hex_encode(tx.hash()),
                err
            );
        }
    }

    Ok(())
}

// -----------------------------------------------------------------------------
// L2 Endpoints
// -----------------------------------------------------------------------------

async fn handle_get_l2_config(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<Json<L2Config>, (StatusCode, &'static str)> {
    const ENDPOINT: &str = "/l2/config";
    if let Err(err) = guard_request(&state, &addr, ENDPOINT).await {
        return Err(deny_request(&state, &addr, ENDPOINT, err).await);
    }

    record_security_success(&state, &addr, ENDPOINT).await;
    Ok(Json(state.l2_config.clone()))
}

async fn handle_list_l2_networks(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<Json<Vec<L2Network>>, (StatusCode, &'static str)> {
    const ENDPOINT: &str = "/l2/networks";
    if let Err(err) = guard_request(&state, &addr, ENDPOINT).await {
        return Err(deny_request(&state, &addr, ENDPOINT, err).await);
    }

    match state.storage.list_l2_networks() {
        Ok(networks) => {
            record_security_success(&state, &addr, ENDPOINT).await;
            Ok(Json(networks))
        }
        Err(err) => {
            error!("Failed to list L2 networks: {}", err);
            record_security_failure(&state, &addr, ENDPOINT, &err.to_string()).await;
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to list L2 networks",
            ))
        }
    }
}

async fn handle_list_l2_commits(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(filter): Query<L2Filter>,
) -> Result<Json<Vec<L2Commit>>, (StatusCode, &'static str)> {
    const ENDPOINT: &str = "/l2/commits";
    if let Err(err) = guard_request(&state, &addr, ENDPOINT).await {
        return Err(deny_request(&state, &addr, ENDPOINT, err).await);
    }

    match state.storage.list_l2_commits(filter.l2_id.as_deref()) {
        Ok(commits) => {
            record_security_success(&state, &addr, ENDPOINT).await;
            Ok(Json(commits))
        }
        Err(err) => {
            error!("Failed to list L2 commits: {}", err);
            record_security_failure(&state, &addr, ENDPOINT, &err.to_string()).await;
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to list L2 commits",
            ))
        }
    }
}

async fn handle_list_l2_exits(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(filter): Query<L2Filter>,
) -> Result<Json<Vec<L2ExitRecord>>, (StatusCode, &'static str)> {
    const ENDPOINT: &str = "/l2/exits";
    if let Err(err) = guard_request(&state, &addr, ENDPOINT).await {
        return Err(deny_request(&state, &addr, ENDPOINT, err).await);
    }

    match state.storage.list_l2_exits(filter.l2_id.as_deref()) {
        Ok(exits) => {
            record_security_success(&state, &addr, ENDPOINT).await;
            Ok(Json(exits))
        }
        Err(err) => {
            error!("Failed to list L2 exits: {}", err);
            record_security_failure(&state, &addr, ENDPOINT, &err.to_string()).await;
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to list L2 exits"))
        }
    }
}

fn parse_hex_32(input: &str) -> std::result::Result<[u8; 32], hex::FromHexError> {
    let trimmed = input.trim();
    let normalized = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);
    let mut bytes = [0u8; 32];
    hex::decode_to_slice(normalized, &mut bytes)?;
    Ok(bytes)
}

fn decode_any_address(input: &str) -> Result<[u8; 32], String> {
    match decode_address(input) {
        Ok(bytes) => Ok(bytes),
        Err(primary) => parse_hex_32(input)
            .map_err(|err| format!("invalid address: {primary}; hex parse error: {err}")),
    }
}

enum BlockIdentifier {
    Hash([u8; 32]),
    Height(u64),
}

fn parse_block_identifier(input: &str) -> Option<BlockIdentifier> {
    let trimmed = input.trim();
    if trimmed.len() <= 20 && trimmed.chars().all(|c| c.is_ascii_digit()) {
        if let Ok(height) = trimmed.parse::<u64>() {
            return Some(BlockIdentifier::Height(height));
        }
    }

    parse_hex_32(trimmed).ok().map(BlockIdentifier::Hash)
}

fn account_to_response(account: Account, transactions: Vec<Transaction>) -> AccountResponse {
    let payments = build_payment_views(&transactions, &account.address);
    let recent_transactions = transactions
        .iter()
        .map(|tx| TransactionView::from_transaction(tx, TransactionStatus::Finalized))
        .collect();

    AccountResponse {
        address: hex_encode(account.address),
        balance_atomic: format_atomic(account.balance as u128),
        nonce: account.nonce,
        recent_transactions,
        recent_payments: payments,
    }
}

fn block_response_with_fee_summary(
    storage: &Arc<dyn Storage + Send + Sync>,
    block: Block,
    height_hint: Option<u64>,
) -> BlockResponse {
    let fee_summary = match storage.get_round_finalization(block.header.round) {
        Ok(Some(record)) => RoundFeeSummaryView::from_record(&record),
        _ => None,
    };
    let block_view = BlockView::from_block(&block, height_hint);
    BlockResponse {
        block: block_view,
        fee_summary,
    }
}

fn build_payment_views(transactions: &[Transaction], perspective: &[u8; 32]) -> Vec<PaymentView> {
    let mut views: Vec<_> = transactions
        .iter()
        .map(|tx| PaymentView::from_transaction(tx, Some(perspective), PaymentStatus::Finalized))
        .collect();
    views.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    views
}

#[derive(Clone)]
struct RateLimiterLayer {
    state: Arc<RateLimiterState>,
    max_requests: u64,
    window: Duration,
}

impl RateLimiterLayer {
    fn new(max_requests: u64, window: Duration) -> Self {
        Self {
            state: Arc::new(RateLimiterState::new()),
            max_requests,
            window,
        }
    }
}

impl<S> Layer<S> for RateLimiterLayer {
    type Service = RateLimiter<S>;

    fn layer(&self, service: S) -> Self::Service {
        RateLimiter {
            inner: service,
            state: Arc::clone(&self.state),
            max_requests: self.max_requests,
            window: self.window,
        }
    }
}

#[derive(Clone)]
struct RateLimiter<S> {
    inner: S,
    state: Arc<RateLimiterState>,
    max_requests: u64,
    window: Duration,
}

impl<S, Request> Service<Request> for RateLimiter<S>
where
    S: Service<Request, Response = Response, Error = Infallible> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut TaskContext<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        if !self.state.allow(self.max_requests, self.window) {
            let response = Response::builder()
                .status(StatusCode::TOO_MANY_REQUESTS)
                .body(Body::from("Rate limit exceeded"))
                .expect("failed to build rate limit response");
            return Box::pin(async move { Ok(response) });
        }

        let future = self.inner.call(request);
        Box::pin(future)
    }
}

struct RateLimiterState {
    window: std::sync::Mutex<RateWindow>,
}

impl RateLimiterState {
    fn new() -> Self {
        Self {
            window: std::sync::Mutex::new(RateWindow {
                start: Instant::now(),
                count: 0,
            }),
        }
    }

    fn allow(&self, max_requests: u64, window: Duration) -> bool {
        let now = Instant::now();
        let mut guard = self.window.lock().expect("rate limiter mutex poisoned");

        if now.duration_since(guard.start) >= window {
            guard.start = now;
            guard.count = 0;
        }

        if guard.count < max_requests {
            guard.count += 1;
            true
        } else {
            false
        }
    }
}

struct RateWindow {
    start: Instant,
    count: u64,
}

#[derive(Clone)]
struct CircuitBreakerLayer {
    state: Arc<CircuitBreakerState>,
    failure_threshold: usize,
    open_duration: Duration,
}

impl CircuitBreakerLayer {
    fn new(failure_threshold: usize, open_duration: Duration) -> Self {
        Self {
            state: Arc::new(CircuitBreakerState::new()),
            failure_threshold,
            open_duration,
        }
    }
}

impl<S> Layer<S> for CircuitBreakerLayer {
    type Service = CircuitBreaker<S>;

    fn layer(&self, service: S) -> Self::Service {
        CircuitBreaker {
            inner: service,
            state: Arc::clone(&self.state),
            failure_threshold: self.failure_threshold,
            open_duration: self.open_duration,
        }
    }
}

struct CircuitBreaker<S> {
    inner: S,
    state: Arc<CircuitBreakerState>,
    failure_threshold: usize,
    open_duration: Duration,
}

impl<S> Clone for CircuitBreaker<S>
where
    S: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            state: Arc::clone(&self.state),
            failure_threshold: self.failure_threshold,
            open_duration: self.open_duration,
        }
    }
}

impl<S, Request> Service<Request> for CircuitBreaker<S>
where
    S: Service<Request, Response = Response, Error = Infallible> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut TaskContext<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        if self.state.is_open() {
            let response = Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .body(Body::from("Circuit breaker open"))
                .expect("failed to build circuit breaker response");
            return Box::pin(async move { Ok(response) });
        }

        let state = Arc::clone(&self.state);
        let failure_threshold = self.failure_threshold;
        let open_duration = self.open_duration;

        let future = self.inner.call(request);
        Box::pin(async move {
            let response = future.await?;
            if response.status().is_server_error() {
                if state.record_failure(failure_threshold, open_duration) {
                    warn!(
                        "Circuit breaker opened after {} consecutive failures; blocking traffic for {:?}",
                        failure_threshold, open_duration
                    );
                }
            } else {
                state.record_success();
            }
            Ok(response)
        })
    }
}

struct CircuitBreakerState {
    failures: AtomicUsize,
    open_until: std::sync::Mutex<Option<Instant>>,
}

impl CircuitBreakerState {
    fn new() -> Self {
        Self {
            failures: AtomicUsize::new(0),
            open_until: std::sync::Mutex::new(None),
        }
    }

    fn is_open(&self) -> bool {
        let mut guard = self
            .open_until
            .lock()
            .expect("circuit breaker mutex poisoned");
        if let Some(until) = *guard {
            if Instant::now() < until {
                return true;
            }
            *guard = None;
        }
        false
    }

    fn record_failure(&self, threshold: usize, open_duration: Duration) -> bool {
        let current = self.failures.fetch_add(1, Ordering::Relaxed) + 1;
        if current >= threshold {
            let mut guard = self
                .open_until
                .lock()
                .expect("circuit breaker mutex poisoned");
            *guard = Some(Instant::now() + open_duration);
            self.failures.store(0, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    fn record_success(&self) {
        self.failures.store(0, Ordering::Relaxed);
        let mut guard = self
            .open_until
            .lock()
            .expect("circuit breaker mutex poisoned");
        *guard = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::P2PConfig;
    use anyhow::anyhow;
    use axum::body::Body;
    use axum::extract::{ConnectInfo, Path as AxumPath, Query};
    use axum::http::Request;
    use axum::Json;
    use ed25519_dalek::SigningKey;
    use ippan_consensus::{handles::HandlePipeline, PoAConfig, Validator};
    use ippan_consensus_dlc::{AiConsensusStatus, DlcConfig as AiDlcConfig, DlcConsensus};
    use ippan_files::{dht::StubFileDhtService, FileDhtService, FileStorage, MemoryFileStorage};
    use ippan_l2_handle_registry::{
        HandleRegistration, HandleRegistryError, PublicKey, StubHandleDhtService,
    };
    use ippan_p2p::NetworkEvent;
    use ippan_security::{RateLimitConfig, SecurityConfig, SecurityManager};
    use ippan_storage::{MemoryStorage, ValidatorTelemetry};
    use ippan_types::{
        address::{encode_address, Address},
        Amount, ChainState, FileDescriptor as ChainFileDescriptor, FileDescriptorId,
        IppanTimeMicros, L2ExitStatus, L2NetworkStatus, RoundCertificate, RoundFinalizationRecord,
        RoundId, RoundWindow,
    };
    use std::collections::{HashMap, HashSet};
    use std::fs;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::path::PathBuf;
    use std::sync::atomic::AtomicUsize;
    use std::sync::Arc;
    use std::time::Instant;
    use tempfile::tempdir;
    use tokio::time::{sleep, Duration};
    use tower::ServiceExt;

    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(prev) = &self.previous {
                std::env::set_var(self.key, prev);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    #[tokio::test]
    async fn test_handle_get_health_basic() {
        let file_storage: Arc<dyn FileStorage> = Arc::new(MemoryFileStorage::default());
        let file_dht: Arc<dyn FileDhtService> = Arc::new(StubFileDhtService::new());
        let handle_registry = Arc::new(L2HandleRegistry::new());
        let handle_anchors = Arc::new(L1HandleAnchorStorage::new());
        let handle_dht: Arc<dyn HandleDhtService> = Arc::new(StubHandleDhtService::new());
        let app_state = Arc::new(AppState {
            storage: Arc::new(MemoryStorage::default()),
            start_time: Instant::now(),
            peer_count: Arc::new(AtomicUsize::new(0)),
            p2p_network: None,
            tx_sender: None,
            node_id: "test-node".into(),
            consensus_mode: "poa".into(),
            consensus: None,
            ai_status: None,
            l2_config: L2Config {
                max_commit_size: 1000,
                min_epoch_gap_ms: 1000,
                challenge_window_ms: 2000,
                da_mode: "test".into(),
                max_l2_count: 1,
            },
            mempool: Arc::new(Mempool::new(1000)),
            unified_ui_dist: None,
            req_count: Arc::new(AtomicUsize::new(0)),
            security: None,
            metrics: None,
            file_storage: Some(file_storage),
            file_dht: Some(file_dht),
            dht_file_mode: "stub".into(),
            dev_mode: true,
            handle_registry,
            handle_anchors,
            handle_dht: Some(handle_dht),
            dht_handle_mode: "stub".into(),
        });

        let addr: SocketAddr = "127.0.0.1:6000".parse().unwrap();
        let Json(status) = handle_get_health(State(app_state), ConnectInfo(addr))
            .await
            .expect("health");
        assert_eq!(status.consensus_mode, "poa");
        assert!(status.rpc_healthy);
        assert_eq!(status.peer_count, 0);
    }

    #[tokio::test]
    async fn test_handle_get_health_reflects_dependency_failures() {
        let failing: Arc<dyn Storage + Send + Sync> = Arc::new(FailingStorage::new(&[
            "get_latest_round_finalization",
            "get_latest_height",
        ]));
        let mut inner = (*build_app_state(None, None)).clone();
        inner.storage = failing;
        inner.consensus = None;
        inner.file_dht = None;
        inner.handle_dht = None;
        let state = Arc::new(inner);

        let addr: SocketAddr = "127.0.0.1:6003".parse().unwrap();
        let Json(status) = handle_get_health(State(state), ConnectInfo(addr))
            .await
            .expect("health");
        assert!(!status.consensus_healthy);
        assert!(!status.storage_healthy);
        assert!(!status.dht_healthy);
    }

    #[tokio::test]
    async fn test_handle_get_ai_status_disabled_by_default() {
        let addr: SocketAddr = "127.0.0.1:6001".parse().unwrap();
        let Json(status) = handle_get_ai_status(State(make_app_state()), ConnectInfo(addr))
            .await
            .expect("ai status");
        assert!(!status.enabled);
        assert!(status.model_hash.is_none());
    }

    #[tokio::test]
    async fn test_handle_get_ai_status_with_provider() {
        let handle = AiStatusHandle::from_static(AiConsensusStatus {
            enabled: true,
            using_stub: false,
            model_hash: Some("deadbeef".into()),
            model_version: Some("v2".into()),
        });

        let mut state = (*make_app_state()).clone();
        state.ai_status = Some(handle);
        let state = Arc::new(state);

        let addr: SocketAddr = "127.0.0.1:6002".parse().unwrap();
        let Json(status) = handle_get_ai_status(State(state), ConnectInfo(addr))
            .await
            .expect("ai status");
        assert!(status.enabled);
        assert!(!status.using_stub);
        assert_eq!(status.model_hash.as_deref(), Some("deadbeef"));
        assert_eq!(status.model_version.as_deref(), Some("v2"));
    }

    #[tokio::test]
    async fn test_handle_get_ai_status_from_dlc_consensus() {
        let _stub_guard = EnvVarGuard::set("IPPAN_DGBDT_ALLOW_STUB", "1");
        let consensus = Arc::new(Mutex::new(DlcConsensus::new(AiDlcConfig::default())));
        let handle = AiStatusHandle::new({
            let consensus = consensus.clone();
            move || {
                let consensus = consensus.clone();
                async move {
                    let snapshot = {
                        let guard = consensus.lock().await;
                        guard.ai_status()
                    };
                    snapshot
                }
            }
        });

        let mut state = (*make_app_state()).clone();
        state.ai_status = Some(handle);
        let state = Arc::new(state);

        let addr: SocketAddr = "127.0.0.1:6003".parse().unwrap();
        let Json(status) = handle_get_ai_status(State(state), ConnectInfo(addr))
            .await
            .expect("ai status");
        assert!(status.enabled);
    }

    fn build_app_state(
        security: Option<Arc<SecurityManager>>,
        unified_ui_dist: Option<PathBuf>,
    ) -> Arc<AppState> {
        let storage: Arc<dyn Storage + Send + Sync> = Arc::new(MemoryStorage::default());
        let file_storage: Arc<dyn FileStorage> = Arc::new(MemoryFileStorage::new());
        let file_dht: Arc<dyn FileDhtService> = Arc::new(StubFileDhtService::new());
        let handle_registry = Arc::new(L2HandleRegistry::new());
        let handle_anchors = Arc::new(L1HandleAnchorStorage::new());
        let handle_dht: Arc<dyn HandleDhtService> = Arc::new(StubHandleDhtService::new());
        Arc::new(AppState {
            storage,
            start_time: Instant::now(),
            peer_count: Arc::new(AtomicUsize::new(0)),
            p2p_network: None,
            tx_sender: None,
            node_id: "test-node".into(),
            consensus_mode: "poa".into(),
            consensus: None,
            ai_status: None,
            l2_config: L2Config {
                max_commit_size: 512,
                min_epoch_gap_ms: 1_000,
                challenge_window_ms: 2_000,
                da_mode: "test".into(),
                max_l2_count: 1,
            },
            mempool: Arc::new(Mempool::new(1_000)),
            unified_ui_dist,
            req_count: Arc::new(AtomicUsize::new(0)),
            security,
            metrics: None,
            file_storage: Some(file_storage),
            file_dht: Some(file_dht),
            dht_file_mode: "stub".into(),
            dev_mode: true,
            handle_registry,
            handle_anchors,
            handle_dht: Some(handle_dht),
            dht_handle_mode: "stub".into(),
        })
    }

    fn make_app_state() -> Arc<AppState> {
        build_app_state(None, None)
    }

    fn sample_private_key(seed: [u8; 32]) -> SigningKey {
        SigningKey::from_bytes(&seed)
    }

    fn sample_public_key(seed: [u8; 32]) -> [u8; 32] {
        sample_private_key(seed).verifying_key().to_bytes()
    }

    fn sample_transaction(from_seed: [u8; 32], to_address: [u8; 32], nonce: u64) -> Transaction {
        let signing_key = sample_private_key(from_seed);
        let from_public = signing_key.verifying_key().to_bytes();
        let mut tx = Transaction::new(
            from_public,
            to_address,
            Amount::from_micro_ipn(10 + nonce),
            nonce,
        );
        let private_bytes = signing_key.to_bytes();
        tx.sign(&private_bytes).expect("sign sample transaction");
        tx
    }

    struct FailingStorage {
        inner: MemoryStorage,
        failures: HashSet<String>,
    }

    impl FailingStorage {
        fn new(failures: &[&str]) -> Self {
            Self {
                inner: MemoryStorage::default(),
                failures: failures.iter().map(|s| s.to_string()).collect(),
            }
        }

        fn should_fail(&self, op: &str) -> bool {
            self.failures.contains(op)
        }

        fn inner(&self) -> &MemoryStorage {
            &self.inner
        }
    }

    impl Storage for FailingStorage {
        fn store_block(&self, block: Block) -> Result<()> {
            if self.should_fail("store_block") {
                Err(anyhow!("forced failure: store_block"))
            } else {
                self.inner.store_block(block)
            }
        }

        fn get_block(&self, hash: &[u8; 32]) -> Result<Option<Block>> {
            if self.should_fail("get_block") {
                Err(anyhow!("forced failure: get_block"))
            } else {
                self.inner.get_block(hash)
            }
        }

        fn get_block_by_height(&self, height: u64) -> Result<Option<Block>> {
            if self.should_fail("get_block_by_height") {
                Err(anyhow!("forced failure: get_block_by_height"))
            } else {
                self.inner.get_block_by_height(height)
            }
        }

        fn store_transaction(&self, tx: Transaction) -> Result<()> {
            if self.should_fail("store_transaction") {
                Err(anyhow!("forced failure: store_transaction"))
            } else {
                self.inner.store_transaction(tx)
            }
        }

        fn get_transaction(&self, hash: &[u8; 32]) -> Result<Option<Transaction>> {
            if self.should_fail("get_transaction") {
                Err(anyhow!("forced failure: get_transaction"))
            } else {
                self.inner.get_transaction(hash)
            }
        }

        fn get_latest_height(&self) -> Result<u64> {
            if self.should_fail("get_latest_height") {
                Err(anyhow!("forced failure: get_latest_height"))
            } else {
                self.inner.get_latest_height()
            }
        }

        fn get_account(&self, address: &[u8; 32]) -> Result<Option<Account>> {
            if self.should_fail("get_account") {
                Err(anyhow!("forced failure: get_account"))
            } else {
                self.inner.get_account(address)
            }
        }

        fn update_account(&self, account: Account) -> Result<()> {
            if self.should_fail("update_account") {
                Err(anyhow!("forced failure: update_account"))
            } else {
                self.inner.update_account(account)
            }
        }

        fn get_all_accounts(&self) -> Result<Vec<Account>> {
            if self.should_fail("get_all_accounts") {
                Err(anyhow!("forced failure: get_all_accounts"))
            } else {
                self.inner.get_all_accounts()
            }
        }

        fn get_transactions_by_address(&self, address: &[u8; 32]) -> Result<Vec<Transaction>> {
            if self.should_fail("get_transactions_by_address") {
                Err(anyhow!("forced failure: get_transactions_by_address"))
            } else {
                self.inner.get_transactions_by_address(address)
            }
        }

        fn get_transaction_count(&self) -> Result<u64> {
            if self.should_fail("get_transaction_count") {
                Err(anyhow!("forced failure: get_transaction_count"))
            } else {
                self.inner.get_transaction_count()
            }
        }

        fn put_l2_network(&self, network: L2Network) -> Result<()> {
            if self.should_fail("put_l2_network") {
                Err(anyhow!("forced failure: put_l2_network"))
            } else {
                self.inner.put_l2_network(network)
            }
        }

        fn get_l2_network(&self, id: &str) -> Result<Option<L2Network>> {
            if self.should_fail("get_l2_network") {
                Err(anyhow!("forced failure: get_l2_network"))
            } else {
                self.inner.get_l2_network(id)
            }
        }

        fn list_l2_networks(&self) -> Result<Vec<L2Network>> {
            if self.should_fail("list_l2_networks") {
                Err(anyhow!("forced failure: list_l2_networks"))
            } else {
                self.inner.list_l2_networks()
            }
        }

        fn store_l2_commit(&self, commit: L2Commit) -> Result<()> {
            if self.should_fail("store_l2_commit") {
                Err(anyhow!("forced failure: store_l2_commit"))
            } else {
                self.inner.store_l2_commit(commit)
            }
        }

        fn list_l2_commits(&self, l2_id: Option<&str>) -> Result<Vec<L2Commit>> {
            if self.should_fail("list_l2_commits") {
                Err(anyhow!("forced failure: list_l2_commits"))
            } else {
                self.inner.list_l2_commits(l2_id)
            }
        }

        fn store_l2_exit(&self, exit: L2ExitRecord) -> Result<()> {
            if self.should_fail("store_l2_exit") {
                Err(anyhow!("forced failure: store_l2_exit"))
            } else {
                self.inner.store_l2_exit(exit)
            }
        }

        fn list_l2_exits(&self, l2_id: Option<&str>) -> Result<Vec<L2ExitRecord>> {
            if self.should_fail("list_l2_exits") {
                Err(anyhow!("forced failure: list_l2_exits"))
            } else {
                self.inner.list_l2_exits(l2_id)
            }
        }

        fn store_round_certificate(&self, certificate: RoundCertificate) -> Result<()> {
            if self.should_fail("store_round_certificate") {
                Err(anyhow!("forced failure: store_round_certificate"))
            } else {
                self.inner.store_round_certificate(certificate)
            }
        }

        fn get_round_certificate(&self, round: RoundId) -> Result<Option<RoundCertificate>> {
            if self.should_fail("get_round_certificate") {
                Err(anyhow!("forced failure: get_round_certificate"))
            } else {
                self.inner.get_round_certificate(round)
            }
        }

        fn store_round_finalization(&self, record: RoundFinalizationRecord) -> Result<()> {
            if self.should_fail("store_round_finalization") {
                Err(anyhow!("forced failure: store_round_finalization"))
            } else {
                self.inner.store_round_finalization(record)
            }
        }

        fn get_round_finalization(
            &self,
            round: RoundId,
        ) -> Result<Option<RoundFinalizationRecord>> {
            if self.should_fail("get_round_finalization") {
                Err(anyhow!("forced failure: get_round_finalization"))
            } else {
                self.inner.get_round_finalization(round)
            }
        }

        fn get_latest_round_finalization(&self) -> Result<Option<RoundFinalizationRecord>> {
            if self.should_fail("get_latest_round_finalization") {
                Err(anyhow!("forced failure: get_latest_round_finalization"))
            } else {
                self.inner.get_latest_round_finalization()
            }
        }

        fn get_chain_state(&self) -> Result<ChainState> {
            if self.should_fail("get_chain_state") {
                Err(anyhow!("forced failure: get_chain_state"))
            } else {
                self.inner.get_chain_state()
            }
        }

        fn update_chain_state(&self, state: &ChainState) -> Result<()> {
            if self.should_fail("update_chain_state") {
                Err(anyhow!("forced failure: update_chain_state"))
            } else {
                self.inner.update_chain_state(state)
            }
        }

        fn store_validator_telemetry(
            &self,
            validator_id: &[u8; 32],
            telemetry: &ValidatorTelemetry,
        ) -> Result<()> {
            if self.should_fail("store_validator_telemetry") {
                Err(anyhow!("forced failure: store_validator_telemetry"))
            } else {
                self.inner
                    .store_validator_telemetry(validator_id, telemetry)
            }
        }

        fn get_validator_telemetry(
            &self,
            validator_id: &[u8; 32],
        ) -> Result<Option<ValidatorTelemetry>> {
            if self.should_fail("get_validator_telemetry") {
                Err(anyhow!("forced failure: get_validator_telemetry"))
            } else {
                self.inner.get_validator_telemetry(validator_id)
            }
        }

        fn get_all_validator_telemetry(&self) -> Result<HashMap<[u8; 32], ValidatorTelemetry>> {
            if self.should_fail("get_all_validator_telemetry") {
                Err(anyhow!("forced failure: get_all_validator_telemetry"))
            } else {
                self.inner.get_all_validator_telemetry()
            }
        }

        fn store_file_descriptor(&self, descriptor: ChainFileDescriptor) -> Result<()> {
            if self.should_fail("store_file_descriptor") {
                Err(anyhow!("forced failure: store_file_descriptor"))
            } else {
                self.inner.store_file_descriptor(descriptor)
            }
        }

        fn get_file_descriptor(
            &self,
            id: &FileDescriptorId,
        ) -> Result<Option<ChainFileDescriptor>> {
            if self.should_fail("get_file_descriptor") {
                Err(anyhow!("forced failure: get_file_descriptor"))
            } else {
                self.inner.get_file_descriptor(id)
            }
        }

        fn list_file_descriptors_by_owner(
            &self,
            owner: &Address,
        ) -> Result<Vec<ChainFileDescriptor>> {
            if self.should_fail("list_file_descriptors_by_owner") {
                Err(anyhow!("forced failure: list_file_descriptors_by_owner"))
            } else {
                self.inner.list_file_descriptors_by_owner(owner)
            }
        }
    }

    #[test]
    fn test_parse_hex_32_success_and_failure() {
        let bytes = parse_hex_32(&"AA".repeat(32)).expect("parse 32 bytes");
        assert_eq!(bytes[0], 0xAA);
        assert!(parse_hex_32("short").is_err());
        assert!(parse_hex_32(&"0G".repeat(32)).is_err());
    }

    #[test]
    fn test_parse_block_identifier_variants() {
        if let Some(BlockIdentifier::Height(h)) = parse_block_identifier("42") {
            assert_eq!(h, 42);
        } else {
            panic!("unexpected identifier variant");
        }

        let hash_input = "ab".repeat(32);
        if let Some(BlockIdentifier::Hash(bytes)) = parse_block_identifier(&hash_input) {
            assert_eq!(bytes.len(), 32);
            assert_eq!(bytes[0], 0xAB);
        } else {
            panic!("expected hash identifier");
        }

        assert!(parse_block_identifier("invalid-hash").is_none());
    }

    #[test]
    fn test_account_to_response_serializes() {
        let account = Account {
            address: sample_public_key([1u8; 32]),
            balance: 1_000,
            nonce: 2,
        };
        let tx = sample_transaction([1u8; 32], sample_public_key([2u8; 32]), 3);
        let response = account_to_response(account, vec![tx.clone()]);
        assert_eq!(response.address, hex::encode(sample_public_key([1u8; 32])));
        assert_eq!(response.balance_atomic, "1000");
        assert_eq!(response.recent_transactions.len(), 1);
        assert_eq!(response.recent_transactions[0].hash.len(), 64);
        assert_eq!(response.recent_transactions[0].hash, hex::encode(tx.hash()));
    }

    #[test]
    fn test_map_security_error_variants() {
        let (status, msg) = map_security_error(&SecurityError::IpBlocked);
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert!(msg.contains("blocked"));

        let (status, msg) = map_security_error(&SecurityError::RateLimitExceeded);
        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
        assert!(msg.contains("Rate limit"));

        let validation_error = SecurityError::ValidationFailed(
            ippan_security::ValidationError::MissingField("field".into()),
        );
        let (status, msg) = map_security_error(&validation_error);
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(msg.contains("Invalid"));

        let audit_error = SecurityError::AuditFailed(anyhow::anyhow!("boom"));
        let (status, _) = map_security_error(&audit_error);
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_guard_request_without_security() {
        let state = make_app_state();
        let addr: SocketAddr = "127.0.0.1:9000".parse().unwrap();
        guard_request(&state, &addr, "/health")
            .await
            .expect("request allowed");
    }

    #[test]
    fn test_message_announced_address_prefers_routable() {
        let message = NetworkMessage::PeerInfo {
            peer_id: "peer".into(),
            addresses: vec![
                "".into(),
                "http://0.0.0.0:9000".into(),
                "http://192.168.1.5:9000".into(),
            ],
            time_us: None,
        };
        let addr = message_announced_address(&message).expect("address");
        assert_eq!(addr, "http://192.168.1.5:9000");
    }

    #[test]
    fn test_resolve_peer_address_fallback() {
        let state = make_app_state();
        let socket: SocketAddr = "10.0.0.5:7000".parse().unwrap();
        let message = NetworkMessage::PeerDiscovery { peers: vec![] };
        let resolved = resolve_peer_address(&state, &socket, &message);
        assert_eq!(resolved, "http://10.0.0.5:7000");
    }

    #[tokio::test]
    async fn test_ingest_block_from_peer_updates_state() {
        let state = make_app_state();
        let tx = sample_transaction([1u8; 32], sample_public_key([2u8; 32]), 1);
        let tx_hash_hex = hex::encode(tx.hash());
        state
            .mempool
            .add_transaction(tx.clone())
            .expect("add tx to mempool");
        let block = Block::new(vec![], vec![tx.clone()], 1, [9u8; 32]);
        let block_hash = block.hash();

        ingest_block_from_peer(&state, &block).expect("ingest block");

        let stored = state
            .storage
            .get_block(&block_hash)
            .expect("query block")
            .expect("block stored");
        assert_eq!(stored.header.round, 1);
        assert!(state.mempool.get_transaction(&tx_hash_hex).is_none());
    }

    #[tokio::test]
    async fn test_ingest_transaction_from_peer_persists() {
        let state = make_app_state();
        let tx = sample_transaction([5u8; 32], sample_public_key([6u8; 32]), 2);
        let tx_hash = tx.hash();

        ingest_transaction_from_peer(&state, &tx).expect("ingest tx");

        let stored = state
            .storage
            .get_transaction(&tx_hash)
            .expect("query tx")
            .expect("tx stored");
        assert_eq!(stored.hash(), tx_hash);
        assert!(state
            .mempool
            .get_transaction(&hex::encode(tx_hash))
            .is_some());
    }

    #[tokio::test]
    async fn test_handle_get_transaction_paths() {
        let state = make_app_state();
        let addr: SocketAddr = "127.0.0.1:9000".parse().unwrap();
        let tx = sample_transaction([8u8; 32], sample_public_key([9u8; 32]), 4);
        let tx_hash = tx.hash();
        state
            .storage
            .store_transaction(tx.clone())
            .expect("store tx");

        let ok = handle_get_transaction(
            State(state.clone()),
            ConnectInfo(addr),
            AxumPath(hex::encode(tx_hash)),
        )
        .await
        .expect("success");
        assert_eq!(ok.0.hash, hex::encode(tx.hash()));

        let missing = handle_get_transaction(
            State(state.clone()),
            ConnectInfo(addr),
            AxumPath(hex::encode(sample_public_key([7u8; 32]))),
        )
        .await
        .expect_err("not found");
        assert_eq!(missing.0, StatusCode::NOT_FOUND);

        let bad = handle_get_transaction(
            State(state.clone()),
            ConnectInfo(addr),
            AxumPath("xyz".to_string()),
        )
        .await
        .expect_err("bad request");
        assert_eq!(bad.0, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_handle_get_transaction_with_security() {
        let dir = tempdir().expect("tempdir");
        let config = SecurityConfig {
            audit_log_path: dir.path().join("audit.log").to_string_lossy().to_string(),
            ..SecurityConfig::default()
        };
        let manager = SecurityManager::new(config).expect("manager");
        let state = build_app_state(Some(Arc::new(manager)), None);
        let addr: SocketAddr = "10.0.0.10:8080".parse().unwrap();
        let tx = sample_transaction([11u8; 32], sample_public_key([12u8; 32]), 5);
        let tx_hash = tx.hash();
        state.storage.store_transaction(tx).expect("store tx");

        let _ = handle_get_transaction(
            State(state.clone()),
            ConnectInfo(addr),
            AxumPath(hex::encode(tx_hash)),
        )
        .await
        .expect("security success");

        let deny_config = SecurityConfig {
            enable_ip_whitelist: true,
            whitelisted_ips: vec![],
            audit_log_path: dir.path().join("blocked.log").to_string_lossy().to_string(),
            ..SecurityConfig::default()
        };
        let deny_state = build_app_state(
            Some(Arc::new(SecurityManager::new(deny_config).unwrap())),
            None,
        );
        let err = handle_get_transaction(
            State(deny_state.clone()),
            ConnectInfo(addr),
            AxumPath(hex::encode(tx_hash)),
        )
        .await
        .expect_err("denied");
        assert_eq!(err.0, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_handle_get_block_variants() {
        let state = make_app_state();
        let addr: SocketAddr = "127.0.0.1:9000".parse().unwrap();
        let block = Block::new(vec![], vec![], 5, [3u8; 32]);
        let block_hash = block.hash();
        state
            .storage
            .store_block(block.clone())
            .expect("store block");

        let by_hash = handle_get_block(
            State(state.clone()),
            ConnectInfo(addr),
            AxumPath(hex::encode(block_hash)),
        )
        .await
        .expect("block by hash");
        assert_eq!(by_hash.0.block.round, 5);
        assert!(by_hash.0.block.height.is_none());
        assert!(by_hash.0.fee_summary.is_none());

        let by_height = handle_get_block(
            State(state.clone()),
            ConnectInfo(addr),
            AxumPath("5".to_string()),
        )
        .await
        .expect("block by height");
        assert_eq!(by_height.0.block.round, 5);
        assert_eq!(by_height.0.block.height, Some(5));
        assert!(by_height.0.fee_summary.is_none());

        let bad = handle_get_block(
            State(state.clone()),
            ConnectInfo(addr),
            AxumPath("not-a-block".to_string()),
        )
        .await
        .expect_err("bad identifier");
        assert_eq!(bad.0, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_handle_get_block_includes_fee_summary() {
        let state = make_app_state();
        let addr: SocketAddr = "127.0.0.1:9100".parse().unwrap();
        let block = Block::new(vec![], vec![], 8, [4u8; 32]);
        let block_hash = block.hash();
        state
            .storage
            .store_block(block.clone())
            .expect("store block");
        let record = RoundFinalizationRecord {
            round: 8,
            window: RoundWindow {
                id: 8,
                start_us: IppanTimeMicros(0),
                end_us: IppanTimeMicros(1),
            },
            ordered_tx_ids: vec![],
            fork_drops: vec![],
            state_root: [0u8; 32],
            proof: RoundCertificate {
                round: 8,
                block_ids: vec![block_hash],
                agg_sig: vec![],
            },
            total_fees_atomic: Some(5000),
            treasury_fees_atomic: Some(2000),
            applied_payments: Some(2),
            rejected_payments: Some(1),
        };
        state
            .storage
            .store_round_finalization(record)
            .expect("store record");

        let response = handle_get_block(
            State(state.clone()),
            ConnectInfo(addr),
            AxumPath(hex::encode(block_hash)),
        )
        .await
        .expect("block");
        let summary = response.0.fee_summary.expect("fee summary");
        assert_eq!(summary.round, 8);
        assert_eq!(summary.total_fees_atomic, format_atomic(5000));
        assert_eq!(summary.treasury_fees_atomic, format_atomic(2000));
        assert_eq!(summary.applied_payments, 2);
        assert_eq!(summary.rejected_payments, 1);
    }

    #[tokio::test]
    async fn test_handle_get_account_branches() {
        let state = make_app_state();
        let addr: SocketAddr = "127.0.0.1:9000".parse().unwrap();
        let account_address = sample_public_key([4u8; 32]);
        let tx = sample_transaction([4u8; 32], sample_public_key([5u8; 32]), 1);
        let account = Account {
            address: account_address,
            balance: 500,
            nonce: 7,
        };
        state
            .storage
            .update_account(account.clone())
            .expect("account");
        state.storage.store_transaction(tx.clone()).expect("tx1");
        let tx2 = sample_transaction([6u8; 32], account.address, 2);
        state.storage.store_transaction(tx2.clone()).expect("tx2");

        let ok = handle_get_account(
            State(state.clone()),
            ConnectInfo(addr),
            AxumPath(hex::encode(account.address)),
        )
        .await
        .expect("account ok");
        assert_eq!(ok.0.balance_atomic, "500");
        assert_eq!(ok.0.recent_transactions.len(), 2);
        assert_eq!(ok.0.recent_payments.len(), 2);

        let missing = handle_get_account(
            State(state.clone()),
            ConnectInfo(addr),
            AxumPath(hex::encode(sample_public_key([9u8; 32]))),
        )
        .await
        .expect_err("missing");
        assert_eq!(missing.0, StatusCode::NOT_FOUND);

        let bad = handle_get_account(
            State(state.clone()),
            ConnectInfo(addr),
            AxumPath("badhex".to_string()),
        )
        .await
        .expect_err("bad request");
        assert_eq!(bad.0, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_handle_payment_tx_success_path() {
        let storage: Arc<dyn Storage + Send + Sync> = Arc::new(MemoryStorage::default());
        let mut config = PoAConfig::default();
        config.validators.push(Validator {
            id: sample_public_key([14u8; 32]),
            address: sample_public_key([15u8; 32]),
            stake: 1_000,
            is_active: true,
        });

        let poa = PoAConsensus::new(config, storage.clone(), sample_public_key([16u8; 32]));
        let mempool = poa.mempool();
        let consensus = Arc::new(Mutex::new(poa));

        let (tx_sender, mut rx) = mpsc::unbounded_channel();
        let handle = ConsensusHandle::new(consensus.clone(), tx_sender.clone(), mempool.clone());

        let mut app_state = (*build_app_state(None, None)).clone();
        app_state.storage = storage.clone();
        app_state.consensus = Some(handle);
        app_state.tx_sender = Some(tx_sender);
        app_state.mempool = mempool.clone();
        let state = Arc::new(app_state);

        let signer = sample_private_key([42u8; 32]);
        let from_public = signer.verifying_key().to_bytes();
        state
            .storage
            .update_account(Account {
                address: from_public,
                balance: 10_000,
                nonce: 3,
            })
            .expect("account");

        let to_public = sample_public_key([43u8; 32]);
        let request = PaymentRequest {
            from: encode_address(&from_public),
            to: encode_address(&to_public),
            amount: 1_000,
            fee: None,
            nonce: None,
            memo: Some("integration".into()),
            signing_key: Some(hex::encode(signer.to_bytes())),
        };

        let addr: SocketAddr = "127.0.0.1:9400".parse().unwrap();
        let response = handle_payment_tx(
            State(state.clone()),
            ConnectInfo(addr),
            ValidatedJson(request),
        )
        .await;
        let json = response.expect("payment ok").0;
        assert_eq!(json.status, PaymentStatus::AcceptedToMempool);
        assert_eq!(json.nonce, 4);
        assert_eq!(json.memo.as_deref(), Some("integration"));

        let dispatched = rx.recv().await.expect("consensus dispatch");
        assert_eq!(dispatched.nonce, 4);
    }

    #[tokio::test]
    async fn test_handle_payment_tx_missing_signing_key() {
        let state = make_app_state();
        let addr: SocketAddr = "127.0.0.1:9401".parse().unwrap();
        let request = PaymentRequest {
            from: encode_address(&sample_public_key([30u8; 32])),
            to: encode_address(&sample_public_key([31u8; 32])),
            amount: 1,
            fee: None,
            nonce: Some(1),
            memo: None,
            signing_key: None,
        };

        let err = handle_payment_tx(State(state), ConnectInfo(addr), ValidatedJson(request))
            .await
            .expect_err("missing key");
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_handle_payment_tx_rejects_invalid_payloads() {
        let state = make_app_state();
        let addr: SocketAddr = "127.0.0.1:9402".parse().unwrap();

        let bad_request = PaymentRequest {
            from: "invalid".into(),
            to: encode_address(&sample_public_key([31u8; 32])),
            amount: 1,
            fee: None,
            nonce: Some(1),
            memo: None,
            signing_key: Some(hex::encode(sample_private_key([32u8; 32]).to_bytes())),
        };

        let err = handle_payment_tx(
            State(state.clone()),
            ConnectInfo(addr),
            ValidatedJson(bad_request),
        )
        .await
        .expect_err("invalid address");
        assert_eq!(err.0, StatusCode::BAD_REQUEST);

        let signer = sample_private_key([33u8; 32]);
        let zero_amount = PaymentRequest {
            from: encode_address(&signer.verifying_key().to_bytes()),
            to: encode_address(&sample_public_key([34u8; 32])),
            amount: 0,
            fee: None,
            nonce: Some(1),
            memo: None,
            signing_key: Some(hex::encode(signer.to_bytes())),
        };

        let err = handle_payment_tx(State(state), ConnectInfo(addr), ValidatedJson(zero_amount))
            .await
            .expect_err("zero amount");
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_payment_endpoint_rejects_malformed_json() {
        let state = make_app_state();
        let request_counter = state.req_count.clone();
        let router = build_router(state);

        let addr: SocketAddr = "127.0.0.1:9405".parse().unwrap();
        let mut request = Request::builder()
            .method("POST")
            .uri("/tx/payment")
            .header(CONTENT_TYPE, "application/json")
            .body(Body::from("{\"from\": \"truncated"))
            .expect("malformed request");
        request.extensions_mut().insert(ConnectInfo(addr));

        let response = router
            .oneshot(request)
            .await
            .expect("response for malformed payload");

        let status = response.status();
        assert!(status.is_client_error(), "unexpected status {status}");
        let observed = request_counter.load(std::sync::atomic::Ordering::SeqCst);
        assert!(observed <= 1);
    }

    #[tokio::test]
    async fn test_payment_endpoint_rejects_wrong_method() {
        let state = make_app_state();
        let router = build_router(state);

        let response = router
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/tx/payment")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response for wrong method");

        assert!(response.status().is_client_error());
    }

    #[tokio::test]
    async fn oversized_body_is_rejected_before_state_changes() {
        let mut security_config = SecurityConfig::default();
        security_config.max_request_size = 64;
        security_config.audit_log_path = "/tmp/ippan-security-tests.log".into();
        let security = Arc::new(SecurityManager::new(security_config).expect("security"));

        let state = build_app_state(Some(security), None);
        let router = build_router(state.clone());

        let addr: SocketAddr = "127.0.0.1:9410".parse().unwrap();
        let initial_mempool_size = state.mempool.size();
        let signer = sample_private_key([41u8; 32]);
        let payload = serde_json::json!({
            "from": encode_address(&signer.verifying_key().to_bytes()),
            "to": encode_address(&sample_public_key([42u8; 32])),
            "amount": 1,
            "fee": 1,
            "nonce": 1,
            "memo": serde_json::Value::Null,
            "signing_key": hex::encode(signer.to_bytes()),
        });
        let body = payload.to_string();
        let mut request = Request::builder()
            .method("POST")
            .uri("/tx/payment")
            .header(CONTENT_TYPE, "application/json")
            .header(axum::http::header::CONTENT_LENGTH, body.len())
            .body(Body::from(body))
            .expect("oversized body request");
        request.extensions_mut().insert(ConnectInfo(addr));

        let response = router.oneshot(request).await.expect("oversized response");

        let status = response.status();
        let (_parts, body) = response.into_parts();
        let bytes = body.collect().await.expect("body bytes").to_bytes();
        assert!(status.is_client_error());
        if let Ok(parsed) = serde_json::from_slice::<serde_json::Value>(&bytes) {
            if let Some(code) = parsed.get("code").and_then(|value| value.as_str()) {
                assert_eq!(code, "body_too_large");
            }
        }
        assert_eq!(state.mempool.size(), initial_mempool_size);
    }

    #[tokio::test]
    async fn security_manager_rate_limits_spammy_client() {
        let mut rate_limit = RateLimitConfig::default();
        rate_limit.requests_per_second = 1;
        rate_limit.burst_capacity = 1;
        rate_limit.endpoint_limits.clear();
        rate_limit.global_requests_per_second = Some(1);

        let mut security_config = SecurityConfig::default();
        security_config.rate_limit = rate_limit;
        security_config.audit_log_path = "/tmp/ippan-security-tests.log".into();

        let security = Arc::new(SecurityManager::new(security_config).expect("security"));
        let state = build_app_state(Some(security), None);
        let router = build_router(state);
        let addr: SocketAddr = "127.0.0.1:9411".parse().unwrap();

        let make_request = |uri: &str| {
            let mut request = Request::builder()
                .method("GET")
                .uri(uri)
                .body(Body::empty())
                .expect("request");
            request.extensions_mut().insert(ConnectInfo(addr));
            request
        };

        let first = router
            .clone()
            .oneshot(make_request("/health"))
            .await
            .expect("first response");
        assert!(first.status().is_success());

        let second = router
            .oneshot(make_request("/health"))
            .await
            .expect("second response");
        assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn test_handle_register_endpoint_success() {
        let storage: Arc<dyn Storage + Send + Sync> = Arc::new(MemoryStorage::default());
        let mut config = PoAConfig::default();
        config.validators.push(Validator {
            id: sample_public_key([80u8; 32]),
            address: sample_public_key([81u8; 32]),
            stake: 1_000,
            is_active: true,
        });

        let poa = PoAConsensus::new(config, storage.clone(), sample_public_key([82u8; 32]));
        let mempool = poa.mempool();
        let consensus = Arc::new(Mutex::new(poa));
        let (tx_sender, mut rx) = mpsc::unbounded_channel();
        let handle = ConsensusHandle::new(consensus.clone(), tx_sender.clone(), mempool.clone());

        let mut app_state = (*build_app_state(None, None)).clone();
        app_state.storage = storage.clone();
        app_state.consensus = Some(handle);
        app_state.tx_sender = Some(tx_sender);
        app_state.mempool = mempool.clone();
        let state = Arc::new(app_state);

        let signer = sample_private_key([83u8; 32]);
        let owner = signer.verifying_key().to_bytes();
        state
            .storage
            .update_account(Account {
                address: owner,
                balance: 5_000,
                nonce: 2,
            })
            .expect("account");

        let mut metadata = BTreeMap::new();
        metadata.insert("display_name".into(), "Alice".into());
        let request = HandleRegisterRequest {
            handle: "@alice.ipn".into(),
            owner: encode_address(&owner),
            metadata,
            expires_at: Some(ippan_time_now() + 60),
            fee: None,
            nonce: None,
            signing_key: hex::encode(signer.to_bytes()),
        };

        let addr: SocketAddr = "127.0.0.1:9450".parse().unwrap();
        let response = handle_register_handle(
            State(state.clone()),
            ConnectInfo(addr),
            ValidatedJson(request),
        )
        .await
        .expect("register");
        assert_eq!(response.0.status, HandleSubmissionStatus::AcceptedToMempool);
        assert_eq!(response.0.handle, "@alice.ipn");
        assert_eq!(response.0.nonce, 3);

        let dispatched = rx.recv().await.expect("tx dispatched");
        let op = dispatched
            .handle_operation()
            .expect("handle operation exists");
        match op {
            HandleOperation::Register(op) => {
                assert_eq!(op.handle, "@alice.ipn");
                assert_eq!(op.owner, owner);
            }
        }
    }

    #[tokio::test]
    async fn test_handle_register_endpoint_rejects_invalid_handle() {
        let state = make_app_state();
        let signer = sample_private_key([84u8; 32]);
        let owner = signer.verifying_key().to_bytes();
        let request = HandleRegisterRequest {
            handle: "invalid".into(),
            owner: encode_address(&owner),
            metadata: BTreeMap::new(),
            expires_at: None,
            fee: None,
            nonce: Some(1),
            signing_key: hex::encode(signer.to_bytes()),
        };
        let addr: SocketAddr = "127.0.0.1:9451".parse().unwrap();
        let err = handle_register_handle(State(state), ConnectInfo(addr), ValidatedJson(request))
            .await
            .expect_err("invalid handle");
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_handle_register_endpoint_rejects_overlong_handle() {
        let state = make_app_state();
        let signer = sample_private_key([85u8; 32]);
        let owner = signer.verifying_key().to_bytes();
        let oversized = format!("@{}{}", "a".repeat(80), ".ipn");
        let request = HandleRegisterRequest {
            handle: oversized.clone(),
            owner: encode_address(&owner),
            metadata: BTreeMap::new(),
            expires_at: None,
            fee: None,
            nonce: Some(1),
            signing_key: hex::encode(signer.to_bytes()),
        };

        let addr: SocketAddr = "127.0.0.1:9452".parse().unwrap();
        let err = handle_register_handle(
            State(state.clone()),
            ConnectInfo(addr),
            ValidatedJson(request),
        )
        .await
        .expect_err("oversized handle should fail");

        assert_eq!(err.0, StatusCode::BAD_REQUEST);
        let lookup = state
            .handle_registry
            .resolve(&Handle::new(oversized))
            .expect_err("handle should not exist");
        assert!(matches!(lookup, HandleRegistryError::HandleNotFound { .. }));
    }

    #[tokio::test]
    async fn test_handle_register_round_trip_with_pipeline_and_dht() {
        let storage: Arc<dyn Storage + Send + Sync> = Arc::new(MemoryStorage::default());
        let mut config = PoAConfig::default();
        config.validators.push(Validator {
            id: sample_public_key([90u8; 32]),
            address: sample_public_key([91u8; 32]),
            stake: 1_000,
            is_active: true,
        });

        let poa = PoAConsensus::new(config, storage.clone(), sample_public_key([92u8; 32]));
        let mempool = poa.mempool();
        let consensus = Arc::new(Mutex::new(poa));
        let (tx_sender, mut rx) = mpsc::unbounded_channel();
        let handle = ConsensusHandle::new(consensus.clone(), tx_sender.clone(), mempool.clone());

        let mut app_state = (*build_app_state(None, None)).clone();
        let stub_handle_dht: Arc<StubHandleDhtService> = Arc::new(StubHandleDhtService::new());
        app_state.storage = storage.clone();
        app_state.consensus = Some(handle);
        app_state.tx_sender = Some(tx_sender);
        app_state.mempool = mempool;
        app_state.handle_dht = Some(stub_handle_dht.clone());
        let state = Arc::new(app_state);

        let signer = sample_private_key([93u8; 32]);
        let owner = signer.verifying_key().to_bytes();
        state
            .storage
            .update_account(Account {
                address: owner,
                balance: 5_000,
                nonce: 0,
            })
            .expect("account");

        let mut metadata = BTreeMap::new();
        metadata.insert("alias".into(), "Pipeline".into());
        let request = HandleRegisterRequest {
            handle: "@pipeline.ipn".into(),
            owner: encode_address(&owner),
            metadata,
            expires_at: Some(ippan_time_now() + 120),
            fee: None,
            nonce: None,
            signing_key: hex::encode(signer.to_bytes()),
        };

        let addr: SocketAddr = "127.0.0.1:9453".parse().unwrap();
        let _ = handle_register_handle(
            State(state.clone()),
            ConnectInfo(addr),
            ValidatedJson(request),
        )
        .await
        .expect("register ok");

        let dispatched = rx.recv().await.expect("tx dispatched");
        let pipeline = HandlePipeline::with_dht(
            state.handle_registry.clone(),
            state.handle_anchors.clone(),
            state.handle_dht.clone(),
        );
        pipeline.apply(&dispatched, 20, 20).expect("apply pipeline");
        tokio::task::yield_now().await;

        let lookup = handle_get_handle(
            State(state.clone()),
            ConnectInfo(addr),
            AxumPath("@pipeline.ipn".to_string()),
        )
        .await
        .expect("lookup");
        assert_eq!(lookup.0.owner, encode_address(&owner));

        let dht_record = stub_handle_dht
            .get(&Handle::new("@pipeline.ipn"))
            .expect("dht record");
        assert_eq!(dht_record.owner.as_bytes(), &owner);
    }

    #[tokio::test]
    async fn test_handle_lookup_endpoint_flow() {
        let state = make_app_state();
        let addr: SocketAddr = "127.0.0.1:9452".parse().unwrap();
        let signer = sample_private_key([85u8; 32]);
        let owner = signer.verifying_key().to_bytes();
        let handle = Handle::new("@lookup.ipn");
        let expires_at = Some(ippan_time_now() + 120);
        let signature =
            sign_handle_registration_payload(&signer, handle.as_str(), &owner, expires_at);
        state
            .handle_registry
            .register(HandleRegistration {
                handle: handle.clone(),
                owner: PublicKey::new(owner),
                signature,
                metadata: HashMap::from([(String::from("alias"), String::from("Lookup"))]),
                expires_at,
            })
            .expect("registry insert");

        let response = handle_get_handle(
            State(state.clone()),
            ConnectInfo(addr),
            AxumPath(handle.as_str().to_string()),
        )
        .await
        .expect("lookup success");
        assert_eq!(response.0.handle, "@lookup.ipn");
        assert_eq!(response.0.owner, encode_address(&owner));
        assert_eq!(
            response.0.metadata.get("alias"),
            Some(&"Lookup".to_string())
        );

        let missing = handle_get_handle(
            State(state.clone()),
            ConnectInfo(addr),
            AxumPath("@unknown.ipn".to_string()),
        )
        .await
        .expect_err("missing");
        assert_eq!(missing.0, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_handle_dev_fund_requires_dev_mode() {
        let base = make_app_state();
        let mut inner = (*base).clone();
        inner.dev_mode = false;
        let state = Arc::new(inner);
        let addr: SocketAddr = "127.0.0.1:9410".parse().unwrap();
        let request = DevFundRequest {
            address: encode_address(&sample_public_key([51u8; 32])),
            amount: 1_000,
            nonce: Some(0),
        };

        let err = handle_dev_fund(State(state), ConnectInfo(addr), ValidatedJson(request))
            .await
            .expect_err("dev mode required");
        assert_eq!(err.0, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_handle_dev_fund_updates_account() {
        let base = make_app_state();
        let mut inner = (*base).clone();
        inner.dev_mode = true;
        let state = Arc::new(inner);
        let addr: SocketAddr = "127.0.0.1:9411".parse().unwrap();
        let target = sample_public_key([52u8; 32]);
        let request = DevFundRequest {
            address: encode_address(&target),
            amount: 5_000,
            nonce: Some(3),
        };

        let response = handle_dev_fund(
            State(state.clone()),
            ConnectInfo(addr),
            ValidatedJson(request),
        )
        .await
        .expect("fund ok")
        .0;
        assert_eq!(response.balance, 5_000);
        assert_eq!(response.nonce, 3);
        assert!(!response.address_hex.is_empty());

        let stored = state
            .storage
            .get_account(&target)
            .expect("storage ok")
            .expect("account exists");
        assert_eq!(stored.balance, 5_000);
        assert_eq!(stored.nonce, 3);
    }

    #[tokio::test]
    async fn test_handle_get_account_payments_endpoint() {
        let state = make_app_state();
        let addr: SocketAddr = "127.0.0.1:9402".parse().unwrap();
        let account_address = sample_public_key([60u8; 32]);
        state
            .storage
            .update_account(Account {
                address: account_address,
                balance: 1_000,
                nonce: 1,
            })
            .expect("account");

        let outgoing = sample_transaction(account_address, sample_public_key([61u8; 32]), 1);
        let incoming = sample_transaction(sample_public_key([62u8; 32]), account_address, 2);
        state
            .storage
            .store_transaction(outgoing.clone())
            .expect("outgoing");
        state
            .storage
            .store_transaction(incoming.clone())
            .expect("incoming");

        let response = handle_get_account_payments(
            State(state.clone()),
            ConnectInfo(addr),
            AxumPath(hex::encode(account_address)),
            Query(PaymentHistoryQuery { limit: Some(1) }),
        )
        .await
        .expect("payments");
        assert_eq!(response.0.len(), 1);
        assert!(
            response.0[0].hash == hex::encode(outgoing.hash())
                || response.0[0].hash == hex::encode(incoming.hash())
        );
    }

    #[tokio::test]
    async fn test_handle_get_account_payments_direction_and_limit() {
        let state = make_app_state();
        let addr: SocketAddr = "127.0.0.1:9403".parse().unwrap();

        let signer = sample_private_key([70u8; 32]);
        let account_address = signer.verifying_key().to_bytes();
        state
            .storage
            .update_account(Account {
                address: account_address,
                balance: 9_000,
                nonce: 0,
            })
            .expect("account");

        let mut outgoing = sample_transaction([70u8; 32], sample_public_key([71u8; 32]), 10);
        outgoing.timestamp = IppanTimeMicros(200);
        let mut incoming = sample_transaction(sample_public_key([72u8; 32]), account_address, 11);
        incoming.timestamp = IppanTimeMicros(300);
        let mut self_transfer = sample_transaction([70u8; 32], account_address, 12);
        self_transfer.timestamp = IppanTimeMicros(100);

        let outgoing_clone = outgoing.clone();
        let incoming_clone = incoming.clone();
        let self_clone = self_transfer.clone();

        state.storage.store_transaction(outgoing).expect("outgoing");
        state.storage.store_transaction(incoming).expect("incoming");
        state
            .storage
            .store_transaction(self_transfer)
            .expect("self");

        let limited = handle_get_account_payments(
            State(state.clone()),
            ConnectInfo(addr),
            AxumPath(hex::encode(account_address)),
            Query(PaymentHistoryQuery { limit: Some(2) }),
        )
        .await
        .expect("limited")
        .0;
        assert_eq!(limited.len(), 2);
        assert_eq!(limited[0].nonce, incoming_clone.nonce);
        assert!(matches!(limited[0].direction, PaymentDirection::Incoming));
        assert!(matches!(limited[1].direction, PaymentDirection::Outgoing));

        let full = handle_get_account_payments(
            State(state.clone()),
            ConnectInfo(addr),
            AxumPath(hex::encode(account_address)),
            Query(PaymentHistoryQuery { limit: None }),
        )
        .await
        .expect("full")
        .0;
        assert_eq!(full.len(), 3);
        assert_eq!(full[2].nonce, self_clone.nonce);
        assert!(matches!(full[2].direction, PaymentDirection::SelfTransfer));
        assert!(matches!(full[0].direction, PaymentDirection::Incoming));
        assert!(matches!(full[1].direction, PaymentDirection::Outgoing));
        assert_eq!(full[1].nonce, outgoing_clone.nonce);
    }

    #[test]
    fn payment_view_includes_total_cost_for_outgoing() {
        let tx = sample_transaction([70u8; 32], sample_public_key([71u8; 32]), 3);
        let outgoing = PaymentView::from_transaction(&tx, Some(&tx.from), PaymentStatus::Finalized);
        let fee = FeePolicy::default().required_fee(&tx) as u128;
        assert_eq!(
            outgoing.total_cost_atomic,
            Some(format_atomic(tx.amount.atomic().saturating_add(fee)))
        );

        let incoming = PaymentView::from_transaction(&tx, Some(&tx.to), PaymentStatus::Finalized);
        assert!(incoming.total_cost_atomic.is_none());
    }

    #[tokio::test]
    async fn test_handle_list_l2_endpoints() {
        let state = make_app_state();
        let network = L2Network {
            id: "demo-l2".to_string(),
            proof_type: "zk".to_string(),
            da_mode: "inline".to_string(),
            status: L2NetworkStatus::Active,
            last_epoch: 1,
            total_commits: 1,
            total_exits: 0,
            last_commit_time: Some(10),
            registered_at: 1,
            challenge_window_ms: Some(60_000),
        };
        state.storage.put_l2_network(network).expect("network");
        state
            .storage
            .store_l2_commit(L2Commit {
                id: "commit-demo".into(),
                l2_id: "demo-l2".into(),
                epoch: 1,
                state_root: "root".into(),
                da_hash: "hash".into(),
                proof_type: "zk".into(),
                proof: None,
                inline_data: None,
                submitted_at: 2,
                hashtimer: "ht".into(),
            })
            .expect("commit");
        state
            .storage
            .store_l2_exit(L2ExitRecord {
                id: "exit-demo".into(),
                l2_id: "demo-l2".into(),
                epoch: 1,
                account: "acct".into(),
                amount: Amount::from_ipn(1),
                nonce: Some(1),
                proof_of_inclusion: "proof".into(),
                status: L2ExitStatus::Pending,
                submitted_at: 3,
                finalized_at: None,
                rejection_reason: None,
                challenge_window_ends_at: None,
            })
            .expect("exit");

        let addr: SocketAddr = "127.0.0.1:6200".parse().unwrap();
        let networks = handle_list_l2_networks(State(state.clone()), ConnectInfo(addr))
            .await
            .expect("networks");
        assert_eq!(networks.0.len(), 1);

        let commits = handle_list_l2_commits(
            State(state.clone()),
            ConnectInfo(addr),
            Query(L2Filter {
                l2_id: Some("demo-l2".into()),
            }),
        )
        .await
        .expect("commits");
        assert_eq!(commits.0.len(), 1);

        let exits = handle_list_l2_exits(
            State(state.clone()),
            ConnectInfo(addr),
            Query(L2Filter {
                l2_id: Some("demo-l2".into()),
            }),
        )
        .await
        .expect("exits");
        assert_eq!(exits.0.len(), 1);
    }

    #[tokio::test]
    async fn test_handle_get_l2_config_and_submit_tx_failure() {
        let state = make_app_state();
        let addr: SocketAddr = "127.0.0.1:9000".parse().unwrap();
        let config = handle_get_l2_config(State(state.clone()), ConnectInfo(addr))
            .await
            .expect("config");
        assert_eq!(config.0.max_l2_count, 1);

        let tx = sample_transaction([2u8; 32], sample_public_key([3u8; 32]), 9);
        let response =
            handle_submit_tx(State(state.clone()), ConnectInfo(addr), ValidatedJson(tx)).await;
        let (status, _) = response.expect_err("consensus unavailable");
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_handle_submit_tx_with_consensus_success_and_failure() {
        let storage: Arc<dyn Storage + Send + Sync> = Arc::new(MemoryStorage::default());
        let mut config = PoAConfig::default();
        config.validators.push(Validator {
            id: sample_public_key([3u8; 32]),
            address: sample_public_key([4u8; 32]),
            stake: 1_000,
            is_active: true,
        });

        let poa = PoAConsensus::new(config, storage.clone(), sample_public_key([9u8; 32]));
        let mempool = poa.mempool();
        let consensus = Arc::new(Mutex::new(poa));

        let (tx_sender_ok, mut rx_ok) = mpsc::unbounded_channel();
        let handle_ok =
            ConsensusHandle::new(consensus.clone(), tx_sender_ok.clone(), mempool.clone());

        let mut ok_state = (*build_app_state(None, None)).clone();
        ok_state.storage = storage.clone();
        ok_state.consensus = Some(handle_ok.clone());
        ok_state.tx_sender = Some(tx_sender_ok);
        ok_state.mempool = mempool.clone();
        let ok_state = Arc::new(ok_state);

        let addr: SocketAddr = "127.0.0.1:9101".parse().unwrap();
        let tx = sample_transaction([5u8; 32], sample_public_key([6u8; 32]), 11);
        let accepted = handle_submit_tx(
            State(ok_state.clone()),
            ConnectInfo(addr),
            ValidatedJson(tx.clone()),
        )
        .await
        .expect("accepted");
        assert_eq!(accepted, "Transaction accepted");
        let received = rx_ok.recv().await.expect("consensus dispatch");
        assert_eq!(received.hash(), tx.hash());

        let (tx_sender_fail, rx_fail) = mpsc::unbounded_channel::<Transaction>();
        drop(rx_fail);
        let handle_fail =
            ConsensusHandle::new(consensus.clone(), tx_sender_fail.clone(), mempool.clone());

        let mut fail_state = (*build_app_state(None, None)).clone();
        fail_state.storage = storage.clone();
        fail_state.consensus = Some(handle_fail);
        fail_state.tx_sender = Some(tx_sender_fail);
        fail_state.mempool = mempool.clone();
        let fail_state = Arc::new(fail_state);

        let rejected = handle_submit_tx(
            State(fail_state),
            ConnectInfo(addr),
            ValidatedJson(sample_transaction(
                [7u8; 32],
                sample_public_key([8u8; 32]),
                12,
            )),
        )
        .await
        .expect_err("submission failed");
        assert_eq!(rejected.0, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_handle_p2p_blocks_and_transactions() {
        let state = make_app_state();
        let addr: SocketAddr = "127.0.0.1:9100".parse().unwrap();
        let tx = sample_transaction([1u8; 32], sample_public_key([2u8; 32]), 3);
        let block = Block::new(vec![], vec![tx.clone()], 2, [7u8; 32]);
        let block_message = NetworkMessage::Block(block.clone());

        let block_result = handle_p2p_blocks(
            State(state.clone()),
            ConnectInfo(addr),
            ValidatedJson(block_message),
        )
        .await
        .expect("block accepted");
        assert_eq!(block_result, "Block accepted");

        let tx_message = NetworkMessage::Transaction(tx.clone());
        let tx_result = handle_p2p_transactions(
            State(state.clone()),
            ConnectInfo(addr),
            ValidatedJson(tx_message),
        )
        .await
        .expect("tx accepted");
        assert_eq!(tx_result, "Transaction accepted");

        let unexpected = handle_p2p_blocks(
            State(state.clone()),
            ConnectInfo(addr),
            ValidatedJson(NetworkMessage::PeerInfo {
                peer_id: "peer".into(),
                addresses: vec![],
                time_us: None,
            }),
        )
        .await;
        assert_eq!(unexpected.unwrap_err().0, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_handle_p2p_peer_info_and_discovery() {
        let state = make_app_state();
        let addr: SocketAddr = "127.0.0.1:7000".parse().unwrap();

        let info = handle_p2p_peer_info(
            State(state.clone()),
            ConnectInfo(addr),
            ValidatedJson(NetworkMessage::PeerInfo {
                peer_id: "peer-1".into(),
                addresses: vec!["http://example.com".into()],
                time_us: Some(1),
            }),
        )
        .await;
        assert_eq!(info.expect("peer info"), "Peer info accepted");

        let discovery = handle_p2p_peer_discovery(
            State(state.clone()),
            ConnectInfo(addr),
            ValidatedJson(NetworkMessage::PeerDiscovery {
                peers: vec!["http://peer".into()],
            }),
        )
        .await;
        assert_eq!(
            discovery.expect("peer discovery"),
            "Peer discovery accepted"
        );

        let unexpected = handle_p2p_peer_info(
            State(state.clone()),
            ConnectInfo(addr),
            ValidatedJson(NetworkMessage::Transaction(sample_transaction(
                [0u8; 32], [1u8; 32], 1,
            ))),
        )
        .await;
        assert_eq!(unexpected.unwrap_err().0, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_forward_to_network_delivers_message() {
        let config = P2PConfig {
            message_timeout: Duration::from_millis(5),
            ..P2PConfig::default()
        };
        let raw_network =
            HttpP2PNetwork::new(config, "http://127.0.0.1:9700".into()).expect("network");
        let mut events = raw_network.take_incoming_events().expect("event receiver");
        let network = Arc::new(raw_network);

        let mut state = (*build_app_state(None, None)).clone();
        state.p2p_network = Some(Arc::clone(&network));
        let state = Arc::new(state);

        let peers = vec!["http://198.51.100.1:9000".into()];
        forward_to_network(
            &state,
            "http://198.51.100.2:9001",
            NetworkMessage::PeerDiscovery {
                peers: peers.clone(),
            },
        )
        .await;

        match events.recv().await.expect("network event") {
            NetworkEvent::PeerDiscovery {
                peers: observed, ..
            } => assert_eq!(observed, peers),
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_handle_p2p_block_request() {
        let state = make_app_state();
        let addr: SocketAddr = "127.0.0.1:7050".parse().unwrap();
        let block = Block::new(vec![], vec![], 9, [4u8; 32]);
        let block_hash = block.hash();
        state
            .storage
            .store_block(block.clone())
            .expect("store block");

        let ok = handle_p2p_block_request(
            State(state.clone()),
            ConnectInfo(addr),
            ValidatedJson(NetworkMessage::BlockRequest { hash: block_hash }),
        )
        .await
        .expect("block response");
        assert!(matches!(ok.0, NetworkMessage::BlockResponse(_)));

        let missing = handle_p2p_block_request(
            State(state.clone()),
            ConnectInfo(addr),
            ValidatedJson(NetworkMessage::BlockRequest { hash: [1u8; 32] }),
        )
        .await
        .expect_err("missing");
        assert_eq!(missing.0, StatusCode::NOT_FOUND);

        let bad = handle_p2p_block_request(
            State(state.clone()),
            ConnectInfo(addr),
            ValidatedJson(NetworkMessage::PeerDiscovery { peers: vec![] }),
        )
        .await
        .expect_err("bad request");
        assert_eq!(bad.0, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_handle_peers_endpoints() {
        let state = make_app_state();
        let addr: SocketAddr = "127.0.0.1:9801".parse().unwrap();
        let peers = handle_get_peers(State(state.clone()), ConnectInfo(addr))
            .await
            .expect("peers");
        assert!(peers.0.is_empty());

        let p2p_peers = handle_get_p2p_peers(State(state), ConnectInfo(addr))
            .await
            .expect("p2p peers");
        assert!(p2p_peers.0.is_empty());

        let config = P2PConfig {
            message_timeout: Duration::from_millis(5),
            ..P2PConfig::default()
        };
        let network =
            Arc::new(HttpP2PNetwork::new(config, "http://127.0.0.1:9800".into()).expect("network"));
        network
            .add_peer("http://203.0.113.1:9001".into())
            .await
            .expect("add peer");

        let mut with_net = (*build_app_state(None, None)).clone();
        with_net.p2p_network = Some(network);
        let with_net = Arc::new(with_net);
        let peers = handle_get_peers(State(with_net.clone()), ConnectInfo(addr))
            .await
            .expect("peers");
        assert_eq!(peers.0.len(), 1);

        let p2p_peers = handle_get_p2p_peers(State(with_net), ConnectInfo(addr))
            .await
            .expect("p2p peers");
        assert_eq!(p2p_peers.0.len(), 1);
    }

    #[tokio::test]
    async fn test_handle_get_transaction_storage_error() {
        let storage: Arc<dyn Storage + Send + Sync> =
            Arc::new(FailingStorage::new(&["get_transaction"]));
        let mut state = (*build_app_state(None, None)).clone();
        state.storage = storage;
        let state = Arc::new(state);

        let addr: SocketAddr = "127.0.0.1:8100".parse().unwrap();
        let hex = "11".repeat(32);
        let err = handle_get_transaction(State(state), ConnectInfo(addr), AxumPath(hex))
            .await
            .expect_err("storage error");
        assert_eq!(err.0, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_handle_get_block_storage_errors() {
        let storage: Arc<dyn Storage + Send + Sync> =
            Arc::new(FailingStorage::new(&["get_block", "get_block_by_height"]));
        let mut state = (*build_app_state(None, None)).clone();
        state.storage = storage;
        let state = Arc::new(state);

        let addr: SocketAddr = "127.0.0.1:8101".parse().unwrap();
        let hash_err = handle_get_block(
            State(state.clone()),
            ConnectInfo(addr),
            AxumPath("22".repeat(32)),
        )
        .await
        .expect_err("hash failure");
        assert_eq!(hash_err.0, StatusCode::INTERNAL_SERVER_ERROR);

        let height_err = handle_get_block(State(state), ConnectInfo(addr), AxumPath("42".into()))
            .await
            .expect_err("height failure");
        assert_eq!(height_err.0, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_handle_get_account_error_paths() {
        let failing = FailingStorage::new(&["get_transactions_by_address"]);
        let account = Account {
            address: sample_public_key([3u8; 32]),
            balance: 5_000,
            nonce: 1,
        };
        failing
            .inner()
            .update_account(account.clone())
            .expect("account");
        let storage_with_tx_error: Arc<dyn Storage + Send + Sync> = Arc::new(failing);

        let mut state = (*build_app_state(None, None)).clone();
        state.storage = storage_with_tx_error.clone();
        let state = Arc::new(state);
        let addr: SocketAddr = "127.0.0.1:8102".parse().unwrap();
        let address_hex = hex::encode(account.address);
        let tx_err = handle_get_account(
            State(state),
            ConnectInfo(addr),
            AxumPath(address_hex.clone()),
        )
        .await
        .expect_err("tx lookup failure");
        assert_eq!(tx_err.0, StatusCode::INTERNAL_SERVER_ERROR);

        let storage_fail_account: Arc<dyn Storage + Send + Sync> =
            Arc::new(FailingStorage::new(&["get_account"]));
        let mut state = (*build_app_state(None, None)).clone();
        state.storage = storage_fail_account;
        let state = Arc::new(state);
        let load_err = handle_get_account(State(state), ConnectInfo(addr), AxumPath(address_hex))
            .await
            .expect_err("account load failure");
        assert_eq!(load_err.0, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_handle_list_l2_endpoints_failures() {
        let storage: Arc<dyn Storage + Send + Sync> = Arc::new(FailingStorage::new(&[
            "list_l2_networks",
            "list_l2_commits",
            "list_l2_exits",
        ]));
        let mut state = (*build_app_state(None, None)).clone();
        state.storage = storage;
        let state = Arc::new(state);

        let addr: SocketAddr = "127.0.0.1:6300".parse().unwrap();

        let networks = handle_list_l2_networks(State(state.clone()), ConnectInfo(addr)).await;
        assert!(matches!(
            networks,
            Err((StatusCode::INTERNAL_SERVER_ERROR, _))
        ));

        let commits = handle_list_l2_commits(
            State(state.clone()),
            ConnectInfo(addr),
            Query(L2Filter::default()),
        )
        .await;
        assert!(matches!(
            commits,
            Err((StatusCode::INTERNAL_SERVER_ERROR, _))
        ));

        let exits =
            handle_list_l2_exits(State(state), ConnectInfo(addr), Query(L2Filter::default())).await;
        assert!(matches!(exits, Err((StatusCode::INTERNAL_SERVER_ERROR, _))));
    }

    #[tokio::test]
    async fn test_p2p_handlers_security_denied() {
        let dir = tempdir().expect("tempdir");
        let config = SecurityConfig {
            enable_ip_whitelist: true,
            whitelisted_ips: vec![],
            audit_log_path: dir.path().join("audit.log").to_string_lossy().to_string(),
            ..SecurityConfig::default()
        };
        let manager = SecurityManager::new(config).expect("manager");
        let state = build_app_state(Some(Arc::new(manager)), None);

        let addr: SocketAddr = "127.0.0.1:8300".parse().unwrap();
        let block = Block::new(vec![], vec![], 1, [9u8; 32]);
        let tx = sample_transaction([1u8; 32], sample_public_key([2u8; 32]), 3);

        let blocked = handle_p2p_blocks(
            State(state.clone()),
            ConnectInfo(addr),
            ValidatedJson(NetworkMessage::Block(block.clone())),
        )
        .await;
        assert_eq!(blocked.unwrap_err().0, StatusCode::FORBIDDEN);

        let blocked_resp = handle_p2p_block_response(
            State(state.clone()),
            ConnectInfo(addr),
            ValidatedJson(NetworkMessage::BlockResponse(block.clone())),
        )
        .await;
        assert_eq!(blocked_resp.unwrap_err().0, StatusCode::FORBIDDEN);

        let blocked_tx = handle_p2p_transactions(
            State(state.clone()),
            ConnectInfo(addr),
            ValidatedJson(NetworkMessage::Transaction(tx.clone())),
        )
        .await;
        assert_eq!(blocked_tx.unwrap_err().0, StatusCode::FORBIDDEN);

        let blocked_info = handle_p2p_peer_info(
            State(state.clone()),
            ConnectInfo(addr),
            ValidatedJson(NetworkMessage::PeerInfo {
                peer_id: "peer".into(),
                addresses: vec!["http://peer".into()],
                time_us: Some(1),
            }),
        )
        .await;
        assert_eq!(blocked_info.unwrap_err().0, StatusCode::FORBIDDEN);

        let blocked_discovery = handle_p2p_peer_discovery(
            State(state.clone()),
            ConnectInfo(addr),
            ValidatedJson(NetworkMessage::PeerDiscovery {
                peers: vec!["http://peer".into()],
            }),
        )
        .await;
        assert_eq!(blocked_discovery.unwrap_err().0, StatusCode::FORBIDDEN);

        let blocked_request = handle_p2p_block_request(
            State(state),
            ConnectInfo(addr),
            ValidatedJson(NetworkMessage::BlockRequest { hash: [0u8; 32] }),
        )
        .await
        .expect_err("blocked request");
        assert_eq!(blocked_request.0, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_misc_endpoints() {
        let state = make_app_state();
        let addr: SocketAddr = "127.0.0.1:8400".parse().unwrap();

        let Json(time) = handle_time(State(state.clone()), ConnectInfo(addr))
            .await
            .expect("time");
        assert!(time.get("timestamp").is_some());

        let Json(version) = handle_version(State(state.clone()), ConnectInfo(addr))
            .await
            .expect("version");
        assert_eq!(
            version.get("version"),
            Some(&serde_json::json!(env!("CARGO_PKG_VERSION")))
        );
        assert_eq!(
            version.get("commit"),
            Some(&serde_json::json!(git_commit_hash()))
        );
        assert_eq!(version.get("mode"), Some(&serde_json::json!("PoA")));

        let metrics_response = handle_metrics(State(state), ConnectInfo(addr))
            .await
            .expect("metrics");
        assert_eq!(metrics_response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_handle_get_metrics_basic() {
        use metrics::{Key, Level, Metadata, Recorder};
        use metrics_exporter_prometheus::PrometheusBuilder;

        let recorder = PrometheusBuilder::new().build_recorder();
        let handle = recorder.handle();
        let key = Key::from_static_name("rpc_test_counter");
        let metadata = Metadata::new("rpc::tests", Level::INFO, Some(module_path!()));
        let counter = recorder.register_counter(&key, &metadata);
        counter.increment(1);

        let mut state = (*make_app_state()).clone();
        state.metrics = Some(handle);
        let state = Arc::new(state);

        let addr: SocketAddr = "127.0.0.1:8450".parse().unwrap();
        let response = handle_metrics(State(state), ConnectInfo(addr))
            .await
            .expect("metrics");
        assert_eq!(response.status(), StatusCode::OK);

        let body = BodyExt::collect(response.into_body())
            .await
            .expect("collect metrics body")
            .to_bytes();
        assert!(!body.is_empty());
    }

    #[tokio::test]
    async fn test_guard_request_with_security_failure() {
        let dir = tempdir().expect("tempdir");
        let config = SecurityConfig {
            enable_ip_whitelist: true,
            whitelisted_ips: vec![],
            audit_log_path: dir.path().join("audit.log").to_string_lossy().to_string(),
            ..SecurityConfig::default()
        };
        let manager = SecurityManager::new(config).expect("manager");
        let state = build_app_state(Some(Arc::new(manager)), None);

        let addr: SocketAddr = "127.0.0.1:9000".parse().unwrap();
        let result = guard_request(&state, &addr, "/health").await;
        assert!(matches!(result, Err(SecurityError::IpNotWhitelisted)));
    }

    #[tokio::test]
    async fn test_guard_request_rate_limits_abusive_ip() {
        let dir = tempdir().expect("tempdir");
        let rate_limit = RateLimitConfig {
            requests_per_second: 2,
            burst_capacity: 2,
            endpoint_limits: HashMap::new(),
            global_requests_per_second: Some(10),
        };
        let config = SecurityConfig {
            rate_limit,
            audit_log_path: dir.path().join("audit.log").to_string_lossy().to_string(),
            ..SecurityConfig::default()
        };

        let manager = SecurityManager::new(config).expect("manager");
        let state = build_app_state(Some(Arc::new(manager)), None);

        let addr: SocketAddr = "127.0.0.1:9050".parse().unwrap();

        assert!(guard_request(&state, &addr, "/health").await.is_ok());
        assert!(guard_request(&state, &addr, "/health").await.is_ok());

        let result = guard_request(&state, &addr, "/health").await;
        assert!(matches!(result, Err(SecurityError::RateLimitExceeded)));

        sleep(Duration::from_millis(600)).await;

        assert!(guard_request(&state, &addr, "/health").await.is_ok());
    }

    #[tokio::test]
    async fn test_guard_request_blocks_after_repeated_failures_then_recovers() {
        let dir = tempdir().expect("tempdir");
        let mut config = SecurityConfig {
            max_failed_attempts: 3,
            block_duration: 1,
            audit_log_path: dir.path().join("audit.log").to_string_lossy().to_string(),
            ..SecurityConfig::default()
        };

        config.rate_limit.endpoint_limits.clear();
        let manager = SecurityManager::new(config).expect("manager");
        let state = build_app_state(Some(Arc::new(manager)), None);

        let addr: SocketAddr = "127.0.0.1:9051".parse().unwrap();
        let endpoint = "/auth";

        for _ in 0..3 {
            state
                .security
                .as_ref()
                .unwrap()
                .record_failed_attempt(addr.ip(), endpoint, "invalid")
                .await
                .unwrap();
        }

        let blocked = guard_request(&state, &addr, endpoint).await;
        assert!(matches!(blocked, Err(SecurityError::IpBlocked)));

        sleep(Duration::from_millis(1100)).await;
        state
            .security
            .as_ref()
            .unwrap()
            .cleanup_expired_blocks()
            .await;

        assert!(guard_request(&state, &addr, endpoint).await.is_ok());
    }

    #[tokio::test]
    async fn test_build_router_static_dir_branches() {
        let dir = tempdir().expect("tempdir");
        let existing = dir.path().join("dist");
        fs::create_dir_all(&existing).expect("create dist dir");

        let state_with_ui = build_app_state(None, Some(existing.clone()));
        let _router = build_router(state_with_ui);

        let missing = dir.path().join("missing");
        let state_missing = build_app_state(None, Some(missing));
        let _router_missing = build_router(state_missing);
    }

    #[tokio::test]
    async fn test_start_server_launch_and_abort() {
        let base = build_app_state(None, None);
        let addr = "127.0.0.1:0";
        let server = tokio::spawn(start_server((*base).clone(), addr));

        sleep(Duration::from_millis(25)).await;
        server.abort();
        let result = server.await.expect_err("server aborted");
        assert!(result.is_cancelled());
    }

    #[tokio::test]
    async fn test_bind_listener_ephemeral() {
        let listener = bind_listener("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        assert_eq!(addr.ip(), IpAddr::from(Ipv4Addr::LOCALHOST));
    }

    #[tokio::test]
    async fn test_consensus_handle_snapshot_and_submit() {
        let storage: Arc<dyn Storage + Send + Sync> = Arc::new(MemoryStorage::default());
        let mut config = PoAConfig::default();
        config.validators.push(Validator {
            id: sample_public_key([42u8; 32]),
            address: sample_public_key([42u8; 32]),
            stake: 1_000,
            is_active: true,
        });

        let poa = PoAConsensus::new(config, storage, sample_public_key([42u8; 32]));
        let mempool = poa.mempool();
        let consensus = Arc::new(Mutex::new(poa));
        let (tx_sender, mut rx) = mpsc::unbounded_channel();
        let handle = ConsensusHandle::new(consensus.clone(), tx_sender, mempool);

        let snapshot = handle.snapshot().await.expect("snapshot");
        assert_eq!(snapshot.validators.len(), 1);

        let tx = sample_transaction([1u8; 32], sample_public_key([2u8; 32]), 1);
        handle.submit_transaction(tx.clone()).expect("submit");
        let received = rx.recv().await.expect("recv");
        assert_eq!(received.hash(), tx.hash());
    }

    #[tokio::test]
    async fn test_resolve_peer_address_with_metadata() {
        let storage: Arc<dyn Storage + Send + Sync> = Arc::new(MemoryStorage::default());
        let config = P2PConfig {
            message_timeout: std::time::Duration::from_millis(5),
            ..P2PConfig::default()
        };
        let network =
            Arc::new(HttpP2PNetwork::new(config, "http://127.0.0.1:9550".into()).expect("network"));

        network
            .add_peer("http://203.0.113.10:9001".into())
            .await
            .expect("add peer");

        let file_storage: Arc<dyn FileStorage> = Arc::new(MemoryFileStorage::new());
        let file_dht: Arc<dyn FileDhtService> = Arc::new(StubFileDhtService::new());
        let handle_registry = Arc::new(L2HandleRegistry::new());
        let handle_anchors = Arc::new(L1HandleAnchorStorage::new());
        let handle_dht: Arc<dyn HandleDhtService> = Arc::new(StubHandleDhtService::new());
        let state = Arc::new(AppState {
            storage,
            start_time: Instant::now(),
            peer_count: Arc::new(AtomicUsize::new(0)),
            p2p_network: Some(network),
            tx_sender: None,
            node_id: "test-node".into(),
            consensus_mode: "poa".into(),
            consensus: None,
            ai_status: None,
            l2_config: L2Config {
                max_commit_size: 512,
                min_epoch_gap_ms: 1_000,
                challenge_window_ms: 2_000,
                da_mode: "test".into(),
                max_l2_count: 1,
            },
            mempool: Arc::new(Mempool::new(10)),
            unified_ui_dist: None,
            req_count: Arc::new(AtomicUsize::new(0)),
            security: None,
            metrics: None,
            file_storage: Some(file_storage),
            file_dht: Some(file_dht),
            dht_file_mode: "stub".into(),
            dev_mode: true,
            handle_registry,
            handle_anchors,
            handle_dht: Some(handle_dht),
            dht_handle_mode: "stub".into(),
        });

        let socket: SocketAddr = "203.0.113.10:9100".parse().unwrap();
        let resolved = resolve_peer_address(
            &state,
            &socket,
            &NetworkMessage::PeerDiscovery { peers: vec![] },
        );

        assert!(resolved.contains("203.0.113.10"));
    }
}
