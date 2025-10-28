use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use axum::body::Body;
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::{header, HeaderValue, Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use ippan_consensus::{ConsensusState, PoAConsensus};
use ippan_mempool::Mempool;
use ippan_p2p::{HttpP2PNetwork, NetworkMessage};
use ippan_storage::{Account, Storage};
use ippan_types::time_service::ippan_time_now;
use ippan_types::{Block, IppanTimeMicros, L2Commit, L2ExitRecord, L2Network, Transaction};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tower::ServiceExt;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use tracing::{debug, info, warn};

#[derive(Clone, Debug, Serialize)]
pub struct L2Config {
    pub max_commit_size: usize,
    pub min_epoch_gap_ms: u64,
    pub challenge_window_ms: u64,
    pub da_mode: String,
    pub max_l2_count: usize,
}

#[derive(Clone)]
pub struct ConsensusHandle {
    consensus: Arc<Mutex<PoAConsensus>>,
    tx_sender: mpsc::UnboundedSender<Transaction>,
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
        Ok(ConsensusStateView::from(guard.get_state()))
    }

    pub fn submit_transaction(&self, tx: Transaction) -> Result<()> {
        self.tx_sender
            .send(tx)
            .map_err(|err| anyhow!("failed to enqueue transaction: {err}"))
    }

    pub fn mempool_size(&self) -> usize {
        self.mempool.size()
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
    pub mempool: Arc<Mempool>,
    pub unified_ui_dist: Option<PathBuf>,
    pub req_count: Arc<AtomicUsize>,
}

impl AppState {
    fn record_request(&self) -> u64 {
        self.req_count.fetch_add(1, Ordering::Relaxed) as u64 + 1
    }

    fn uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    fn current_peer_count(&self) -> usize {
        if let Some(network) = &self.p2p_network {
            network.get_peer_count()
        } else {
            self.peer_count.load(Ordering::Relaxed)
        }
    }

    fn peers(&self) -> Vec<String> {
        if let Some(network) = &self.p2p_network {
            network.get_peers()
        } else {
            Vec::new()
        }
    }

    fn local_peer_id(&self) -> Option<String> {
        self.p2p_network
            .as_ref()
            .map(|network| network.get_local_peer_id())
    }

    fn mempool_size(&self) -> usize {
        self.mempool.size()
    }

    fn static_assets_root(&self) -> Option<PathBuf> {
        self.unified_ui_dist.clone()
    }
}

type SharedState = Arc<AppState>;

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    node_id: String,
    uptime_secs: u64,
    peer_count: usize,
    mempool_size: usize,
    consensus: Option<ConsensusStateView>,
    req_total: u64,
    time_us: u64,
    l2_config: L2Config,
}

#[derive(Debug, Serialize)]
pub struct ConsensusStateView {
    pub current_slot: u64,
    pub current_round: u64,
    pub latest_block_height: u64,
    pub validator_count: usize,
    pub is_proposing: bool,
    pub current_proposer: Option<String>,
}

impl From<ConsensusState> for ConsensusStateView {
    fn from(state: ConsensusState) -> Self {
        Self {
            current_slot: state.current_slot,
            current_round: state.current_round,
            latest_block_height: state.latest_block_height,
            validator_count: state.validator_count,
            is_proposing: state.is_proposing,
            current_proposer: state.current_proposer.map(hex::encode),
        }
    }
}

#[derive(Debug, Serialize)]
struct TimeResponse {
    node_id: String,
    ippan_time: IppanTimeMicros,
    unix_time_us: u64,
    req_total: u64,
}

#[derive(Debug, Serialize)]
struct VersionResponse {
    node_id: String,
    version: &'static str,
}

#[derive(Debug, Serialize)]
struct SubmitTransactionResponse {
    status: &'static str,
    queued: bool,
    mempool_size: usize,
    req_total: u64,
}

#[derive(Debug, Serialize)]
struct PeersResponse {
    node_id: String,
    local_peer_id: Option<String>,
    peer_count: usize,
    peers: Vec<String>,
    req_total: u64,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn new<S: Into<String>>(status: StatusCode, message: S) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let payload = Json(ErrorResponse {
            error: self.message,
        });
        (self.status, payload).into_response()
    }
}

