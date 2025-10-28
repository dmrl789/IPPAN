use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use ippan_types::{ippan_time_now, Block, Transaction};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{debug, info, warn};

use crate::{HttpP2PNetwork, NetworkMessage, P2PConfig};

/// Application state
#[derive(Clone)]
pub struct AppState {
    pub p2p_network: Arc<HttpP2PNetwork>,
    pub l2_config: L2Config,
}

/// L2 configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2Config {
    pub chain_id: u64,
    pub block_time_ms: u64,
    pub max_transactions_per_block: usize,
    pub gas_limit: u64,
    pub min_gas_price: u64,
    pub max_gas_price: u64,
    pub consensus_threshold: f64,
    pub validator_reward: u64,
    pub transaction_fee_percent: f64,
    pub max_block_size_bytes: usize,
    pub max_transaction_size_bytes: usize,
    pub max_accounts: usize,
    pub max_contracts: usize,
    pub max_storage_per_account: usize,
    pub max_computation_per_transaction: u64,
    pub max_memory_per_transaction: usize,
    pub max_stack_depth: usize,
    pub max_call_depth: usize,
    pub max_events_per_transaction: usize,
    pub max_logs_per_transaction: usize,
    pub max_contract_code_size: usize,
    pub max_contract_data_size: usize,
    pub max_contract_storage_size: usize,
    pub max_contract_calls_per_transaction: usize,
    pub max_contract_creates_per_transaction: usize,
    pub max_contract_destroys_per_transaction: usize,
    pub max_contract_updates_per_transaction: usize,
    pub max_contract_reads_per_transaction: usize,
    pub max_contract_writes_per_transaction: usize,
    pub max_contract_deletes_per_transaction: usize,
    pub max_contract_list_per_transaction: usize,
    pub max_contract_count_per_transaction: usize,
    pub max_contract_size_per_transaction: usize,
    pub max_contract_data_per_transaction: usize,
    pub max_contract_storage_per_transaction: usize,
    pub max_contract_calls_per_block: usize,
    pub max_contract_creates_per_block: usize,
    pub max_contract_destroys_per_block: usize,
    pub max_contract_updates_per_block: usize,
    pub max_contract_reads_per_block: usize,
    pub max_contract_writes_per_block: usize,
    pub max_contract_deletes_per_block: usize,
    pub max_contract_list_per_block: usize,
    pub max_contract_count_per_block: usize,
    pub max_contract_size_per_block: usize,
    pub max_contract_data_per_block: usize,
    pub max_contract_storage_per_block: usize,
    pub max_contract_calls_per_second: usize,
    pub max_contract_creates_per_second: usize,
    pub max_contract_destroys_per_second: usize,
    pub max_contract_updates_per_second: usize,
    pub max_contract_reads_per_second: usize,
    pub max_contract_writes_per_second: usize,
    pub max_contract_deletes_per_second: usize,
    pub max_contract_list_per_second: usize,
    pub max_contract_count_per_second: usize,
    pub max_contract_size_per_second: usize,
    pub max_contract_data_per_second: usize,
    pub max_contract_storage_per_second: usize,
    pub max_contract_calls_per_minute: usize,
    pub max_contract_creates_per_minute: usize,
    pub max_contract_destroys_per_minute: usize,
    pub max_contract_updates_per_minute: usize,
    pub max_contract_reads_per_minute: usize,
    pub max_contract_writes_per_minute: usize,
    pub max_contract_deletes_per_minute: usize,
    pub max_contract_list_per_minute: usize,
    pub max_contract_count_per_minute: usize,
    pub max_contract_size_per_minute: usize,
    pub max_contract_data_per_minute: usize,
    pub max_contract_storage_per_minute: usize,
    pub max_contract_calls_per_hour: usize,
    pub max_contract_creates_per_hour: usize,
    pub max_contract_destroys_per_hour: usize,
    pub max_contract_updates_per_hour: usize,
    pub max_contract_reads_per_hour: usize,
    pub max_contract_writes_per_hour: usize,
    pub max_contract_deletes_per_hour: usize,
    pub max_contract_list_per_hour: usize,
    pub max_contract_count_per_hour: usize,
    pub max_contract_size_per_hour: usize,
    pub max_contract_data_per_hour: usize,
    pub max_contract_storage_per_hour: usize,
    pub max_contract_calls_per_day: usize,
    pub max_contract_creates_per_day: usize,
    pub max_contract_destroys_per_day: usize,
    pub max_contract_updates_per_day: usize,
    pub max_contract_reads_per_day: usize,
    pub max_contract_writes_per_day: usize,
    pub max_contract_deletes_per_day: usize,
    pub max_contract_list_per_day: usize,
    pub max_contract_count_per_day: usize,
    pub max_contract_size_per_day: usize,
    pub max_contract_data_per_day: usize,
    pub max_contract_storage_per_day: usize,
    pub max_contract_calls_per_week: usize,
    pub max_contract_creates_per_week: usize,
    pub max_contract_destroys_per_week: usize,
    pub max_contract_updates_per_week: usize,
    pub max_contract_reads_per_week: usize,
    pub max_contract_writes_per_week: usize,
    pub max_contract_deletes_per_week: usize,
    pub max_contract_list_per_week: usize,
    pub max_contract_count_per_week: usize,
    pub max_contract_size_per_week: usize,
    pub max_contract_data_per_week: usize,
    pub max_contract_storage_per_week: usize,
    pub max_contract_calls_per_month: usize,
    pub max_contract_creates_per_month: usize,
    pub max_contract_destroys_per_month: usize,
    pub max_contract_updates_per_month: usize,
    pub max_contract_reads_per_month: usize,
    pub max_contract_writes_per_month: usize,
    pub max_contract_deletes_per_month: usize,
    pub max_contract_list_per_month: usize,
    pub max_contract_count_per_month: usize,
    pub max_contract_size_per_month: usize,
    pub max_contract_data_per_month: usize,
    pub max_contract_storage_per_month: usize,
    pub max_contract_calls_per_year: usize,
    pub max_contract_creates_per_year: usize,
    pub max_contract_destroys_per_year: usize,
    pub max_contract_updates_per_year: usize,
    pub max_contract_reads_per_year: usize,
    pub max_contract_writes_per_year: usize,
    pub max_contract_deletes_per_year: usize,
    pub max_contract_list_per_year: usize,
    pub max_contract_count_per_year: usize,
    pub max_contract_size_per_year: usize,
    pub max_contract_data_per_year: usize,
    pub max_contract_storage_per_year: usize,
}

