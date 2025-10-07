use anyhow::Result;
use axum::extract::{Path, Query, State};
use axum::http::{Method, StatusCode};
use axum::routing::{get, get_service, post};
use axum::{Json, Router};
use futures::future::join_all;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tracing::info;

use hex::{decode, encode};
use ippan_consensus::{ConsensusState, PoAConsensus, Validator};
use ippan_p2p::HttpP2PNetwork;
use ippan_storage::Storage;
use ippan_types::{
    ippan_time_ingest_sample, random_nonce, Block, HashTimer, IppanTimeMicros, Transaction,
};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

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

#[derive(Debug, Serialize, Deserialize)]
struct RemoteHealth {
    status: Option<String>,
    uptime_ms: Option<u128>,
    req_count: Option<usize>,
    peer_count: Option<usize>,
    node_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct PeerStatus {
    address: String,
    node_id: Option<String>,
    connected: bool,
    latency_ms: Option<u128>,
    last_seen: Option<String>,
    peer_count: Option<usize>,
    status: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct LocalPeerSummary {
    node_id: String,
    listen_address: String,
    announce_address: String,
    peer_count: usize,
}

#[derive(Debug, Serialize)]
struct PeerListResponse {
    peers: Vec<String>,
    total: usize,
    connected: usize,
    local_peer: LocalPeerSummary,
    details: Vec<PeerStatus>,
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

#[derive(Debug, Serialize)]
struct NodeInfo {
    is_running: bool,
    uptime_seconds: u64,
    version: String,
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

#[derive(Debug, Serialize)]
struct ApiNodeStatus {
    node_id: String,
    status: String,
    current_block: u64,
    total_transactions: u64,
    network_peers: usize,
    uptime_seconds: u64,
    version: String,
    node: NodeInfo,
    network: NetworkInfo,
    mempool: MempoolInfo,
    blockchain: BlockchainInfo,
}

#[derive(Debug, Serialize)]
struct NetworkStatsResponse {
    total_peers: usize,
    connected_peers: usize,
    network_id: String,
    protocol_version: String,
    uptime_seconds: u64,
}

#[derive(Debug, Serialize)]
struct MempoolStatsResponse {
    total_transactions: usize,
    total_senders: usize,
    total_size: u64,
    fee_distribution: Value,
}

#[derive(Debug, Serialize)]
struct ConsensusStatsResponse {
    current_round: u64,
    validators_count: usize,
    block_height: u64,
    consensus_status: String,
}

#[derive(Debug, Serialize)]
struct ValidatorInfoResponse {
    node_id: String,
    address: String,
    stake_amount: u64,
    is_active: bool,
}

#[derive(Debug, Serialize)]
struct RecentBlocksResponsePayload {
    latest_height: u64,
    blocks: Vec<BlockSummaryResponse>,
}

#[derive(Debug, Serialize)]
struct BlockSummaryResponse {
    height: u64,
    hash: String,
    parent_hashes: Vec<String>,
    proposer: String,
    transaction_count: usize,
    timestamp_micros: u64,
}

#[derive(Debug, Serialize)]
struct BlockDetailResponsePayload {
    block: BlockDetailResponse,
}

#[derive(Debug, Serialize)]
struct BlockDetailResponse {
    height: u64,
    hash: String,
    parent_hashes: Vec<String>,
    proposer: String,
    transaction_count: usize,
    timestamp_micros: u64,
    transactions: Vec<TransactionViewResponse>,
}

#[derive(Debug, Serialize)]
struct TransactionViewResponse {
    id: String,
    from: String,
    to: String,
    amount: u64,
    nonce: u64,
    timestamp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    direction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hashtimer: Option<String>,
}

#[derive(Debug, Serialize)]
struct WalletBalanceResponse {
    account: String,
    address: String,
    balance: u64,
    staked: u64,
    rewards: u64,
    nonce: u64,
    pending_transactions: Vec<String>,
}

#[derive(Debug, Serialize)]
struct WalletTransactionsResponse {
    transactions: Vec<TransactionViewResponse>,
}

#[derive(Debug, Serialize)]
struct AddressValidationResponse {
    valid: bool,
}

fn format_address(bytes: &[u8; 32]) -> String {
    format!("i{}", encode(bytes))
}

fn parse_wallet_address(value: &str) -> Result<[u8; 32], String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("address is required".to_string());
    }

    let normalized = trimmed
        .strip_prefix('i')
        .or_else(|| trimmed.strip_prefix('I'))
        .unwrap_or(trimmed);

    if normalized.len() != 64 {
        return Err("addresses must contain 64 hexadecimal characters".to_string());
    }

    let raw = decode(normalized).map_err(|err| format!("invalid hexadecimal address: {err}"))?;

    if raw.len() != 32 {
        return Err("address must decode to 32 bytes".to_string());
    }

    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&raw);
    Ok(bytes)
}

fn transaction_to_view(
    tx: &Transaction,
    perspective: Option<&[u8; 32]>,
) -> TransactionViewResponse {
    let direction = perspective.and_then(|address| {
        if &tx.from == address {
            Some("outgoing".to_string())
        } else if &tx.to == address {
            Some("incoming".to_string())
        } else {
            None
        }
    });

    TransactionViewResponse {
        id: encode(tx.hash()),
        from: format_address(&tx.from),
        to: format_address(&tx.to),
        amount: tx.amount,
        nonce: tx.nonce,
        timestamp: tx.timestamp.0,
        direction,
        hashtimer: Some(tx.hashtimer.to_string()),
    }
}

fn pending_hashes_for_address(mempool: &[Transaction], address: &[u8; 32]) -> Vec<String> {
    mempool
        .iter()
        .filter(|tx| &tx.from == address)
        .map(|tx| encode(tx.hash()))
        .collect()
}

fn mempool_total_size(mempool: &[Transaction]) -> u64 {
    mempool
        .iter()
        .map(|tx| {
            serde_json::to_vec(tx)
                .map(|bytes| bytes.len() as u64)
                .unwrap_or(0)
        })
        .sum()
}

fn internal_error<E: std::fmt::Display>(error: E) -> (StatusCode, String) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        format!("internal error: {error}"),
    )
}

