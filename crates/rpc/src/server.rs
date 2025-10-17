use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{anyhow, Context, Result};
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, get_service, post};
use axum::{Json, Router};
use hex::FromHex;
use ippan_consensus::{ConsensusState, PoAConsensus};
use ippan_p2p::HttpP2PNetwork;
use ippan_storage::{Account, Storage};
use ippan_types::time_service::ippan_time_now;
use ippan_types::{Block, IppanTimeMicros, Transaction};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tower::ServiceExt;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use tracing::{info, warn};

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
        self.mempool.read().len()
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
    pub mempool: Arc<RwLock<Vec<Transaction>>>,
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
        self.mempool.read().len()
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
struct ConsensusStateView {
    current_slot: u64,
    current_round: u64,
    latest_block_height: u64,
    validator_count: usize,
    is_proposing: bool,
    current_proposer: Option<String>,
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
        .route("/l2/config", get(handle_get_l2_config))
        .route("/l2/networks", get(handle_list_l2_networks))
        .route("/l2/commits", get(handle_list_l2_commits))
        .route("/l2/commits/:l2_id", get(handle_list_l2_commits_for_l2))
        .route("/l2/exits", get(handle_list_l2_exits))
        .route("/l2/exits/:l2_id", get(handle_list_l2_exits_for_l2))
        .with_state(state.clone())
        .layer(TraceLayer::new_for_http());

    if let Some(static_root) = state.static_assets_root() {
        if Path::new(&static_root).exists() {
            info!("Serving Unified UI assets from {:?}", static_root);
            let index_path = static_root.join("index.html");
            let service = ServeDir::new(static_root.clone())
                .append_index_html_on_directories(true)
                .not_found_service(ServeFile::new(index_path));

            router = router.fallback_service(get_service(service).handle_error(
                |err: std::io::Error| async move {
                    warn!("Static asset error: {}", err);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("failed to serve static asset: {err}"),
                    )
                },
            ));
        } else {
            warn!(
                "Unified UI assets directory {:?} does not exist",
                static_root
            );
        }
    }

    router
}

async fn handle_health(State(state): State<SharedState>) -> impl IntoResponse {
    let req_total = state.record_request();
    let consensus_view = if let Some(handle) = &state.consensus {
        match handle.snapshot().await {
            Ok(view) => Some(view),
            Err(err) => {
                warn!("Failed to fetch consensus state: {}", err);
                None
            }
        }
    } else {
        None
    };

    let response = HealthResponse {
        status: "ok",
        node_id: state.node_id.clone(),
        uptime_secs: state.uptime_seconds(),
        peer_count: state.current_peer_count(),
        mempool_size: state.mempool_size(),
        consensus: consensus_view,
        req_total,
        time_us: ippan_time_now(),
        l2_config: state.l2_config.clone(),
    };

    Json(response)
}

async fn handle_time(State(state): State<SharedState>) -> impl IntoResponse {
    let req_total = state.record_request();
    let response = TimeResponse {
        node_id: state.node_id.clone(),
        ippan_time: IppanTimeMicros::now(),
        unix_time_us: ippan_time_now(),
        req_total,
    };
    Json(response)
}

async fn handle_version(State(state): State<SharedState>) -> impl IntoResponse {
    state.record_request();
    Json(VersionResponse {
        node_id: state.node_id.clone(),
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn handle_metrics(State(state): State<SharedState>) -> impl IntoResponse {
    let total = state.record_request();
    let body = format!("ippan_rpc_requests_total {}\n", total);
    ([(header::CONTENT_TYPE, "text/plain; version=0.0.4")], body)
}

async fn handle_submit_tx(
    State(state): State<SharedState>,
    Json(tx): Json<Transaction>,
) -> Result<Json<SubmitTransactionResponse>, ApiError> {
    let req_total = state.record_request();

    if let Some(sender) = &state.tx_sender {
        sender.send(tx.clone()).map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to enqueue transaction: {err}"),
            )
        })?;
    } else if let Some(handle) = &state.consensus {
        handle
            .submit_transaction(tx.clone())
            .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
    } else {
        return Err(ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "transaction submission unavailable",
        ));
    }

    if let Err(err) = state.storage.store_transaction(tx.clone()) {
        warn!(
            "Failed to persist transaction {}: {}",
            hex::encode(tx.id),
            err
        );
    }

    let mempool_size = state.mempool_size();
    Ok(Json(SubmitTransactionResponse {
        status: "accepted",
        queued: true,
        mempool_size,
        req_total,
    }))
}

async fn handle_get_transaction(
    State(state): State<SharedState>,
    AxumPath(hash): AxumPath<String>,
) -> Result<Json<Transaction>, ApiError> {
    state.record_request();
    let hash_bytes = decode_hex_32(&hash).ok_or_else(|| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "transaction hash must be 32-byte hex string",
        )
    })?;

    match state.storage.get_transaction(&hash_bytes) {
        Ok(Some(tx)) => Ok(Json(tx)),
        Ok(None) => Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "transaction not found",
        )),
        Err(err) => Err(ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("storage error: {err}"),
        )),
    }
}