impl Default for L2Config {
    fn default() -> Self {
        Self {
            chain_id: 1,
            block_time_ms: 1000,
            max_transactions_per_block: 1000,
            gas_limit: 1000000,
            min_gas_price: 1,
            max_gas_price: 1000000,
            consensus_threshold: 0.67,
            validator_reward: 100,
            transaction_fee_percent: 0.01,
            max_block_size_bytes: 1024 * 1024,
            max_transaction_size_bytes: 1024,
            max_accounts: 1000000,
            max_contracts: 100000,
            max_storage_per_account: 1024 * 1024,
            max_computation_per_transaction: 1000000,
            max_memory_per_transaction: 1024 * 1024,
            max_stack_depth: 1024,
            max_call_depth: 1024,
            max_events_per_transaction: 1000,
            max_logs_per_transaction: 1000,
            max_contract_code_size: 1024 * 1024,
            max_contract_data_size: 1024 * 1024,
            max_contract_storage_size: 1024 * 1024,
            max_contract_calls_per_transaction: 1000,
            max_contract_creates_per_transaction: 100,
            max_contract_destroys_per_transaction: 100,
            max_contract_updates_per_transaction: 1000,
            max_contract_reads_per_transaction: 1000,
            max_contract_writes_per_transaction: 1000,
            max_contract_deletes_per_transaction: 1000,
            max_contract_list_per_transaction: 1000,
            max_contract_count_per_transaction: 1000,
            max_contract_size_per_transaction: 1024 * 1024,
            max_contract_data_per_transaction: 1024 * 1024,
            max_contract_storage_per_transaction: 1024 * 1024,
            max_contract_calls_per_block: 10000,
            max_contract_creates_per_block: 1000,
            max_contract_destroys_per_block: 1000,
            max_contract_updates_per_block: 10000,
            max_contract_reads_per_block: 10000,
            max_contract_writes_per_block: 10000,
            max_contract_deletes_per_block: 10000,
            max_contract_list_per_block: 10000,
            max_contract_count_per_block: 10000,
            max_contract_size_per_block: 1024 * 1024 * 10,
            max_contract_data_per_block: 1024 * 1024 * 10,
            max_contract_storage_per_block: 1024 * 1024 * 10,
            max_contract_calls_per_second: 1000,
            max_contract_creates_per_second: 100,
            max_contract_destroys_per_second: 100,
            max_contract_updates_per_second: 1000,
            max_contract_reads_per_second: 1000,
            max_contract_writes_per_second: 1000,
            max_contract_deletes_per_second: 1000,
            max_contract_list_per_second: 1000,
            max_contract_count_per_second: 1000,
            max_contract_size_per_second: 1024 * 1024,
            max_contract_data_per_second: 1024 * 1024,
            max_contract_storage_per_second: 1024 * 1024,
            max_contract_calls_per_minute: 60000,
            max_contract_creates_per_minute: 6000,
            max_contract_destroys_per_minute: 6000,
            max_contract_updates_per_minute: 60000,
            max_contract_reads_per_minute: 60000,
            max_contract_writes_per_minute: 60000,
            max_contract_deletes_per_minute: 60000,
            max_contract_list_per_minute: 60000,
            max_contract_count_per_minute: 60000,
            max_contract_size_per_minute: 1024 * 1024 * 60,
            max_contract_data_per_minute: 1024 * 1024 * 60,
            max_contract_storage_per_minute: 1024 * 1024 * 60,
            max_contract_calls_per_hour: 3600000,
            max_contract_creates_per_hour: 360000,
            max_contract_destroys_per_hour: 360000,
            max_contract_updates_per_hour: 3600000,
            max_contract_reads_per_hour: 3600000,
            max_contract_writes_per_hour: 3600000,
            max_contract_deletes_per_hour: 3600000,
            max_contract_list_per_hour: 3600000,
            max_contract_count_per_hour: 3600000,
            max_contract_size_per_hour: 1024 * 1024 * 3600,
            max_contract_data_per_hour: 1024 * 1024 * 3600,
            max_contract_storage_per_hour: 1024 * 1024 * 3600,
            max_contract_calls_per_day: 86400000,
            max_contract_creates_per_day: 8640000,
            max_contract_destroys_per_day: 8640000,
            max_contract_updates_per_day: 86400000,
            max_contract_reads_per_day: 86400000,
            max_contract_writes_per_day: 86400000,
            max_contract_deletes_per_day: 86400000,
            max_contract_list_per_day: 86400000,
            max_contract_count_per_day: 86400000,
            max_contract_size_per_day: 1024 * 1024 * 86400,
            max_contract_data_per_day: 1024 * 1024 * 86400,
            max_contract_storage_per_day: 1024 * 1024 * 86400,
            max_contract_calls_per_week: 604800000,
            max_contract_creates_per_week: 60480000,
            max_contract_destroys_per_week: 60480000,
            max_contract_updates_per_week: 604800000,
            max_contract_reads_per_week: 604800000,
            max_contract_writes_per_week: 604800000,
            max_contract_deletes_per_week: 604800000,
            max_contract_list_per_week: 604800000,
            max_contract_count_per_week: 604800000,
            max_contract_size_per_week: 1024 * 1024 * 604800,
            max_contract_data_per_week: 1024 * 1024 * 604800,
            max_contract_storage_per_week: 1024 * 1024 * 604800,
            max_contract_calls_per_month: 2592000000,
            max_contract_creates_per_month: 259200000,
            max_contract_destroys_per_month: 259200000,
            max_contract_updates_per_month: 2592000000,
            max_contract_reads_per_month: 2592000000,
            max_contract_writes_per_month: 2592000000,
            max_contract_deletes_per_month: 2592000000,
            max_contract_list_per_month: 2592000000,
            max_contract_count_per_month: 2592000000,
            max_contract_size_per_month: 1024 * 1024 * 2592000,
            max_contract_data_per_month: 1024 * 1024 * 2592000,
            max_contract_storage_per_month: 1024 * 1024 * 2592000,
            max_contract_calls_per_year: 31536000000,
            max_contract_creates_per_year: 3153600000,
            max_contract_destroys_per_year: 3153600000,
            max_contract_updates_per_year: 31536000000,
            max_contract_reads_per_year: 31536000000,
            max_contract_writes_per_year: 31536000000,
            max_contract_deletes_per_year: 31536000000,
            max_contract_list_per_year: 31536000000,
            max_contract_count_per_year: 31536000000,
            max_contract_size_per_year: 1024 * 1024 * 31536000,
            max_contract_data_per_year: 1024 * 1024 * 31536000,
            max_contract_storage_per_year: 1024 * 1024 * 31536000,
        }
    }
}