fn forward_transaction(app: &AppState, tx: Transaction) -> Result<(), (StatusCode, String)> {
    if let Some(consensus) = app.consensus.clone() {
        consensus.submit_tx(tx).map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("submit failed: {err}"),
            )
        })?
    } else if let Some(sender) = app.tx_sender.clone() {
        sender.send(tx).map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("submit failed: {err}"),
            )
        })?
    } else {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "transaction submission unavailable".to_string(),
        ));
    }

    Ok(())
}

fn wallet_balance_for(
    app: &AppState,
    address: &str,
) -> Result<WalletBalanceResponse, (StatusCode, String)> {
    let address_bytes =
        parse_wallet_address(address).map_err(|err| (StatusCode::BAD_REQUEST, err))?;

    let account = app
        .storage
        .get_account(&address_bytes)
        .map_err(internal_error)?;

    let (balance, nonce) = account
        .map(|acct| (acct.balance, acct.nonce))
        .unwrap_or((0, 0));

    let mempool = app.mempool.read();
    let pending = pending_hashes_for_address(&mempool, &address_bytes);
    drop(mempool);

    let formatted = format_address(&address_bytes);

    Ok(WalletBalanceResponse {
        account: formatted.clone(),
        address: formatted,
        balance,
        staked: 0,
        rewards: 0,
        nonce,
        pending_transactions: pending,
    })
}

/// Generic OK response.
#[derive(Debug, Serialize)]
struct OkResponse {
    ok: bool,
}

