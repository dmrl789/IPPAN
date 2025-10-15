use std::{
    collections::{BTreeMap, HashSet},
    net::SocketAddr,
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Instant,
};

use anyhow::{anyhow, Context, Result};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use ippan_consensus::{ConsensusState, PoAConsensus, Validator};
use ippan_p2p::HttpP2PNetwork;
use ippan_storage::{Account, SledStorage, Storage};
use ippan_types::{HashTimer, Transaction};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::{
    net::TcpListener,
    sync::{mpsc, Mutex},
};
use tower_http::{services::ServeDir, trace::TraceLayer};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2Config {
    pub max_commit_size: usize,
    pub min_epoch_gap_ms: u64,
    pub challenge_window_ms: u64,
    pub da_mode: String,
    pub max_l2_count: usize,
}

pub struct AppState {
    pub storage: Arc<SledStorage>,
    pub start_time: Instant,
    pub peer_count: Arc<AtomicUsize>,
    pub p2p_network: Option<Arc<HttpP2PNetwork>>,
    pub tx_sender: Option<mpsc::UnboundedSender<Transaction>>,
    pub node_id: String,
    pub consensus: Option<ConsensusHandle>,
    pub l2_config: L2Config,
    pub mempool: Arc<RwLock<Vec<Transaction>>>,
    pub unified_ui_dist: Option<PathBuf>,
    pub req_count: Arc<AtomicUsize>,
}

#[derive(Clone)]
pub struct ConsensusHandle {
    consensus: Arc<Mutex<PoAConsensus>>,
    tx_sender: mpsc::UnboundedSender<Transaction>,
    mempool: Arc<RwLock<Vec<Transaction>>>,
}

impl ConsensusHandle {
    pub fn new(
        consensus: Arc<Mutex<PoAConsensus>>,
        tx_sender: mpsc::UnboundedSender<Transaction>,
        mempool: Arc<RwLock<Vec<Transaction>>>,
    ) -> Self {
        Self {
            consensus,
            tx_sender,
            mempool,
        }
    }

    pub async fn get_state(&self) -> ConsensusState {
        let guard = self.consensus.lock().await;
        guard.get_state()
    }

    pub async fn get_validators(&self) -> Vec<Validator> {
        let guard = self.consensus.lock().await;
        guard.get_validators().to_vec()
    }

    pub fn mempool(&self) -> Arc<RwLock<Vec<Transaction>>> {
        self.mempool.clone()
    }

    pub fn submit_transaction(&self, tx: Transaction) -> Result<()> {
        self.tx_sender
            .send(tx)
            .map_err(|err| anyhow!("failed to enqueue transaction: {err}"))
    }
}

#[derive(Debug)]
enum ApiError {
    BadRequest(String),
    NotFound(String),
    Internal(String),
}

impl ApiError {
    fn bad_request<M: Into<String>>(message: M) -> Self {
        Self::BadRequest(message.into())
    }

    fn not_found<M: Into<String>>(message: M) -> Self {
        Self::NotFound(message.into())
    }

    fn internal<M: Into<String>>(message: M) -> Self {
        Self::Internal(message.into())
    }
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    success: bool,
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(ErrorResponse {
            success: false,
            error: message,
        });

        (status, body).into_response()
    }
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    uptime_seconds: u64,
    requests_served: u64,
    peers: usize,
}

#[derive(Debug, Serialize)]
struct NodeInfo {
    is_running: bool,
    uptime_seconds: u64,
    version: String,
    node_id: String,
}

#[derive(Debug, Serialize)]
struct NetworkInfo {
    total_peers: usize,
    connected_peers: usize,
    network_id: String,
    protocol_version: String,
    peer_id: Option<String>,
    listen_address: Option<String>,
    p2p_enabled: bool,
}

#[derive(Debug, Serialize)]
struct ConsensusOverview {
    current_round: u64,
    latest_block_height: u64,
    validator_count: usize,
    proposer: Option<String>,
    is_proposing: bool,
}

#[derive(Debug, Serialize)]
struct NodeStatusResponse {
    node_id: String,
    status: &'static str,
    current_block: u64,
    total_transactions: u64,
    network_peers: usize,
    uptime_seconds: u64,
    version: String,
    node: NodeInfo,
    network: NetworkInfo,
    consensus: Option<ConsensusOverview>,
    l2_config: L2Config,
}

