use anyhow::Result;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use ippan_storage::Storage;
use ippan_types::{ippan_time_now, Block, Transaction};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{error, info, warn};

/// RPC response wrapper
#[derive(Debug, Serialize, Deserialize)]
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

/// Submit transaction response
#[derive(Debug, Serialize, Deserialize)]
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
#[derive(Debug, Serialize, Deserialize)]
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
#[derive(Debug, Serialize, Deserialize)]
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
    pub tx_sender: Option<UnboundedSender<Transaction>>,
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
async fn health_check(State(_state): State<AppState>) -> Json<RpcResponse<HealthResponse>> {
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
    Json(mut tx): Json<Transaction>,
) -> Result<Json<RpcResponse<SubmitTransactionResponse>>, StatusCode> {
    // Ensure transaction identifier reflects the provided contents
    let computed_hash = tx.hash();
    if tx.id != computed_hash {
        tx.id = computed_hash;
    }

    // Validate transaction before accepting it
    if !tx.is_valid() {
        warn!("Rejected invalid transaction from {}", hex::encode(tx.from));
        return Err(StatusCode::BAD_REQUEST);
    }

    // Store transaction locally
    state
        .storage
        .store_transaction(tx.clone())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Submit to the consensus mempool if available
    if let Some(sender) = &state.tx_sender {
        if let Err(error) = sender.send(tx.clone()) {
            error!("Failed to forward transaction to consensus: {}", error);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    // Broadcast the transaction to peers if networking is available
    if let Some(network) = &state.p2p_network {
        if let Err(error) = network.broadcast_transaction(tx.clone()).await {
            warn!(
                "Failed to broadcast transaction {}: {}",
                hex::encode(tx.id),
                error
            );
        }
    }

    let response = SubmitTransactionResponse {
        tx_hash: hex::encode(tx.id),
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
    if !block.is_valid() {
        warn!(
            "Rejected invalid block from peer: {}",
            hex::encode(block.hash())
        );
        return Err(StatusCode::BAD_REQUEST);
    }

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
    if !tx.is_valid() {
        warn!(
            "Rejected invalid transaction from peer: {}",
            hex::encode(tx.hash())
        );
        return Err(StatusCode::BAD_REQUEST);
    }

    // Store the received transaction
    if let Err(e) = state.storage.store_transaction(tx.clone()) {
        error!("Failed to store received transaction: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    if let Some(sender) = &state.tx_sender {
        if let Err(error) = sender.send(tx.clone()) {
            error!(
                "Failed to forward received transaction to consensus: {}",
                error
            );
        }
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
        if let Ok(Some(_block)) = state.storage.get_block(&hash) {
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
        if !block.is_valid() {
            warn!(
                "Rejected invalid block response from peer: {}",
                hex::encode(block.hash())
            );
            return Err(StatusCode::BAD_REQUEST);
        }

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
    use axum::body::{self, Body};
    use axum::http::{Method, Request, StatusCode};
    use ippan_storage::MemoryStorage;
    use std::convert::TryFrom;
    use std::sync::atomic::AtomicUsize;
    use tokio::sync::mpsc;
    use tower::Service;

    use ed25519_dalek::SigningKey;

    #[tokio::test]
    async fn test_health_check() {
        let storage: Arc<dyn Storage + Send + Sync> = Arc::new(MemoryStorage::new());
        let state = AppState {
            storage,
            start_time: std::time::Instant::now(),
            peer_count: Arc::new(AtomicUsize::new(0)),
            p2p_network: None,
            tx_sender: None,
        };

        let mut app = create_router(state);
        let response = Service::call(
            &mut app,
            Request::builder()
                .method(Method::GET)
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let rpc_response: RpcResponse<HealthResponse> = serde_json::from_slice(&body).unwrap();
        assert!(rpc_response.success);
        assert_eq!(rpc_response.data.unwrap().status, "healthy");
    }

    #[tokio::test]
    async fn test_get_time() {
        let storage: Arc<dyn Storage + Send + Sync> = Arc::new(MemoryStorage::new());
        let state = AppState {
            storage,
            start_time: std::time::Instant::now(),
            peer_count: Arc::new(AtomicUsize::new(0)),
            p2p_network: None,
            tx_sender: None,
        };

        let mut app = create_router(state);
        let response = Service::call(
            &mut app,
            Request::builder()
                .method(Method::GET)
                .uri("/time")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let rpc_response: RpcResponse<GetTimeResponse> = serde_json::from_slice(&body).unwrap();
        assert!(rpc_response.success);
        assert!(rpc_response.data.unwrap().time_us > 0);
    }

    #[tokio::test]
    async fn test_submit_transaction_flow() {
        let storage: Arc<dyn Storage + Send + Sync> = Arc::new(MemoryStorage::new());
        let (tx_sender, mut tx_receiver) = mpsc::unbounded_channel();

        let state = AppState {
            storage: storage.clone(),
            start_time: std::time::Instant::now(),
            peer_count: Arc::new(AtomicUsize::new(0)),
            p2p_network: None,
            tx_sender: Some(tx_sender),
        };

        let mut app = create_router(state);

        let secret_bytes = [7u8; 32];
        let signing_key = SigningKey::try_from(&secret_bytes[..]).unwrap();
        let public_key = signing_key.verifying_key().to_bytes();

        let mut tx = Transaction::new(public_key, [9u8; 32], 500, 0);
        tx.sign(&secret_bytes).unwrap();

        let body_bytes = serde_json::to_vec(&tx).unwrap();

        let response = Service::call(
            &mut app,
            Request::builder()
                .method(Method::POST)
                .uri("/tx")
                .header("content-type", "application/json")
                .body(Body::from(body_bytes))
                .unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let rpc_response: RpcResponse<SubmitTransactionResponse> =
            serde_json::from_slice(&body).unwrap();
        assert!(rpc_response.success);
        assert_eq!(rpc_response.data.unwrap().tx_hash, hex::encode(tx.id));

        let stored = storage.get_transaction(&tx.id).unwrap();
        assert!(stored.is_some());

        let forwarded = tx_receiver.try_recv().unwrap();
        assert_eq!(forwarded.id, tx.id);
    }
}
