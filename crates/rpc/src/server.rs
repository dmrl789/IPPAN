use std::convert::Infallible;
use std::future::Future;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::task::{Context as TaskContext, Poll};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context as AnyhowContext, Result};
use axum::body::Body;
use axum::extract::{ConnectInfo, Path as AxumPath, Query, State};
use axum::http::header::{HeaderValue, CONTENT_TYPE};
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use ippan_consensus::PoAConsensus;
use ippan_mempool::Mempool;
use ippan_security::{SecurityError, SecurityManager};
use ippan_storage::{Account, Storage};
use ippan_types::time_service::ippan_time_now;
use ippan_types::{Block, L2Commit, L2ExitRecord, L2Network, Transaction};
use metrics_exporter_prometheus::PrometheusHandle;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, Mutex};
use tower::Layer;
use tower::Service;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing::{debug, error, info, warn};

use hex::encode as hex_encode;

use crate::{HttpP2PNetwork, NetworkMessage};

const RATE_LIMIT_PER_SECOND: u64 = 200;
const CIRCUIT_BREAKER_FAILURE_THRESHOLD: usize = 5;
const CIRCUIT_BREAKER_OPEN_SECS: u64 = 30;

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
    pub security: Option<Arc<SecurityManager>>,
    pub metrics: Option<PrometheusHandle>,
}

/// Consensus handle abstraction
#[derive(Clone)]
pub struct ConsensusHandle {
    consensus: Arc<Mutex<PoAConsensus>>,
    tx_sender: mpsc::UnboundedSender<Transaction>,
    #[allow(dead_code)]
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
        let state = guard.get_state();
        let validators: Vec<String> = guard
            .config
            .validators
            .iter()
            .map(|v| hex::encode(v.id))
            .collect();
        Ok(ConsensusStateView {
            round: state.current_round,
            validators,
        })
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

/// Transaction lookup response payload used by JSON responses.
#[derive(Debug, Serialize)]
struct TransactionEnvelope {
    hash: String,
    transaction: Transaction,
}

/// Account lookup response payload with recent transaction history.
#[derive(Debug, Serialize)]
struct AccountResponse {
    address: String,
    balance: u64,
    nonce: u64,
    transactions: Vec<TransactionEnvelope>,
}

/// Optional filter parameters for L2 endpoints.
#[derive(Debug, Default, Deserialize)]
struct L2Filter {
    #[serde(default)]
    l2_id: Option<String>,
}

/// Start the RPC server
pub async fn start_server(state: AppState, addr: &str) -> Result<()> {
    info!("Starting RPC server on {}", addr);
    let shared = Arc::new(state);
    let app = build_router(shared.clone());
    let listener = bind_listener(addr).await?;
    let bound_addr = listener.local_addr()?;
    info!("RPC server listening on {}", bound_addr);
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
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
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    let rate_limiter = RateLimiterLayer::new(RATE_LIMIT_PER_SECOND, Duration::from_secs(1));
    let circuit_breaker = CircuitBreakerLayer::new(
        CIRCUIT_BREAKER_FAILURE_THRESHOLD,
        Duration::from_secs(CIRCUIT_BREAKER_OPEN_SECS),
    );

    let mut router = Router::new()
        .route("/health", get(handle_health))
        .route("/status", get(handle_status))
        .route("/time", get(handle_time))
        .route("/version", get(handle_version))
        .route("/metrics", get(handle_metrics))
        .route("/tx", post(handle_submit_tx))
        .route("/tx/:hash", get(handle_get_transaction))
        .route("/block/:id", get(handle_get_block))
        .route("/account/:address", get(handle_get_account))
        .route("/peers", get(handle_get_peers))
        .route("/p2p/peers", get(handle_get_p2p_peers))
        .route("/p2p/blocks", post(handle_p2p_blocks))
        .route("/p2p/transactions", post(handle_p2p_transactions))
        .route("/p2p/peer-info", post(handle_p2p_peer_info))
        .route("/p2p/peer-discovery", post(handle_p2p_peer_discovery))
        .route("/p2p/block-request", post(handle_p2p_block_request))
        .route("/p2p/block-response", post(handle_p2p_block_response))
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

    router
        .layer(cors)
        .layer(rate_limiter)
        .layer(circuit_breaker)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn guard_request(
    state: &Arc<AppState>,
    addr: &SocketAddr,
    endpoint: &str,
) -> Result<(), SecurityError> {
    if let Some(security) = &state.security {
        security.check_request(addr.ip(), endpoint).await?;
    }

    Ok(())
}

async fn record_security_success(state: &Arc<AppState>, addr: &SocketAddr, endpoint: &str) {
    if let Some(security) = &state.security {
        if let Err(err) = security.record_success(addr.ip(), endpoint).await {
            warn!(
                "Failed to record security success for {} from {}: {}",
                endpoint, addr, err
            );
        }
    }
}

async fn record_security_failure(
    state: &Arc<AppState>,
    addr: &SocketAddr,
    endpoint: &str,
    reason: &str,
) {
    if let Some(security) = &state.security {
        if let Err(err) = security
            .record_failed_attempt(addr.ip(), endpoint, reason)
            .await
        {
            warn!(
                "Failed to record security failed attempt for {} from {}: {}",
                endpoint, addr, err
            );
        }

        if let Err(err) = security.record_failure(addr.ip(), endpoint, reason).await {
            warn!(
                "Failed to record security failure for {} from {}: {}",
                endpoint, addr, err
            );
        }
    }
}

fn map_security_error(err: &SecurityError) -> (StatusCode, &'static str) {
    match err {
        SecurityError::IpBlocked => (StatusCode::FORBIDDEN, "IP address blocked"),
        SecurityError::IpNotWhitelisted => (StatusCode::FORBIDDEN, "IP address not permitted"),
        SecurityError::RateLimitExceeded => (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded"),
        SecurityError::CircuitBreakerOpen => (
            StatusCode::SERVICE_UNAVAILABLE,
            "Service temporarily unavailable",
        ),
        SecurityError::ValidationFailed(_) => (StatusCode::BAD_REQUEST, "Invalid request payload"),
        SecurityError::AuditFailed(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, "Security audit failure")
        }
    }
}

async fn deny_request(
    state: &Arc<AppState>,
    addr: &SocketAddr,
    endpoint: &str,
    err: SecurityError,
) -> (StatusCode, &'static str) {
    let reason = err.to_string();
    warn!(
        "Security rejected request {} from {}: {}",
        endpoint, addr, reason
    );
    record_security_failure(state, addr, endpoint, &reason).await;
    map_security_error(&err)
}

// -----------------------------------------------------------------------------
// Handlers
// -----------------------------------------------------------------------------

async fn handle_health(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    metrics::gauge!("node_health").set(1.0);
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": ippan_time_now(),
        "version": env!("CARGO_PKG_VERSION"),
        "peer_count": state.peer_count.load(Ordering::Relaxed),
        "chain_id": state.l2_config.max_l2_count
    }))
}