#[derive(Debug, Serialize)]
struct NetworkStatsResponse {
    total_peers: usize,
    connected_peers: usize,
    network_id: String,
    protocol_version: String,
    uptime_seconds: u64,
}

#[derive(Debug, Serialize)]
struct MempoolStatsResponse {
    total_transactions: usize,
    total_senders: usize,
    total_size: usize,
    fee_distribution: BTreeMap<String, usize>,
}

#[derive(Debug, Serialize)]
struct ConsensusStatsResponse {
    current_round: u64,
    validators_count: usize,
    block_height: u64,
    consensus_status: &'static str,
}

#[derive(Debug, Serialize)]
struct ValidatorInfoResponse {
    node_id: String,
    address: String,
    stake_amount: u64,
    is_active: bool,
}

#[derive(Debug, Serialize)]
struct TransactionView {
    id: String,
    from: String,
    to: String,
    amount: u64,
    nonce: u64,
    timestamp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    direction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hashtimer: Option<String>,
}

#[derive(Debug, Serialize)]
struct TransactionsResponse {
    transactions: Vec<TransactionView>,
}

#[derive(Debug, Serialize)]
struct BlockSummaryResponse {
    height: u64,
    hash: String,
    parent_hashes: Vec<String>,
    proposer: String,
    transaction_count: usize,
    timestamp_micros: u64,
}

#[derive(Debug, Serialize)]
struct RecentBlocksResponse {
    latest_height: u64,
    blocks: Vec<BlockSummaryResponse>,
}

#[derive(Debug, Serialize)]
struct BlockDetailResponse {
    block: BlockDetail,
}

#[derive(Debug, Serialize)]
struct BlockDetail {
    height: u64,
    hash: String,
    parent_hashes: Vec<String>,
    proposer: String,
    transaction_count: usize,
    timestamp_micros: u64,
    transactions: Vec<TransactionView>,
}

#[derive(Debug, Serialize)]
struct BalanceResponse {
    account: String,
    address: String,
    balance: u64,
    staked: u64,
    rewards: u64,
    nonce: u64,
    pending_transactions: Vec<String>,
}

#[derive(Debug, Serialize)]
struct AddressValidationResponse {
    valid: bool,
}

#[derive(Debug, Serialize)]
struct PeersResponse {
    peers: Vec<String>,
}

#[derive(Debug, Serialize)]
struct SubmitTransactionResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<SubmitTransactionData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct SubmitTransactionData {
    tx_hash: String,
}

#[derive(Debug, Deserialize)]
struct RecentBlocksQuery {
    #[serde(default = "default_blocks_limit")]
    limit: usize,
}