#[derive(Debug, Deserialize)]
struct L2Filter {
    #[serde(default)]
    l2_id: Option<String>,
}

pub async fn start_server(state: AppState, addr: &str) -> Result<()> {
    let shared = Arc::new(state);
    let app = build_router(shared.clone());
    let listener = bind_listener(addr).await?;
    axum::serve(listener, app)
        .await
        .context("RPC server terminated unexpectedly")
}

async fn bind_listener(addr: &str) -> Result<tokio::net::TcpListener> {
    if let Ok(socket_addr) = addr.parse::<SocketAddr>() {
        tokio::net::TcpListener::bind(socket_addr)
            .await
            .with_context(|| format!("failed to bind RPC listener on {socket_addr}"))
    } else {
        tokio::net::TcpListener::bind(addr)
            .await
            .with_context(|| format!("failed to bind RPC listener on {addr}"))
    }
}

fn build_router(state: SharedState) -> Router {
    let mut router = Router::new()
        .route("/health", get(handle_health))
        .route("/time", get(handle_time))
        .route("/version", get(handle_version))
        .route("/metrics", get(handle_metrics))
        .route("/tx", post(handle_submit_tx))
        .route("/tx/:hash", get(handle_get_transaction))
        .route("/block/:id", get(handle_get_block))
        .route("/account/:address", get(handle_get_account))
        .route("/peers", get(handle_get_peers))
        // Inbound HTTP P2P endpoints
        .route("/p2p/blocks", post(handle_p2p_blocks))
        .route("/p2p/transactions", post(handle_p2p_transactions))
        .route("/p2p/block-request", post(handle_p2p_block_request))
        .route("/p2p/block-response", post(handle_p2p_block_response))
        .route("/p2p/peer-info", post(handle_p2p_peer_info))
        .route("/p2p/peer-discovery", post(handle_p2p_peer_discovery))
        .route("/p2p/peers", get(handle_p2p_list_peers))
        .route("/l2/config", get(handle_get_l2_config))
        .route("/l2/networks", get(handle_list_l2_networks))
        .route("/l2/commits", get(handle_list_l2_commits))
        .route("/l2/commits/:l2_id", get(handle_list_l2_commits_for_l2))
        .route("/l2/exits", get(handle_list_l2_exits))
        .route("/l2/exits/:l2_id", get(handle_list_l2_exits_for_l2));

    if let Some(static_root) = state.static_assets_root() {
        if Path::new(&static_root).exists() {
            info!("Serving Unified UI assets from {:?}", static_root);
            router = router.fallback(serve_static_assets);
        } else {
            warn!(
                "Unified UI assets directory {:?} does not exist",
                static_root
            );
        }
    }

    router.layer(TraceLayer::new_for_http()).with_state(state)
}

async fn serve_static_assets(State(state): State<SharedState>, req: Request<Body>) -> Response {
    if let Some(static_root) = state.static_assets_root() {
        if Path::new(&static_root).exists() {
            let index_path = static_root.join("index.html");
            let service = ServeDir::new(static_root)
                .append_index_html_on_directories(true)
                .not_found_service(ServeFile::new(index_path));

            match service.oneshot(req).await {
                Ok(response) => response.into_response(),
                Err(err) => {
                    warn!("Static asset error: {}", err);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("failed to serve static asset: {err}"),
                    )
                        .into_response()
                }
            }
        } else {
            (StatusCode::NOT_FOUND, "Not Found").into_response()
        }
    } else {
        (StatusCode::NOT_FOUND, "Not Found").into_response()
    }
}

