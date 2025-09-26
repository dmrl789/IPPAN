use anyhow::Result;
use axum::extract::{Path, Query, State};
use axum::http::{Method, StatusCode};
use axum::routing::{get, get_service, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tracing::info;

use ippan_consensus::{ConsensusState, PoAConsensus, Validator};
use ippan_p2p::HttpP2PNetwork;
use ippan_storage::Storage;
use ippan_types::Transaction;

#[derive(Debug, Clone, Default)]
pub struct L2Config {
    pub max_commit_size: usize,
    pub min_epoch_gap_ms: u64,
    pub challenge_window_ms: u64,
    pub da_mode: String,
    pub max_l2_count: usize,
}

#[derive(Clone)]
pub struct ConsensusHandle {
    consensus: Arc<tokio::sync::Mutex<PoAConsensus>>,
    pub tx_sender: mpsc::UnboundedSender<Transaction>,
    mempool: Arc<parking_lot::RwLock<Vec<Transaction>>>,
}

impl ConsensusHandle {
    pub fn new(
        consensus: Arc<tokio::sync::Mutex<PoAConsensus>>,
        tx_sender: mpsc::UnboundedSender<Transaction>,
        mempool: Arc<parking_lot::RwLock<Vec<Transaction>>>,
    ) -> Self {
        Self {
            consensus,
            tx_sender,
            mempool,
        }
    }

    pub async fn get_state(&self) -> ConsensusState {
        let consensus = self.consensus.lock().await;
        consensus.get_state()
    }

    pub async fn get_validators(&self) -> Vec<Validator> {
        let consensus = self.consensus.lock().await;
        consensus.get_validators().to_vec()
    }

    pub fn mempool(&self) -> Arc<parking_lot::RwLock<Vec<Transaction>>> {
        self.mempool.clone()
    }
}

#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<dyn Storage + Send + Sync>,
    pub start_time: Instant,
    pub peer_count: Arc<AtomicUsize>,
    pub p2p_network: Option<Arc<HttpP2PNetwork>>,
    pub tx_sender: Option<mpsc::UnboundedSender<Transaction>>,
    pub node_id: String,
    pub consensus: Option<ConsensusHandle>,
    pub l2_config: L2Config,
    pub mempool: Arc<parking_lot::RwLock<Vec<Transaction>>>,
    pub unified_ui_dist: Option<PathBuf>,
}

pub async fn start_server(state: AppState, addr: &str) -> Result<()> {
    let shared_state = Arc::new(state);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(Any)
        .allow_credentials(false);

    let mut app = Router::new()
        .route("/health", get(health_handler))
        .route("/api/v1/status", get(node_status_handler))
        .route("/api/v1/network", get(network_handler))
        .route("/api/v1/mempool", get(mempool_handler))
        .route("/api/v1/consensus", get(consensus_handler))
        .route("/api/v1/balance", get(balance_handler))
        .route("/api/v1/balance/:address", get(balance_by_path_handler))
        .route("/api/v1/transactions", get(transactions_handler))
        .route("/api/v1/transaction", post(submit_transaction_handler))
        .route("/api/v1/address/validate", get(validate_address_handler))
        .route("/accounts", get(accounts_handler));

    if let Some(dist_dir) = &shared_state.unified_ui_dist {
        let index_html = dist_dir.join("index.html");
        let spa_service =
            ServeDir::new(dist_dir.clone()).not_found_service(ServeFile::new(index_html));

        info!(path = %dist_dir.display(), "Unified UI enabled");
        app = app.fallback_service(get_service(spa_service));
    }

    let app = app.with_state(shared_state).layer(cors);

    let addr: SocketAddr = addr.parse()?;
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

type ApiResult<T> = Result<Json<T>, (StatusCode, Json<ErrorResponse>)>;

fn internal_error<E: std::fmt::Display>(err: E) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: err.to_string(),
        }),
    )
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    node_id: String,
    version: &'static str,
    uptime_seconds: u64,
    peer_count: usize,
}

async fn health_handler(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    let uptime = state.start_time.elapsed().as_secs();
    let peer_count = current_peer_count(&state);

    Json(HealthResponse {
        status: "healthy",
        node_id: state.node_id.clone(),
        version: env!("CARGO_PKG_VERSION"),
        uptime_seconds: uptime,
        peer_count,
    })
}

#[derive(Debug, Serialize)]
struct NodeStatusResponse {
    node: NodeInfo,
    network: NetworkInfo,
    mempool: MempoolInfo,
    blockchain: BlockchainInfo,
}

#[derive(Debug, Serialize)]
struct NodeInfo {
    is_running: bool,
    uptime_seconds: u64,
    version: &'static str,
    node_id: String,
}

#[derive(Debug, Serialize)]
struct NetworkInfo {
    connected_peers: usize,
    known_peers: usize,
    total_peers: usize,
}

#[derive(Debug, Serialize)]
struct MempoolInfo {
    total_transactions: usize,
    pending_transactions: usize,
}