async fn handle_status(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let uptime_seconds = state.start_time.elapsed().as_secs();
    let peer_count = state.peer_count.load(Ordering::Relaxed);
    let requests_served = state.req_count.load(Ordering::Relaxed);
    let mempool_size = state.mempool.size();
    metrics::gauge!("mempool_size").set(mempool_size as f64);
    let mut consensus_round_metric = 0.0;

    let consensus_view = if let Some(consensus) = &state.consensus {
        match consensus.snapshot().await {
            Ok(view) => {
                consensus_round_metric = view.round as f64;
                Some(serde_json::json!({
                    "round": view.round,
                    "validator_count": view.validators.len(),
                    "validators": view.validators,
                }))
            }
            Err(err) => {
                warn!("Failed to snapshot consensus state: {}", err);
                None
            }
        }
    } else {
        None
    };
    metrics::gauge!("consensus_round").set(consensus_round_metric);

    Json(serde_json::json!({
        "status": "ok",
        "node_id": state.node_id.clone(),
        "version": env!("CARGO_PKG_VERSION"),
        "peer_count": peer_count,
        "uptime_seconds": uptime_seconds,
        "requests_served": requests_served,
        "network_active": state.p2p_network.is_some(),
        "consensus": consensus_view,
        "mempool_size": mempool_size
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

async fn handle_metrics(State(state): State<Arc<AppState>>) -> Response {
    if let Some(handle) = &state.metrics {
        let mut response = Response::new(Body::from(handle.render()));
        *response.status_mut() = StatusCode::OK;
        response.headers_mut().insert(
            CONTENT_TYPE,
            HeaderValue::from_static("text/plain; version=0.0.4"),
        );
        response
    } else {
        let mut response = Response::new(Body::from("Prometheus metrics disabled"));
        *response.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
        response
    }
}

async fn handle_submit_tx(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(tx): Json<Transaction>,
) -> (StatusCode, &'static str) {
    if let Err(err) = guard_request(&state, &addr, "/tx").await {
        return deny_request(&state, &addr, "/tx", err).await;
    }

    if let Some(consensus) = &state.consensus {
        if let Err(e) = consensus.submit_transaction(tx.clone()) {
            warn!("Failed to enqueue transaction: {}", e);
            record_security_failure(&state, &addr, "/tx", &e.to_string()).await;
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to submit tx");
        }
        record_security_success(&state, &addr, "/tx").await;
        (StatusCode::OK, "Transaction accepted")
    } else {
        record_security_failure(&state, &addr, "/tx", "Consensus not active").await;
        (StatusCode::SERVICE_UNAVAILABLE, "Consensus not active")
    }
}

async fn handle_get_transaction(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    AxumPath(hash): AxumPath<String>,
) -> Result<Json<TransactionEnvelope>, (StatusCode, &'static str)> {
    if let Err(err) = guard_request(&state, &addr, "/tx/:hash").await {
        return Err(deny_request(&state, &addr, "/tx/:hash", err).await);
    }

    let hash_bytes = match parse_hex_32(&hash) {
        Ok(bytes) => bytes,
        Err(err) => {
            warn!("Invalid transaction hash from {}: {} ({})", addr, hash, err);
            record_security_failure(&state, &addr, "/tx/:hash", "Invalid transaction hash").await;
            return Err((StatusCode::BAD_REQUEST, "Invalid transaction hash"));
        }
    };

    match state.storage.get_transaction(&hash_bytes) {
        Ok(Some(tx)) => {
            let envelope = TransactionEnvelope {
                hash: hex_encode(tx.hash()),
                transaction: tx,
            };
            record_security_success(&state, &addr, "/tx/:hash").await;
            Ok(Json(envelope))
        }
        Ok(None) => {
            record_security_success(&state, &addr, "/tx/:hash").await;
            Err((StatusCode::NOT_FOUND, "Transaction not found"))
        }
        Err(err) => {
            error!("Failed fetching transaction {} for {}: {}", hash, addr, err);
            record_security_failure(&state, &addr, "/tx/:hash", &err.to_string()).await;
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to load transaction",
            ))
        }
    }
}

async fn handle_get_block(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<Block>, (StatusCode, &'static str)> {
    if let Err(err) = guard_request(&state, &addr, "/block/:id").await {
        return Err(deny_request(&state, &addr, "/block/:id", err).await);
    }

    let identifier = match parse_block_identifier(&id) {
        Some(identifier) => identifier,
        None => {
            warn!("Invalid block identifier from {}: {}", addr, id);
            record_security_failure(&state, &addr, "/block/:id", "Invalid block identifier").await;
            return Err((StatusCode::BAD_REQUEST, "Invalid block identifier"));
        }
    };

    let block_result = match identifier {
        BlockIdentifier::Hash(hash) => state.storage.get_block(&hash),
        BlockIdentifier::Height(height) => state.storage.get_block_by_height(height),
    };

    match block_result {
        Ok(Some(block)) => {
            record_security_success(&state, &addr, "/block/:id").await;
            Ok(Json(block))
        }
        Ok(None) => {
            record_security_success(&state, &addr, "/block/:id").await;
            Err((StatusCode::NOT_FOUND, "Block not found"))
        }
        Err(err) => {
            error!("Failed fetching block {} for {}: {}", id, addr, err);
            record_security_failure(&state, &addr, "/block/:id", &err.to_string()).await;
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to load block"))
        }
    }
}

async fn handle_get_account(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    AxumPath(address): AxumPath<String>,
) -> Result<Json<AccountResponse>, (StatusCode, &'static str)> {
    if let Err(err) = guard_request(&state, &addr, "/account/:address").await {
        return Err(deny_request(&state, &addr, "/account/:address", err).await);
    }

    let address_bytes = match parse_hex_32(&address) {
        Ok(bytes) => bytes,
        Err(err) => {
            warn!(
                "Invalid account address from {}: {} ({})",
                addr, address, err
            );
            record_security_failure(
                &state,
                &addr,
                "/account/:address",
                "Invalid account address",
            )
            .await;
            return Err((StatusCode::BAD_REQUEST, "Invalid account address"));
        }
    };

    match state.storage.get_account(&address_bytes) {
        Ok(Some(account)) => match state.storage.get_transactions_by_address(&address_bytes) {
            Ok(transactions) => {
                let response = account_to_response(account, transactions);
                record_security_success(&state, &addr, "/account/:address").await;
                Ok(Json(response))
            }
            Err(err) => {
                error!(
                    "Failed fetching transactions for account {} ({}): {}",
                    address, addr, err
                );
                record_security_failure(&state, &addr, "/account/:address", &err.to_string()).await;
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to load account transactions",
                ))
            }
        },
        Ok(None) => {
            record_security_success(&state, &addr, "/account/:address").await;
            Err((StatusCode::NOT_FOUND, "Account not found"))
        }
        Err(err) => {
            error!("Failed fetching account {} for {}: {}", address, addr, err);
            record_security_failure(&state, &addr, "/account/:address", &err.to_string()).await;
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to load account"))
        }
    }
}

async fn handle_get_peers(State(state): State<Arc<AppState>>) -> Json<Vec<String>> {
    if let Some(net) = &state.p2p_network {
        Json(net.get_peers())
    } else {
        Json(vec![])
    }
}

async fn handle_get_p2p_peers(State(state): State<Arc<AppState>>) -> Json<Vec<String>> {
    handle_get_peers(State(state)).await
}

// -----------------------------------------------------------------------------
// P2P Handlers
// -----------------------------------------------------------------------------

async fn handle_p2p_blocks(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(message): Json<NetworkMessage>,
) -> (StatusCode, &'static str) {
    if let Err(err) = guard_request(&state, &addr, "/p2p/blocks").await {
        return deny_request(&state, &addr, "/p2p/blocks", err).await;
    }

    let from = resolve_peer_address(&state, &addr, &message);
    forward_to_network(&state, &from, message.clone()).await;

    match message {
        NetworkMessage::Block(block) => match ingest_block_from_peer(&state, &block) {
            Ok(()) => {
                record_security_success(&state, &addr, "/p2p/blocks").await;
                (StatusCode::OK, "Block accepted")
            }
            Err(err) => {
                error!("Failed to persist block from {}: {}", from, err);
                record_security_failure(&state, &addr, "/p2p/blocks", &err.to_string()).await;
                (StatusCode::INTERNAL_SERVER_ERROR, "Failed to persist block")
            }
        },
        other => {
            warn!(
                "Unexpected payload on /p2p/blocks from {}: {:?}",
                from, other
            );
            let reason = format!("Unexpected payload: {:?}", other);
            record_security_failure(&state, &addr, "/p2p/blocks", &reason).await;
            (StatusCode::BAD_REQUEST, "Expected block message")
        }
    }
}

async fn handle_p2p_block_response(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(message): Json<NetworkMessage>,
) -> (StatusCode, &'static str) {
    if let Err(err) = guard_request(&state, &addr, "/p2p/block-response").await {
        return deny_request(&state, &addr, "/p2p/block-response", err).await;
    }

    let from = resolve_peer_address(&state, &addr, &message);
    forward_to_network(&state, &from, message.clone()).await;

    match message {
        NetworkMessage::BlockResponse(block) => match ingest_block_from_peer(&state, &block) {
            Ok(()) => {
                record_security_success(&state, &addr, "/p2p/block-response").await;
                (StatusCode::OK, "Block response accepted")
            }
            Err(err) => {
                error!("Failed to handle block response from {}: {}", from, err);
                record_security_failure(&state, &addr, "/p2p/block-response", &err.to_string())
                    .await;
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to handle block response",
                )
            }
        },
        other => {
            warn!(
                "Unexpected payload on /p2p/block-response from {}: {:?}",
                from, other
            );
            let reason = format!("Unexpected payload: {:?}", other);
            record_security_failure(&state, &addr, "/p2p/block-response", &reason).await;
            (StatusCode::BAD_REQUEST, "Expected block response message")
        }
    }
}

async fn handle_p2p_transactions(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(message): Json<NetworkMessage>,
) -> (StatusCode, &'static str) {
    if let Err(err) = guard_request(&state, &addr, "/p2p/transactions").await {
        return deny_request(&state, &addr, "/p2p/transactions", err).await;
    }

    let from = resolve_peer_address(&state, &addr, &message);
    forward_to_network(&state, &from, message.clone()).await;

    match message {
        NetworkMessage::Transaction(tx) => match ingest_transaction_from_peer(&state, &tx) {
            Ok(()) => {
                record_security_success(&state, &addr, "/p2p/transactions").await;
                (StatusCode::OK, "Transaction accepted")
            }
            Err(err) => {
                error!("Failed to ingest transaction from {}: {}", from, err);
                record_security_failure(&state, &addr, "/p2p/transactions", &err.to_string()).await;
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to ingest transaction",
                )
            }
        },
        other => {
            warn!(
                "Unexpected payload on /p2p/transactions from {}: {:?}",
                from, other
            );
            let reason = format!("Unexpected payload: {:?}", other);
            record_security_failure(&state, &addr, "/p2p/transactions", &reason).await;
            (StatusCode::BAD_REQUEST, "Expected transaction message")
        }
    }
}