async fn handle_health(State(state): State<SharedState>) -> Result<Json<HealthResponse>, ApiError> {
    let req_total = state.record_request();
    let consensus =
        if let Some(consensus) = state.consensus.clone() {
            Some(consensus.snapshot().await.map_err(|err| {
                ApiError::internal(format!("failed to snapshot consensus: {err}"))
            })?)
        } else {
            None
        };

    let response = HealthResponse {
        status: "ok",
        node_id: state.node_id.clone(),
        uptime_secs: state.uptime_seconds(),
        peer_count: state.current_peer_count(),
        mempool_size: state.mempool_size(),
        consensus,
        req_total,
        time_us: ippan_time_now(),
        l2_config: state.l2_config.clone(),
    };

    Ok(Json(response))
}

async fn handle_time(State(state): State<SharedState>) -> Json<TimeResponse> {
    let req_total = state.record_request();
    let ippan_time = IppanTimeMicros::now();
    let unix_time_us = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|dur| dur.as_micros() as u64)
        .unwrap_or_default();

    Json(TimeResponse {
        node_id: state.node_id.clone(),
        ippan_time,
        unix_time_us,
        req_total,
    })
}

async fn handle_version(State(state): State<SharedState>) -> Json<VersionResponse> {
    state.record_request();
    Json(VersionResponse {
        node_id: state.node_id.clone(),
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn handle_metrics(State(state): State<SharedState>) -> Result<Response, ApiError> {
    let req_total = state.record_request();
    let uptime = state.uptime_seconds();
    let peer_count = state.current_peer_count();
    let mempool_size = state.mempool_size();

    let mut metrics =
        "# HELP ippan_http_requests_total Total number of RPC requests handled\n".to_string();
    metrics.push_str("# TYPE ippan_http_requests_total counter\n");
    metrics.push_str(&format!("ippan_http_requests_total {req_total}\n"));
    metrics.push_str("# HELP ippan_uptime_seconds Uptime of the node in seconds\n");
    metrics.push_str("# TYPE ippan_uptime_seconds gauge\n");
    metrics.push_str(&format!("ippan_uptime_seconds {uptime}\n"));
    metrics.push_str("# HELP ippan_peer_count Current connected peer count\n");
    metrics.push_str("# TYPE ippan_peer_count gauge\n");
    metrics.push_str(&format!("ippan_peer_count {peer_count}\n"));
    metrics.push_str("# HELP ippan_mempool_size Number of transactions in mempool\n");
    metrics.push_str("# TYPE ippan_mempool_size gauge\n");
    metrics.push_str(&format!("ippan_mempool_size {mempool_size}\n"));

    if let Some(consensus) = state.consensus.clone() {
        if let Ok(snapshot) = consensus.snapshot().await {
            metrics.push_str(
                "# HELP ippan_consensus_latest_block_height Latest finalized block height\n",
            );
            metrics.push_str("# TYPE ippan_consensus_latest_block_height gauge\n");
            metrics.push_str(&format!(
                "ippan_consensus_latest_block_height {}\n",
                snapshot.latest_block_height
            ));
            metrics.push_str("# HELP ippan_consensus_current_slot Current consensus slot\n");
            metrics.push_str("# TYPE ippan_consensus_current_slot gauge\n");
            metrics.push_str(&format!(
                "ippan_consensus_current_slot {}\n",
                snapshot.current_slot
            ));
        }
    }

    let mut response = Response::new(Body::from(metrics));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; version=0.0.4"),
    );
    Ok(response)
}

async fn handle_submit_tx(
    State(state): State<SharedState>,
    Json(tx): Json<Transaction>,
) -> Result<Json<SubmitTransactionResponse>, ApiError> {
    let req_total = state.record_request();

    let sender_result = if let Some(consensus) = state.consensus.clone() {
        consensus
            .submit_transaction(tx.clone())
            .map_err(|err| ApiError::internal(format!("failed to queue transaction: {err}")))
    } else if let Some(sender) = state.tx_sender.clone() {
        sender
            .send(tx.clone())
            .map_err(|err| ApiError::internal(format!("failed to enqueue transaction: {err}")))
    } else {
        Err(ApiError::service_unavailable(
            "transaction submission is disabled",
        ))
    };

    sender_result?;

    let response = SubmitTransactionResponse {
        status: "ok",
        queued: true,
        mempool_size: state.mempool_size(),
        req_total,
    };

    Ok(Json(response))
}

async fn handle_get_transaction(
    State(state): State<SharedState>,
    AxumPath(hash): AxumPath<String>,
) -> Result<Json<Transaction>, ApiError> {
    state.record_request();
    let hash_bytes = parse_hex_array::<32>(&hash, "transaction hash")?;

    let transaction = state
        .storage
        .get_transaction(&hash_bytes)
        .map_err(|err| ApiError::internal(format!("failed to fetch transaction: {err}")))?;

    transaction
        .map(Json)
        .ok_or_else(|| ApiError::not_found("transaction not found"))
}

async fn handle_get_block(
    State(state): State<SharedState>,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<Block>, ApiError> {
    state.record_request();

    let maybe_block = match parse_block_lookup(&id)? {
        BlockLookup::Height(height) => state
            .storage
            .get_block_by_height(height)
            .map_err(|err| ApiError::internal(format!("failed to fetch block by height: {err}")))?,
        BlockLookup::Hash(hash) => state
            .storage
            .get_block(&hash)
            .map_err(|err| ApiError::internal(format!("failed to fetch block by hash: {err}")))?,
    };

    maybe_block
        .map(Json)
        .ok_or_else(|| ApiError::not_found("block not found"))
}

async fn handle_get_account(
    State(state): State<SharedState>,
    AxumPath(address): AxumPath<String>,
) -> Result<Json<Account>, ApiError> {
    state.record_request();
    let address_bytes = parse_hex_array::<32>(&address, "account address")?;

    let account = state
        .storage
        .get_account(&address_bytes)
        .map_err(|err| ApiError::internal(format!("failed to fetch account: {err}")))?;

    account
        .map(Json)
        .ok_or_else(|| ApiError::not_found("account not found"))
}

async fn handle_get_peers(State(state): State<SharedState>) -> Json<PeersResponse> {
    let req_total = state.record_request();
    Json(PeersResponse {
        node_id: state.node_id.clone(),
        local_peer_id: state.local_peer_id(),
        peer_count: state.current_peer_count(),
        peers: state.peers(),
        req_total,
    })
}

// -------------------------
// Inbound HTTP P2P handlers
// -------------------------

async fn handle_p2p_blocks(
    State(state): State<SharedState>,
    Json(msg): Json<NetworkMessage>,
) -> Result<StatusCode, ApiError> {
    match msg {
        NetworkMessage::Block(block) => {
            state
                .storage
                .store_block(block)
                .map_err(|e| ApiError::internal(format!("failed to store block: {e}")))?;
            Ok(StatusCode::OK)
        }
        other => {
            warn!("/p2p/blocks received unexpected payload: {:?}", other);
            Ok(StatusCode::OK)
        }
    }
}

async fn handle_p2p_transactions(
    State(state): State<SharedState>,
    Json(msg): Json<NetworkMessage>,
) -> Result<StatusCode, ApiError> {
    match msg {
        NetworkMessage::Transaction(tx) => {
            // Reuse existing submission path (consensus or raw sender)
            if let Some(consensus) = state.consensus.clone() {
                consensus.submit_transaction(tx).map_err(|err| {
                    ApiError::internal(format!("failed to queue transaction: {err}"))
                })?;
            } else if let Some(sender) = state.tx_sender.clone() {
                sender
                    .send(tx)
                    .map_err(|err| ApiError::internal(format!("failed to enqueue transaction: {err}")))?;
            }
            Ok(StatusCode::OK)
        }
        other => {
            warn!("/p2p/transactions received unexpected payload: {:?}", other);
            Ok(StatusCode::OK)
        }
    }
}

async fn handle_p2p_block_request(
    State(state): State<SharedState>,
    Json(msg): Json<NetworkMessage>,
) -> Result<Response, ApiError> {
    if let NetworkMessage::BlockRequest { hash } = msg {
        // Log the block request for debugging
        debug!("Received block request for hash: {}", hex::encode(hash));
        
        // Check if we have the block
        match state
            .storage
            .get_block(&hash)
            .map_err(|e| ApiError::internal(format!("failed to read block: {e}")))?
        {
            Some(block) => {
                debug!("Block found for requested hash: {}", hex::encode(hash));
                // Return the block as a BlockResponse in the HTTP response body
                let response = NetworkMessage::BlockResponse(block);
                let json = serde_json::to_string(&response)
                    .map_err(|e| ApiError::internal(format!("failed to serialize block response: {e}")))?;
                
                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", "application/json")
                    .body(json.into())
                    .map_err(|e| ApiError::internal(format!("failed to build response: {e}")))?)
            }
            None => {
                debug!("Block not found for requested hash: {}", hex::encode(hash));
                Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body("Block not found".into())
                    .map_err(|e| ApiError::internal(format!("failed to build response: {e}")))?)
            }
        }
    } else {
        Ok(Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body("Invalid message type".into())
            .map_err(|e| ApiError::internal(format!("failed to build response: {e}")))?)
    }
}

async fn handle_p2p_block_response(
    State(state): State<SharedState>,
    Json(msg): Json<NetworkMessage>,
) -> Result<StatusCode, ApiError> {
    match msg {
        NetworkMessage::BlockResponse(block) => {
            state
                .storage
                .store_block(block)
                .map_err(|e| ApiError::internal(format!("failed to store block response: {e}")))?;
            Ok(StatusCode::OK)
        }
        other => {
            warn!("/p2p/block-response received unexpected payload: {:?}", other);
            Ok(StatusCode::OK)
        }
    }
}

#[derive(Debug, Deserialize)]
struct PeerInfoRequest {
    peer_id: String,
    addresses: Vec<String>,
    #[serde(default)]
    time_us: Option<u64>,
}

async fn handle_p2p_peer_info(
    State(state): State<SharedState>,
    Json(msg): Json<NetworkMessage>,
) -> Result<StatusCode, ApiError> {
    match msg {
        NetworkMessage::PeerInfo { peer_id, addresses, .. } => {
            if let Some(network) = &state.p2p_network {
                for addr in addresses {
                    if let Err(e) = network.add_peer(addr.clone()).await {
                        warn!("failed to add announced peer {addr} from {peer_id}: {e}");
                    }
                }
            }
            Ok(StatusCode::OK)
        }
        other => {
            warn!("/p2p/peer-info received unexpected payload: {:?}", other);
            Ok(StatusCode::OK)
        }
    }
}

async fn handle_p2p_peer_discovery(
    State(state): State<SharedState>,
    Json(msg): Json<NetworkMessage>,
) -> Result<StatusCode, ApiError> {
    match msg {
        NetworkMessage::PeerDiscovery { peers } => {
            if let Some(network) = &state.p2p_network {
                for addr in peers {
                    if let Err(e) = network.add_peer(addr.clone()).await {
                        warn!("failed to add discovered peer {addr}: {e}");
                    }
                }
            }
            Ok(StatusCode::OK)
        }
        other => {
            warn!("/p2p/peer-discovery received unexpected payload: {:?}", other);
            Ok(StatusCode::OK)
        }
    }
}

async fn handle_p2p_list_peers(State(state): State<SharedState>) -> Json<Vec<String>> {
    Json(state.peers())
}

async fn handle_get_l2_config(State(state): State<SharedState>) -> Json<L2ConfigResponse> {
    let req_total = state.record_request();
    Json(L2ConfigResponse {
        config: state.l2_config.clone(),
        req_total,
    })
}

async fn handle_list_l2_networks(
    State(state): State<SharedState>,
) -> Result<Json<L2NetworksResponse>, ApiError> {
    let req_total = state.record_request();
    let networks = state
        .storage
        .list_l2_networks()
        .map_err(|err| ApiError::internal(format!("failed to list L2 networks: {err}")))?;

    let total = networks.len() as u64;
    let response = L2NetworksResponse {
        networks,
        total,
        req_total,
    };

    Ok(Json(response))
}

async fn handle_list_l2_commits(
    State(state): State<SharedState>,
    Query(filter): Query<L2Filter>,
) -> Result<Json<L2CommitsResponse>, ApiError> {
    let req_total = state.record_request();
    let commits = state
        .storage
        .list_l2_commits(filter.l2_id.as_deref())
        .map_err(|err| ApiError::internal(format!("failed to list L2 commits: {err}")))?;

    let total = commits.len() as u64;
    let response = L2CommitsResponse {
        commits,
        total,
        req_total,
        l2_id: filter.l2_id,
    };

    Ok(Json(response))
}

async fn handle_list_l2_commits_for_l2(
    State(state): State<SharedState>,
    AxumPath(l2_id): AxumPath<String>,
) -> Result<Json<L2CommitsResponse>, ApiError> {
    let req_total = state.record_request();
    let commits = state
        .storage
        .list_l2_commits(Some(l2_id.as_str()))
        .map_err(|err| ApiError::internal(format!("failed to list L2 commits: {err}")))?;

    let total = commits.len() as u64;
    let response = L2CommitsResponse {
        total,
        req_total,
        commits,
        l2_id: Some(l2_id),
    };

    Ok(Json(response))
}

async fn handle_list_l2_exits(
    State(state): State<SharedState>,
    Query(filter): Query<L2Filter>,
) -> Result<Json<L2ExitsResponse>, ApiError> {
    let req_total = state.record_request();
    let exits = state
        .storage
        .list_l2_exits(filter.l2_id.as_deref())
        .map_err(|err| ApiError::internal(format!("failed to list L2 exits: {err}")))?;

    let total = exits.len() as u64;
    let response = L2ExitsResponse {
        exits,
        total,
        req_total,
        l2_id: filter.l2_id,
    };

    Ok(Json(response))
}

async fn handle_list_l2_exits_for_l2(
    State(state): State<SharedState>,
    AxumPath(l2_id): AxumPath<String>,
) -> Result<Json<L2ExitsResponse>, ApiError> {
    let req_total = state.record_request();
    let exits = state
        .storage
        .list_l2_exits(Some(l2_id.as_str()))
        .map_err(|err| ApiError::internal(format!("failed to list L2 exits: {err}")))?;

    let total = exits.len() as u64;
    let response = L2ExitsResponse {
        exits,
        total,
        req_total,
        l2_id: Some(l2_id),
    };

    Ok(Json(response))
}

fn parse_block_lookup(id: &str) -> Result<BlockLookup, ApiError> {
    if let Ok(height) = id.parse::<u64>() {
        Ok(BlockLookup::Height(height))
    } else {
        let hash = parse_hex_array::<32>(id, "block identifier")?;
        Ok(BlockLookup::Hash(hash))
    }
}

fn parse_hex_array<const N: usize>(value: &str, field: &str) -> Result<[u8; N], ApiError> {
    let normalized = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .unwrap_or(value)
        .trim();

    let mut output = [0u8; N];
    hex::decode_to_slice(normalized, &mut output).map_err(|_| {
        ApiError::bad_request(format!("invalid {field}: expected {N}-byte hex string"))
    })?;
    Ok(output)
}

enum BlockLookup {
    Height(u64),
    Hash([u8; 32]),
}

#[derive(Debug, Serialize)]
struct L2ConfigResponse {
    config: L2Config,
    req_total: u64,
}

#[derive(Debug, Serialize)]
struct L2NetworksResponse {
    networks: Vec<L2Network>,
    total: u64,
    req_total: u64,
}

#[derive(Debug, Serialize)]
struct L2CommitsResponse {
    commits: Vec<L2Commit>,
    total: u64,
    req_total: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    l2_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct L2ExitsResponse {
    exits: Vec<L2ExitRecord>,
    total: u64,
    req_total: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    l2_id: Option<String>,
}

impl ApiError {
    fn bad_request<S: Into<String>>(message: S) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }

    fn not_found<S: Into<String>>(message: S) -> Self {
        Self::new(StatusCode::NOT_FOUND, message)
    }

    fn internal<S: Into<String>>(message: S) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, message)
    }

    fn service_unavailable<S: Into<String>>(message: S) -> Self {
        Self::new(StatusCode::SERVICE_UNAVAILABLE, message)
    }
}
