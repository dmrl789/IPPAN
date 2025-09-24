use anyhow::Result;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use ippan_consensus::{ConsensusState, PoAConsensus};
use ippan_storage::Storage;
use ippan_types::{
    decode_address, encode_address, ippan_time_now, is_valid_address, Block, IppanTimeMicros,
    L2Exit, L2ExitStatus, L2ProofType, L2StateCommit, Transaction,
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{error, info, warn};

/// RPC response wrapper
#[derive(Debug, Serialize)]
pub struct RpcResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

/// Static bridge policy used by RPC handlers.
#[derive(Debug, Clone)]
pub struct L2Config {
    /// Maximum size allowed for commitment payloads (proof + inline data) in bytes.
    pub max_commit_size: usize,
    /// Minimum gap between commitments for the same rollup in milliseconds.
    pub min_epoch_gap_ms: u64,
    /// Challenge window applied to exits in milliseconds.
    pub challenge_window_ms: u64,
    /// Maximum number of simultaneously registered L2 networks.
    pub max_l2_count: usize,
    /// Default data availability mode for new commits.
    pub da_mode: String,
}

impl Default for L2Config {
    fn default() -> Self {
        Self {
            max_commit_size: 16 * 1024,
            min_epoch_gap_ms: 250,
            challenge_window_ms: 60_000,
            max_l2_count: 100,
            da_mode: "external".to_string(),
        }
    }
}

impl<T> RpcResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}

/// Submit transaction request
#[derive(Debug, Deserialize)]
pub struct SubmitTransactionRequest {
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub nonce: u64,
    pub signature: String,
}

/// Submit L2 commitment request
#[derive(Debug, Deserialize)]
pub struct L2CommitRequest {
    pub l2_id: String,
    pub epoch: u64,
    pub state_root: String,
    pub da_hash: Option<String>,
    pub proof_type: Option<L2ProofType>,
    pub proof: Option<String>,
    pub inline_data: Option<String>,
}

/// Response returned after storing an L2 commitment
#[derive(Debug, Serialize)]
pub struct L2CommitResponse {
    pub l2_id: String,
    pub epoch: u64,
    pub commit_id: String,
    pub hashtimer: String,
    pub timestamp: u64,
}

/// L2 exit verification request
#[derive(Debug, Deserialize)]
pub struct L2ExitVerificationRequest {
    pub l2_id: String,
    pub epoch: u64,
    pub proof_of_inclusion: String,
    pub account: String,
    pub amount: f64,
    pub nonce: u64,
}

/// Response returned after accepting an L2 exit
#[derive(Debug, Serialize)]
pub struct L2ExitVerificationResponse {
    pub exit_id: String,
    pub status: L2ExitStatus,
    pub hashtimer: String,
    pub submitted_at: u64,
    pub challenge_window_ends_at: u64,
}

#[derive(Debug, Deserialize)]
struct L2FilterQuery {
    l2_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AddressQuery {
    address: String,
}

/// Submit transaction response
#[derive(Debug, Serialize)]
pub struct SubmitTransactionResponse {
    pub tx_hash: String,
}

/// Get block query parameters
#[derive(Debug, Deserialize)]
pub struct GetBlockQuery {
    pub hash: Option<String>,
    pub height: Option<u64>,
}

/// Get account response
#[derive(Debug, Serialize)]
pub struct GetAccountResponse {
    pub address: String,
    pub balance: u64,
    pub nonce: u64,
}

/// Get time response
#[derive(Debug, Serialize)]
pub struct GetTimeResponse {
    pub time_us: u64,
}

/// Node status response
#[derive(Debug, Serialize)]
pub struct NodeStatusResponse {
    pub node_id: String,
    pub version: String,
    pub latest_height: u64,
    pub uptime_seconds: u64,
    pub peer_count: usize,
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: u64,
}

#[derive(Clone)]
pub struct ConsensusHandle {
    inner: Arc<Mutex<PoAConsensus>>,
    tx_sender: mpsc::UnboundedSender<Transaction>,
    mempool: Arc<RwLock<Vec<Transaction>>>,
}

impl ConsensusHandle {
    pub fn new(
        inner: Arc<Mutex<PoAConsensus>>,
        tx_sender: mpsc::UnboundedSender<Transaction>,
        mempool: Arc<RwLock<Vec<Transaction>>>,
    ) -> Self {
        Self {
            inner,
            tx_sender,
            mempool,
        }
    }

    pub fn sender(&self) -> mpsc::UnboundedSender<Transaction> {
        self.tx_sender.clone()
    }

    pub async fn state(&self) -> ConsensusState {
        let consensus = self.inner.lock().await;
        consensus.get_state()
    }

    pub fn mempool_snapshot(&self) -> Vec<Transaction> {
        self.mempool.read().clone()
    }

    pub fn mempool_metrics(&self) -> (usize, usize) {
        let mempool = self.mempool.read();
        let total = mempool.len();
        let unique: HashSet<[u8; 32]> = mempool.iter().map(|tx| tx.from).collect();
        (total, unique.len())
    }

