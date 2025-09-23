use anyhow::Result;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use ippan_storage::{Account, Storage};
use ippan_types::{ippan_time_now, Block, Transaction};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{error, info, warn};

/// RPC response wrapper
#[derive(Debug, Serialize)]
pub struct RpcResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> RpcResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}

/// Submit transaction request
#[derive(Debug, Deserialize)]
pub struct SubmitTransactionRequest {
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub nonce: u64,
    pub signature: String,
}

/// Submit transaction response
#[derive(Debug, Serialize)]
pub struct SubmitTransactionResponse {
    pub tx_hash: String,
}

/// Get block query parameters
#[derive(Debug, Deserialize)]
pub struct GetBlockQuery {
    pub hash: Option<String>,
    pub height: Option<u64>,
}

/// Get account response
#[derive(Debug, Serialize)]
pub struct GetAccountResponse {
    pub address: String,
    pub balance: u64,
    pub nonce: u64,
}

/// Get time response
#[derive(Debug, Serialize)]
pub struct GetTimeResponse {
    pub time_us: u64,
}

/// Node status response
#[derive(Debug, Serialize)]
pub struct NodeStatusResponse {
    pub version: String,
    pub latest_height: u64,
    pub uptime_seconds: u64,
    pub peer_count: usize,
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: u64,
}

/// Application state
#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<dyn Storage + Send + Sync>,
    pub start_time: std::time::Instant,
    pub peer_count: Arc<std::sync::atomic::AtomicUsize>,
    pub p2p_network: Option<Arc<ippan_p2p::HttpP2PNetwork>>,
}

/// Create the Axum router with all RPC endpoints
pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Health and status endpoints
        .route("/health", get(health_check))
        .route("/status", get(node_status))
        .route("/time", get(get_time))
        // Blockchain endpoints
        .route("/block", get(get_block))
        .route("/block/:hash", get(get_block_by_hash))
        .route("/block/height/:height", get(get_block_by_height))
        .route("/tx", post(submit_transaction))
        .route("/tx/:hash", get(get_transaction))
        // Account endpoints
        .route("/account/:address", get(get_account))
        .route("/accounts", get(get_all_accounts))
        // P2P endpoints
        .route("/p2p/blocks", post(receive_block))
        .route("/p2p/transactions", post(receive_transaction))
        .route("/p2p/block-request", post(receive_block_request))
        .route("/p2p/block-response", post(receive_block_response))
        .route("/p2p/peer-info", post(receive_peer_info))
        .route("/p2p/peer-discovery", post(receive_peer_discovery))
        .route("/p2p/peers", get(get_peers))
        // Add middleware
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
        .with_state(state)
}

/// Health check endpoint
async fn health_check(State(state): State<AppState>) -> Json<RpcResponse<HealthResponse>> {
    let response = HealthResponse {
        status: "healthy".to_string(),
        timestamp: ippan_time_now(),
    };
    Json(RpcResponse::success(response))
}

/// Node status endpoint
async fn node_status(
    State(state): State<AppState>,
) -> Result<Json<RpcResponse<NodeStatusResponse>>, StatusCode> {
    let latest_height = state
        .storage
        .get_latest_height()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let uptime = state.start_time.elapsed().as_secs();
    let peer_count = state.peer_count.load(std::sync::atomic::Ordering::Relaxed);

    let response = NodeStatusResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        latest_height,
        uptime_seconds: uptime,
        peer_count,
    };

    Ok(Json(RpcResponse::success(response)))
}

/// Get current IPPAN time
async fn get_time() -> Json<RpcResponse<GetTimeResponse>> {
    let response = GetTimeResponse {
        time_us: ippan_time_now(),
    };
    Json(RpcResponse::success(response))
}

/// Get block by hash or height
async fn get_block(
    State(state): State<AppState>,
    Query(params): Query<GetBlockQuery>,
) -> Result<Json<RpcResponse<Option<Block>>>, StatusCode> {
    let block = if let Some(hash_str) = params.hash {
        let hash = hex::decode(&hash_str).map_err(|_| StatusCode::BAD_REQUEST)?;

        if hash.len() != 32 {
            return Err(StatusCode::BAD_REQUEST);
        }

        let mut hash_bytes = [0u8; 32];
        hash_bytes.copy_from_slice(&hash);

        state
            .storage
            .get_block(&hash_bytes)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    } else if let Some(height) = params.height {
        state
            .storage
            .get_block_by_height(height)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    } else {
        return Err(StatusCode::BAD_REQUEST);
    };

    Ok(Json(RpcResponse::success(block)))
}