/// Response payload for the `/time` endpoint.
#[derive(Debug, Serialize)]
struct TimeResponse {
    /// Current IPPAN Time in microseconds.
    ippan_time_microseconds: u64,
    /// 14-hex prefix extracted from the HashTimer time component.
    time_prefix_hex: String,
    /// Full 64-character HashTimer representing this sample.
    hashtimer: String,
    /// Wall-clock observation timestamp in RFC3339 format.
    observed_at: String,
    /// Milliseconds since the node started.
    uptime_ms: u128,
    /// Node identifier reporting the time sample.
    node_id: String,
    /// Monotonic request counter for observability.
    request_count: usize,
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

#[derive(Debug, Deserialize)]
struct BlocksQuery {
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct WalletAddressQuery {
    address: Option<String>,
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
        .route("/time", get(get_time))
        .route("/state", get(get_state))
        .route("/validators", get(get_validators))
        .route("/mempool", get(get_mempool))
        .route("/mempool/clear", post(clear_mempool))
        .route("/config/l2", get(get_l2_config))
        // txs
        .route("/tx", post(submit_tx))
        // unified REST API used by the frontend
        .route("/api/v1/status", get(api_status))
        .route("/api/v1/network", get(api_network))
        .route("/api/v1/mempool", get(api_mempool))
        .route("/api/v1/consensus", get(api_consensus))
        .route("/api/v1/validators", get(api_validators_list))
        .route("/api/v1/blocks/recent", get(api_recent_blocks))
        .route("/api/v1/blocks/:height", get(api_block_by_height))
        .route("/api/v1/balance", get(api_balance_query))
        .route("/api/v1/balance/:address", get(api_balance_path))
        .route("/api/v1/transactions", get(api_transactions))
        .route("/api/v1/address/validate", get(api_validate_address))
        .route("/api/v1/transaction", post(api_submit_transaction))
        .route("/p2p/peers", get(get_p2p_peer_list))
        .route("/peers", get(get_p2p_peers))
        .route("/p2p/peers/details", get(get_p2p_peers))
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

async fn get_time(State(app): State<AppState>) -> (StatusCode, Json<TimeResponse>) {
    let request_count = app.req_count.fetch_add(1, Ordering::Relaxed) + 1;
    let uptime_ms = app.start_time.elapsed().as_millis();

    let now = IppanTimeMicros::now();
    let nonce = random_nonce();
    let hashtimer = HashTimer::derive(
        "rpc_time",
        now,
        b"rpc_time",
        &now.0.to_be_bytes(),
        &nonce,
        app.node_id.as_bytes(),
    );
    let hashtimer_hex = hashtimer.to_hex();
    let time_prefix_hex = hashtimer_hex[..14].to_string();

    let observed = OffsetDateTime::now_utc();
    let observed_at = observed
        .format(&Rfc3339)
        .unwrap_or_else(|_| observed.unix_timestamp().to_string());

    (
        StatusCode::OK,
        Json(TimeResponse {
            ippan_time_microseconds: now.0,
            time_prefix_hex,
            hashtimer: hashtimer_hex,
            observed_at,
            uptime_ms,
            node_id: app.node_id.clone(),
            request_count,
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
    forward_transaction(&app, body.tx)?;
    Ok(Json(OkResponse { ok: true }))
}

async fn api_status(
    State(app): State<AppState>,
) -> Result<Json<ApiNodeStatus>, (StatusCode, String)> {
    let uptime_seconds = app.start_time.elapsed().as_secs();
    let peer_count = app.peer_count.load(Ordering::Relaxed);
    let latest_height = app.storage.get_latest_height().map_err(internal_error)?;
    let total_transactions = app
        .storage
        .get_transaction_count()
        .map_err(internal_error)?;

    let mempool = app.mempool.read();
    let mempool_len = mempool.len();
    drop(mempool);

    let version =
        env::var("IPPAN_NODE_VERSION").unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string());
    let status = if peer_count == 0 {
        "degraded"
    } else {
        "healthy"
    };
    let total_blocks = latest_height.saturating_add(1);

    Ok(Json(ApiNodeStatus {
        node_id: app.node_id.clone(),
        status: status.to_string(),
        current_block: latest_height,
        total_transactions,
        network_peers: peer_count,
        uptime_seconds,
        version: version.clone(),
        node: NodeInfo {
            is_running: true,
            uptime_seconds,
            version: version.clone(),
            node_id: app.node_id.clone(),
        },
        network: NetworkInfo {
            connected_peers: peer_count,
            known_peers: peer_count,
            total_peers: peer_count,
        },
        mempool: MempoolInfo {
            total_transactions: mempool_len,
            pending_transactions: mempool_len,
        },
        blockchain: BlockchainInfo {
            current_height: latest_height,
            total_blocks,
            total_transactions,
        },
    }))
}

async fn api_network(
    State(app): State<AppState>,
) -> Result<Json<NetworkStatsResponse>, (StatusCode, String)> {
    let peer_count = app.peer_count.load(Ordering::Relaxed);
    let uptime_seconds = app.start_time.elapsed().as_secs();
    let network_id = env::var("IPPAN_NETWORK_ID").unwrap_or_else(|_| "ippan-devnet".to_string());
    let protocol_version = env::var("IPPAN_PROTOCOL_VERSION")
        .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string());

    Ok(Json(NetworkStatsResponse {
        total_peers: peer_count,
        connected_peers: peer_count,
        network_id,
        protocol_version,
        uptime_seconds,
    }))
}

async fn get_p2p_peer_list(
    State(app): State<AppState>,
) -> Result<Json<Vec<String>>, (StatusCode, String)> {
    let network = app.p2p_network.clone().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        "network unavailable".to_string(),
    ))?;

    Ok(Json(network.get_peers()))
}

async fn get_p2p_peers(
    State(app): State<AppState>,
) -> Result<Json<PeerListResponse>, (StatusCode, String)> {
    let network = app.p2p_network.clone().ok_or(()).map_err(|_| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "network unavailable".to_string(),
        )
    })?;

