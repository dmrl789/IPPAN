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
use ippan_types::{ippan_time_now, Block, HashTimer, IppanTimeMicros, Transaction};
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
#[derive(Debug, Serialize, Deserialize)]
pub struct RpcResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
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

/// Submit transaction response
#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitTransactionResponse {
    pub tx_hash: String,
}

/// Submit transaction request payload for the V1 API
#[derive(Debug, Deserialize)]
pub struct SubmitTransactionRequest {
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub nonce: u64,
    pub signature: String,
    pub hashtimer: Option<String>,
    pub timestamp: Option<u64>,
    pub id: Option<String>,
}

/// Get block query parameters
#[derive(Debug, Deserialize)]
pub struct GetBlockQuery {
    pub hash: Option<String>,
    pub height: Option<u64>,
}

/// Query parameter for endpoints that expect an address value
#[derive(Debug, Deserialize)]
pub struct AddressQuery {
    pub address: String,
}

/// Get account response
#[derive(Debug, Serialize)]
pub struct GetAccountResponse {
    pub address: String,
    pub balance: u64,
    pub nonce: u64,
}

/// Get time response
#[derive(Debug, Serialize, Deserialize)]
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
#[derive(Debug, Serialize, Deserialize)]
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

fn parse_address_string(address: &str) -> Result<[u8; 32], StatusCode> {
    let decoded = hex::decode(address).map_err(|_| StatusCode::BAD_REQUEST)?;
    if decoded.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&decoded);
    Ok(bytes)
}

fn is_valid_address(address: &str) -> bool {
    address.len() == 64 && address.chars().all(|c| c.is_ascii_hexdigit())
}

fn encode_address(address: &[u8; 32]) -> String {
    hex::encode(address)
}

fn derive_transaction_hashtimer(
    from: &[u8; 32],
    to: &[u8; 32],
    amount: u64,
    nonce: u64,
    timestamp: IppanTimeMicros,
) -> HashTimer {
    let mut payload = Vec::with_capacity(32 + 32 + 8 + 8);
    payload.extend_from_slice(from);
    payload.extend_from_slice(to);
    payload.extend_from_slice(&amount.to_be_bytes());
    payload.extend_from_slice(&nonce.to_be_bytes());

    HashTimer::derive(
        "transaction",
        timestamp,
        b"transaction",
        &payload,
        &nonce.to_be_bytes(),
        from,
    )
}

fn balance_response_for(
    state: &AppState,
    address: [u8; 32],
) -> Result<BalanceResponseV1, StatusCode> {
    let account = state
        .storage
        .get_account(&address)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let (balance, nonce) = account
        .map(|acc| (acc.balance, acc.nonce))
        .unwrap_or((0, 0));

    let pending_transactions = state
        .consensus
        .as_ref()
        .map(|handle| handle.pending_for_address(&address))
        .unwrap_or_default()
        .into_iter()
        .map(|tx| hex::encode(tx.hash()))
        .collect();

    let address_hex = encode_address(&address);
    Ok(BalanceResponseV1 {
        account: address_hex.clone(),
        address: address_hex,
        balance,
        staked: 0,
        staked_amount: 0,
        rewards: 0,
        nonce,
        pending_transactions,
    })
}

async fn submit_transaction_common(
    state: &AppState,
    request: SubmitTransactionRequest,
) -> Result<Transaction, StatusCode> {
    let tx = request.into_transaction()?;
    process_transaction(state, tx).await
}