/// Get block by hash
async fn get_block_by_hash(
    State(state): State<AppState>,
    Path(hash_str): Path<String>,
) -> Result<Json<RpcResponse<Option<Block>>>, StatusCode> {
    let hash = hex::decode(&hash_str).map_err(|_| StatusCode::BAD_REQUEST)?;

    if hash.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mut hash_bytes = [0u8; 32];
    hash_bytes.copy_from_slice(&hash);

    let block = state
        .storage
        .get_block(&hash_bytes)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(RpcResponse::success(block)))
}

/// Get block by height
async fn get_block_by_height(
    State(state): State<AppState>,
    Path(height): Path<u64>,
) -> Result<Json<RpcResponse<Option<Block>>>, StatusCode> {
    let block = state
        .storage
        .get_block_by_height(height)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(RpcResponse::success(block)))
}

/// Submit a transaction
async fn submit_transaction(
    State(state): State<AppState>,
    Json(request): Json<SubmitTransactionRequest>,
) -> Result<Json<RpcResponse<SubmitTransactionResponse>>, StatusCode> {
    // Parse addresses
    let from = hex::decode(&request.from).map_err(|_| StatusCode::BAD_REQUEST)?;
    let to = hex::decode(&request.to).map_err(|_| StatusCode::BAD_REQUEST)?;
    let signature = hex::decode(&request.signature).map_err(|_| StatusCode::BAD_REQUEST)?;

    if from.len() != 32 || to.len() != 32 || signature.len() != 64 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mut from_bytes = [0u8; 32];
    let mut to_bytes = [0u8; 32];
    let mut signature_bytes = [0u8; 64];

    from_bytes.copy_from_slice(&from);
    to_bytes.copy_from_slice(&to);
    signature_bytes.copy_from_slice(&signature);

    // Create transaction
    let mut tx = Transaction::new(from_bytes, to_bytes, request.amount, request.nonce);
    tx.signature = signature_bytes;

    // Validate transaction
    if !tx.is_valid() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Store transaction
    state
        .storage
        .store_transaction(tx.clone())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = SubmitTransactionResponse {
        tx_hash: hex::encode(tx.hash()),
    };

    info!("Submitted transaction: {}", response.tx_hash);
    Ok(Json(RpcResponse::success(response)))
}

/// Get transaction by hash
async fn get_transaction(
    State(state): State<AppState>,
    Path(hash_str): Path<String>,
) -> Result<Json<RpcResponse<Option<Transaction>>>, StatusCode> {
    let hash = hex::decode(&hash_str).map_err(|_| StatusCode::BAD_REQUEST)?;

    if hash.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mut hash_bytes = [0u8; 32];
    hash_bytes.copy_from_slice(&hash);

    let tx = state
        .storage
        .get_transaction(&hash_bytes)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(RpcResponse::success(tx)))
}

/// Get account information
async fn get_account(
    State(state): State<AppState>,
    Path(address_str): Path<String>,
) -> Result<Json<RpcResponse<Option<GetAccountResponse>>>, StatusCode> {
    let address = hex::decode(&address_str).map_err(|_| StatusCode::BAD_REQUEST)?;

    if address.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mut address_bytes = [0u8; 32];
    address_bytes.copy_from_slice(&address);

    let account = state
        .storage
        .get_account(&address_bytes)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = account.map(|acc| GetAccountResponse {
        address: address_str,
        balance: acc.balance,
        nonce: acc.nonce,
    });

    Ok(Json(RpcResponse::success(response)))
}

/// Get all accounts
async fn get_all_accounts(
    State(state): State<AppState>,
) -> Result<Json<RpcResponse<Vec<GetAccountResponse>>>, StatusCode> {
    let accounts = state
        .storage
        .get_all_accounts()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response: Vec<GetAccountResponse> = accounts
        .into_iter()
        .map(|acc| GetAccountResponse {
            address: hex::encode(acc.address),
            balance: acc.balance,
            nonce: acc.nonce,
        })
        .collect();

    Ok(Json(RpcResponse::success(response)))
}