    pub fn pending_for_address(&self, address: &[u8; 32]) -> Vec<Transaction> {
        self.mempool
            .read()
            .iter()
            .filter(|tx| &tx.from == address || &tx.to == address)
            .cloned()
            .collect()
    }
}

#[derive(Debug, Serialize)]
pub struct NodeInfoV1 {
    pub is_running: bool,
    pub uptime_seconds: u64,
    pub version: String,
    pub node_id: String,
}

#[derive(Debug, Serialize)]
pub struct NetworkInfoV1 {
    pub connected_peers: usize,
    pub known_peers: usize,
    pub total_peers: usize,
}

#[derive(Debug, Serialize)]
pub struct MempoolInfoV1 {
    pub total_transactions: usize,
    pub pending_transactions: usize,
}

#[derive(Debug, Serialize)]
pub struct BlockchainInfoV1 {
    pub current_height: u64,
    pub total_blocks: u64,
    pub total_transactions: u64,
}

#[derive(Debug, Serialize)]
pub struct NodeStatusV1 {
    pub node_id: String,
    pub status: String,
    pub current_block: u64,
    pub total_transactions: u64,
    pub network_peers: usize,
    pub uptime_seconds: u64,
    pub version: String,
    pub node: NodeInfoV1,
    pub network: NetworkInfoV1,
    pub mempool: MempoolInfoV1,
    pub blockchain: BlockchainInfoV1,
}

#[derive(Debug, Serialize)]
pub struct NetworkStatsV1 {
    pub total_peers: usize,
    pub connected_peers: usize,
    pub network_id: String,
    pub protocol_version: String,
    pub uptime_seconds: u64,
}

#[derive(Debug, Serialize)]
pub struct MempoolStatsV1 {
    pub total_transactions: usize,
    pub total_senders: usize,
    pub total_size: usize,
    pub fee_distribution: HashMap<String, u64>,
}

#[derive(Debug, Serialize)]
pub struct ConsensusStatsV1 {
    pub current_round: u64,
    pub validators_count: usize,
    pub block_height: u64,
    pub consensus_status: String,
}

#[derive(Debug, Serialize)]
pub struct BalanceResponseV1 {
    pub account: String,
    pub address: String,
    pub balance: u64,
    pub staked: u64,
    pub staked_amount: u64,
    pub rewards: u64,
    pub nonce: u64,
    pub pending_transactions: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct AccountTransactionInfo {
    pub id: String,
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub nonce: u64,
    pub timestamp: u64,
    pub direction: String,
    pub hashtimer: String,
}

#[derive(Debug, Serialize)]
pub struct TransactionsResponse {
    pub transactions: Vec<AccountTransactionInfo>,
}

#[derive(Debug, Serialize)]
pub struct AddressValidationResponse {
    pub valid: bool,
}

/// Application state
#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<dyn Storage + Send + Sync>,
    pub start_time: std::time::Instant,
    pub peer_count: Arc<std::sync::atomic::AtomicUsize>,
    pub p2p_network: Option<Arc<ippan_p2p::HttpP2PNetwork>>,
    pub node_id: String,
    pub consensus: Option<ConsensusHandle>,
    pub l2_config: L2Config,
}

fn parse_address_string(address: &str) -> Result<[u8; 32], StatusCode> {
    if let Ok(decoded) = decode_address(address) {
        return Ok(decoded);
    }

    let trimmed = address.strip_prefix("0x").unwrap_or(address);
    let bytes = hex::decode(trimmed).map_err(|_| StatusCode::BAD_REQUEST)?;
    if bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mut addr = [0u8; 32];
    addr.copy_from_slice(&bytes);
    Ok(addr)
}

fn parse_signature(signature: &str) -> Result<Option<[u8; 64]>, StatusCode> {
    if signature.trim().is_empty() {
        return Ok(None);
    }

    let trimmed = signature.strip_prefix("0x").unwrap_or(signature);
    let bytes = hex::decode(trimmed).map_err(|_| StatusCode::BAD_REQUEST)?;
    if bytes.len() != 64 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mut sig = [0u8; 64];
    sig.copy_from_slice(&bytes);
    Ok(Some(sig))
}

const L2_AMOUNT_SCALE: u128 = 1_000_000;

fn convert_exit_amount(amount: f64) -> Result<u128, StatusCode> {
    if !amount.is_finite() || amount <= 0.0 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let scaled = (amount * L2_AMOUNT_SCALE as f64).round();
    if scaled < 0.0 || scaled > u128::MAX as f64 {
        return Err(StatusCode::BAD_REQUEST);
    }

    Ok(scaled as u128)
}

async fn submit_transaction_common(
    state: &AppState,
    request: SubmitTransactionRequest,
) -> Result<Transaction, StatusCode> {
    let from = parse_address_string(&request.from)?;
    let to = parse_address_string(&request.to)?;

    let mut tx = Transaction::new(from, to, request.amount, request.nonce);

    if let Some(signature) = parse_signature(&request.signature)? {
        tx.signature = signature;
        tx.refresh_id();
    } else {
        tx.sign(&[0u8; 32])
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    if !tx.is_valid() {
        tx.sign(&[0u8; 32])
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    if !tx.is_valid() {
        return Err(StatusCode::BAD_REQUEST);
    }

    state
        .storage
        .store_transaction(tx.clone())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(consensus) = &state.consensus {
        if let Err(err) = consensus.sender().send(tx.clone()) {
            warn!("Failed to enqueue transaction for consensus: {}", err);
        }
    }

    if let Some(network) = &state.p2p_network {
        if let Err(err) = network.broadcast_transaction(tx.clone()).await {
            warn!("Failed to broadcast transaction to peers: {}", err);
        }
    }

    Ok(tx)
}

async fn submit_l2_commit(
    State(state): State<AppState>,
    Json(request): Json<L2CommitRequest>,
) -> Result<Json<RpcResponse<L2CommitResponse>>, StatusCode> {
    if request.l2_id.trim().is_empty() || request.state_root.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let proof_type = request.proof_type.unwrap_or_default();
    let payload_size = request
        .proof
        .as_ref()
        .map(|s| s.as_bytes().len())
        .unwrap_or(0)
        + request
            .inline_data
            .as_ref()
            .map(|s| s.as_bytes().len())
            .unwrap_or(0);

    if payload_size > state.l2_config.max_commit_size {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }

    let now = IppanTimeMicros::now();
    let min_gap = state.l2_config.min_epoch_gap_ms.saturating_mul(1_000);

    if let Ok(existing) = state.storage.get_l2_commits(Some(request.l2_id.as_str())) {
        if let Some(latest) = existing.first() {
            if request.epoch <= latest.epoch {
                return Err(StatusCode::CONFLICT);
            }

            if min_gap > 0 && now.0 < latest.timestamp.0.saturating_add(min_gap) {
                return Err(StatusCode::TOO_MANY_REQUESTS);
            }
        }
    }

    let commit = L2StateCommit::with_timestamp(
        request.l2_id.clone(),
        request.epoch,
        request.state_root.clone(),
        request.da_hash.clone(),
        proof_type,
        request.proof.clone(),
        request.inline_data.clone(),
        now,
        state.node_id.as_bytes(),
    );

    state
        .storage
        .store_l2_commit(commit.clone())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    info!(
        "Anchored L2 commitment {} epoch {}",
        commit.l2_id, commit.epoch
    );

    let response = L2CommitResponse {
        l2_id: commit.l2_id.clone(),
        epoch: commit.epoch,
        commit_id: hex::encode(commit.id),
        hashtimer: commit.hashtimer.to_hex(),
        timestamp: commit.timestamp.0,
    };

    Ok(Json(RpcResponse::success(response)))
}

async fn list_l2_commits(
    State(state): State<AppState>,
    Query(filter): Query<L2FilterQuery>,
) -> Result<Json<RpcResponse<Vec<L2StateCommit>>>, StatusCode> {
    let commits = state
        .storage
        .get_l2_commits(filter.l2_id.as_deref())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(RpcResponse::success(commits)))
}

async fn verify_l2_exit(
    State(state): State<AppState>,
    Json(request): Json<L2ExitVerificationRequest>,
) -> Result<Json<RpcResponse<L2ExitVerificationResponse>>, StatusCode> {
    if request.l2_id.trim().is_empty()
        || request.account.trim().is_empty()
        || request.proof_of_inclusion.trim().is_empty()
    {
        return Err(StatusCode::BAD_REQUEST);
    }

    let amount = convert_exit_amount(request.amount)?;
    let now = IppanTimeMicros::now();
    let mut exit = L2Exit::with_timestamp(
        request.l2_id.clone(),
        request.epoch,
        request.account.clone(),
        amount,
        request.nonce,
        request.proof_of_inclusion.clone(),
        now,
        state.node_id.as_bytes(),
    );

    if state.l2_config.challenge_window_ms > 0 {
        exit.status = L2ExitStatus::ChallengeWindow;
    } else {
        exit.status = L2ExitStatus::Finalized;
        exit.finalized_at = Some(now);
    }

    state
        .storage
        .store_l2_exit(exit.clone())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = L2ExitVerificationResponse {
        exit_id: hex::encode(exit.id),
        status: exit.status.clone(),
        hashtimer: exit.hashtimer.to_hex(),
        submitted_at: exit.submitted_at.0,
        challenge_window_ends_at: exit
            .challenge_window_deadline(state.l2_config.challenge_window_ms)
            .0,
    };

    info!(
        "Accepted L2 exit for {} epoch {} amount {}",
        exit.l2_id, exit.epoch, request.amount
    );

    Ok(Json(RpcResponse::success(response)))
}

async fn list_l2_exits(
    State(state): State<AppState>,
    Query(filter): Query<L2FilterQuery>,
) -> Result<Json<RpcResponse<Vec<L2Exit>>>, StatusCode> {
    let exits = state
        .storage
        .get_l2_exits(filter.l2_id.as_deref())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(RpcResponse::success(exits)))
}

async fn get_l2_exit(
    State(state): State<AppState>,
    Path(exit_id): Path<String>,
) -> Result<Json<RpcResponse<Option<L2Exit>>>, StatusCode> {
    let bytes = hex::decode(exit_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    if bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut id = [0u8; 32];
    id.copy_from_slice(&bytes);

    let exit = state
        .storage
        .get_l2_exit(&id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(RpcResponse::success(exit)))
}

fn balance_response_for(
    state: &AppState,
    address_bytes: [u8; 32],
) -> Result<BalanceResponseV1, StatusCode> {
    let account = state
        .storage
        .get_account(&address_bytes)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let (balance, nonce) = account
        .map(|acct| (acct.balance, acct.nonce))
        .unwrap_or((0, 0));

    let pending = state
        .consensus
        .as_ref()
        .map(|handle| handle.pending_for_address(&address_bytes))
        .unwrap_or_default();

    let pending_hashes = pending
        .into_iter()
        .map(|tx| hex::encode(tx.hash()))
        .collect();

    Ok(BalanceResponseV1 {
        account: encode_address(&address_bytes),
        address: encode_address(&address_bytes),
        balance,
        staked: 0,
        staked_amount: 0,
        rewards: 0,
        nonce,
        pending_transactions: pending_hashes,
    })
}

/// Create the Axum router with all RPC endpoints
pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Health and status endpoints
        .route("/health", get(health_check))
        .route("/status", get(node_status))
        .route("/time", get(get_time))
        // Unified UI API v1
        .route("/api/v1/status", get(api_v1_status))
        .route("/api/v1/network", get(api_v1_network))
        .route("/api/v1/mempool", get(api_v1_mempool))
        .route("/api/v1/consensus", get(api_v1_consensus))
        .route("/api/v1/balance", get(get_balance_v1))
        .route("/api/v1/balance/:address", get(get_balance_v1_path))
        .route("/api/v1/transactions", get(get_transactions_v1))
        .route("/api/v1/transaction", post(submit_transaction_v1))
        .route("/api/v1/address/validate", get(validate_address_v1))
        .route("/api/v1/l2/commit", post(submit_l2_commit))
        .route("/api/v1/l2/commits", get(list_l2_commits))
        .route("/api/v1/l2/verify_exit", post(verify_l2_exit))
        .route("/api/v1/l2/exits", get(list_l2_exits))
        .route("/api/v1/l2/exits/:exit_id", get(get_l2_exit))
        // Blockchain endpoints
        .route("/block", get(get_block))
        .route("/block/:hash", get(get_block_by_hash))
        .route("/block/height/:height", get(get_block_by_height))
        .route("/tx", post(submit_transaction))
        .route("/tx/:hash", get(get_transaction))
        // Account endpoints
        .route("/account/:address", get(get_account))
        .route("/accounts", get(get_all_accounts))
        // P2P endpoints
        .route("/p2p/blocks", post(receive_block))
        .route("/p2p/transactions", post(receive_transaction))
        .route("/p2p/block-request", post(receive_block_request))
        .route("/p2p/block-response", post(receive_block_response))
        .route("/p2p/peer-info", post(receive_peer_info))
        .route("/p2p/peer-discovery", post(receive_peer_discovery))
        .route("/p2p/peers", get(get_peers))
        // Add middleware
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(
                    CorsLayer::new()
                        .allow_origin(Any)
                        .allow_methods(Any)
                        .allow_headers(Any),
                ),
        )
        .with_state(state)
}