/// Health check response
#[derive(Serialize)]
struct HealthResponse {
    status: String,
    timestamp: u64,
    version: String,
    peer_count: usize,
    chain_id: u64,
}

/// Time response
#[derive(Serialize)]
struct TimeResponse {
    timestamp: u64,
    time_us: u64,
}

/// Version response
#[derive(Serialize)]
struct VersionResponse {
    version: String,
    build_time: String,
    git_commit: String,
    rust_version: String,
}

/// Transaction submission request
#[derive(Deserialize)]
struct TransactionRequest {
    transaction: Transaction,
}

/// Transaction submission response
#[derive(Serialize)]
struct TransactionResponse {
    success: bool,
    transaction_hash: String,
    message: String,
}

/// Block request
#[derive(Deserialize)]
struct BlockRequest {
    hash: Option<String>,
    height: Option<u64>,
}

/// Account request
#[derive(Deserialize)]
struct AccountRequest {
    address: String,
}

/// P2P message handler
async fn handle_p2p_message(
    State(_state): State<AppState>,
    Json(message): Json<NetworkMessage>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    debug!("Received P2P message: {:?}", message);

    match message {
        NetworkMessage::Block(block) => {
            info!("Received block from P2P: {:?}", block);
            // TODO: Process block
        }
        NetworkMessage::Transaction(tx) => {
            info!("Received transaction from P2P: {:?}", tx);
            // TODO: Process transaction
        }
        NetworkMessage::BlockRequest { hash } => {
            info!("Received block request for hash: {:?}", hash);
            // TODO: Send block response
        }
        NetworkMessage::BlockResponse(block) => {
            info!("Received block response: {:?}", block);
            // TODO: Process block response
        }
        NetworkMessage::PeerInfo { peer_id, addresses, time_us } => {
            info!("Received peer info: {} at {:?} (time: {:?})", peer_id, addresses, time_us);
            // TODO: Update peer information
        }
        NetworkMessage::PeerDiscovery { peers } => {
            info!("Received peer discovery: {:?}", peers);
            // TODO: Add discovered peers
        }
    }

    Ok(Json(serde_json::json!({"status": "ok"})))
}