#[derive(Debug, Serialize)]
struct BlockchainInfo {
    current_height: u64,
    total_blocks: u64,
    total_transactions: u64,
}

async fn node_status_handler(State(state): State<Arc<AppState>>) -> ApiResult<NodeStatusResponse> {
    let uptime = state.start_time.elapsed().as_secs();
    let connected_peers = current_peer_count(&state);
    let known_peers = state
        .p2p_network
        .as_ref()
        .map(|p| p.get_peers().len())
        .unwrap_or(0);

    let total_transactions = state
        .storage
        .get_transaction_count()
        .map_err(internal_error)?;

    let current_height = state.storage.get_latest_height().map_err(internal_error)?;

    let mempool = state.mempool.read();
    let mempool_count = mempool.len();

    let response = NodeStatusResponse {
        node: NodeInfo {
            is_running: true,
            uptime_seconds: uptime,
            version: env!("CARGO_PKG_VERSION"),
            node_id: state.node_id.clone(),
        },
        network: NetworkInfo {
            connected_peers,
            known_peers,
            total_peers: connected_peers.max(known_peers),
        },
        mempool: MempoolInfo {
            total_transactions: mempool_count,
            pending_transactions: mempool_count,
        },
        blockchain: BlockchainInfo {
            current_height,
            total_blocks: current_height,
            total_transactions,
        },
    };

    Ok(Json(response))
}

#[derive(Debug, Serialize)]
struct NetworkStats {
    total_peers: usize,
    connected_peers: usize,
    network_id: String,
    protocol_version: &'static str,
    uptime_seconds: u64,
}

async fn network_handler(State(state): State<Arc<AppState>>) -> ApiResult<NetworkStats> {
    let connected_peers = current_peer_count(&state);
    let known_peers = state
        .p2p_network
        .as_ref()
        .map(|p| p.get_peers().len())
        .unwrap_or(0);

    let response = NetworkStats {
        total_peers: connected_peers.max(known_peers),
        connected_peers,
        network_id: format!("ippan-{}", state.node_id),
        protocol_version: env!("CARGO_PKG_VERSION"),
        uptime_seconds: state.start_time.elapsed().as_secs(),
    };

    Ok(Json(response))
}

#[derive(Debug, Serialize)]
struct MempoolStats {
    total_transactions: usize,
    total_senders: usize,
    total_size: u64,
    fee_distribution: HashMap<String, u64>,
}

async fn mempool_handler(State(state): State<Arc<AppState>>) -> ApiResult<MempoolStats> {
    let mempool = state.mempool.read();
    let total_transactions = mempool.len();
    let total_senders: HashSet<[u8; 32]> = mempool.iter().map(|tx| tx.from).collect();
    let mut total_size = 0u64;

    for tx in mempool.iter() {
        if let Ok(bytes) = serde_json::to_vec(tx) {
            total_size += bytes.len() as u64;
        }
    }

    let mut fee_distribution = HashMap::new();
    fee_distribution.insert("low".to_string(), 0);
    fee_distribution.insert("medium".to_string(), 0);
    fee_distribution.insert("high".to_string(), 0);

    Ok(Json(MempoolStats {
        total_transactions,
        total_senders: total_senders.len(),
        total_size,
        fee_distribution,
    }))
}

#[derive(Debug, Serialize)]
struct ConsensusStats {
    current_round: u64,
    validators_count: usize,
    block_height: u64,
    consensus_status: String,
}

async fn consensus_handler(State(state): State<Arc<AppState>>) -> ApiResult<ConsensusStats> {
    let consensus = state
        .consensus
        .as_ref()
        .ok_or_else(|| internal_error("Consensus engine not available"))?;

    let consensus_state = consensus.get_state().await;
    let validators = consensus.get_validators().await;

    let status = if validators.is_empty() {
        "degraded"
    } else if consensus_state.is_proposing {
        "active"
    } else {
        "healthy"
    };

    Ok(Json(ConsensusStats {
        current_round: consensus_state.current_slot,
        validators_count: validators.len(),
        block_height: consensus_state.latest_block_height,
        consensus_status: status.to_string(),
    }))
}

#[derive(Debug, Deserialize)]
struct BalanceQuery {
    address: Option<String>,
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

async fn balance_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<BalanceQuery>,
) -> ApiResult<BalanceResponse> {
    let address = query.address.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "address query parameter is required".into(),
            }),
        )
    })?;

    balance_for_address(state, &address)
}

async fn balance_by_path_handler(
    State(state): State<Arc<AppState>>,
    Path(address): Path<String>,
) -> ApiResult<BalanceResponse> {
    balance_for_address(state, &address)
}