/// Health check endpoint
async fn health_check(State(_state): State<AppState>) -> Json<RpcResponse<HealthResponse>> {
    let response = HealthResponse {
        status: "healthy".to_string(),
        timestamp: ippan_time_now(),
    };
    Json(RpcResponse::success(response))
}

/// Node status endpoint
async fn node_status(
    State(state): State<AppState>,
) -> Result<Json<RpcResponse<NodeStatusResponse>>, StatusCode> {
    let latest_height = state
        .storage
        .get_latest_height()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let uptime = state.start_time.elapsed().as_secs();
    let peer_count = state.peer_count.load(std::sync::atomic::Ordering::Relaxed);

    let response = NodeStatusResponse {
        node_id: state.node_id.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        latest_height,
        uptime_seconds: uptime,
        peer_count,
    };

    Ok(Json(RpcResponse::success(response)))
}

/// Get current IPPAN time
async fn get_time() -> Json<RpcResponse<GetTimeResponse>> {
    let response = GetTimeResponse {
        time_us: ippan_time_now(),
    };
    Json(RpcResponse::success(response))
}

async fn api_v1_status(State(state): State<AppState>) -> Result<Json<NodeStatusV1>, StatusCode> {
    let latest_height = state
        .storage
        .get_latest_height()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let total_transactions = state
        .storage
        .get_transaction_count()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let uptime_seconds = state.start_time.elapsed().as_secs();
    let network_peers = state.peer_count.load(std::sync::atomic::Ordering::Relaxed);

    let consensus_handle = state.consensus.clone();
    let (mempool_total, _) = consensus_handle
        .as_ref()
        .map(|handle| handle.mempool_metrics())
        .unwrap_or((0, 0));

    let consensus_state = if let Some(handle) = consensus_handle.clone() {
        Some(handle.state().await)
    } else {
        None
    };

    let current_block = consensus_state
        .as_ref()
        .map(|s| s.latest_block_height)
        .unwrap_or(latest_height);

    let node_info = NodeInfoV1 {
        is_running: consensus_state.is_some(),
        uptime_seconds,
        version: env!("CARGO_PKG_VERSION").to_string(),
        node_id: state.node_id.clone(),
    };

    let network_info = NetworkInfoV1 {
        connected_peers: network_peers,
        known_peers: state
            .p2p_network
            .as_ref()
            .map(|net| net.get_peers().len())
            .unwrap_or(0),
        total_peers: network_peers,
    };

    let mempool_info = MempoolInfoV1 {
        total_transactions: mempool_total,
        pending_transactions: mempool_total,
    };

    let blockchain_info = BlockchainInfoV1 {
        current_height: current_block,
        total_blocks: current_block.saturating_add(1),
        total_transactions,
    };

    let status = NodeStatusV1 {
        node_id: state.node_id.clone(),
        status: if consensus_state.is_some() {
            "running".to_string()
        } else {
            "starting".to_string()
        },
        current_block,
        total_transactions,
        network_peers,
        uptime_seconds,
        version: env!("CARGO_PKG_VERSION").to_string(),
        node: node_info,
        network: network_info,
        mempool: mempool_info,
        blockchain: blockchain_info,
    };

    Ok(Json(status))
}