async fn handle_get_block(
    State(state): State<SharedState>,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<Block>, ApiError> {
    state.record_request();
    match parse_block_lookup(&id) {
        BlockLookup::Height(height) => match state.storage.get_block_by_height(height) {
            Ok(Some(block)) => Ok(Json(block)),
            Ok(None) => Err(ApiError::new(StatusCode::NOT_FOUND, "block not found")),
            Err(err) => Err(ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("storage error: {err}"),
            )),
        },
        BlockLookup::Hash(hash) => match state.storage.get_block(&hash) {
            Ok(Some(block)) => Ok(Json(block)),
            Ok(None) => Err(ApiError::new(StatusCode::NOT_FOUND, "block not found")),
            Err(err) => Err(ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("storage error: {err}"),
            )),
        },
        BlockLookup::Invalid(message) => Err(ApiError::new(StatusCode::BAD_REQUEST, message)),
    }
}

async fn handle_get_account(
    State(state): State<SharedState>,
    AxumPath(address): AxumPath<String>,
) -> Result<Json<Account>, ApiError> {
    state.record_request();
    let address_bytes = decode_hex_32(&address).ok_or_else(|| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "account address must be 32-byte hex string",
        )
    })?;

    match state.storage.get_account(&address_bytes) {
        Ok(Some(account)) => Ok(Json(account)),
        Ok(None) => Err(ApiError::new(StatusCode::NOT_FOUND, "account not found")),
        Err(err) => Err(ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("storage error: {err}"),
        )),
    }
}

async fn handle_get_peers(State(state): State<SharedState>) -> impl IntoResponse {
    let req_total = state.record_request();
    Json(PeersResponse {
        node_id: state.node_id.clone(),
        local_peer_id: state.local_peer_id(),
        peer_count: state.current_peer_count(),
        peers: state.peers(),
        req_total,
    })
}

async fn handle_get_l2_config(State(state): State<SharedState>) -> impl IntoResponse {
    state.record_request();
    Json(state.l2_config.clone())
}

async fn handle_list_l2_networks(State(state): State<SharedState>) -> impl IntoResponse {
    state.record_request();
    match state.storage.list_l2_networks() {
        Ok(networks) => Json(networks).into_response(),
        Err(err) => ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("storage error: {err}"),
        )
        .into_response(),
    }
}

async fn handle_list_l2_commits(
    State(state): State<SharedState>,
    Query(filter): Query<L2Filter>,
) -> impl IntoResponse {
    state.record_request();
    let result = state.storage.list_l2_commits(filter.l2_id.as_deref());

    match result {
        Ok(commits) => Json(commits).into_response(),
        Err(err) => ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("storage error: {err}"),
        )
        .into_response(),
    }
}

async fn handle_list_l2_commits_for_l2(
    State(state): State<SharedState>,
    AxumPath(l2_id): AxumPath<String>,
) -> impl IntoResponse {
    state.record_request();
    match state.storage.list_l2_commits(Some(&l2_id)) {
        Ok(commits) => Json(commits).into_response(),
        Err(err) => ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("storage error: {err}"),
        )
        .into_response(),
    }
}

async fn handle_list_l2_exits(
    State(state): State<SharedState>,
    Query(filter): Query<L2Filter>,
) -> impl IntoResponse {
    state.record_request();
    let result = state.storage.list_l2_exits(filter.l2_id.as_deref());
    match result {
        Ok(exits) => Json(exits).into_response(),
        Err(err) => ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("storage error: {err}"),
        )
        .into_response(),
    }
}

async fn handle_list_l2_exits_for_l2(
    State(state): State<SharedState>,
    AxumPath(l2_id): AxumPath<String>,
) -> impl IntoResponse {
    state.record_request();
    match state.storage.list_l2_exits(Some(&l2_id)) {
        Ok(exits) => Json(exits).into_response(),
        Err(err) => ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("storage error: {err}"),
        )
        .into_response(),
    }
}

fn decode_hex_32(value: &str) -> Option<[u8; 32]> {
    let trimmed = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .unwrap_or(value);
    let bytes = <Vec<u8>>::from_hex(trimmed).ok()?;
    if bytes.len() != 32 {
        return None;
    }
    let mut buf = [0u8; 32];
    buf.copy_from_slice(&bytes);
    Some(buf)
}

enum BlockLookup {
    Height(u64),
    Hash([u8; 32]),
    Invalid(String),
}

fn parse_block_lookup(input: &str) -> BlockLookup {
    if let Ok(height) = input.parse::<u64>() {
        return BlockLookup::Height(height);
    }

    match decode_hex_32(input) {
        Some(hash) => BlockLookup::Hash(hash),
        None => BlockLookup::Invalid("block identifier must be height or 32-byte hex".into()),
    }
}