async fn process_transaction(
    state: &AppState,
    mut tx: Transaction,
) -> Result<Transaction, StatusCode> {
    tx.refresh_id();

    if !tx.is_valid() {
        warn!("Rejected invalid transaction from {}", hex::encode(tx.from));
        return Err(StatusCode::BAD_REQUEST);
    }

    state
        .storage
        .store_transaction(tx.clone())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(sender) = &state.tx_sender {
        sender
            .send(tx.clone())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    if let Some(network) = &state.p2p_network {
        if let Err(error) = network.broadcast_transaction(tx.clone()).await {
            warn!(
                "Failed to broadcast transaction {}: {}",
                hex::encode(tx.hash()),
                error
            );
        }
    }

    Ok(tx)
}

impl SubmitTransactionRequest {
    fn into_transaction(self) -> Result<Transaction, StatusCode> {
        let from = parse_address_string(&self.from)?;
        let to = parse_address_string(&self.to)?;
        let timestamp = self
            .timestamp
            .map(IppanTimeMicros)
            .unwrap_or_else(|| IppanTimeMicros::now());

        let hashtimer = if let Some(hex) = self.hashtimer.as_ref() {
            HashTimer::from_hex(hex).map_err(|_| StatusCode::BAD_REQUEST)?
        } else {
            derive_transaction_hashtimer(&from, &to, self.amount, self.nonce, timestamp)
        };

        let mut tx = Transaction {
            id: [0u8; 32],
            from,
            to,
            amount: self.amount,
            nonce: self.nonce,
            signature: [0u8; 64],
            hashtimer,
            timestamp,
        };

        let signature_bytes = hex::decode(&self.signature).map_err(|_| StatusCode::BAD_REQUEST)?;
        if signature_bytes.len() != 64 {
            return Err(StatusCode::BAD_REQUEST);
        }
        tx.signature.copy_from_slice(&signature_bytes);

        tx.refresh_id();

        if let Some(id_hex) = self.id.as_ref() {
            let id_bytes = hex::decode(id_hex).map_err(|_| StatusCode::BAD_REQUEST)?;
            if id_bytes.len() != 32 {
                return Err(StatusCode::BAD_REQUEST);
            }
            let mut provided_id = [0u8; 32];
            provided_id.copy_from_slice(&id_bytes);
            if provided_id != tx.id {
                return Err(StatusCode::BAD_REQUEST);
            }
        }

        Ok(tx)
    }
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
    pub tx_sender: Option<mpsc::UnboundedSender<Transaction>>,
    pub node_id: String,
    pub consensus: Option<ConsensusHandle>,
}

/// Create the Axum router with all RPC endpoints
pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Health and status endpoints
        .route("/health", get(health_check))
        .route("/status", get(node_status))
        .route("/time", get(get_time))
        // Blockchain endpoints
        .route("/block", get(get_block))
        .route("/block/:hash", get(get_block_by_hash))
        .route("/block/height/:height", get(get_block_by_height))
        .route("/tx", post(submit_transaction))
        .route("/tx/:hash", get(get_transaction))
        // Account endpoints
        .route("/account/:address", get(get_account))
        .route("/accounts", get(get_all_accounts))
        // API v1 endpoints
        .route("/api/v1/status", get(api_v1_status))
        .route("/api/v1/network", get(api_v1_network))
        .route("/api/v1/mempool", get(api_v1_mempool))
        .route("/api/v1/consensus", get(api_v1_consensus))
        .route("/api/v1/balance", get(get_balance_v1))
        .route("/api/v1/balance/:address", get(get_balance_v1_path))
        .route("/api/v1/transactions", get(get_transactions_v1))
        .route("/api/v1/transaction", post(submit_transaction_v1))
        .route("/api/v1/address/validate", get(validate_address_v1))
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
    Json(tx): Json<Transaction>,
) -> Result<Json<RpcResponse<SubmitTransactionResponse>>, StatusCode> {
    let processed = process_transaction(&state, tx).await?;
    let response = SubmitTransactionResponse {
        tx_hash: hex::encode(processed.id),
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
    let address = hex::decode(&address_str).map_err(|_| StatusCode::BAD_REQUEST)?;

    if address.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mut address_bytes = [0u8; 32];
    address_bytes.copy_from_slice(&address);

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
            address: hex::encode(acc.address),
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
    if !block.is_valid() {
        warn!(
            "Rejected invalid block from peer: {}",
            hex::encode(block.hash())
        );
        return Err(StatusCode::BAD_REQUEST);
    }

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
    if !tx.is_valid() {
        warn!(
            "Rejected invalid transaction from peer: {}",
            hex::encode(tx.hash())
        );
        return Err(StatusCode::BAD_REQUEST);
    }

    // Store the received transaction
    if let Err(e) = state.storage.store_transaction(tx.clone()) {
        error!("Failed to store received transaction: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    if let Some(sender) = &state.tx_sender {
        if let Err(error) = sender.send(tx.clone()) {
            error!(
                "Failed to forward received transaction to consensus: {}",
                error
            );
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
        if !block.is_valid() {
            warn!(
                "Rejected invalid block response from peer: {}",
                hex::encode(block.hash())
            );
            return Err(StatusCode::BAD_REQUEST);
        }

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
    State(_state): State<AppState>,
    Json(info): Json<ippan_p2p::NetworkMessage>,
) -> Result<Json<RpcResponse<String>>, StatusCode> {
    if let ippan_p2p::NetworkMessage::PeerInfo { peer_id, addresses } = info {
        info!(
            "Received peer info: {} with addresses: {:?}",
            peer_id, addresses
        );
        Ok(Json(RpcResponse::success("Peer info received".to_string())))
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}

/// P2P endpoint: Receive peer discovery from peer
async fn receive_peer_discovery(
    State(_state): State<AppState>,
    Json(discovery): Json<ippan_p2p::NetworkMessage>,
) -> Result<Json<RpcResponse<String>>, StatusCode> {
    if let ippan_p2p::NetworkMessage::PeerDiscovery { peers } = discovery {
        info!("Received peer discovery with {} peers", peers.len());
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
    use axum::body::{self, Body};
    use axum::http::{Method, Request, StatusCode};
    use ippan_storage::MemoryStorage;
    use std::convert::TryFrom;
    use std::sync::atomic::AtomicUsize;
    use tokio::sync::mpsc;
    use tower::Service;

    use ed25519_dalek::SigningKey;

    #[tokio::test]
    async fn test_health_check() {
        let storage: Arc<dyn Storage + Send + Sync> = Arc::new(MemoryStorage::new());
        let state = AppState {
            storage,
            start_time: std::time::Instant::now(),
            peer_count: Arc::new(AtomicUsize::new(0)),
            p2p_network: None,
            tx_sender: None,
            node_id: "test-node".to_string(),
            consensus: None,
        };

        let mut app = create_router(state);
        let response = Service::call(
            &mut app,
            Request::builder()
                .method(Method::GET)
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let rpc_response: RpcResponse<HealthResponse> = serde_json::from_slice(&body).unwrap();
        assert!(rpc_response.success);
        assert_eq!(rpc_response.data.unwrap().status, "healthy");
    }

    #[tokio::test]
    async fn test_get_time() {
        let storage: Arc<dyn Storage + Send + Sync> = Arc::new(MemoryStorage::new());
        let state = AppState {
            storage,
            start_time: std::time::Instant::now(),
            peer_count: Arc::new(AtomicUsize::new(0)),
            p2p_network: None,
            tx_sender: None,
            node_id: "test-node".to_string(),
            consensus: None,
        };

        let mut app = create_router(state);
        let response = Service::call(
            &mut app,
            Request::builder()
                .method(Method::GET)
                .uri("/time")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let rpc_response: RpcResponse<GetTimeResponse> = serde_json::from_slice(&body).unwrap();
        assert!(rpc_response.success);
        assert!(rpc_response.data.unwrap().time_us > 0);
    }

    #[tokio::test]
    async fn test_submit_transaction_flow() {
        let storage: Arc<dyn Storage + Send + Sync> = Arc::new(MemoryStorage::new());
        let (tx_sender, mut tx_receiver) = mpsc::unbounded_channel();

        let state = AppState {
            storage: storage.clone(),
            start_time: std::time::Instant::now(),
            peer_count: Arc::new(AtomicUsize::new(0)),
            p2p_network: None,
            tx_sender: Some(tx_sender),
            node_id: "test-node".to_string(),
            consensus: None,
        };

        let mut app = create_router(state);

        let secret_bytes = [7u8; 32];
        let signing_key = SigningKey::try_from(&secret_bytes[..]).unwrap();
        let public_key = signing_key.verifying_key().to_bytes();

        let mut tx = Transaction::new(public_key, [9u8; 32], 500, 0);
        tx.sign(&secret_bytes).unwrap();

        let body_bytes = serde_json::to_vec(&tx).unwrap();

        let response = Service::call(
            &mut app,
            Request::builder()
                .method(Method::POST)
                .uri("/tx")
                .header("content-type", "application/json")
                .body(Body::from(body_bytes))
                .unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let rpc_response: RpcResponse<SubmitTransactionResponse> =
            serde_json::from_slice(&body).unwrap();
        assert!(rpc_response.success);
        assert_eq!(rpc_response.data.unwrap().tx_hash, hex::encode(tx.id));

        let stored = storage.get_transaction(&tx.id).unwrap();
        assert!(stored.is_some());

        let forwarded = tx_receiver.try_recv().unwrap();
        assert_eq!(forwarded.id, tx.id);
    }
}