async fn api_v1_network(State(state): State<AppState>) -> Result<Json<NetworkStatsV1>, StatusCode> {
    let peer_count = state.peer_count.load(std::sync::atomic::Ordering::Relaxed);
    let total_peers = state
        .p2p_network
        .as_ref()
        .map(|net| net.get_peers().len())
        .unwrap_or(0);

    let stats = NetworkStatsV1 {
        total_peers,
        connected_peers: peer_count,
        network_id: "ippan-local".to_string(),
        protocol_version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: state.start_time.elapsed().as_secs(),
    };

    Ok(Json(stats))
}

async fn api_v1_mempool(State(state): State<AppState>) -> Result<Json<MempoolStatsV1>, StatusCode> {
    let consensus_handle = state.consensus.clone();
    let snapshot = consensus_handle
        .as_ref()
        .map(|handle| handle.mempool_snapshot())
        .unwrap_or_default();
    let (total_transactions, total_senders) = consensus_handle
        .as_ref()
        .map(|handle| handle.mempool_metrics())
        .unwrap_or((0, 0));

    let total_size = snapshot
        .iter()
        .map(|tx| serde_json::to_vec(tx).map(|v| v.len()).unwrap_or(0))
        .sum();

    let mut fee_distribution = HashMap::new();
    fee_distribution.insert("low".to_string(), 0);
    fee_distribution.insert("medium".to_string(), 0);
    fee_distribution.insert("high".to_string(), 0);

    let stats = MempoolStatsV1 {
        total_transactions,
        total_senders,
        total_size,
        fee_distribution,
    };

    Ok(Json(stats))
}