#[derive(Debug, Deserialize)]
struct TransactionsQuery {
    address: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct BalanceQuery {
    address: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AddressValidationQuery {
    address: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SubmitTransactionRequest {
    from: String,
    to: String,
    amount: u64,
    nonce: u64,
    #[serde(default)]
    signature: Option<String>,
    #[serde(default)]
    hashtimer: Option<String>,
    #[serde(default)]
    _fee: Option<u64>,
}

const VERSION: &str = env!("CARGO_PKG_VERSION");
const MAX_BLOCK_QUERY: usize = 100;

fn default_blocks_limit() -> usize {
    20
}

fn record_request(state: &Arc<AppState>) {
    state.req_count.fetch_add(1, Ordering::Relaxed);
}

fn strip_hex_prefix(value: &str) -> &str {
    value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .unwrap_or(value)
}

fn decode_hex<const N: usize>(value: &str) -> Result<[u8; N], ApiError> {
    let clean = strip_hex_prefix(value);
    let bytes = hex::decode(clean)
        .map_err(|e| ApiError::bad_request(format!("invalid hex string: {e}")))?;
    if bytes.len() != N {
        return Err(ApiError::bad_request(format!(
            "expected {N} bytes, got {}",
            bytes.len()
        )));
    }
    let mut array = [0u8; N];
    array.copy_from_slice(&bytes);
    Ok(array)
}

fn transaction_to_view(tx: &Transaction, perspective: Option<&[u8; 32]>) -> TransactionView {
    let direction = perspective.and_then(|addr| {
        if &tx.from == addr {
            Some("outgoing".to_string())
        } else if &tx.to == addr {
            Some("incoming".to_string())
        } else {
            None
        }
    });

    TransactionView {
        id: hex::encode(tx.id),
        from: hex::encode(tx.from),
        to: hex::encode(tx.to),
        amount: tx.amount,
        nonce: tx.nonce,
        timestamp: tx.timestamp.0,
        direction,
        hashtimer: Some(tx.hashtimer.to_hex()),
    }
}

fn build_block_summary(block: &ippan_types::Block) -> BlockSummaryResponse {
    let parent_hashes = if !block.header.prev_hashes.is_empty() {
        block.header.prev_hashes.clone()
    } else {
        block.header.parent_ids.iter().map(hex::encode).collect()
    };

    BlockSummaryResponse {
        height: block.header.round,
        hash: hex::encode(block.hash()),
        parent_hashes,
        proposer: hex::encode(block.header.creator),
        transaction_count: block.transactions.len(),
        timestamp_micros: block.header.hashtimer.time().0,
    }
}

fn build_block_detail(block: &ippan_types::Block) -> BlockDetail {
    let summary = build_block_summary(block);
    let transactions = block
        .transactions
        .iter()
        .map(|tx| transaction_to_view(tx, None))
        .collect();

    BlockDetail {
        height: summary.height,
        hash: summary.hash,
        parent_hashes: summary.parent_hashes,
        proposer: summary.proposer,
        transaction_count: summary.transaction_count,
        timestamp_micros: summary.timestamp_micros,
        transactions,
    }
}

fn gather_pending_transactions(
    mempool: &Arc<RwLock<Vec<Transaction>>>,
    address: &[u8; 32],
) -> Vec<String> {
    let guard = mempool.read();
    guard
        .iter()
        .filter(|tx| &tx.from == address || &tx.to == address)
        .map(|tx| hex::encode(tx.id))
        .collect()
}

pub async fn start_server(app_state: AppState, addr: &str) -> Result<()> {
    let static_dir = app_state.unified_ui_dist.clone();
    let state = Arc::new(app_state);

    let mut router = Router::new()
        .route("/health", get(health_handler))
        .route("/api/v1/status", get(status_handler))
        .route("/api/v1/network", get(network_handler))
        .route("/api/v1/mempool", get(mempool_handler))
        .route("/api/v1/consensus", get(consensus_handler))
        .route("/api/v1/validators", get(validators_handler))
        .route("/api/v1/blocks/recent", get(recent_blocks_handler))
        .route("/api/v1/blocks/:height", get(block_by_height_handler))
        .route("/api/v1/balance", get(balance_query_handler))
        .route("/api/v1/balance/:address", get(balance_path_handler))
        .route("/api/v1/transactions", get(transactions_handler))
        .route("/api/v1/transaction", post(submit_transaction_handler))
        .route("/api/v1/address/validate", get(address_validation_handler))
        .route("/p2p/peers", get(peers_handler))
        .with_state(state.clone())
        .layer(TraceLayer::new_for_http());

    if let Some(dir) = static_dir {
        router = router.nest_service(
            "/",
            ServeDir::new(dir).append_index_html_on_directories(true),
        );
    }

    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind RPC server on {addr}"))?;

    let socket: SocketAddr = listener.local_addr()?;
    tracing::info!("RPC server listening on {socket}");

    axum::serve(listener, router)
        .await
        .context("RPC server terminated unexpectedly")?;
    Ok(())
}

async fn health_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<HealthResponse>, ApiError> {
    record_request(&state);
    let uptime = state.start_time.elapsed().as_secs();
    let requests = state.req_count.load(Ordering::Relaxed) as u64;
    let peers = state.peer_count.load(Ordering::Relaxed);

    Ok(Json(HealthResponse {
        status: "ok",
        uptime_seconds: uptime,
        requests_served: requests,
        peers,
    }))
}

async fn status_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<NodeStatusResponse>, ApiError> {
    record_request(&state);

    let latest_height = state
        .storage
        .get_latest_height()
        .map_err(|e| ApiError::internal(format!("storage error: {e}")))?;
    let total_transactions = state
        .storage
        .get_transaction_count()
        .map_err(|e| ApiError::internal(format!("storage error: {e}")))?;
    let peers = state.peer_count.load(Ordering::Relaxed);
    let uptime = state.start_time.elapsed().as_secs();

    let consensus = if let Some(handle) = &state.consensus {
        let snapshot = handle.get_state().await;
        Some(ConsensusOverview {
            current_round: snapshot.current_round,
            latest_block_height: snapshot.latest_block_height,
            validator_count: snapshot.validator_count,
            proposer: snapshot.current_proposer.map(hex::encode),
            is_proposing: snapshot.is_proposing,
        })
    } else {
        None
    };

    let (peer_id, listen_address, connected_peers, p2p_enabled) =
        if let Some(network) = &state.p2p_network {
            (
                Some(network.get_local_peer_id()),
                Some(network.get_listening_address()),
                network.get_peer_count(),
                true,
            )
        } else {
            (None, None, peers, false)
        };

    let response = NodeStatusResponse {
        node_id: state.node_id.clone(),
        status: "ok",
        current_block: latest_height,
        total_transactions,
        network_peers: peers,
        uptime_seconds: uptime,
        version: VERSION.to_string(),
        node: NodeInfo {
            is_running: true,
            uptime_seconds: uptime,
            version: VERSION.to_string(),
            node_id: state.node_id.clone(),
        },
        network: NetworkInfo {
            total_peers: peers,
            connected_peers,
            network_id: state.node_id.clone(),
            protocol_version: VERSION.to_string(),
            peer_id,
            listen_address,
            p2p_enabled,
        },
        consensus,
        l2_config: state.l2_config.clone(),
    };

    Ok(Json(response))
}

async fn network_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<NetworkStatsResponse>, ApiError> {
    record_request(&state);
    let peers = state.peer_count.load(Ordering::Relaxed);
    let connected = state
        .p2p_network
        .as_ref()
        .map(|network| network.get_peer_count())
        .unwrap_or(peers);
    let uptime = state.start_time.elapsed().as_secs();

    Ok(Json(NetworkStatsResponse {
        total_peers: peers,
        connected_peers: connected,
        network_id: state.node_id.clone(),
        protocol_version: VERSION.to_string(),
        uptime_seconds: uptime,
    }))
}

async fn mempool_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<MempoolStatsResponse>, ApiError> {
    record_request(&state);
    let guard = state.mempool.read();
    let total_transactions = guard.len();
    let unique_senders: HashSet<[u8; 32]> = guard.iter().map(|tx| tx.from).collect();

    Ok(Json(MempoolStatsResponse {
        total_transactions,
        total_senders: unique_senders.len(),
        total_size: total_transactions,
        fee_distribution: BTreeMap::new(),
    }))
}

async fn consensus_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ConsensusStatsResponse>, ApiError> {
    record_request(&state);
    let Some(handle) = &state.consensus else {
        return Err(ApiError::internal("consensus handle unavailable"));
    };
    let snapshot = handle.get_state().await;

    Ok(Json(ConsensusStatsResponse {
        current_round: snapshot.current_round,
        validators_count: snapshot.validator_count,
        block_height: snapshot.latest_block_height,
        consensus_status: if snapshot.validator_count > 0 {
            "running"
        } else {
            "idle"
        },
    }))
}

async fn validators_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ValidatorInfoResponse>>, ApiError> {
    record_request(&state);
    let Some(handle) = &state.consensus else {
        return Err(ApiError::internal("consensus handle unavailable"));
    };
    let validators = handle.get_validators().await;

    let response = validators
        .into_iter()
        .map(|validator| ValidatorInfoResponse {
            node_id: hex::encode(validator.id),
            address: hex::encode(validator.address),
            stake_amount: validator.stake,
            is_active: validator.is_active,
        })
        .collect();

    Ok(Json(response))
}

async fn recent_blocks_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<RecentBlocksQuery>,
) -> Result<Json<RecentBlocksResponse>, ApiError> {
    record_request(&state);
    let limit = query.limit.min(MAX_BLOCK_QUERY).max(1);
    let latest_height = state
        .storage
        .get_latest_height()
        .map_err(|e| ApiError::internal(format!("storage error: {e}")))?;

    let mut blocks = Vec::new();
    let mut height = latest_height;
    while blocks.len() < limit && height > 0 {
        match state
            .storage
            .get_block_by_height(height)
            .map_err(|e| ApiError::internal(format!("storage error: {e}")))?
        {
            Some(block) => blocks.push(build_block_summary(&block)),
            None => break,
        }
        height = height.saturating_sub(1);
    }

    Ok(Json(RecentBlocksResponse {
        latest_height,
        blocks,
    }))
}

async fn block_by_height_handler(
    State(state): State<Arc<AppState>>,
    Path(height): Path<u64>,
) -> Result<Json<BlockDetailResponse>, ApiError> {
    record_request(&state);
    let block = state
        .storage
        .get_block_by_height(height)
        .map_err(|e| ApiError::internal(format!("storage error: {e}")))?
        .ok_or_else(|| ApiError::not_found(format!("block {height} not found")))?;

    Ok(Json(BlockDetailResponse {
        block: build_block_detail(&block),
    }))
}

async fn balance_query_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<BalanceQuery>,
) -> Result<Json<BalanceResponse>, ApiError> {
    record_request(&state);
    let address_str = query
        .address
        .ok_or_else(|| ApiError::bad_request("address query parameter is required"))?;
    balance_for_address(&state, &address_str).await
}

async fn balance_path_handler(
    State(state): State<Arc<AppState>>,
    Path(address): Path<String>,
) -> Result<Json<BalanceResponse>, ApiError> {
    record_request(&state);
    balance_for_address(&state, &address).await
}

async fn balance_for_address(
    state: &Arc<AppState>,
    address: &str,
) -> Result<Json<BalanceResponse>, ApiError> {
    let address_bytes = decode_hex::<32>(address)?;
    let account_opt = state
        .storage
        .get_account(&address_bytes)
        .map_err(|e| ApiError::internal(format!("storage error: {e}")))?;

    let account = account_opt.unwrap_or(Account {
        address: address_bytes,
        balance: 0,
        nonce: 0,
    });

    let pending = gather_pending_transactions(&state.mempool, &address_bytes);

    Ok(Json(BalanceResponse {
        account: hex::encode(account.address),
        address: hex::encode(account.address),
        balance: account.balance,
        staked: 0,
        rewards: 0,
        nonce: account.nonce,
        pending_transactions: pending,
    }))
}

async fn transactions_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<TransactionsQuery>,
) -> Result<Json<TransactionsResponse>, ApiError> {
    record_request(&state);
    let address_str = query
        .address
        .ok_or_else(|| ApiError::bad_request("address query parameter is required"))?;
    let address_bytes = decode_hex::<32>(&address_str)?;

    let mut transactions = state
        .storage
        .get_transactions_by_address(&address_bytes)
        .map_err(|e| ApiError::internal(format!("storage error: {e}")))?;

    if let Some(limit) = query.limit {
        if transactions.len() > limit {
            transactions.truncate(limit);
        }
    }

    let response = transactions
        .iter()
        .map(|tx| transaction_to_view(tx, Some(&address_bytes)))
        .collect();

    Ok(Json(TransactionsResponse {
        transactions: response,
    }))
}

