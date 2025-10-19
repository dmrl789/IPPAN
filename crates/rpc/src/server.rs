use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{anyhow, Context, Result};
use axum::body::Body;
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::{header, Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use hex::FromHex;
use ippan_consensus::{ConsensusState, PoAConsensus};
use ippan_mempool::Mempool;
use ippan_p2p::HttpP2PNetwork;
use ippan_storage::{Account, Storage};
use ippan_types::time_service::ippan_time_now;
use ippan_types::{Block, IppanTimeMicros, Transaction};
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
