use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use ippan_p2p::{HttpP2PNetwork, NetworkMessage};
use ippan_storage::Storage;
use ippan_types::{ippan_time_now, Block, Transaction};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{info, warn};

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

/// Node status response
#[derive(Debug, Serialize, Deserialize)]
pub struct NodeStatusResponse {
    pub node_id: String,
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

#[derive(Clone)]
pub struct AppState {
    pub node_id: String,
    pub storage: Arc<dyn Storage + Send + Sync>,
    pub start_time: std::time::Instant,
    pub peer_count: Arc<AtomicUsize>,
    pub p2p_network: Option<Arc<HttpP2PNetwork>>,
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
    let peer_count = state.peer_count.load(Ordering::Relaxed);

    let response = NodeStatusResponse {
        node_id: state.node_id.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        latest_height,
        uptime_seconds: uptime,
        peer_count,
    };

    Ok(Json(RpcResponse::success(response)))
}

/// Get current IPPAN time
async fn get_time() -> Json<RpcResponse<HealthResponse>> {
    let response = HealthResponse {
        status: "ok".to_string(),
        timestamp: ippan_time_now(),
    };
    Json(RpcResponse::success(response))
}

/// Submit a transaction via RPC
async fn submit_transaction(
    State(state): State<AppState>,
    Json(mut tx): Json<Transaction>,
) -> Result<Json<RpcResponse<SubmitTransactionResponse>>, StatusCode> {
    tx.refresh_id();

    if !tx.is_valid() {
        return Err(StatusCode::BAD_REQUEST);
    }

    state
        .storage
        .store_transaction(tx.clone())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(sender) = &state.tx_sender {
        if let Err(error) = sender.send(tx.clone()) {
            warn!("Failed to forward transaction to consensus: {}", error);
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
    let hash = decode_hex_hash(&hash_str)?;

    let tx = state
        .storage
        .get_transaction(&hash)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(RpcResponse::success(tx)))
}

/// Get block by hash
async fn get_block_by_hash(
    State(state): State<AppState>,
    Path(hash_str): Path<String>,
) -> Result<Json<RpcResponse<Option<Block>>>, StatusCode> {
    let hash = decode_hex_hash(&hash_str)?;

    let block = state
        .storage
        .get_block(&hash)
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

/// Get account information
async fn get_account(
    State(state): State<AppState>,
    Path(address_str): Path<String>,
) -> Result<Json<RpcResponse<Option<AccountResponse>>>, StatusCode> {
    let address = decode_hex_hash(&address_str)?;

    let account = state
        .storage
        .get_account(&address)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = account.map(|acc| AccountResponse {
        address: hex::encode(acc.address),
        balance: acc.balance,
        nonce: acc.nonce,
    });

    Ok(Json(RpcResponse::success(response)))
}

/// Get all accounts
async fn get_all_accounts(
    State(state): State<AppState>,
) -> Result<Json<RpcResponse<Vec<AccountResponse>>>, StatusCode> {
    let accounts = state
        .storage
        .get_all_accounts()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response: Vec<AccountResponse> = accounts
        .into_iter()
        .map(|acc| AccountResponse {
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

    state
        .storage
        .store_block(block.clone())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    info!("Received block from peer: {}", hex::encode(block.hash()));
    Ok(Json(RpcResponse::success("Block received".to_string())))
}

/// P2P endpoint: Receive transaction from peer
async fn receive_transaction(
    State(state): State<AppState>,
    Json(mut tx): Json<Transaction>,
) -> Result<Json<RpcResponse<String>>, StatusCode> {
    tx.refresh_id();

    if !tx.is_valid() {
        warn!(
            "Rejected invalid transaction from peer: {}",
            hex::encode(tx.hash())
        );
        return Err(StatusCode::BAD_REQUEST);
    }

    state
        .storage
        .store_transaction(tx.clone())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(sender) = &state.tx_sender {
        if let Err(error) = sender.send(tx.clone()) {
            warn!("Failed to forward received transaction: {}", error);
        }
    }

    info!("Received transaction from peer: {}", hex::encode(tx.hash()));
    Ok(Json(RpcResponse::success(
        "Transaction received".to_string(),
    )))
}

/// P2P endpoint: Receive peer info from peer
async fn receive_peer_info(
    State(state): State<AppState>,
    Json(info): Json<NetworkMessage>,
) -> Result<Json<RpcResponse<String>>, StatusCode> {
    if let NetworkMessage::PeerInfo { addresses, .. } = info {
        if let Some(network) = &state.p2p_network {
            for address in addresses {
                if let Err(error) = network.add_peer(address.clone()).await {
                    warn!("Failed to add announced peer {}: {}", address, error);
                }
            }
        }

        Ok(Json(RpcResponse::success("Peer info received".to_string())))
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}

/// P2P endpoint: Receive peer discovery from peer
async fn receive_peer_discovery(
    State(state): State<AppState>,
    Json(discovery): Json<NetworkMessage>,
) -> Result<Json<RpcResponse<String>>, StatusCode> {
    if let NetworkMessage::PeerDiscovery { peers } = discovery {
        if let Some(network) = &state.p2p_network {
            for peer in peers {
                if let Err(error) = network.add_peer(peer.clone()).await {
                    warn!("Failed to add discovered peer {}: {}", peer, error);
                }
            }
        }

        Ok(Json(RpcResponse::success(
            "Peer discovery received".to_string(),
        )))
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}

/// P2P endpoint: Get list of peers
async fn get_peers(State(state): State<AppState>) -> Json<Vec<String>> {
    if let Some(network) = &state.p2p_network {
        let mut peers = network.get_peers();
        peers.push(network.get_announce_address());
        peers.sort();
        peers.dedup();
        Json(peers)
    } else {
        Json(Vec::new())
    }
}

/// Account information for RPC responses
#[derive(Debug, Serialize, Deserialize)]
pub struct AccountResponse {
    pub address: String,
    pub balance: u64,
    pub nonce: u64,
}

fn decode_hex_hash(input: &str) -> Result<[u8; 32], StatusCode> {
    let bytes = hex::decode(input).map_err(|_| StatusCode::BAD_REQUEST)?;
    if bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mut array = [0u8; 32];
    array.copy_from_slice(&bytes);
    Ok(array)
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
    info!("  POST /tx - Submit transaction");
    info!("  GET  /block/:hash - Get block by hash");
    info!("  GET  /block/height/:height - Get block by height");
    info!("  GET  /account/:address - Get account information");
    info!("  GET  /accounts - List accounts");

    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{self, Body};
    use axum::http::{Method, Request};
    use ippan_storage::MemoryStorage;
    use rand_core::{OsRng, RngCore};
    use std::convert::TryFrom;
    use tokio::sync::mpsc;
    use tower::Service;

    use ed25519_dalek::SigningKey;

    fn generate_transaction() -> (Transaction, [u8; 32]) {
        let mut secret = [0u8; 32];
        OsRng.fill_bytes(&mut secret);
        let signing_key = SigningKey::try_from(secret.as_slice()).unwrap();
        let public_key = signing_key.verifying_key().to_bytes();

        let mut tx = Transaction::new(public_key, [9u8; 32], 500, 0);
        tx.sign(&secret).unwrap();
        (tx, secret)
    }

    #[tokio::test]
    async fn test_health_check() {
        let storage: Arc<dyn Storage + Send + Sync> = Arc::new(MemoryStorage::new());
        let state = AppState {
            node_id: "test".to_string(),
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
            node_id: "test".to_string(),
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
        let rpc_response: RpcResponse<HealthResponse> = serde_json::from_slice(&body).unwrap();
        assert!(rpc_response.success);
        assert!(rpc_response.data.unwrap().timestamp > 0);
    }

    #[tokio::test]
    async fn test_submit_transaction_flow() {
        let storage: Arc<dyn Storage + Send + Sync> = Arc::new(MemoryStorage::new());
        let (tx_sender, mut tx_receiver) = mpsc::unbounded_channel();

        let state = AppState {
            node_id: "test".to_string(),
            storage: storage.clone(),
            start_time: std::time::Instant::now(),
            peer_count: Arc::new(AtomicUsize::new(0)),
            p2p_network: None,
            tx_sender: Some(tx_sender),
        };

        let mut app = create_router(state);

        let (tx, _secret) = generate_transaction();
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