async fn handle_p2p_peer_info(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(message): Json<NetworkMessage>,
) -> (StatusCode, &'static str) {
    if let Err(err) = guard_request(&state, &addr, "/p2p/peer-info").await {
        return deny_request(&state, &addr, "/p2p/peer-info", err).await;
    }

    let from = resolve_peer_address(&state, &addr, &message);
    forward_to_network(&state, &from, message.clone()).await;

    match message {
        NetworkMessage::PeerInfo { .. } => {
            record_security_success(&state, &addr, "/p2p/peer-info").await;
            (StatusCode::OK, "Peer info accepted")
        }
        other => {
            warn!(
                "Unexpected payload on /p2p/peer-info from {}: {:?}",
                from, other
            );
            let reason = format!("Unexpected payload: {:?}", other);
            record_security_failure(&state, &addr, "/p2p/peer-info", &reason).await;
            (StatusCode::BAD_REQUEST, "Expected peer info message")
        }
    }
}

async fn handle_p2p_peer_discovery(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(message): Json<NetworkMessage>,
) -> (StatusCode, &'static str) {
    if let Err(err) = guard_request(&state, &addr, "/p2p/peer-discovery").await {
        return deny_request(&state, &addr, "/p2p/peer-discovery", err).await;
    }

    let from = resolve_peer_address(&state, &addr, &message);
    forward_to_network(&state, &from, message.clone()).await;

    match message {
        NetworkMessage::PeerDiscovery { .. } => {
            record_security_success(&state, &addr, "/p2p/peer-discovery").await;
            (StatusCode::OK, "Peer discovery accepted")
        }
        other => {
            warn!(
                "Unexpected payload on /p2p/peer-discovery from {}: {:?}",
                from, other
            );
            let reason = format!("Unexpected payload: {:?}", other);
            record_security_failure(&state, &addr, "/p2p/peer-discovery", &reason).await;
            (StatusCode::BAD_REQUEST, "Expected peer discovery message")
        }
    }
}

async fn handle_p2p_block_request(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(message): Json<NetworkMessage>,
) -> Result<Json<NetworkMessage>, StatusCode> {
    if let Err(err) = guard_request(&state, &addr, "/p2p/block-request").await {
        let (status, _) = deny_request(&state, &addr, "/p2p/block-request", err).await;
        return Err(status);
    }

    let from = resolve_peer_address(&state, &addr, &message);
    forward_to_network(&state, &from, message.clone()).await;

    match message {
        NetworkMessage::BlockRequest { hash } => match state.storage.get_block(&hash) {
            Ok(Some(block)) => {
                record_security_success(&state, &addr, "/p2p/block-request").await;
                Ok(Json(NetworkMessage::BlockResponse(block)))
            }
            Ok(None) => {
                debug!(
                    "Block request from {} not found: {}",
                    from,
                    hex_encode(hash)
                );
                record_security_success(&state, &addr, "/p2p/block-request").await;
                Err(StatusCode::NOT_FOUND)
            }
            Err(err) => {
                error!("Failed to serve block request from {}: {}", from, err);
                record_security_failure(&state, &addr, "/p2p/block-request", &err.to_string())
                    .await;
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        },
        other => {
            warn!(
                "Unexpected payload on /p2p/block-request from {}: {:?}",
                from, other
            );
            let reason = format!("Unexpected payload: {:?}", other);
            record_security_failure(&state, &addr, "/p2p/block-request", &reason).await;
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

async fn forward_to_network(state: &Arc<AppState>, from: &str, message: NetworkMessage) {
    if let Some(net) = &state.p2p_network {
        if let Err(err) = net.process_incoming_message(from, message).await {
            warn!(
                "Failed to process inbound P2P message from {}: {}",
                from, err
            );
        }
    }
}

fn message_announced_address(message: &NetworkMessage) -> Option<String> {
    match message {
        NetworkMessage::PeerInfo { addresses, .. } => addresses
            .iter()
            .find(|addr| !addr.is_empty() && !addr.contains("0.0.0.0"))
            .cloned()
            .or_else(|| addresses.first().cloned()),
        _ => None,
    }
}

fn resolve_peer_address(
    state: &Arc<AppState>,
    socket: &SocketAddr,
    message: &NetworkMessage,
) -> String {
    if let Some(addr) = message_announced_address(message) {
        return addr;
    }

    if let Some(net) = &state.p2p_network {
        let host = socket.ip().to_string();
        if let Some(info) = net
            .get_peer_metadata()
            .into_iter()
            .find(|info| info.address.contains(&host))
        {
            return info.address;
        }
    }

    format!("http://{}:{}", socket.ip(), socket.port())
}

fn ingest_block_from_peer(state: &Arc<AppState>, block: &Block) -> Result<()> {
    state.storage.store_block(block.clone())?;

    for tx in &block.transactions {
        let hash_hex = hex_encode(tx.hash());
        if let Err(err) = state.mempool.remove_transaction(&hash_hex) {
            debug!(
                "Failed to prune transaction {} from mempool after block import: {}",
                hash_hex, err
            );
        }
    }

    Ok(())
}

fn ingest_transaction_from_peer(state: &Arc<AppState>, tx: &Transaction) -> Result<()> {
    state.storage.store_transaction(tx.clone())?;

    match state.mempool.add_transaction(tx.clone()) {
        Ok(true) => {}
        Ok(false) => debug!(
            "Duplicate transaction from peer ignored: {}",
            hex_encode(tx.hash())
        ),
        Err(err) => return Err(err),
    }

    if let Some(consensus) = &state.consensus {
        if let Err(err) = consensus.submit_transaction(tx.clone()) {
            warn!(
                "Consensus rejected transaction {} from peer: {}",
                hex_encode(tx.hash()),
                err
            );
        }
    }

    Ok(())
}

// -----------------------------------------------------------------------------
// L2 Endpoints
// -----------------------------------------------------------------------------

async fn handle_get_l2_config(State(state): State<Arc<AppState>>) -> Json<L2Config> {
    Json(state.l2_config.clone())
}

async fn handle_list_l2_networks(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<L2Network>>, (StatusCode, &'static str)> {
    match state.storage.list_l2_networks() {
        Ok(networks) => Ok(Json(networks)),
        Err(err) => {
            error!("Failed to list L2 networks: {}", err);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to list L2 networks",
            ))
        }
    }
}

async fn handle_list_l2_commits(
    State(state): State<Arc<AppState>>,
    Query(filter): Query<L2Filter>,
) -> Result<Json<Vec<L2Commit>>, (StatusCode, &'static str)> {
    match state.storage.list_l2_commits(filter.l2_id.as_deref()) {
        Ok(commits) => Ok(Json(commits)),
        Err(err) => {
            error!("Failed to list L2 commits: {}", err);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to list L2 commits",
            ))
        }
    }
}

async fn handle_list_l2_exits(
    State(state): State<Arc<AppState>>,
    Query(filter): Query<L2Filter>,
) -> Result<Json<Vec<L2ExitRecord>>, (StatusCode, &'static str)> {
    match state.storage.list_l2_exits(filter.l2_id.as_deref()) {
        Ok(exits) => Ok(Json(exits)),
        Err(err) => {
            error!("Failed to list L2 exits: {}", err);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to list L2 exits"))
        }
    }
}

fn parse_hex_32(input: &str) -> std::result::Result<[u8; 32], hex::FromHexError> {
    let trimmed = input.trim();
    let normalized = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);
    let mut bytes = [0u8; 32];
    hex::decode_to_slice(normalized, &mut bytes)?;
    Ok(bytes)
}

enum BlockIdentifier {
    Hash([u8; 32]),
    Height(u64),
}

fn parse_block_identifier(input: &str) -> Option<BlockIdentifier> {
    let trimmed = input.trim();
    if trimmed.len() <= 20 && trimmed.chars().all(|c| c.is_ascii_digit()) {
        if let Ok(height) = trimmed.parse::<u64>() {
            return Some(BlockIdentifier::Height(height));
        }
    }

    parse_hex_32(trimmed).ok().map(BlockIdentifier::Hash)
}

fn account_to_response(account: Account, transactions: Vec<Transaction>) -> AccountResponse {
    let transactions = transactions
        .into_iter()
        .map(|tx| TransactionEnvelope {
            hash: hex_encode(tx.hash()),
            transaction: tx,
        })
        .collect();

    AccountResponse {
        address: hex_encode(account.address),
        balance: account.balance,
        nonce: account.nonce,
        transactions,
    }
}

#[derive(Clone)]
struct RateLimiterLayer {
    state: Arc<RateLimiterState>,
    max_requests: u64,
    window: Duration,
}

impl RateLimiterLayer {
    fn new(max_requests: u64, window: Duration) -> Self {
        Self {
            state: Arc::new(RateLimiterState::new()),
            max_requests,
            window,
        }
    }
}

impl<S> Layer<S> for RateLimiterLayer {
    type Service = RateLimiter<S>;

    fn layer(&self, service: S) -> Self::Service {
        RateLimiter {
            inner: service,
            state: Arc::clone(&self.state),
            max_requests: self.max_requests,
            window: self.window,
        }
    }
}

#[derive(Clone)]
struct RateLimiter<S> {
    inner: S,
    state: Arc<RateLimiterState>,
    max_requests: u64,
    window: Duration,
}

impl<S, Request> Service<Request> for RateLimiter<S>
where
    S: Service<Request, Response = Response, Error = Infallible> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut TaskContext<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        if !self.state.allow(self.max_requests, self.window) {
            let response = Response::builder()
                .status(StatusCode::TOO_MANY_REQUESTS)
                .body(Body::from("Rate limit exceeded"))
                .expect("failed to build rate limit response");
            return Box::pin(async move { Ok(response) });
        }

        let future = self.inner.call(request);
        Box::pin(async move { future.await })
    }
}