    let listen_address = network.get_listening_address();
    let announce_address = network.get_announce_address();
    let peer_addresses = network.get_peers();
    let local_peer_count = app.peer_count.load(Ordering::Relaxed);

    let client = Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .map_err(internal_error)?;

    let statuses = join_all(peer_addresses.into_iter().map(|address| {
        let client = client.clone();
        async move { fetch_peer_status(client, address).await }
    }))
    .await;

    let mut peers = Vec::new();
    let mut connected = 0usize;
    let mut details = Vec::with_capacity(statuses.len());

    for status in statuses {
        if status.connected {
            connected += 1;
            if let Some(ref node_id) = status.node_id {
                peers.push(node_id.clone());
            } else {
                peers.push(status.address.clone());
            }
        }
        details.push(status);
    }

    let response = PeerListResponse {
        peers,
        total: details.len(),
        connected,
        local_peer: LocalPeerSummary {
            node_id: app.node_id.clone(),
            listen_address,
            announce_address,
            peer_count: local_peer_count,
        },
        details,
    };

    Ok(Json(response))
}

async fn fetch_peer_status(client: Client, address: String) -> PeerStatus {
    let mut status = PeerStatus {
        address: address.clone(),
        node_id: None,
        connected: false,
        latency_ms: None,
        last_seen: None,
        peer_count: None,
        status: None,
        error: None,
    };

    let start = Instant::now();
    let request = client.get(format!("{address}/health"));

    match request.send().await {
        Ok(response) => {
            status.latency_ms = Some(start.elapsed().as_millis());
            let code = response.status();
            if code.is_success() {
                status.connected = true;
                match response.json::<RemoteHealth>().await {
                    Ok(health) => {
                        status.node_id = health.node_id;
                        status.peer_count = health.peer_count;
                        status.status = health.status;
                        status.last_seen = OffsetDateTime::now_utc().format(&Rfc3339).ok();
                    }
                    Err(err) => {
                        status.error = Some(format!("decode error: {err}"));
                    }
                }
            } else {
                status.error = Some(format!("status {}", code));
            }
        }
        Err(err) => {
            status.error = Some(err.to_string());
        }
    }

    status
}

