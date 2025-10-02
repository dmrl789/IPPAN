use anyhow::Result;
use axum::extract::{Query, State};
use axum::http::{Method, StatusCode};
use axum::routing::{get, get_service, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
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

use hex::encode;
use ippan_consensus::{ConsensusState, PoAConsensus, Validator};
use ippan_p2p::HttpP2PNetwork;
use ippan_storage::Storage;
use ippan_types::{ippan_time_ingest_sample, Block, Transaction};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
        Arc::clone(&self.mempool)
    }

    pub fn submit_tx(&self, tx: Transaction) -> Result<()> {
        // Push to inbound channel (non-blocking)
        self.tx_sender
            .send(tx)
            .map_err(|e| anyhow::anyhow!("failed to send tx to consensus: {e}"))
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
    pub req_count: Arc<AtomicUsize>,
}

/// Shallow health response.
#[derive(Debug, Serialize)]
struct Health {
    status: &'static str,
    uptime_ms: u128,
    req_count: usize,
    peer_count: usize,
    node_id: String,
}

/// Consensus state envelope serialized for RPC consumers.
#[derive(Debug, Serialize)]
struct StateEnvelope {
    current_slot: u64,
    current_proposer: Option<String>,
    is_proposing: bool,
    validator_count: usize,
    latest_block_height: u64,
    current_round: u64,
    mempool_len: usize,
}

impl StateEnvelope {
    fn from_state(state: ConsensusState, mempool_len: usize) -> Self {
        Self {
            current_slot: state.current_slot,
            current_proposer: state.current_proposer.map(encode),
            is_proposing: state.is_proposing,
            validator_count: state.validator_count,
            latest_block_height: state.latest_block_height,
            current_round: state.current_round,
            mempool_len,
        }
    }
}

/// Generic OK response.
#[derive(Debug, Serialize)]
struct OkResponse {
    ok: bool,
}

/// Submit-transaction DTO.
#[derive(Debug, Deserialize, Serialize)]
struct SubmitTx {
    tx: Transaction,
}

/// Optional query for paging.
#[derive(Debug, Deserialize)]
struct PageQuery {
    #[serde(default)]
    offset: usize,
    #[serde(default)]
    limit: usize,
}

/// Simple network broadcast body.
#[derive(Debug, Deserialize)]
struct BroadcastBody {
    topic: String,
    payload: String,
}

pub async fn start_server(app_state: AppState, bind_addr: &str) -> Result<()> {
    let socket_addr: SocketAddr = bind_addr.parse()?;
    run_rpc_server(app_state, socket_addr).await
}

async fn run_rpc_server(app_state: AppState, bind_addr: SocketAddr) -> Result<()> {
    // Touch symbol to avoid unused import warnings when feature-gated elsewhere.
    let _ = &ippan_time_ingest_sample;

    let mut router = Router::new()
        // health & basic info
        .route("/health", get(health))
        .route("/state", get(get_state))
        .route("/validators", get(get_validators))
        .route("/mempool", get(get_mempool))
        .route("/mempool/clear", post(clear_mempool))
        .route("/config/l2", get(get_l2_config))
        // txs
        .route("/tx", post(submit_tx))
        // network broadcast (basic diagnostic)
        .route("/network/broadcast", post(broadcast));

    // Serve static (UI) if provided; index.html at root.
    if let Some(dir) = &app_state.unified_ui_dist {
        router = router.nest_service(
            "/",
            get_service(ServeDir::new(dir).fallback(ServeFile::new(dir.join("index.html")))),
        );
    }

    // Add permissive CORS for tooling; tighten in production as needed.
    router = router.layer(
        CorsLayer::new()
            .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
            .allow_origin(Any)
            .allow_headers(Any),
    );

    let router = router.with_state(app_state);

    let listener = TcpListener::bind(bind_addr).await?;
    info!("RPC server listening on http://{bind_addr}");

    axum::serve(listener, router).await?;
    Ok(())
}