/// P2P endpoint: Receive block from peer
async fn receive_block(
    State(state): State<AppState>,
    Json(block): Json<Block>,
) -> Result<Json<RpcResponse<String>>, StatusCode> {
    // Store the received block
    if let Err(e) = state.storage.store_block(block.clone()) {
        error!("Failed to store received block: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    info!("Received block from peer: {}", hex::encode(block.hash()));
    Ok(Json(RpcResponse::success("Block received".to_string())))
}

/// P2P endpoint: Receive transaction from peer
async fn receive_transaction(
    State(state): State<AppState>,
    Json(tx): Json<Transaction>,
) -> Result<Json<RpcResponse<String>>, StatusCode> {
    // Store the received transaction
    if let Err(e) = state.storage.store_transaction(tx.clone()) {
        error!("Failed to store received transaction: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    info!("Received transaction from peer: {}", hex::encode(tx.hash()));
    Ok(Json(RpcResponse::success(
        "Transaction received".to_string(),
    )))
}

/// P2P endpoint: Receive block request from peer
async fn receive_block_request(
    State(state): State<AppState>,
    Json(request): Json<ippan_p2p::NetworkMessage>,
) -> Result<Json<RpcResponse<String>>, StatusCode> {
    if let ippan_p2p::NetworkMessage::BlockRequest { hash } = request {
        // Try to find the requested block
        if let Ok(Some(block)) = state.storage.get_block(&hash) {
            // In a real implementation, we would send the block back to the requesting peer
            info!("Block request received for: {}", hex::encode(hash));
            Ok(Json(RpcResponse::success(
                "Block request processed".to_string(),
            )))
        } else {
            warn!("Block not found for request: {}", hex::encode(hash));
            Err(StatusCode::NOT_FOUND)
        }
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}

/// P2P endpoint: Receive block response from peer
async fn receive_block_response(
    State(state): State<AppState>,
    Json(response): Json<ippan_p2p::NetworkMessage>,
) -> Result<Json<RpcResponse<String>>, StatusCode> {
    if let ippan_p2p::NetworkMessage::BlockResponse(block) = response {
        // Store the received block
        if let Err(e) = state.storage.store_block(block.clone()) {
            error!("Failed to store received block response: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }

        info!("Received block response: {}", hex::encode(block.hash()));
        Ok(Json(RpcResponse::success(
            "Block response received".to_string(),
        )))
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}

/// P2P endpoint: Receive peer info from peer
async fn receive_peer_info(
    State(_state): State<AppState>,
    Json(info): Json<ippan_p2p::NetworkMessage>,
) -> Result<Json<RpcResponse<String>>, StatusCode> {
    if let ippan_p2p::NetworkMessage::PeerInfo { peer_id, addresses } = info {
        info!(
            "Received peer info: {} with addresses: {:?}",
            peer_id, addresses
        );
        Ok(Json(RpcResponse::success("Peer info received".to_string())))
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}

/// P2P endpoint: Receive peer discovery from peer
async fn receive_peer_discovery(
    State(_state): State<AppState>,
    Json(discovery): Json<ippan_p2p::NetworkMessage>,
) -> Result<Json<RpcResponse<String>>, StatusCode> {
    if let ippan_p2p::NetworkMessage::PeerDiscovery { peers } = discovery {
        info!("Received peer discovery with {} peers", peers.len());
        Ok(Json(RpcResponse::success(
            "Peer discovery received".to_string(),
        )))
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}

/// P2P endpoint: Get list of peers
async fn get_peers(
    State(state): State<AppState>,
) -> Result<Json<RpcResponse<Vec<String>>>, StatusCode> {
    if let Some(p2p_network) = &state.p2p_network {
        let peers = p2p_network.get_peers();
        Ok(Json(RpcResponse::success(peers)))
    } else {
        Ok(Json(RpcResponse::success(vec![])))
    }
}

/// Start the HTTP server
pub async fn start_server(state: AppState, addr: &str) -> Result<()> {
    let app = create_router(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("RPC server listening on {}", addr);
    info!("Available endpoints:");
    info!("  GET  /health - Health check");
    info!("  GET  /status - Node status");
    info!("  GET  /time - Current IPPAN time");
    info!("  GET  /block?hash=<hash> - Get block by hash");
    info!("  GET  /block?height=<height> - Get block by height");
    info!("  GET  /block/<hash> - Get block by hash");
    info!("  GET  /block/height/<height> - Get block by height");
    info!("  POST /tx - Submit transaction");
    info!("  GET  /tx/<hash> - Get transaction by hash");
    info!("  GET  /account/<address> - Get account info");
    info!("  GET  /accounts - Get all accounts");

    axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ippan_storage::MemoryStorage;
    use std::sync::atomic::AtomicUsize;

    #[tokio::test]
    async fn test_health_check() {
        let storage = Arc::new(MemoryStorage::new());
        let state = AppState {
            storage,
            start_time: std::time::Instant::now(),
            peer_count: Arc::new(AtomicUsize::new(0)),
            p2p_network: None,
        };

        let Json(response) = health_check(State(state)).await;
        assert!(response.success);
        let health = response.data.expect("health check should return a payload");
        assert_eq!(health.status, "healthy");
    }

    #[tokio::test]
    async fn test_get_time() {
        let Json(response) = get_time().await;
        assert!(response.success);
        let time = response
            .data
            .expect("time endpoint should return a payload");
        assert!(time.time_us > 0);
    }
}