struct RateLimiterState {
    window: std::sync::Mutex<RateWindow>,
}

impl RateLimiterState {
    fn new() -> Self {
        Self {
            window: std::sync::Mutex::new(RateWindow {
                start: Instant::now(),
                count: 0,
            }),
        }
    }

    fn allow(&self, max_requests: u64, window: Duration) -> bool {
        let now = Instant::now();
        let mut guard = self
            .window
            .lock()
            .expect("rate limiter mutex poisoned");

        if now.duration_since(guard.start) >= window {
            guard.start = now;
            guard.count = 0;
        }

        if guard.count < max_requests {
            guard.count += 1;
            true
        } else {
            false
        }
    }
}

struct RateWindow {
    start: Instant,
    count: u64,
}

#[derive(Clone)]
struct CircuitBreakerLayer {
    state: Arc<CircuitBreakerState>,
    failure_threshold: usize,
    open_duration: Duration,
}

impl CircuitBreakerLayer {
    fn new(failure_threshold: usize, open_duration: Duration) -> Self {
        Self {
            state: Arc::new(CircuitBreakerState::new()),
            failure_threshold,
            open_duration,
        }
    }
}

impl<S> Layer<S> for CircuitBreakerLayer {
    type Service = CircuitBreaker<S>;

    fn layer(&self, service: S) -> Self::Service {
        CircuitBreaker {
            inner: service,
            state: Arc::clone(&self.state),
            failure_threshold: self.failure_threshold,
            open_duration: self.open_duration,
        }
    }
}

struct CircuitBreaker<S> {
    inner: S,
    state: Arc<CircuitBreakerState>,
    failure_threshold: usize,
    open_duration: Duration,
}

impl<S> Clone for CircuitBreaker<S>
where
    S: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            state: Arc::clone(&self.state),
            failure_threshold: self.failure_threshold,
            open_duration: self.open_duration,
        }
    }
}

impl<S, Request> Service<Request> for CircuitBreaker<S>
where
    S: Service<Request, Response = Response, Error = Infallible> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut TaskContext<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        if self.state.is_open() {
            let response = Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .body(Body::from("Circuit breaker open"))
                .expect("failed to build circuit breaker response");
            return Box::pin(async move { Ok(response) });
        }

        let state = Arc::clone(&self.state);
        let failure_threshold = self.failure_threshold;
        let open_duration = self.open_duration;

        let future = self.inner.call(request);
        Box::pin(async move {
            let response = future.await?;
            if response.status().is_server_error() {
                if state.record_failure(failure_threshold, open_duration) {
                    warn!(
                        "Circuit breaker opened after {} consecutive failures; blocking traffic for {:?}",
                        failure_threshold, open_duration
                    );
                }
            } else {
                state.record_success();
            }
            Ok(response)
        })
    }
}

struct CircuitBreakerState {
    failures: AtomicUsize,
    open_until: std::sync::Mutex<Option<Instant>>,
}

impl CircuitBreakerState {
    fn new() -> Self {
        Self {
            failures: AtomicUsize::new(0),
            open_until: std::sync::Mutex::new(None),
        }
    }

    fn is_open(&self) -> bool {
        let mut guard = self
            .open_until
            .lock()
            .expect("circuit breaker mutex poisoned");
        if let Some(until) = *guard {
            if Instant::now() < until {
                return true;
            }
            *guard = None;
        }
        false
    }

