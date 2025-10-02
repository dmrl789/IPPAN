use anyhow::Result;
use axum::extract::{Path, Query, State};
use axum::http::{Method, StatusCode};
use axum::routing::{get, get_service, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
// NOTE: removed `use std::convert::TryFrom;` to satisfy clippy (unused import)
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tracing::{info, warn};

use ippan_consensus::{ConsensusState, PoAConsensus, Validator};
use ippan_p2p::{HttpP2PNetwork, NetworkMessage};
use ippan_storage::Storage;
use ippan_types::{ippan_time_ingest_sample, Block, Transaction};

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
        Self { consensus, tx_sender, mempool }
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
    pub storage: Storage,
    pub consensus: ConsensusHandle,
    pub network: HttpP2PNetwork,
    pub start_at: Instant,
    pub req_count: Arc<AtomicUsize>,
    pub static_dir: Option<PathBuf>,
    pub l2_config: L2Config,
}

/// Shallow health response.
#[derive(Debug, Serialize)]
struct Health {
    status: &'static str,
    uptime_ms: u128,
    req_count: usize,
}

/// Consensus state envelope (so we can add fields later without breaking JSON shape).
#[derive(Debug, Serialize)]
struct StateEnvelope {
    state: ConsensusState,
    mempool_len: usize,
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

pub async fn run_rpc_server(
    storage: Storage,
    consensus: Arc<tokio::sync::Mutex<PoAConsensus>>,
    tx_sender: mpsc::UnboundedSender<Transaction>,
    mempool: Arc<parking_lot::RwLock<Vec<Transaction>>>,
    network: HttpP2PNetwork,
    bind_addr: SocketAddr,
    static_dir: Option<PathBuf>,
    l2_config: L2Config,
) -> Result<()> {
    // Touch symbol to avoid unused import warnings when feature-gated elsewhere.
    let _ = &ippan_time_ingest_sample;

    let consensus = ConsensusHandle::new(consensus, tx_sender, mempool);
    let state = AppState {
        storage,
        consensus,
        network,
        start_at: Instant::now(),
        req_count: Arc::new(AtomicUsize::new(0)),
        static_dir: static_dir.clone(),
        l2_config,
    };

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
    if let Some(dir) = &static_dir {
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

    let router = router.with_state(state);

    let listener = TcpListener::bind(bind_addr).await?;
    info!("RPC server listening on http://{bind_addr}");

    axum::serve(listener, router).await?;
    Ok(())
}

async fn health(State(app): State<AppState>) -> (StatusCode, Json<Health>) {
    let count = app.req_count.fetch_add(1, Ordering::Relaxed) + 1;
    let uptime_ms = app.start_at.elapsed().as_millis();

    (StatusCode::OK, Json(Health { status: "ok", uptime_ms, req_count: count }))
}

async fn get_state(State(app): State<AppState>) -> Result<Json<StateEnvelope>, (StatusCode, String)> {
    let state = app.consensus.get_state().await;
    let mempool_len = app.consensus.mempool().read().len(); // no unnecessary cast
    Ok(Json(StateEnvelope { state, mempool_len }))
}

async fn get_validators(State(app): State<AppState>) -> Result<Json<Vec<Validator>>, (StatusCode, String)> {
    let v = app.consensus.get_validators().await;
    Ok(Json(v))
}

async fn get_mempool(
    State(app): State<AppState>,
    Query(PageQuery { offset, limit }): Query<PageQuery>,
) -> Result<Json<Vec<Transaction>>, (StatusCode, String)> {
    let mem = app.consensus.mempool();
    let mem = mem.read();
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

async fn clear_mempool(State(app): State<AppState>) -> Result<Json<OkResponse>, (StatusCode, String)> {
    let mut mem = app.consensus.mempool().write();
    mem.clear();
    Ok(Json(OkResponse { ok: true }))
}

async fn get_l2_config(State(app): State<AppState>) -> Result<Json<L2Config>, (StatusCode, String)> {
    Ok(Json(app.l2_config.clone()))
}

async fn submit_tx(
    State(app): State<AppState>,
    Json(body): Json<SubmitTx>,
) -> Result<Json<OkResponse>, (StatusCode, String)> {
    app.consensus
        .submit_tx(body.tx)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("submit failed: {e}")))?;
    Ok(Json(OkResponse { ok: true }))
}

async fn broadcast(
    State(app): State<AppState>,
    Json(body): Json<BroadcastBody>,
) -> Result<Json<OkResponse>, (StatusCode, String)> {
    let msg = NetworkMessage::Custom {
        topic: body.topic.clone(),
        payload: body.payload.as_bytes().to_vec(),
    };
    app.network
        .broadcast(msg)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("broadcast failed: {e}")))?;
    Ok(Json(OkResponse { ok: true }))
}

/* ---------- Optional helpers for future extension ---------- */

#[allow(dead_code)]
fn _headers_map<'a>(pairs: impl IntoIterator<Item = (&'a str, &'a str)>) -> HashMap<String, String> {
    pairs
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[allow(dead_code)]
fn _set_to_vec<T: Clone + std::cmp::Eq + std::hash::Hash>(set: &HashSet<T>) -> Vec<T> {
    set.iter().cloned().collect()
}