async fn health(State(app): State<AppState>) -> (StatusCode, Json<Health>) {
    let count = app.req_count.fetch_add(1, Ordering::Relaxed) + 1;
    let uptime_ms = app.start_time.elapsed().as_millis();
    let peer_count = app.peer_count.load(Ordering::Relaxed);

    (
        StatusCode::OK,
        Json(Health {
            status: "ok",
            uptime_ms,
            req_count: count,
            peer_count,
            node_id: app.node_id.clone(),
        }),
    )
}

async fn get_state(
    State(app): State<AppState>,
) -> Result<Json<StateEnvelope>, (StatusCode, String)> {
    let consensus = app.consensus.clone().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        "consensus unavailable".to_string(),
    ))?;

    let state = consensus.get_state().await;
    let mempool_len = app.mempool.read().len();
    Ok(Json(StateEnvelope::from_state(state, mempool_len)))
}

async fn get_validators(
    State(app): State<AppState>,
) -> Result<Json<Vec<Validator>>, (StatusCode, String)> {
    let consensus = app.consensus.clone().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        "consensus unavailable".to_string(),
    ))?;

    let v = consensus.get_validators().await;
    Ok(Json(v))
}

async fn get_mempool(
    State(app): State<AppState>,
    Query(PageQuery { offset, limit }): Query<PageQuery>,
) -> Result<Json<Vec<Transaction>>, (StatusCode, String)> {
    let mem = app.mempool.read();
    let total = mem.len();

    let start = offset.min(total);
    // default limit = 0 -> return all from start
    let end = if limit == 0 {
        total
    } else {
        (start.saturating_add(limit)).min(total)
    };

    let slice = mem[start..end].to_vec();
    Ok(Json(slice))
}

async fn clear_mempool(
    State(app): State<AppState>,
) -> Result<Json<OkResponse>, (StatusCode, String)> {
    app.mempool.write().clear();
    Ok(Json(OkResponse { ok: true }))
}

async fn get_l2_config(
    State(app): State<AppState>,
) -> Result<Json<L2Config>, (StatusCode, String)> {
    Ok(Json(app.l2_config.clone()))
}

async fn submit_tx(
    State(app): State<AppState>,
    Json(body): Json<SubmitTx>,
) -> Result<Json<OkResponse>, (StatusCode, String)> {
    let tx = body.tx;

    if let Some(consensus) = app.consensus.clone() {
        consensus.submit_tx(tx).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("submit failed: {e}"),
            )
        })?;
    } else if let Some(sender) = app.tx_sender.clone() {
        sender.send(tx).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("submit failed: {e}"),
            )
        })?;
    } else {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "transaction submission unavailable".to_string(),
        ));
    }

    Ok(Json(OkResponse { ok: true }))
}

async fn broadcast(
    State(app): State<AppState>,
    Json(body): Json<BroadcastBody>,
) -> Result<Json<OkResponse>, (StatusCode, String)> {
    let network = app.p2p_network.clone().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        "network unavailable".to_string(),
    ))?;

    match body.topic.as_str() {
        "block" => {
            let block: Block = serde_json::from_str(&body.payload).map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    format!("invalid block payload: {e}"),
                )
            })?;
            network
                .broadcast_block(block)
                .await
                .map_err(|e| (StatusCode::BAD_GATEWAY, format!("broadcast failed: {e}")))?;
        }
        "transaction" => {
            let tx: Transaction = serde_json::from_str(&body.payload).map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    format!("invalid transaction payload: {e}"),
                )
            })?;
            network
                .broadcast_transaction(tx)
                .await
                .map_err(|e| (StatusCode::BAD_GATEWAY, format!("broadcast failed: {e}")))?;
        }
        "announce" => {
            network
                .announce_self()
                .map_err(|e| (StatusCode::BAD_GATEWAY, format!("broadcast failed: {e}")))?;
        }
        other => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("unsupported broadcast topic: {other}"),
            ));
        }
    }

    Ok(Json(OkResponse { ok: true }))
}