async fn api_v1_consensus(
    State(state): State<AppState>,
) -> Result<Json<ConsensusStatsV1>, StatusCode> {
    let consensus_handle = match state.consensus.clone() {
        Some(handle) => handle,
        None => {
            let stats = ConsensusStatsV1 {
                current_round: 0,
                validators_count: 0,
                block_height: state
                    .storage
                    .get_latest_height()
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
                consensus_status: "stopped".to_string(),
            };
            return Ok(Json(stats));
        }
    };

    let consensus_state = consensus_handle.state().await;
    let stats = ConsensusStatsV1 {
        current_round: consensus_state.current_slot,
        validators_count: consensus_state.validator_count,
        block_height: consensus_state.latest_block_height,
        consensus_status: if consensus_state.is_proposing {
            "proposing".to_string()
        } else {
            "running".to_string()
        },
    };

    Ok(Json(stats))
}

async fn get_balance_v1(
    State(state): State<AppState>,
    Query(params): Query<AddressQuery>,
) -> Result<Json<BalanceResponseV1>, StatusCode> {
    let address_bytes = parse_address_string(&params.address)?;
    let response = balance_response_for(&state, address_bytes)?;
    Ok(Json(response))
}

async fn get_balance_v1_path(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> Result<Json<BalanceResponseV1>, StatusCode> {
    let address_bytes = parse_address_string(&address)?;
    let response = balance_response_for(&state, address_bytes)?;
    Ok(Json(response))
}

async fn get_transactions_v1(
    State(state): State<AppState>,
    Query(params): Query<AddressQuery>,
) -> Result<Json<TransactionsResponse>, StatusCode> {
    let address_bytes = parse_address_string(&params.address)?;
    let transactions = state
        .storage
        .get_transactions_by_address(&address_bytes)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let items = transactions
        .into_iter()
        .map(|tx| {
            let direction = if tx.from == address_bytes && tx.to == address_bytes {
                "self".to_string()
            } else if tx.from == address_bytes {
                "send".to_string()
            } else if tx.to == address_bytes {
                "receive".to_string()
            } else {
                "other".to_string()
            };

            AccountTransactionInfo {
                id: hex::encode(tx.hash()),
                from: encode_address(&tx.from),
                to: encode_address(&tx.to),
                amount: tx.amount,
                nonce: tx.nonce,
                timestamp: tx.timestamp.0,
                direction,
                hashtimer: tx.hashtimer.to_hex(),
            }
        })
        .collect();

    let response = TransactionsResponse {
        transactions: items,
    };
    Ok(Json(response))
}

async fn submit_transaction_v1(
    State(state): State<AppState>,
    Json(request): Json<SubmitTransactionRequest>,
) -> Result<Json<RpcResponse<SubmitTransactionResponse>>, StatusCode> {
    let tx = submit_transaction_common(&state, request).await?;
    let response = SubmitTransactionResponse {
        tx_hash: hex::encode(tx.hash()),
    };

    Ok(Json(RpcResponse::success(response)))
}

async fn validate_address_v1(
    Query(params): Query<AddressQuery>,
) -> Json<AddressValidationResponse> {
    let valid = is_valid_address(&params.address) || parse_address_string(&params.address).is_ok();
    Json(AddressValidationResponse { valid })
}

/// Get block by hash or height
async fn get_block(
    State(state): State<AppState>,
    Query(params): Query<GetBlockQuery>,
) -> Result<Json<RpcResponse<Option<Block>>>, StatusCode> {
    let block = if let Some(hash_str) = params.hash {
        let hash = hex::decode(&hash_str).map_err(|_| StatusCode::BAD_REQUEST)?;

        if hash.len() != 32 {
            return Err(StatusCode::BAD_REQUEST);
        }

        let mut hash_bytes = [0u8; 32];
        hash_bytes.copy_from_slice(&hash);

        state
            .storage
            .get_block(&hash_bytes)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    } else if let Some(height) = params.height {
        state
            .storage
            .get_block_by_height(height)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    } else {
        return Err(StatusCode::BAD_REQUEST);
    };

    Ok(Json(RpcResponse::success(block)))
}

/// Get block by hash
async fn get_block_by_hash(
    State(state): State<AppState>,
    Path(hash_str): Path<String>,
) -> Result<Json<RpcResponse<Option<Block>>>, StatusCode> {
    let hash = hex::decode(&hash_str).map_err(|_| StatusCode::BAD_REQUEST)?;

    if hash.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mut hash_bytes = [0u8; 32];
    hash_bytes.copy_from_slice(&hash);

    let block = state
        .storage
        .get_block(&hash_bytes)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(RpcResponse::success(block)))
}