async fn api_mempool(
    State(app): State<AppState>,
) -> Result<Json<MempoolStatsResponse>, (StatusCode, String)> {
    let mempool = app.mempool.read();
    let mut senders = HashSet::new();
    for tx in mempool.iter() {
        senders.insert(tx.from);
    }

    let total_size = mempool_total_size(&mempool);

    Ok(Json(MempoolStatsResponse {
        total_transactions: mempool.len(),
        total_senders: senders.len(),
        total_size,
        fee_distribution: json!({}),
    }))
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

async fn api_consensus(
    State(app): State<AppState>,
) -> Result<Json<ConsensusStatsResponse>, (StatusCode, String)> {
    let consensus = app.consensus.clone().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        "consensus unavailable".to_string(),
    ))?;

    let state = consensus.get_state().await;
    let status = if state.validator_count == 0 {
        "initializing"
    } else if state.is_proposing {
        "healthy"
    } else {
        "idle"
    };

    Ok(Json(ConsensusStatsResponse {
        current_round: state.current_round,
        validators_count: state.validator_count,
        block_height: state.latest_block_height,
        consensus_status: status.to_string(),
    }))
}

async fn api_validators_list(
    State(app): State<AppState>,
) -> Result<Json<Vec<ValidatorInfoResponse>>, (StatusCode, String)> {
    let consensus = app.consensus.clone().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        "consensus unavailable".to_string(),
    ))?;

    let validators = consensus.get_validators().await;
    let payload = validators
        .into_iter()
        .map(|validator| ValidatorInfoResponse {
            node_id: format_address(&validator.id),
            address: format_address(&validator.address),
            stake_amount: validator.stake,
            is_active: validator.is_active,
        })
        .collect();

    Ok(Json(payload))
}

async fn api_recent_blocks(
    State(app): State<AppState>,
    Query(params): Query<BlocksQuery>,
) -> Result<Json<RecentBlocksResponsePayload>, (StatusCode, String)> {
    let limit = params.limit.unwrap_or(20).clamp(1, 100) as u64;
    let latest_height = app.storage.get_latest_height().map_err(internal_error)?;

    let start_height = latest_height.saturating_sub(limit.saturating_sub(1));
    let mut blocks = Vec::new();

    for height in (start_height..=latest_height).rev() {
        if let Some(block) = app
            .storage
            .get_block_by_height(height)
            .map_err(internal_error)?
        {
            #[allow(clippy::redundant_closure)]
            let parent_hashes: Vec<String> = block
                .header
                .parent_ids
                .iter()
                .map(|id| encode(id))
                .collect();
            blocks.push(BlockSummaryResponse {
                height,
                hash: encode(block.header.id),
                parent_hashes,
                proposer: format_address(&block.header.creator),
                transaction_count: block.transactions.len(),
                timestamp_micros: block.header.hashtimer.time().0,
            });
        }
    }

    Ok(Json(RecentBlocksResponsePayload {
        latest_height,
        blocks,
    }))
}

async fn api_block_by_height(
    State(app): State<AppState>,
    Path(height): Path<u64>,
) -> Result<Json<BlockDetailResponsePayload>, (StatusCode, String)> {
    let block = app
        .storage
        .get_block_by_height(height)
        .map_err(internal_error)?
        .ok_or((StatusCode::NOT_FOUND, format!("block {height} not found")))?;

    #[allow(clippy::redundant_closure)]
    let parent_hashes: Vec<String> = block
        .header
        .parent_ids
        .iter()
        .map(|id| encode(id))
        .collect();

    let transactions = block
        .transactions
        .iter()
        .map(|tx| transaction_to_view(tx, None))
        .collect();

    Ok(Json(BlockDetailResponsePayload {
        block: BlockDetailResponse {
            height,
            hash: encode(block.header.id),
            parent_hashes,
            proposer: format_address(&block.header.creator),
            transaction_count: block.transactions.len(),
            timestamp_micros: block.header.hashtimer.time().0,
            transactions,
        },
    }))
}