/// Health check endpoint
async fn health_check(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        timestamp: ippan_time_now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        peer_count: state.p2p_network.get_peer_count(),
        chain_id: state.l2_config.chain_id,
    })
}

/// Time endpoint
async fn time_endpoint() -> Json<TimeResponse> {
    let now = ippan_time_now();
    Json(TimeResponse {
        timestamp: now,
        time_us: now,
    })
}

/// Version endpoint
async fn version_endpoint() -> Json<VersionResponse> {
    Json(VersionResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        build_time: "unknown".to_string(),
        git_commit: "unknown".to_string(),
        rust_version: "unknown".to_string(),
    })
}

/// Metrics endpoint
async fn metrics_endpoint() -> impl IntoResponse {
    let metrics = format!(
        "# HELP ippan_node_info Node information
# TYPE ippan_node_info gauge
ippan_node_info{{version=\"{}\",chain_id=\"{}\"}} 1
# HELP ippan_peer_count Number of connected peers
# TYPE ippan_peer_count gauge
ippan_peer_count 0
# HELP ippan_uptime_seconds Node uptime in seconds
# TYPE ippan_uptime_seconds counter
ippan_uptime_seconds 0
",
        env!("CARGO_PKG_VERSION"),
        1
    );

    (StatusCode::OK, metrics)
}

/// Submit transaction endpoint
async fn submit_transaction(
    State(_state): State<AppState>,
    Json(request): Json<TransactionRequest>,
) -> Result<Json<TransactionResponse>, StatusCode> {
    let tx = request.transaction;
    let tx_hash = format!("{}", hex::encode(blake3::hash(&bincode::serialize(&tx).unwrap_or_default()).as_bytes()));

    // TODO: Validate transaction
    // TODO: Add to mempool
    // TODO: Broadcast to peers

    info!("Transaction submitted: {}", tx_hash);

    Ok(Json(TransactionResponse {
        success: true,
        transaction_hash: tx_hash,
        message: "Transaction submitted successfully".to_string(),
    }))
}

/// Get block endpoint
async fn get_block(
    State(_state): State<AppState>,
    Query(_params): Query<BlockRequest>,
) -> Result<Json<Block>, StatusCode> {
    // TODO: Implement block retrieval
    Err(StatusCode::NOT_IMPLEMENTED)
}

/// Get account endpoint
async fn get_account(
    State(_state): State<AppState>,
    Path(_address): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // TODO: Implement account retrieval
    Err(StatusCode::NOT_IMPLEMENTED)
}

/// Get peers endpoint
async fn get_peers(State(state): State<AppState>) -> Json<Vec<String>> {
    Json(state.p2p_network.get_peers())
}

/// Add peer endpoint
async fn add_peer(
    State(state): State<AppState>,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let peer_address = request
        .get("address")
        .and_then(|v| v.as_str())
        .ok_or(StatusCode::BAD_REQUEST)?;

    if let Err(e) = state.p2p_network.add_peer(peer_address.to_string()).await {
        warn!("Failed to add peer {}: {}", peer_address, e);
        return Err(StatusCode::BAD_REQUEST);
    }

    Ok(Json(serde_json::json!({"status": "ok"})))
}

/// Remove peer endpoint
async fn remove_peer(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    state.p2p_network.remove_peer(&address);
    Ok(Json(serde_json::json!({"status": "ok"})))
}

/// Serve static files
async fn serve_static() -> Html<&'static str> {
    Html(include_str!("static/index.html"))
}

/// Create the application router
pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Health and info endpoints
        .route("/health", get(health_check))
        .route("/time", get(time_endpoint))
        .route("/version", get(version_endpoint))
        .route("/metrics", get(metrics_endpoint))
        
        // Blockchain endpoints
        .route("/transactions", post(submit_transaction))
        .route("/blocks", get(get_block))
        .route("/accounts/:address", get(get_account))
        
        // P2P endpoints
        .route("/p2p/blocks", post(handle_p2p_message))
        .route("/p2p/transactions", post(handle_p2p_message))
        .route("/p2p/block-request", post(handle_p2p_message))
        .route("/p2p/block-response", post(handle_p2p_message))
        .route("/p2p/peer-info", post(handle_p2p_message))
        .route("/p2p/peer-discovery", post(handle_p2p_message))
        .route("/p2p/peers", get(get_peers))
        .route("/p2p/peers", post(add_peer))
        .route("/p2p/peers/:address", axum::routing::delete(remove_peer))
        
        // Static file serving
        .route("/", get(serve_static))
        .route("/static/*path", get(serve_static))
        
        .with_state(state)
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
}

