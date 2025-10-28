use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{anyhow, Context, Result};
use axum::extract::{Path as AxumPath, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use ippan_consensus::{ConsensusState, PoAConsensus};
use ippan_mempool::Mempool;
use ippan_storage::{Account, Storage};
use ippan_types::time_service::ippan_time_now;
use ippan_types::{Block, L2Commit, L2ExitRecord, L2Network, Transaction};
use serde::Serialize;
use tokio::sync::{mpsc, Mutex};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing::{debug, info, warn};

use crate::{HttpP2PNetwork, NetworkMessage};

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
    pub storage: Arc<Storage>,
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

/// Consensus handle abstraction
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
}

/// Simplified consensus state view
#[derive(Clone, Debug, Serialize)]
pub struct ConsensusStateView {
    pub round: u64,
    pub validators: Vec<String>,
}

impl From<ConsensusState> for ConsensusStateView {
    fn from(state: ConsensusState) -> Self {
        Self {
            round: state.round,
            validators: state.validators,
        }
    }
}

/// Start the RPC server
pub async fn start_server(state: AppState, addr: &str) -> Result<()> {
    info!("Starting RPC server on {}", addr);
    let shared = Arc::new(state);
    let app = build_router(shared.clone());
    let listener = bind_listener(addr).await?;
    info!("RPC server listening on {}", listener.local_addr()?);
    axum::serve(listener, app)
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
    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);
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
        .route("/p2p/blocks", post(handle_p2p_blocks))
        .route("/p2p/transactions", post(handle_p2p_transactions))
        .route("/p2p/peer-info", post(handle_p2p_peer_info))
        .route("/p2p/peer-discovery", post(handle_p2p_peer_discovery))
        .route("/l2/config", get(handle_get_l2_config))
        .route("/l2/networks", get(handle_list_l2_networks))
        .route("/l2/commits", get(handle_list_l2_commits))
        .route("/l2/exits", get(handle_list_l2_exits));

    if let Some(static_root) = &state.unified_ui_dist {
        if Path::new(static_root).exists() {
            info!("Serving Unified UI from {:?}", static_root);
            router = router.fallback_service(ServeDir::new(static_root));
        } else {
            warn!("Static UI directory {:?} not found", static_root);
        }
    }

    router.layer(cors).layer(TraceLayer::new_for_http()).with_state(state)
}

// -----------------------------------------------------------------------------
// Handlers
// -----------------------------------------------------------------------------

async fn handle_health(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": ippan_time_now(),
        "version": env!("CARGO_PKG_VERSION"),
        "peer_count": state.peer_count.load(Ordering::Relaxed),
        "chain_id": state.l2_config.max_l2_count
    }))
}

async fn handle_time() -> Json<serde_json::Value> {
    let now = ippan_time_now();
    Json(serde_json::json!({ "timestamp": now, "time_us": now }))
}

async fn handle_version() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "build_time": "unknown",
        "git_commit": "unknown"
    }))
}

async fn handle_metrics() -> (StatusCode, &'static str) {
    (
        StatusCode::OK,
        "# HELP ippan_peer_count Connected peers\nippan_peer_count 0\n",
    )
}

async fn handle_submit_tx(
    State(state): State<Arc<AppState>>,
    Json(tx): Json<Transaction>,
) -> (StatusCode, &'static str) {
    if let Some(consensus) = &state.consensus {
        if let Err(e) = consensus.submit_transaction(tx.clone()) {
            warn!("Failed to enqueue transaction: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to submit tx");
        }
        (StatusCode::OK, "Transaction accepted")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "Consensus not active")
    }
}

async fn handle_get_transaction(AxumPath(_hash): AxumPath<String>) -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "Transaction not found")
}

async fn handle_get_block(AxumPath(_id): AxumPath<String>) -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "Block not found")
}

async fn handle_get_account(AxumPath(_addr): AxumPath<String>) -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "Account not found")
}

async fn handle_get_peers(State(state): State<Arc<AppState>>) -> Json<Vec<String>> {
    if let Some(net) = &state.p2p_network {
        Json(net.get_peers())
    } else {
        Json(vec![])
    }
}

// -----------------------------------------------------------------------------
// P2P Handlers
// -----------------------------------------------------------------------------

async fn handle_p2p_blocks() -> StatusCode {
    StatusCode::OK
}

async fn handle_p2p_transactions() -> StatusCode {
    StatusCode::OK
}

async fn handle_p2p_peer_info() -> StatusCode {
    StatusCode::OK
}

async fn handle_p2p_peer_discovery() -> StatusCode {
    StatusCode::OK
}

// -----------------------------------------------------------------------------
// L2 Endpoints
// -----------------------------------------------------------------------------

async fn handle_get_l2_config(State(state): State<Arc<AppState>>) -> Json<L2Config> {
    Json(state.l2_config.clone())
}

async fn handle_list_l2_networks() -> Json<Vec<L2Network>> {
    Json(vec![])
}

async fn handle_list_l2_commits() -> Json<Vec<L2Commit>> {
    Json(vec![])
}

async fn handle_list_l2_exits() -> Json<Vec<L2ExitRecord>> {
    Json(vec![])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_endpoint() {
        let app_state = Arc::new(AppState {
            storage: Arc::new(Storage::default()),
            start_time: Instant::now(),
            peer_count: Arc::new(AtomicUsize::new(0)),
            p2p_network: None,
            tx_sender: None,
            node_id: "test-node".into(),
            consensus: None,
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
        });

        let response = handle_health(State(app_state)).await;
        let json = response.0;
        assert_eq!(json.get("status").unwrap(), "healthy");
    }
}