fn balance_for_address(state: Arc<AppState>, address: &str) -> ApiResult<BalanceResponse> {
    let parsed = decode_address(address).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "invalid address".into(),
            }),
        )
    })?;

    let account = state
        .storage
        .get_account(&parsed)
        .map_err(internal_error)?
        .unwrap_or_else(|| ippan_storage::Account {
            address: parsed,
            balance: 0,
            nonce: 0,
        });

    let mempool = state.mempool.read();
    let pending: Vec<String> = mempool
        .iter()
        .filter(|tx| tx.from == parsed)
        .map(|tx| hex::encode(tx.id))
        .collect();

    let response = BalanceResponse {
        account: encode_address(&account.address),
        address: encode_address(&account.address),
        balance: account.balance,
        staked: 0,
        rewards: 0,
        nonce: account.nonce,
        pending_transactions: pending,
    };

    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
struct TransactionsQuery {
    address: Option<String>,
}

#[derive(Debug, Serialize)]
struct TransactionsResponse {
    transactions: Vec<TransactionView>,
}

#[derive(Debug, Serialize)]
struct TransactionView {
    id: String,
    from: String,
    to: String,
    amount: u64,
    nonce: u64,
    timestamp: u64,
    direction: Option<String>,
    hashtimer: String,
}

async fn transactions_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<TransactionsQuery>,
) -> ApiResult<TransactionsResponse> {
    let address = query.address;
    let parsed_address = address.as_deref().and_then(decode_address);

    let transactions = if let Some(addr) = parsed_address {
        state
            .storage
            .get_transactions_by_address(&addr)
            .map_err(internal_error)?
    } else {
        Vec::new()
    };

    let transactions = transactions
        .into_iter()
        .map(|mut tx| {
            if tx.id == [0u8; 32] {
                tx.refresh_id();
            }
            let direction = parsed_address.map(|addr| {
                if tx.from == addr {
                    "outgoing"
                } else {
                    "incoming"
                }
                .to_string()
            });
            TransactionView {
                id: hex::encode(tx.id),
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

    Ok(Json(TransactionsResponse { transactions }))
}

#[derive(Debug, Deserialize)]
struct SubmitTransactionRequest {
    from: Option<String>,
    to: Option<String>,
    amount: Option<u64>,
    nonce: Option<u64>,
    signature: Option<String>,
}

#[derive(Debug, Serialize)]
struct SubmitTransactionResponse {
    success: bool,
    error: Option<String>,
}

async fn submit_transaction_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SubmitTransactionRequest>,
) -> ApiResult<SubmitTransactionResponse> {
    if state.consensus.is_none() {
        return Ok(Json(SubmitTransactionResponse {
            success: false,
            error: Some("Consensus engine unavailable".into()),
        }));
    }

    if body.from.is_none()
        || body.to.is_none()
        || body.amount.is_none()
        || body.nonce.is_none()
        || body.signature.is_none()
    {
        return Ok(Json(SubmitTransactionResponse {
            success: false,
            error: Some("Missing required transaction fields".into()),
        }));
    }

    Ok(Json(SubmitTransactionResponse {
        success: false,
        error: Some("Submitting fully signed transactions is not yet implemented".into()),
    }))
}

#[derive(Debug, Deserialize)]
struct ValidateAddressQuery {
    address: Option<String>,
}

#[derive(Debug, Serialize)]
struct ValidateAddressResponse {
    valid: bool,
}

async fn validate_address_handler(
    Query(query): Query<ValidateAddressQuery>,
) -> ApiResult<ValidateAddressResponse> {
    let address = query.address.unwrap_or_default();
    let valid = decode_address(&address).is_some();
    Ok(Json(ValidateAddressResponse { valid }))
}

#[derive(Debug, Serialize)]
struct AccountsResponse {
    success: bool,
    data: Vec<AccountSummary>,
}

#[derive(Debug, Serialize)]
struct AccountSummary {
    address: String,
    balance: u64,
    nonce: u64,
}

async fn accounts_handler(State(state): State<Arc<AppState>>) -> ApiResult<AccountsResponse> {
    let accounts = state
        .storage
        .get_all_accounts()
        .map_err(internal_error)?
        .into_iter()
        .map(|account| AccountSummary {
            address: encode_address(&account.address),
            balance: account.balance,
            nonce: account.nonce,
        })
        .collect();

    Ok(Json(AccountsResponse {
        success: true,
        data: accounts,
    }))
}

fn current_peer_count(state: &AppState) -> usize {
    if let Some(network) = state.p2p_network.as_ref() {
        let count = network.get_peer_count();
        state.peer_count.store(count, Ordering::Relaxed);
        count
    } else {
        state.peer_count.load(Ordering::Relaxed)
    }
}

fn decode_address(input: &str) -> Option<[u8; 32]> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    let candidate = if trimmed.starts_with('i') {
        &trimmed[1..]
    } else {
        trimmed
    };

    if candidate.len() != 64 {
        return None;
    }

    let mut bytes = [0u8; 32];
    hex::decode_to_slice(candidate, &mut bytes).ok()?;
    Some(bytes)
}

fn encode_address(address: &[u8; 32]) -> String {
    format!("i{}", hex::encode(address))
}