/// Start the RPC server
pub async fn start_server(
    listen_address: &str,
    p2p_config: P2PConfig,
    l2_config: L2Config,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create P2P network
    let p2p_network = HttpP2PNetwork::new(p2p_config, listen_address.to_string())?;
    let p2p_network = Arc::new(p2p_network);

    // Start P2P network
    let mut p2p_network_mut = (*p2p_network).clone();
    p2p_network_mut.start().await?;

    // Create application state
    let state = AppState {
        p2p_network,
        l2_config,
    };

    // Create router
    let app = create_router(state);

    // Parse listen address
    let url = url::Url::parse(listen_address)?;
    let host = url.host_str().unwrap_or("0.0.0.0");
    let port = url.port_or_known_default().unwrap_or(8080);

    // Start server
    let listener = TcpListener::bind(format!("{}:{}", host, port)).await?;
    info!("RPC server listening on {}:{}", host, port);

    axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check() {
        let config = P2PConfig::default();
        let p2p_network = HttpP2PNetwork::new(config, "http://localhost:9000".to_string()).unwrap();
        let state = AppState {
            p2p_network: Arc::new(p2p_network),
            l2_config: L2Config::default(),
        };

        let response = health_check(State(state)).await;
        assert_eq!(response.status, "healthy");
    }

    #[tokio::test]
    async fn test_time_endpoint() {
        let response = time_endpoint().await;
        assert!(response.timestamp > 0);
        assert_eq!(response.timestamp, response.time_us);
    }

    #[tokio::test]
    async fn test_version_endpoint() {
        let response = version_endpoint().await;
        assert!(!response.version.is_empty());
        assert!(!response.build_time.is_empty());
        assert!(!response.git_commit.is_empty());
        assert!(!response.rust_version.is_empty());
    }
}