async fn submit_transaction_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SubmitTransactionRequest>,
) -> Result<Json<SubmitTransactionResponse>, ApiError> {
    record_request(&state);

    let Some(signature_hex) = payload.signature.as_ref() else {
        return Ok(Json(SubmitTransactionResponse {
            success: false,
            data: None,
            error: Some("signature is required".to_string()),
        }));
    };

    let from = decode_hex::<32>(&payload.from)?;
    let to = decode_hex::<32>(&payload.to)?;
    let mut tx = Transaction::new(from, to, payload.amount, payload.nonce);

    let signature = decode_hex::<64>(signature_hex)?;
    tx.signature.copy_from_slice(&signature);
    if let Some(hashtimer_hex) = payload.hashtimer.as_ref() {
        let hashtimer =
            HashTimer::from_hex(strip_hex_prefix(hashtimer_hex)).map_err(ApiError::bad_request)?;
        tx.hashtimer = hashtimer;
    }
    tx.refresh_id();

    if !tx.verify() {
        return Ok(Json(SubmitTransactionResponse {
            success: false,
            data: None,
            error: Some("invalid transaction signature".to_string()),
        }));
    }

    let Some(handle) = &state.consensus else {
        return Err(ApiError::internal("consensus handle unavailable"));
    };

    handle
        .submit_transaction(tx.clone())
        .map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(Json(SubmitTransactionResponse {
        success: true,
        data: Some(SubmitTransactionData {
            tx_hash: hex::encode(tx.id),
        }),
        error: None,
    }))
}

async fn address_validation_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AddressValidationQuery>,
) -> Result<Json<AddressValidationResponse>, ApiError> {
    record_request(&state);
    let Some(address) = query.address else {
        return Err(ApiError::bad_request("address query parameter is required"));
    };

    let valid = decode_hex::<32>(&address).is_ok();
    Ok(Json(AddressValidationResponse { valid }))
}

async fn peers_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<PeersResponse>, ApiError> {
    record_request(&state);
    let peers = state
        .p2p_network
        .as_ref()
        .map(|network| network.get_peers())
        .unwrap_or_default();

    Ok(Json(PeersResponse { peers }))
}