async fn api_balance_query(
    State(app): State<AppState>,
    Query(params): Query<WalletAddressQuery>,
) -> Result<Json<WalletBalanceResponse>, (StatusCode, String)> {
    let address = params.address.ok_or((
        StatusCode::BAD_REQUEST,
        "address query parameter is required".to_string(),
    ))?;
    let response = wallet_balance_for(&app, &address)?;
    Ok(Json(response))
}

async fn api_balance_path(
    State(app): State<AppState>,
    Path(address): Path<String>,
) -> Result<Json<WalletBalanceResponse>, (StatusCode, String)> {
    let response = wallet_balance_for(&app, &address)?;
    Ok(Json(response))
}

async fn api_transactions(
    State(app): State<AppState>,
    Query(params): Query<WalletAddressQuery>,
) -> Result<Json<WalletTransactionsResponse>, (StatusCode, String)> {
    let address = params.address.ok_or((
        StatusCode::BAD_REQUEST,
        "address query parameter is required".to_string(),
    ))?;
    let address_bytes =
        parse_wallet_address(&address).map_err(|err| (StatusCode::BAD_REQUEST, err))?;

    let mut transactions = app
        .storage
        .get_transactions_by_address(&address_bytes)
        .map_err(internal_error)?;

    transactions.sort_by_key(|tx| tx.timestamp.0);
    transactions.reverse();

    let views = transactions
        .iter()
        .map(|tx| transaction_to_view(tx, Some(&address_bytes)))
        .collect();

    Ok(Json(WalletTransactionsResponse {
        transactions: views,
    }))
}

async fn api_validate_address(
    Query(params): Query<WalletAddressQuery>,
) -> Json<AddressValidationResponse> {
    let valid = params
        .address
        .as_deref()
        .map(|candidate| parse_wallet_address(candidate).is_ok())
        .unwrap_or(false);

    Json(AddressValidationResponse { valid })
}

async fn api_submit_transaction(
    State(app): State<AppState>,
    Json(body): Json<SubmitTx>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let tx_hash = encode(body.tx.hash());
    forward_transaction(&app, body.tx)?;

    Ok(Json(json!({
        "success": true,
        "data": { "tx_hash": tx_hash },
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::State;
    use ippan_storage::SledStorage;
    use std::sync::Arc;
    use tempfile::tempdir;

    #[tokio::test]
    async fn time_endpoint_returns_hashtimer_and_counts_requests() {
        let temp_dir = tempdir().expect("tempdir");
        let storage = SledStorage::new(temp_dir.path()).expect("storage");
        storage.initialize().expect("init storage");
        let storage: Arc<dyn Storage + Send + Sync> = Arc::new(storage);

        let app_state = AppState {
            storage,
            start_time: Instant::now(),
            peer_count: Arc::new(AtomicUsize::new(0)),
            p2p_network: None,
            tx_sender: None,
            node_id: "test-node".to_string(),
            consensus: None,
            l2_config: L2Config::default(),
            mempool: Arc::new(parking_lot::RwLock::new(Vec::new())),
            unified_ui_dist: None,
            req_count: Arc::new(AtomicUsize::new(0)),
        };

        let (status_first, Json(first)) = get_time(State(app_state.clone())).await;
        assert_eq!(status_first, StatusCode::OK);
        assert_eq!(first.request_count, 1);
        assert_eq!(first.node_id, "test-node");
        assert!(first.ippan_time_microseconds > 0);
        assert_eq!(first.hashtimer.len(), 64);
        assert_eq!(first.time_prefix_hex.len(), 14);
        assert!(first.hashtimer.starts_with(&first.time_prefix_hex));

        let (status_second, Json(second)) = get_time(State(app_state)).await;
        assert_eq!(status_second, StatusCode::OK);
        assert_eq!(second.request_count, 2);
        assert_eq!(second.hashtimer.len(), 64);
    }
}