    fn record_failure(&self, threshold: usize, open_duration: Duration) -> bool {
        let current = self.failures.fetch_add(1, Ordering::Relaxed) + 1;
        if current >= threshold {
            let mut guard = self
                .open_until
                .lock()
                .expect("circuit breaker mutex poisoned");
            *guard = Some(Instant::now() + open_duration);
            self.failures.store(0, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    fn record_success(&self) {
        self.failures.store(0, Ordering::Relaxed);
        let mut guard = self
            .open_until
            .lock()
            .expect("circuit breaker mutex poisoned");
        *guard = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::P2PConfig;
    use anyhow::anyhow;
    use axum::extract::{ConnectInfo, Path as AxumPath, Query};
    use axum::Json;
    use ed25519_dalek::SigningKey;
    use ippan_consensus::{PoAConfig, Validator};
    use ippan_p2p::NetworkEvent;
    use ippan_security::{SecurityConfig, SecurityManager};
    use ippan_storage::{ChainState, MemoryStorage, ValidatorTelemetry};
    use ippan_types::{
        Amount, L2ExitStatus, L2NetworkStatus, RoundCertificate, RoundFinalizationRecord, RoundId,
    };
    use std::collections::{HashMap, HashSet};
    use std::fs;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::path::PathBuf;
    use std::sync::atomic::AtomicUsize;
    use std::sync::Arc;
    use std::time::Instant;
    use tempfile::tempdir;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_health_endpoint() {
        let app_state = Arc::new(AppState {
            storage: Arc::new(MemoryStorage::default()),
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
            security: None,
            metrics: None,
        });

        let response = handle_health(State(app_state)).await;
        let json = response.0;
        assert_eq!(json.get("status").unwrap(), "healthy");
    }

    fn build_app_state(
        security: Option<Arc<SecurityManager>>,
        unified_ui_dist: Option<PathBuf>,
    ) -> Arc<AppState> {
        let storage: Arc<dyn Storage + Send + Sync> = Arc::new(MemoryStorage::default());
        Arc::new(AppState {
            storage,
            start_time: Instant::now(),
            peer_count: Arc::new(AtomicUsize::new(0)),
            p2p_network: None,
            tx_sender: None,
            node_id: "test-node".into(),
            consensus: None,
            l2_config: L2Config {
                max_commit_size: 512,
                min_epoch_gap_ms: 1_000,
                challenge_window_ms: 2_000,
                da_mode: "test".into(),
                max_l2_count: 1,
            },
            mempool: Arc::new(Mempool::new(1_000)),
            unified_ui_dist,
            req_count: Arc::new(AtomicUsize::new(0)),
            security,
            metrics: None,
        })
    }

    fn make_app_state() -> Arc<AppState> {
        build_app_state(None, None)
    }

    fn sample_private_key(seed: [u8; 32]) -> SigningKey {
        SigningKey::from_bytes(&seed)
    }

    fn sample_public_key(seed: [u8; 32]) -> [u8; 32] {
        sample_private_key(seed).verifying_key().to_bytes()
    }

    fn sample_transaction(from_seed: [u8; 32], to_address: [u8; 32], nonce: u64) -> Transaction {
        let signing_key = sample_private_key(from_seed);
        let from_public = signing_key.verifying_key().to_bytes();
        let mut tx = Transaction::new(
            from_public,
            to_address,
            Amount::from_micro_ipn(10 + nonce),
            nonce,
        );
        let private_bytes = signing_key.to_bytes();
        tx.sign(&private_bytes).expect("sign sample transaction");
        tx
    }

    struct FailingStorage {
        inner: MemoryStorage,
        failures: HashSet<String>,
    }

    impl FailingStorage {
        fn new(failures: &[&str]) -> Self {
            Self {
                inner: MemoryStorage::default(),
                failures: failures.iter().map(|s| s.to_string()).collect(),
            }
        }

        fn should_fail(&self, op: &str) -> bool {
            self.failures.contains(op)
        }

        fn inner(&self) -> &MemoryStorage {
            &self.inner
        }
    }

    impl Storage for FailingStorage {
        fn store_block(&self, block: Block) -> Result<()> {
            if self.should_fail("store_block") {
                Err(anyhow!("forced failure: store_block"))
            } else {
                self.inner.store_block(block)
            }
        }

        fn get_block(&self, hash: &[u8; 32]) -> Result<Option<Block>> {
            if self.should_fail("get_block") {
                Err(anyhow!("forced failure: get_block"))
            } else {
                self.inner.get_block(hash)
            }
        }

        fn get_block_by_height(&self, height: u64) -> Result<Option<Block>> {
            if self.should_fail("get_block_by_height") {
                Err(anyhow!("forced failure: get_block_by_height"))
            } else {
                self.inner.get_block_by_height(height)
            }
        }

        fn store_transaction(&self, tx: Transaction) -> Result<()> {
            if self.should_fail("store_transaction") {
                Err(anyhow!("forced failure: store_transaction"))
            } else {
                self.inner.store_transaction(tx)
            }
        }

        fn get_transaction(&self, hash: &[u8; 32]) -> Result<Option<Transaction>> {
            if self.should_fail("get_transaction") {
                Err(anyhow!("forced failure: get_transaction"))
            } else {
                self.inner.get_transaction(hash)
            }
        }

        fn get_latest_height(&self) -> Result<u64> {
            if self.should_fail("get_latest_height") {
                Err(anyhow!("forced failure: get_latest_height"))
            } else {
                self.inner.get_latest_height()
            }
        }

        fn get_account(&self, address: &[u8; 32]) -> Result<Option<Account>> {
            if self.should_fail("get_account") {
                Err(anyhow!("forced failure: get_account"))
            } else {
                self.inner.get_account(address)
            }
        }

        fn update_account(&self, account: Account) -> Result<()> {
            if self.should_fail("update_account") {
                Err(anyhow!("forced failure: update_account"))
            } else {
                self.inner.update_account(account)
            }
        }

        fn get_all_accounts(&self) -> Result<Vec<Account>> {
            if self.should_fail("get_all_accounts") {
                Err(anyhow!("forced failure: get_all_accounts"))
            } else {
                self.inner.get_all_accounts()
            }
        }

        fn get_transactions_by_address(&self, address: &[u8; 32]) -> Result<Vec<Transaction>> {
            if self.should_fail("get_transactions_by_address") {
                Err(anyhow!("forced failure: get_transactions_by_address"))
            } else {
                self.inner.get_transactions_by_address(address)
            }
        }

        fn get_transaction_count(&self) -> Result<u64> {
            if self.should_fail("get_transaction_count") {
                Err(anyhow!("forced failure: get_transaction_count"))
            } else {
                self.inner.get_transaction_count()
            }
        }

        fn put_l2_network(&self, network: L2Network) -> Result<()> {
            if self.should_fail("put_l2_network") {
                Err(anyhow!("forced failure: put_l2_network"))
            } else {
                self.inner.put_l2_network(network)
            }
        }

        fn get_l2_network(&self, id: &str) -> Result<Option<L2Network>> {
            if self.should_fail("get_l2_network") {
                Err(anyhow!("forced failure: get_l2_network"))
            } else {
                self.inner.get_l2_network(id)
            }
        }

        fn list_l2_networks(&self) -> Result<Vec<L2Network>> {
            if self.should_fail("list_l2_networks") {
                Err(anyhow!("forced failure: list_l2_networks"))
            } else {
                self.inner.list_l2_networks()
            }
        }

        fn store_l2_commit(&self, commit: L2Commit) -> Result<()> {
            if self.should_fail("store_l2_commit") {
                Err(anyhow!("forced failure: store_l2_commit"))
            } else {
                self.inner.store_l2_commit(commit)
            }
        }

        fn list_l2_commits(&self, l2_id: Option<&str>) -> Result<Vec<L2Commit>> {
            if self.should_fail("list_l2_commits") {
                Err(anyhow!("forced failure: list_l2_commits"))
            } else {
                self.inner.list_l2_commits(l2_id)
            }
        }

        fn store_l2_exit(&self, exit: L2ExitRecord) -> Result<()> {
            if self.should_fail("store_l2_exit") {
                Err(anyhow!("forced failure: store_l2_exit"))
            } else {
                self.inner.store_l2_exit(exit)
            }
        }

        fn list_l2_exits(&self, l2_id: Option<&str>) -> Result<Vec<L2ExitRecord>> {
            if self.should_fail("list_l2_exits") {
                Err(anyhow!("forced failure: list_l2_exits"))
            } else {
                self.inner.list_l2_exits(l2_id)
            }
        }

        fn store_round_certificate(&self, certificate: RoundCertificate) -> Result<()> {
            if self.should_fail("store_round_certificate") {
                Err(anyhow!("forced failure: store_round_certificate"))
            } else {
                self.inner.store_round_certificate(certificate)
            }
        }

        fn get_round_certificate(&self, round: RoundId) -> Result<Option<RoundCertificate>> {
            if self.should_fail("get_round_certificate") {
                Err(anyhow!("forced failure: get_round_certificate"))
            } else {
                self.inner.get_round_certificate(round)
            }
        }

        fn store_round_finalization(&self, record: RoundFinalizationRecord) -> Result<()> {
            if self.should_fail("store_round_finalization") {
                Err(anyhow!("forced failure: store_round_finalization"))
            } else {
                self.inner.store_round_finalization(record)
            }
        }

        fn get_round_finalization(
            &self,
            round: RoundId,
        ) -> Result<Option<RoundFinalizationRecord>> {
            if self.should_fail("get_round_finalization") {
                Err(anyhow!("forced failure: get_round_finalization"))
            } else {
                self.inner.get_round_finalization(round)
            }
        }

        fn get_latest_round_finalization(&self) -> Result<Option<RoundFinalizationRecord>> {
            if self.should_fail("get_latest_round_finalization") {
                Err(anyhow!("forced failure: get_latest_round_finalization"))
            } else {
                self.inner.get_latest_round_finalization()
            }
        }

        fn get_chain_state(&self) -> Result<ChainState> {
            if self.should_fail("get_chain_state") {
                Err(anyhow!("forced failure: get_chain_state"))
            } else {
                self.inner.get_chain_state()
            }
        }

        fn update_chain_state(&self, state: &ChainState) -> Result<()> {
            if self.should_fail("update_chain_state") {
                Err(anyhow!("forced failure: update_chain_state"))
            } else {
                self.inner.update_chain_state(state)
            }
        }

        fn store_validator_telemetry(
            &self,
            validator_id: &[u8; 32],
            telemetry: &ValidatorTelemetry,
        ) -> Result<()> {
            if self.should_fail("store_validator_telemetry") {
                Err(anyhow!("forced failure: store_validator_telemetry"))
            } else {
                self.inner
                    .store_validator_telemetry(validator_id, telemetry)
            }
        }

        fn get_validator_telemetry(
            &self,
            validator_id: &[u8; 32],
        ) -> Result<Option<ValidatorTelemetry>> {
            if self.should_fail("get_validator_telemetry") {
                Err(anyhow!("forced failure: get_validator_telemetry"))
            } else {
                self.inner.get_validator_telemetry(validator_id)
            }
        }

        fn get_all_validator_telemetry(&self) -> Result<HashMap<[u8; 32], ValidatorTelemetry>> {
            if self.should_fail("get_all_validator_telemetry") {
                Err(anyhow!("forced failure: get_all_validator_telemetry"))
            } else {
                self.inner.get_all_validator_telemetry()
            }
        }
    }

    #[test]
    fn test_parse_hex_32_success_and_failure() {
        let bytes = parse_hex_32(&"AA".repeat(32)).expect("parse 32 bytes");
        assert_eq!(bytes[0], 0xAA);
        assert!(parse_hex_32("short").is_err());
        assert!(parse_hex_32(&"0G".repeat(32)).is_err());
    }

    #[test]
    fn test_parse_block_identifier_variants() {
        if let Some(BlockIdentifier::Height(h)) = parse_block_identifier("42") {
            assert_eq!(h, 42);
        } else {
            panic!("unexpected identifier variant");
        }

        let hash_input = "ab".repeat(32);
        if let Some(BlockIdentifier::Hash(bytes)) = parse_block_identifier(&hash_input) {
            assert_eq!(bytes.len(), 32);
            assert_eq!(bytes[0], 0xAB);
        } else {
            panic!("expected hash identifier");
        }

        assert!(parse_block_identifier("invalid-hash").is_none());
    }

    #[test]
    fn test_account_to_response_serializes() {
        let account = Account {
            address: sample_public_key([1u8; 32]),
            balance: 1_000,
            nonce: 2,
        };
        let tx = sample_transaction([1u8; 32], sample_public_key([2u8; 32]), 3);
        let response = account_to_response(account, vec![tx.clone()]);
        assert_eq!(response.address, hex::encode(sample_public_key([1u8; 32])));
        assert_eq!(response.transactions.len(), 1);
        assert_eq!(response.transactions[0].hash.len(), 64);
        assert_eq!(response.transactions[0].transaction.hash(), tx.hash());
    }

    #[test]
    fn test_map_security_error_variants() {
        let (status, msg) = map_security_error(&SecurityError::IpBlocked);
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert!(msg.contains("blocked"));

        let (status, msg) = map_security_error(&SecurityError::RateLimitExceeded);
        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
        assert!(msg.contains("Rate limit"));

        let validation_error = SecurityError::ValidationFailed(
            ippan_security::ValidationError::MissingField("field".into()),
        );
        let (status, msg) = map_security_error(&validation_error);
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(msg.contains("Invalid"));

        let audit_error = SecurityError::AuditFailed(anyhow::anyhow!("boom"));
        let (status, _) = map_security_error(&audit_error);
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_guard_request_without_security() {
        let state = make_app_state();
        let addr: SocketAddr = "127.0.0.1:9000".parse().unwrap();
        guard_request(&state, &addr, "/health")
            .await
            .expect("request allowed");
    }

    #[test]
    fn test_message_announced_address_prefers_routable() {
        let message = NetworkMessage::PeerInfo {
            peer_id: "peer".into(),
            addresses: vec![
                "".into(),
                "http://0.0.0.0:9000".into(),
                "http://192.168.1.5:9000".into(),
            ],
            time_us: None,
        };
        let addr = message_announced_address(&message).expect("address");
        assert_eq!(addr, "http://192.168.1.5:9000");
    }

    #[test]
    fn test_resolve_peer_address_fallback() {
        let state = make_app_state();
        let socket: SocketAddr = "10.0.0.5:7000".parse().unwrap();
        let message = NetworkMessage::PeerDiscovery { peers: vec![] };
        let resolved = resolve_peer_address(&state, &socket, &message);
        assert_eq!(resolved, "http://10.0.0.5:7000");
    }

    #[tokio::test]
    async fn test_ingest_block_from_peer_updates_state() {
        let state = make_app_state();
        let tx = sample_transaction([1u8; 32], sample_public_key([2u8; 32]), 1);
        let tx_hash_hex = hex::encode(tx.hash());
        state
            .mempool
            .add_transaction(tx.clone())
            .expect("add tx to mempool");
        let block = Block::new(vec![], vec![tx.clone()], 1, [9u8; 32]);
        let block_hash = block.hash();

        ingest_block_from_peer(&state, &block).expect("ingest block");

        let stored = state
            .storage
            .get_block(&block_hash)
            .expect("query block")
            .expect("block stored");
        assert_eq!(stored.header.round, 1);
        assert!(state.mempool.get_transaction(&tx_hash_hex).is_none());
    }

    #[tokio::test]
    async fn test_ingest_transaction_from_peer_persists() {
        let state = make_app_state();
        let tx = sample_transaction([5u8; 32], sample_public_key([6u8; 32]), 2);
        let tx_hash = tx.hash();

        ingest_transaction_from_peer(&state, &tx).expect("ingest tx");

        let stored = state
            .storage
            .get_transaction(&tx_hash)
            .expect("query tx")
            .expect("tx stored");
        assert_eq!(stored.hash(), tx_hash);
        assert!(state
            .mempool
            .get_transaction(&hex::encode(tx_hash))
            .is_some());
    }

    #[tokio::test]
    async fn test_handle_get_transaction_paths() {
        let state = make_app_state();
        let addr: SocketAddr = "127.0.0.1:9000".parse().unwrap();
        let tx = sample_transaction([8u8; 32], sample_public_key([9u8; 32]), 4);
        let tx_hash = tx.hash();
        state
            .storage
            .store_transaction(tx.clone())
            .expect("store tx");

        let ok = handle_get_transaction(
            State(state.clone()),
            ConnectInfo(addr),
            AxumPath(hex::encode(tx_hash)),
        )
        .await
        .expect("success");
        assert_eq!(ok.0.hash, hex::encode(tx.hash()));

        let missing = handle_get_transaction(
            State(state.clone()),
            ConnectInfo(addr),
            AxumPath(hex::encode(sample_public_key([7u8; 32]))),
        )
        .await
        .expect_err("not found");
        assert_eq!(missing.0, StatusCode::NOT_FOUND);

        let bad = handle_get_transaction(
            State(state.clone()),
            ConnectInfo(addr),
            AxumPath("xyz".to_string()),
        )
        .await
        .expect_err("bad request");
        assert_eq!(bad.0, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_handle_get_transaction_with_security() {
        let dir = tempdir().expect("tempdir");
        let config = SecurityConfig {
            audit_log_path: dir.path().join("audit.log").to_string_lossy().to_string(),
            ..SecurityConfig::default()
        };
        let manager = SecurityManager::new(config).expect("manager");
        let state = build_app_state(Some(Arc::new(manager)), None);
        let addr: SocketAddr = "10.0.0.10:8080".parse().unwrap();
        let tx = sample_transaction([11u8; 32], sample_public_key([12u8; 32]), 5);
        let tx_hash = tx.hash();
        state.storage.store_transaction(tx).expect("store tx");

        let _ = handle_get_transaction(
            State(state.clone()),
            ConnectInfo(addr),
            AxumPath(hex::encode(tx_hash)),
        )
        .await
        .expect("security success");

        let deny_config = SecurityConfig {
            enable_ip_whitelist: true,
            whitelisted_ips: vec![],
            audit_log_path: dir.path().join("blocked.log").to_string_lossy().to_string(),
            ..SecurityConfig::default()
        };
        let deny_state = build_app_state(
            Some(Arc::new(SecurityManager::new(deny_config).unwrap())),
            None,
        );
        let err = handle_get_transaction(
            State(deny_state.clone()),
            ConnectInfo(addr),
            AxumPath(hex::encode(tx_hash)),
        )
        .await
        .expect_err("denied");
        assert_eq!(err.0, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_handle_get_block_variants() {
        let state = make_app_state();
        let addr: SocketAddr = "127.0.0.1:9000".parse().unwrap();
        let block = Block::new(vec![], vec![], 5, [3u8; 32]);
        let block_hash = block.hash();
        state
            .storage
            .store_block(block.clone())
            .expect("store block");

        let by_hash = handle_get_block(
            State(state.clone()),
            ConnectInfo(addr),
            AxumPath(hex::encode(block_hash)),
        )
        .await
        .expect("block by hash");
        assert_eq!(by_hash.0.header.round, 5);

        let by_height = handle_get_block(
            State(state.clone()),
            ConnectInfo(addr),
            AxumPath("5".to_string()),
        )
        .await
        .expect("block by height");
        assert_eq!(by_height.0.header.round, 5);

        let bad = handle_get_block(
            State(state.clone()),
            ConnectInfo(addr),
            AxumPath("not-a-block".to_string()),
        )
        .await
        .expect_err("bad identifier");
        assert_eq!(bad.0, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_handle_get_account_branches() {
        let state = make_app_state();
        let addr: SocketAddr = "127.0.0.1:9000".parse().unwrap();
        let account_address = sample_public_key([4u8; 32]);
        let tx = sample_transaction([4u8; 32], sample_public_key([5u8; 32]), 1);
        let account = Account {
            address: account_address,
            balance: 500,
            nonce: 7,
        };
        state
            .storage
            .update_account(account.clone())
            .expect("account");
        state.storage.store_transaction(tx.clone()).expect("tx1");
        let tx2 = sample_transaction([6u8; 32], account.address, 2);
        state.storage.store_transaction(tx2.clone()).expect("tx2");

        let ok = handle_get_account(
            State(state.clone()),
            ConnectInfo(addr),
            AxumPath(hex::encode(account.address)),
        )
        .await
        .expect("account ok");
        assert_eq!(ok.0.balance, 500);
        assert_eq!(ok.0.transactions.len(), 2);

        let missing = handle_get_account(
            State(state.clone()),
            ConnectInfo(addr),
            AxumPath(hex::encode(sample_public_key([9u8; 32]))),
        )
        .await
        .expect_err("missing");
        assert_eq!(missing.0, StatusCode::NOT_FOUND);

        let bad = handle_get_account(
            State(state.clone()),
            ConnectInfo(addr),
            AxumPath("badhex".to_string()),
        )
        .await
        .expect_err("bad request");
        assert_eq!(bad.0, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_handle_list_l2_endpoints() {
        let state = make_app_state();
        let network = L2Network {
            id: "demo-l2".to_string(),
            proof_type: "zk".to_string(),
            da_mode: "inline".to_string(),
            status: L2NetworkStatus::Active,
            last_epoch: 1,
            total_commits: 1,
            total_exits: 0,
            last_commit_time: Some(10),
            registered_at: 1,
            challenge_window_ms: Some(60_000),
        };
        state.storage.put_l2_network(network).expect("network");
        state
            .storage
            .store_l2_commit(L2Commit {
                id: "commit-demo".into(),
                l2_id: "demo-l2".into(),
                epoch: 1,
                state_root: "root".into(),
                da_hash: "hash".into(),
                proof_type: "zk".into(),
                proof: None,
                inline_data: None,
                submitted_at: 2,
                hashtimer: "ht".into(),
            })
            .expect("commit");
        state
            .storage
            .store_l2_exit(L2ExitRecord {
                id: "exit-demo".into(),
                l2_id: "demo-l2".into(),
                epoch: 1,
                account: "acct".into(),
                amount: 1.0,
                nonce: Some(1),
                proof_of_inclusion: "proof".into(),
                status: L2ExitStatus::Pending,
                submitted_at: 3,
                finalized_at: None,
                rejection_reason: None,
                challenge_window_ends_at: None,
            })
            .expect("exit");

        let networks = handle_list_l2_networks(State(state.clone()))
            .await
            .expect("networks");
        assert_eq!(networks.0.len(), 1);

        let commits = handle_list_l2_commits(
            State(state.clone()),
            Query(L2Filter {
                l2_id: Some("demo-l2".into()),
            }),
        )
        .await
        .expect("commits");
        assert_eq!(commits.0.len(), 1);

        let exits = handle_list_l2_exits(
            State(state.clone()),
            Query(L2Filter {
                l2_id: Some("demo-l2".into()),
            }),
        )
        .await
        .expect("exits");
        assert_eq!(exits.0.len(), 1);
    }

    #[tokio::test]
    async fn test_handle_get_l2_config_and_submit_tx_failure() {
        let state = make_app_state();
        let config = handle_get_l2_config(State(state.clone())).await;
        assert_eq!(config.0.max_l2_count, 1);

        let addr: SocketAddr = "127.0.0.1:9000".parse().unwrap();
        let tx = sample_transaction([2u8; 32], sample_public_key([3u8; 32]), 9);
        let response = handle_submit_tx(State(state.clone()), ConnectInfo(addr), Json(tx)).await;
        assert_eq!(response.0, StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_handle_submit_tx_with_consensus_success_and_failure() {
        let storage: Arc<dyn Storage + Send + Sync> = Arc::new(MemoryStorage::default());
        let mut config = PoAConfig::default();
        config.validators.push(Validator {
            id: sample_public_key([3u8; 32]),
            address: sample_public_key([4u8; 32]),
            stake: 1_000,
            is_active: true,
        });

        let poa = PoAConsensus::new(config, storage.clone(), sample_public_key([9u8; 32]));
        let mempool = poa.mempool();
        let consensus = Arc::new(Mutex::new(poa));

        let (tx_sender_ok, mut rx_ok) = mpsc::unbounded_channel();
        let handle_ok =
            ConsensusHandle::new(consensus.clone(), tx_sender_ok.clone(), mempool.clone());

        let mut ok_state = (*build_app_state(None, None)).clone();
        ok_state.storage = storage.clone();
        ok_state.consensus = Some(handle_ok.clone());
        ok_state.tx_sender = Some(tx_sender_ok);
        ok_state.mempool = mempool.clone();
        let ok_state = Arc::new(ok_state);

        let addr: SocketAddr = "127.0.0.1:9101".parse().unwrap();
        let tx = sample_transaction([5u8; 32], sample_public_key([6u8; 32]), 11);
        let accepted =
            handle_submit_tx(State(ok_state.clone()), ConnectInfo(addr), Json(tx.clone())).await;
        assert_eq!(accepted.0, StatusCode::OK);
        let received = rx_ok.recv().await.expect("consensus dispatch");
        assert_eq!(received.hash(), tx.hash());

        let (tx_sender_fail, rx_fail) = mpsc::unbounded_channel::<Transaction>();
        drop(rx_fail);
        let handle_fail =
            ConsensusHandle::new(consensus.clone(), tx_sender_fail.clone(), mempool.clone());

        let mut fail_state = (*build_app_state(None, None)).clone();
        fail_state.storage = storage.clone();
        fail_state.consensus = Some(handle_fail);
        fail_state.tx_sender = Some(tx_sender_fail);
        fail_state.mempool = mempool.clone();
        let fail_state = Arc::new(fail_state);

        let rejected = handle_submit_tx(
            State(fail_state),
            ConnectInfo(addr),
            Json(sample_transaction(
                [7u8; 32],
                sample_public_key([8u8; 32]),
                12,
            )),
        )
        .await;
        assert_eq!(rejected.0, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_handle_p2p_blocks_and_transactions() {
        let state = make_app_state();
        let addr: SocketAddr = "127.0.0.1:9100".parse().unwrap();
        let tx = sample_transaction([1u8; 32], sample_public_key([2u8; 32]), 3);
        let block = Block::new(vec![], vec![tx.clone()], 2, [7u8; 32]);
        let block_message = NetworkMessage::Block(block.clone());

        let block_result =
            handle_p2p_blocks(State(state.clone()), ConnectInfo(addr), Json(block_message)).await;
        assert_eq!(block_result.0, StatusCode::OK);

        let tx_message = NetworkMessage::Transaction(tx.clone());
        let tx_result =
            handle_p2p_transactions(State(state.clone()), ConnectInfo(addr), Json(tx_message))
                .await;
        assert_eq!(tx_result.0, StatusCode::OK);

        let unexpected = handle_p2p_blocks(
            State(state.clone()),
            ConnectInfo(addr),
            Json(NetworkMessage::PeerInfo {
                peer_id: "peer".into(),
                addresses: vec![],
                time_us: None,
            }),
        )
        .await;
        assert_eq!(unexpected.0, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_handle_p2p_peer_info_and_discovery() {
        let state = make_app_state();
        let addr: SocketAddr = "127.0.0.1:7000".parse().unwrap();

        let info = handle_p2p_peer_info(
            State(state.clone()),
            ConnectInfo(addr),
            Json(NetworkMessage::PeerInfo {
                peer_id: "peer-1".into(),
                addresses: vec!["http://example.com".into()],
                time_us: Some(1),
            }),
        )
        .await;
        assert_eq!(info.0, StatusCode::OK);

        let discovery = handle_p2p_peer_discovery(
            State(state.clone()),
            ConnectInfo(addr),
            Json(NetworkMessage::PeerDiscovery {
                peers: vec!["http://peer".into()],
            }),
        )
        .await;
        assert_eq!(discovery.0, StatusCode::OK);

        let unexpected = handle_p2p_peer_info(
            State(state.clone()),
            ConnectInfo(addr),
            Json(NetworkMessage::Transaction(sample_transaction(
                [0u8; 32], [1u8; 32], 1,
            ))),
        )
        .await;
        assert_eq!(unexpected.0, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_forward_to_network_delivers_message() {
        let config = P2PConfig {
            message_timeout: Duration::from_millis(5),
            ..P2PConfig::default()
        };
        let raw_network =
            HttpP2PNetwork::new(config, "http://127.0.0.1:9700".into()).expect("network");
        let mut events = raw_network.take_incoming_events().expect("event receiver");
        let network = Arc::new(raw_network);

        let mut state = (*build_app_state(None, None)).clone();
        state.p2p_network = Some(Arc::clone(&network));
        let state = Arc::new(state);

        let peers = vec!["http://198.51.100.1:9000".into()];
        forward_to_network(
            &state,
            "http://198.51.100.2:9001",
            NetworkMessage::PeerDiscovery {
                peers: peers.clone(),
            },
        )
        .await;

        match events.recv().await.expect("network event") {
            NetworkEvent::PeerDiscovery {
                peers: observed, ..
            } => assert_eq!(observed, peers),
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_handle_p2p_block_request() {
        let state = make_app_state();
        let addr: SocketAddr = "127.0.0.1:7050".parse().unwrap();
        let block = Block::new(vec![], vec![], 9, [4u8; 32]);
        let block_hash = block.hash();
        state
            .storage
            .store_block(block.clone())
            .expect("store block");

        let ok = handle_p2p_block_request(
            State(state.clone()),
            ConnectInfo(addr),
            Json(NetworkMessage::BlockRequest { hash: block_hash }),
        )
        .await
        .expect("block response");
        assert!(matches!(ok.0, NetworkMessage::BlockResponse(_)));

        let missing = handle_p2p_block_request(
            State(state.clone()),
            ConnectInfo(addr),
            Json(NetworkMessage::BlockRequest { hash: [1u8; 32] }),
        )
        .await
        .expect_err("missing");
        assert_eq!(missing, StatusCode::NOT_FOUND);

        let bad = handle_p2p_block_request(
            State(state.clone()),
            ConnectInfo(addr),
            Json(NetworkMessage::PeerDiscovery { peers: vec![] }),
        )
        .await
        .expect_err("bad request");
        assert_eq!(bad, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_handle_peers_endpoints() {
        let state = make_app_state();
        let peers = handle_get_peers(State(state.clone())).await;
        assert!(peers.0.is_empty());

        let p2p_peers = handle_get_p2p_peers(State(state)).await;
        assert!(p2p_peers.0.is_empty());

        let config = P2PConfig {
            message_timeout: Duration::from_millis(5),
            ..P2PConfig::default()
        };
        let network =
            Arc::new(HttpP2PNetwork::new(config, "http://127.0.0.1:9800".into()).expect("network"));
        network
            .add_peer("http://203.0.113.1:9001".into())
            .await
            .expect("add peer");

        let mut with_net = (*build_app_state(None, None)).clone();
        with_net.p2p_network = Some(network);
        let with_net = Arc::new(with_net);
        let peers = handle_get_peers(State(with_net.clone())).await;
        assert_eq!(peers.0.len(), 1);

        let p2p_peers = handle_get_p2p_peers(State(with_net)).await;
        assert_eq!(p2p_peers.0.len(), 1);
    }

    #[tokio::test]
    async fn test_handle_get_transaction_storage_error() {
        let storage: Arc<dyn Storage + Send + Sync> =
            Arc::new(FailingStorage::new(&["get_transaction"]));
        let mut state = (*build_app_state(None, None)).clone();
        state.storage = storage;
        let state = Arc::new(state);

        let addr: SocketAddr = "127.0.0.1:8100".parse().unwrap();
        let hex = "11".repeat(32);
        let err = handle_get_transaction(State(state), ConnectInfo(addr), AxumPath(hex))
            .await
            .expect_err("storage error");
        assert_eq!(err.0, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_handle_get_block_storage_errors() {
        let storage: Arc<dyn Storage + Send + Sync> =
            Arc::new(FailingStorage::new(&["get_block", "get_block_by_height"]));
        let mut state = (*build_app_state(None, None)).clone();
        state.storage = storage;
        let state = Arc::new(state);

        let addr: SocketAddr = "127.0.0.1:8101".parse().unwrap();
        let hash_err = handle_get_block(
            State(state.clone()),
            ConnectInfo(addr),
            AxumPath("22".repeat(32)),
        )
        .await
        .expect_err("hash failure");
        assert_eq!(hash_err.0, StatusCode::INTERNAL_SERVER_ERROR);

        let height_err = handle_get_block(State(state), ConnectInfo(addr), AxumPath("42".into()))
            .await
            .expect_err("height failure");
        assert_eq!(height_err.0, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_handle_get_account_error_paths() {
        let failing = FailingStorage::new(&["get_transactions_by_address"]);
        let account = Account {
            address: sample_public_key([3u8; 32]),
            balance: 5_000,
            nonce: 1,
        };
        failing
            .inner()
            .update_account(account.clone())
            .expect("account");
        let storage_with_tx_error: Arc<dyn Storage + Send + Sync> = Arc::new(failing);

        let mut state = (*build_app_state(None, None)).clone();
        state.storage = storage_with_tx_error.clone();
        let state = Arc::new(state);
        let addr: SocketAddr = "127.0.0.1:8102".parse().unwrap();
        let address_hex = hex::encode(account.address);
        let tx_err = handle_get_account(
            State(state),
            ConnectInfo(addr),
            AxumPath(address_hex.clone()),
        )
        .await
        .expect_err("tx lookup failure");
        assert_eq!(tx_err.0, StatusCode::INTERNAL_SERVER_ERROR);

        let storage_fail_account: Arc<dyn Storage + Send + Sync> =
            Arc::new(FailingStorage::new(&["get_account"]));
        let mut state = (*build_app_state(None, None)).clone();
        state.storage = storage_fail_account;
        let state = Arc::new(state);
        let load_err = handle_get_account(State(state), ConnectInfo(addr), AxumPath(address_hex))
            .await
            .expect_err("account load failure");
        assert_eq!(load_err.0, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_handle_list_l2_endpoints_failures() {
        let storage: Arc<dyn Storage + Send + Sync> = Arc::new(FailingStorage::new(&[
            "list_l2_networks",
            "list_l2_commits",
            "list_l2_exits",
        ]));
        let mut state = (*build_app_state(None, None)).clone();
        state.storage = storage;
        let state = Arc::new(state);

        let networks = handle_list_l2_networks(State(state.clone())).await;
        assert!(matches!(
            networks,
            Err((StatusCode::INTERNAL_SERVER_ERROR, _))
        ));

        let commits =
            handle_list_l2_commits(State(state.clone()), Query(L2Filter::default())).await;
        assert!(matches!(
            commits,
            Err((StatusCode::INTERNAL_SERVER_ERROR, _))
        ));

        let exits = handle_list_l2_exits(State(state), Query(L2Filter::default())).await;
        assert!(matches!(exits, Err((StatusCode::INTERNAL_SERVER_ERROR, _))));
    }

    #[tokio::test]
    async fn test_p2p_handlers_security_denied() {
        let dir = tempdir().expect("tempdir");
        let config = SecurityConfig {
            enable_ip_whitelist: true,
            whitelisted_ips: vec![],
            audit_log_path: dir.path().join("audit.log").to_string_lossy().to_string(),
            ..SecurityConfig::default()
        };
        let manager = SecurityManager::new(config).expect("manager");
        let state = build_app_state(Some(Arc::new(manager)), None);

        let addr: SocketAddr = "127.0.0.1:8300".parse().unwrap();
        let block = Block::new(vec![], vec![], 1, [9u8; 32]);
        let tx = sample_transaction([1u8; 32], sample_public_key([2u8; 32]), 3);

        let blocked = handle_p2p_blocks(
            State(state.clone()),
            ConnectInfo(addr),
            Json(NetworkMessage::Block(block.clone())),
        )
        .await;
        assert_eq!(blocked.0, StatusCode::FORBIDDEN);

        let blocked_resp = handle_p2p_block_response(
            State(state.clone()),
            ConnectInfo(addr),
            Json(NetworkMessage::BlockResponse(block.clone())),
        )
        .await;
        assert_eq!(blocked_resp.0, StatusCode::FORBIDDEN);

        let blocked_tx = handle_p2p_transactions(
            State(state.clone()),
            ConnectInfo(addr),
            Json(NetworkMessage::Transaction(tx.clone())),
        )
        .await;
        assert_eq!(blocked_tx.0, StatusCode::FORBIDDEN);

        let blocked_info = handle_p2p_peer_info(
            State(state.clone()),
            ConnectInfo(addr),
            Json(NetworkMessage::PeerInfo {
                peer_id: "peer".into(),
                addresses: vec!["http://peer".into()],
                time_us: Some(1),
            }),
        )
        .await;
        assert_eq!(blocked_info.0, StatusCode::FORBIDDEN);

        let blocked_discovery = handle_p2p_peer_discovery(
            State(state.clone()),
            ConnectInfo(addr),
            Json(NetworkMessage::PeerDiscovery {
                peers: vec!["http://peer".into()],
            }),
        )
        .await;
        assert_eq!(blocked_discovery.0, StatusCode::FORBIDDEN);

        let blocked_request = handle_p2p_block_request(
            State(state),
            ConnectInfo(addr),
            Json(NetworkMessage::BlockRequest { hash: [0u8; 32] }),
        )
        .await
        .expect_err("blocked request");
        assert_eq!(blocked_request, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_misc_endpoints() {
        let time = handle_time().await;
        assert!(time.0.get("timestamp").is_some());
        let version = handle_version().await;
        assert_eq!(
            version.0.get("version"),
            Some(&serde_json::json!(env!("CARGO_PKG_VERSION")))
        );
        let metrics_response = handle_metrics(State(make_app_state())).await;
        assert_eq!(metrics_response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_guard_request_with_security_failure() {
        let dir = tempdir().expect("tempdir");
        let config = SecurityConfig {
            enable_ip_whitelist: true,
            whitelisted_ips: vec![],
            audit_log_path: dir.path().join("audit.log").to_string_lossy().to_string(),
            ..SecurityConfig::default()
        };
        let manager = SecurityManager::new(config).expect("manager");
        let state = build_app_state(Some(Arc::new(manager)), None);

        let addr: SocketAddr = "127.0.0.1:9000".parse().unwrap();
        let result = guard_request(&state, &addr, "/health").await;
        assert!(matches!(result, Err(SecurityError::IpNotWhitelisted)));
    }

    #[tokio::test]
    async fn test_build_router_static_dir_branches() {
        let dir = tempdir().expect("tempdir");
        let existing = dir.path().join("dist");
        fs::create_dir_all(&existing).expect("create dist dir");

        let state_with_ui = build_app_state(None, Some(existing.clone()));
        let _router = build_router(state_with_ui);

        let missing = dir.path().join("missing");
        let state_missing = build_app_state(None, Some(missing));
        let _router_missing = build_router(state_missing);
    }

    #[tokio::test]
    async fn test_start_server_launch_and_abort() {
        let base = build_app_state(None, None);
        let addr = "127.0.0.1:0";
        let server = tokio::spawn(start_server((*base).clone(), addr));

        sleep(Duration::from_millis(25)).await;
        server.abort();
        let result = server.await.expect_err("server aborted");
        assert!(result.is_cancelled());
    }

    #[tokio::test]
    async fn test_bind_listener_ephemeral() {
        let listener = bind_listener("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        assert_eq!(addr.ip(), IpAddr::from(Ipv4Addr::LOCALHOST));
    }

    #[tokio::test]
    async fn test_consensus_handle_snapshot_and_submit() {
        let storage: Arc<dyn Storage + Send + Sync> = Arc::new(MemoryStorage::default());
        let mut config = PoAConfig::default();
        config.validators.push(Validator {
            id: sample_public_key([42u8; 32]),
            address: sample_public_key([42u8; 32]),
            stake: 1_000,
            is_active: true,
        });

        let poa = PoAConsensus::new(config, storage, sample_public_key([42u8; 32]));
        let mempool = poa.mempool();
        let consensus = Arc::new(Mutex::new(poa));
        let (tx_sender, mut rx) = mpsc::unbounded_channel();
        let handle = ConsensusHandle::new(consensus.clone(), tx_sender, mempool);

        let snapshot = handle.snapshot().await.expect("snapshot");
        assert_eq!(snapshot.validators.len(), 1);

        let tx = sample_transaction([1u8; 32], sample_public_key([2u8; 32]), 1);
        handle.submit_transaction(tx.clone()).expect("submit");
        let received = rx.recv().await.expect("recv");
        assert_eq!(received.hash(), tx.hash());
    }

    #[tokio::test]
    async fn test_resolve_peer_address_with_metadata() {
        let storage: Arc<dyn Storage + Send + Sync> = Arc::new(MemoryStorage::default());
        let config = P2PConfig {
            message_timeout: std::time::Duration::from_millis(5),
            ..P2PConfig::default()
        };
        let network =
            Arc::new(HttpP2PNetwork::new(config, "http://127.0.0.1:9550".into()).expect("network"));

        network
            .add_peer("http://203.0.113.10:9001".into())
            .await
            .expect("add peer");

        let state = Arc::new(AppState {
            storage,
            start_time: Instant::now(),
            peer_count: Arc::new(AtomicUsize::new(0)),
            p2p_network: Some(network),
            tx_sender: None,
            node_id: "test-node".into(),
            consensus: None,
            l2_config: L2Config {
                max_commit_size: 512,
                min_epoch_gap_ms: 1_000,
                challenge_window_ms: 2_000,
                da_mode: "test".into(),
                max_l2_count: 1,
            },
            mempool: Arc::new(Mempool::new(10)),
            unified_ui_dist: None,
            req_count: Arc::new(AtomicUsize::new(0)),
            security: None,
            metrics: None,
        });

        let socket: SocketAddr = "203.0.113.10:9100".parse().unwrap();
        let resolved = resolve_peer_address(
            &state,
            &socket,
            &NetworkMessage::PeerDiscovery { peers: vec![] },
        );

        assert!(resolved.contains("203.0.113.10"));
    }
}