/// Get block by height
async fn get_block_by_height(
    State(state): State<AppState>,
    Path(height): Path<u64>,
) -> Result<Json<RpcResponse<Option<Block>>>, StatusCode> {
    let block = state
        .storage
        .get_block_by_height(height)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(RpcResponse::success(block)))
}

/// Submit a transaction
async fn submit_transaction(
    State(state): State<AppState>,
    Json(request): Json<SubmitTransactionRequest>,
) -> Result<Json<RpcResponse<SubmitTransactionResponse>>, StatusCode> {
    let tx = submit_transaction_common(&state, request).await?;
    let response = SubmitTransactionResponse {
        tx_hash: hex::encode(tx.hash()),
    };

    info!("Submitted transaction: {}", response.tx_hash);
    Ok(Json(RpcResponse::success(response)))
}

/// Get transaction by hash
async fn get_transaction(
    State(state): State<AppState>,
    Path(hash_str): Path<String>,
) -> Result<Json<RpcResponse<Option<Transaction>>>, StatusCode> {
    let hash = hex::decode(&hash_str).map_err(|_| StatusCode::BAD_REQUEST)?;

    if hash.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mut hash_bytes = [0u8; 32];
    hash_bytes.copy_from_slice(&hash);

    let tx = state
        .storage
        .get_transaction(&hash_bytes)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(RpcResponse::success(tx)))
}

/// Get account information
async fn get_account(
    State(state): State<AppState>,
    Path(address_str): Path<String>,
) -> Result<Json<RpcResponse<Option<GetAccountResponse>>>, StatusCode> {
    let address_bytes = parse_address_string(&address_str)?;

    let account = state
        .storage
        .get_account(&address_bytes)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = account.map(|acc| GetAccountResponse {
        address: encode_address(&acc.address),
        balance: acc.balance,
        nonce: acc.nonce,
    });

    Ok(Json(RpcResponse::success(response)))
}

/// Get all accounts
async fn get_all_accounts(
    State(state): State<AppState>,
) -> Result<Json<RpcResponse<Vec<GetAccountResponse>>>, StatusCode> {
    let accounts = state
        .storage
        .get_all_accounts()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response: Vec<GetAccountResponse> = accounts
        .into_iter()
        .map(|acc| GetAccountResponse {
            address: encode_address(&acc.address),
            balance: acc.balance,
            nonce: acc.nonce,
        })
        .collect();

    Ok(Json(RpcResponse::success(response)))
}

/// P2P endpoint: Receive block from peer
async fn receive_block(
    State(state): State<AppState>,
    Json(block): Json<Block>,
) -> Result<Json<RpcResponse<String>>, StatusCode> {
    // Store the received block
    if let Err(e) = state.storage.store_block(block.clone()) {
        error!("Failed to store received block: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    info!("Received block from peer: {}", hex::encode(block.hash()));
    Ok(Json(RpcResponse::success("Block received".to_string())))
}

/// P2P endpoint: Receive transaction from peer
async fn receive_transaction(
    State(state): State<AppState>,
    Json(tx): Json<Transaction>,
) -> Result<Json<RpcResponse<String>>, StatusCode> {
    let mut tx = tx;
    tx.refresh_id();

    if let Err(e) = state.storage.store_transaction(tx.clone()) {
        error!("Failed to store received transaction: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    if let Some(consensus) = &state.consensus {
        if let Err(err) = consensus.sender().send(tx.clone()) {
            warn!("Failed to enqueue received transaction: {}", err);
        }
    }

    info!("Received transaction from peer: {}", hex::encode(tx.hash()));
    Ok(Json(RpcResponse::success(
        "Transaction received".to_string(),
    )))
}

/// P2P endpoint: Receive block request from peer
async fn receive_block_request(
    State(state): State<AppState>,
    Json(request): Json<ippan_p2p::NetworkMessage>,
) -> Result<Json<RpcResponse<String>>, StatusCode> {
    if let ippan_p2p::NetworkMessage::BlockRequest { hash } = request {
        // Try to find the requested block
        if let Ok(Some(_block)) = state.storage.get_block(&hash) {
            // In a real implementation, we would send the block back to the requesting peer
            info!("Block request received for: {}", hex::encode(hash));
            Ok(Json(RpcResponse::success(
                "Block request processed".to_string(),
            )))
        } else {
            warn!("Block not found for request: {}", hex::encode(hash));
            Err(StatusCode::NOT_FOUND)
        }
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}

/// P2P endpoint: Receive block response from peer
async fn receive_block_response(
    State(state): State<AppState>,
    Json(response): Json<ippan_p2p::NetworkMessage>,
) -> Result<Json<RpcResponse<String>>, StatusCode> {
    if let ippan_p2p::NetworkMessage::BlockResponse(block) = response {
        // Store the received block
        if let Err(e) = state.storage.store_block(block.clone()) {
            error!("Failed to store received block response: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }

        info!("Received block response: {}", hex::encode(block.hash()));
        Ok(Json(RpcResponse::success(
            "Block response received".to_string(),
        )))
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}

/// P2P endpoint: Receive peer info from peer
async fn receive_peer_info(
    State(state): State<AppState>,
    Json(info): Json<ippan_p2p::NetworkMessage>,
) -> Result<Json<RpcResponse<String>>, StatusCode> {
    if let ippan_p2p::NetworkMessage::PeerInfo { peer_id, addresses } = info {
        info!(
            "Received peer info: {} with addresses: {:?}",
            peer_id, addresses
        );

        if let Some(network) = state.p2p_network.as_ref() {
            for address in &addresses {
                if let Err(e) = network.add_peer(address.clone()).await {
                    warn!("Failed to add announced peer {}: {}", address, e);
                }
            }
        }

        Ok(Json(RpcResponse::success("Peer info received".to_string())))
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}

/// P2P endpoint: Receive peer discovery from peer
async fn receive_peer_discovery(
    State(state): State<AppState>,
    Json(discovery): Json<ippan_p2p::NetworkMessage>,
) -> Result<Json<RpcResponse<String>>, StatusCode> {
    if let ippan_p2p::NetworkMessage::PeerDiscovery { peers } = discovery {
        info!("Received peer discovery with {} peers", peers.len());

        if let Some(network) = state.p2p_network.as_ref() {
            for peer in &peers {
                if let Err(e) = network.add_peer(peer.clone()).await {
                    warn!("Failed to add discovered peer {}: {}", peer, e);
                }
            }
        }

        Ok(Json(RpcResponse::success(
            "Peer discovery received".to_string(),
        )))
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}

/// P2P endpoint: Get list of peers
async fn get_peers(
    State(state): State<AppState>,
) -> Result<Json<RpcResponse<Vec<String>>>, StatusCode> {
    if let Some(p2p_network) = &state.p2p_network {
        let mut peers = p2p_network.get_peers();
        peers.push(p2p_network.get_announce_address());
        peers.sort();
        peers.dedup();

        Ok(Json(RpcResponse::success(peers)))
    } else {
        Ok(Json(RpcResponse::success(vec![])))
    }
}

/// Start the HTTP server
pub async fn start_server(state: AppState, addr: &str) -> Result<()> {
    let app = create_router(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("RPC server listening on {}", addr);
    info!("Available endpoints:");
    info!("  GET  /health - Health check");
    info!("  GET  /status - Node status");
    info!("  GET  /time - Current IPPAN time");
    info!("  GET  /api/v1/status - Detailed node status");
    info!("  GET  /api/v1/network - Network statistics");
    info!("  GET  /api/v1/mempool - Mempool statistics");
    info!("  GET  /api/v1/consensus - Consensus state");
    info!("  GET  /api/v1/balance?address=<address> - Account balance");
    info!("  GET  /api/v1/transactions?address=<address> - Account transactions");
    info!("  POST /api/v1/transaction - Submit transaction");
    info!("  GET  /api/v1/address/validate?address=<address> - Validate address");
    info!("  POST /api/v1/l2/commit - Submit L2 state commitment");
    info!("  GET  /api/v1/l2/commits - List L2 commitments");
    info!("  POST /api/v1/l2/verify_exit - Submit L2 exit proof");
    info!("  GET  /api/v1/l2/exits - List L2 exit requests");
    info!("  GET  /api/v1/l2/exits/<id> - Get exit by identifier");
    info!("  GET  /block?hash=<hash> - Get block by hash");
    info!("  GET  /block?height=<height> - Get block by height");
    info!("  GET  /block/<hash> - Get block by hash");
    info!("  GET  /block/height/<height> - Get block by height");
    info!("  POST /tx - Submit transaction");
    info!("  GET  /tx/<hash> - Get transaction by hash");
    info!("  GET  /account/<address> - Get account info");
    info!("  GET  /accounts - Get all accounts");

    axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ippan_storage::MemoryStorage;
    use std::sync::atomic::AtomicUsize;

    #[tokio::test]
    async fn test_health_check() {
        let storage = Arc::new(MemoryStorage::new());
        let state = AppState {
            storage,
            start_time: std::time::Instant::now(),
            peer_count: Arc::new(AtomicUsize::new(0)),
            p2p_network: None,
            node_id: "test-node".to_string(),
            consensus: None,
            l2_config: L2Config::default(),
        };

        let Json(body) = health_check(State(state)).await;
        assert!(body.success);
        assert_eq!(body.data.unwrap().status, "healthy");
    }

    #[tokio::test]
    async fn test_get_time() {
        let Json(body) = get_time().await;
        assert!(body.success);
        assert!(body.data.unwrap().time_us > 0);
    }

    #[tokio::test]
    async fn test_submit_l2_commit() {
        let storage = Arc::new(MemoryStorage::new());
        let state = AppState {
            storage: storage.clone(),
            start_time: std::time::Instant::now(),
            peer_count: Arc::new(AtomicUsize::new(0)),
            p2p_network: None,
            node_id: "test-node".to_string(),
            consensus: None,
            l2_config: L2Config::default(),
        };

        let request = L2CommitRequest {
            l2_id: "rollup-1".to_string(),
            epoch: 1,
            state_root: "0xabc".to_string(),
            da_hash: None,
            proof_type: Some(L2ProofType::Optimistic),
            proof: Some("proof".to_string()),
            inline_data: None,
        };

        let Json(response) = submit_l2_commit(State(state.clone()), Json(request))
            .await
            .expect("commit should succeed");

        assert!(response.success);
        let data = response.data.unwrap();
        assert_eq!(data.l2_id, "rollup-1");
        assert_eq!(data.epoch, 1);

        let commits = storage.get_l2_commits(Some("rollup-1")).unwrap();
        assert_eq!(commits.len(), 1);
    }

    #[tokio::test]
    async fn test_verify_l2_exit() {
        let storage = Arc::new(MemoryStorage::new());
        let state = AppState {
            storage: storage.clone(),
            start_time: std::time::Instant::now(),
            peer_count: Arc::new(AtomicUsize::new(0)),
            p2p_network: None,
            node_id: "test-node".to_string(),
            consensus: None,
            l2_config: L2Config::default(),
        };

        let request = L2ExitVerificationRequest {
            l2_id: "rollup-1".to_string(),
            epoch: 1,
            proof_of_inclusion: "proof".to_string(),
            account: "0xabc".to_string(),
            amount: 10.5,
            nonce: 0,
        };

        let Json(response) = verify_l2_exit(State(state.clone()), Json(request))
            .await
            .expect("exit should succeed");

        assert!(response.success);
        let data = response.data.unwrap();
        assert_eq!(data.status, L2ExitStatus::ChallengeWindow);

        let exits = storage.get_l2_exits(Some("rollup-1")).unwrap();
        assert_eq!(exits.len(), 1);
    }
}
