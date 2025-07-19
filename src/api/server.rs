use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use axum::{
    routing::{get, post, put, delete},
    Router, Json, extract::{Path, Query, State},
    http::{StatusCode, HeaderMap},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};
use sha2::{Sha256, Digest};
use hmac::{Hmac, Mac};
use base64::{Engine as _, engine::general_purpose};
use tower_http::{
    cors::{CorsLayer, Any},
    limit::RequestBodyLimitLayer,
    trace::TraceLayer,
};
use std::net::SocketAddr;

/// API Server configuration
#[derive(Debug, Clone)]
pub struct ApiServerConfig {
    pub host: String,
    pub port: u16,
    pub max_request_size: usize,
    pub rate_limit_requests: u32,
    pub rate_limit_window_seconds: u64,
    pub enable_cors: bool,
    pub enable_metrics: bool,
    pub api_key: Option<String>,
    pub jwt_secret: Option<String>,
}

/// API Response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: u64,
    pub request_id: String,
}

/// API Error types
#[derive(Debug, Serialize)]
pub enum ApiError {
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    NotFound(String),
    InternalServerError(String),
    RateLimitExceeded,
    ValidationError(Vec<String>),
}

/// Rate limiting information
#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    pub requests: u32,
    pub window_start: u64,
    pub limit: u32,
    pub window_seconds: u64,
}

/// API Server State
pub struct ApiServerState {
    pub config: ApiServerConfig,
    pub rate_limits: Arc<RwLock<HashMap<String, RateLimitInfo>>>,
    pub metrics: Arc<RwLock<ApiMetrics>>,
    pub storage_manager: Option<Arc<DistributedStorageManager>>,
    pub network_manager: Option<Arc<NetworkProtocolManager>>,
}

/// API Metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_response_time_ms: f64,
    pub requests_per_minute: f64,
    pub active_connections: u32,
    pub last_request_time: u64,
}

/// Blockchain status response
#[derive(Debug, Serialize)]
pub struct BlockchainStatus {
    pub node_id: String,
    pub status: String,
    pub current_block: u64,
    pub total_transactions: u64,
    pub network_peers: u32,
    pub uptime_seconds: u64,
    pub version: String,
}

/// Transaction submission request
#[derive(Debug, Deserialize)]
pub struct TransactionRequest {
    pub from_address: String,
    pub to_address: String,
    pub amount: u64,
    pub fee: u64,
    pub data: Option<String>,
    pub signature: String,
}

/// Transaction response
#[derive(Debug, Serialize)]
pub struct TransactionResponse {
    pub transaction_hash: String,
    pub status: String,
    pub block_number: Option<u64>,
    pub timestamp: u64,
    pub fee: u64,
}

/// Block information
#[derive(Debug, Serialize)]
pub struct BlockInfo {
    pub block_hash: String,
    pub block_number: u64,
    pub timestamp: u64,
    pub transactions_count: u32,
    pub total_amount: u64,
    pub miner_address: String,
    pub difficulty: u64,
    pub parent_hash: String,
}

/// Network peer information
#[derive(Debug, Serialize)]
pub struct PeerInfo {
    pub node_id: String,
    pub address: String,
    pub status: String,
    pub last_seen: u64,
    pub latency_ms: Option<u64>,
    pub trust_score: f64,
}

/// Storage node information
#[derive(Debug, Serialize)]
pub struct StorageNodeInfo {
    pub node_id: String,
    pub address: String,
    pub status: String,
    pub capacity_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub replication_factor: u32,
}

/// API Server
pub struct ApiServer {
    state: Arc<ApiServerState>,
    router: Router,
}

impl ApiServer {
    /// Create new API server
    pub fn new(config: ApiServerConfig) -> Self {
        let state = Arc::new(ApiServerState {
            config: config.clone(),
            rate_limits: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(ApiMetrics::default())),
            storage_manager: None,
            network_manager: None,
        });

        let router = Self::create_router(state.clone());

        ApiServer { state, router }
    }

    /// Create router with all endpoints
    fn create_router(state: Arc<ApiServerState>) -> Router {
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        Router::new()
            // Health and status endpoints
            .route("/health", get(Self::health_check))
            .route("/status", get(Self::blockchain_status))
            .route("/metrics", get(Self::get_metrics))
            
            // Blockchain endpoints
            .route("/blocks/:block_number", get(Self::get_block))
            .route("/blocks/latest", get(Self::get_latest_block))
            .route("/transactions", post(Self::submit_transaction))
            .route("/transactions/:tx_hash", get(Self::get_transaction))
            .route("/addresses/:address/balance", get(Self::get_balance))
            .route("/addresses/:address/transactions", get(Self::get_address_transactions))
            
            // Network endpoints
            .route("/peers", get(Self::get_peers))
            .route("/peers/:node_id", get(Self::get_peer))
            .route("/network/stats", get(Self::get_network_stats))
            
            // Storage endpoints
            .route("/storage/nodes", get(Self::get_storage_nodes))
            .route("/storage/data/:data_id", get(Self::get_storage_data))
            .route("/storage/data", post(Self::store_data))
            .route("/storage/data/:data_id", delete(Self::delete_storage_data))
            .route("/storage/metrics", get(Self::get_storage_metrics))
            
            // Consensus endpoints
            .route("/consensus/validators", get(Self::get_validators))
            .route("/consensus/rounds/:round_number", get(Self::get_consensus_round))
            .route("/consensus/stats", get(Self::get_consensus_stats))
            
            // Admin endpoints
            .route("/admin/config", get(Self::get_config))
            .route("/admin/config", put(Self::update_config))
            .route("/admin/restart", post(Self::restart_node))
            .route("/admin/logs", get(Self::get_logs))
            
            .layer(cors)
            .layer(RequestBodyLimitLayer::new(10 * 1024 * 1024)) // 10MB limit
            .layer(TraceLayer::new_for_http())
            .with_state(state)
    }

    /// Start the API server
    pub async fn start(&self) -> Result<(), String> {
        let addr = format!("{}:{}", self.state.config.host, self.state.config.port)
            .parse::<SocketAddr>()
            .map_err(|e| format!("Invalid address: {}", e))?;

        println!("Starting API server on {}", addr);

        axum::Server::bind(&addr)
            .serve(self.router.clone().into_make_service())
            .await
            .map_err(|e| format!("Server error: {}", e))
    }

    /// Health check endpoint
    async fn health_check(State(state): State<Arc<ApiServerState>>) -> impl IntoResponse {
        let response = ApiResponse {
            success: true,
            data: Some(json!({
                "status": "healthy",
                "timestamp": SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                "version": "1.0.0"
            })),
            error: None,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            request_id: Self::generate_request_id(),
        };

        (StatusCode::OK, Json(response))
    }

    /// Blockchain status endpoint
    async fn blockchain_status(State(state): State<Arc<ApiServerState>>) -> impl IntoResponse {
        let status = BlockchainStatus {
            node_id: "test_node".to_string(),
            status: "running".to_string(),
            current_block: 12345,
            total_transactions: 1000000,
            network_peers: 50,
            uptime_seconds: 86400,
            version: "1.0.0".to_string(),
        };

        let response = ApiResponse {
            success: true,
            data: Some(status),
            error: None,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            request_id: Self::generate_request_id(),
        };

        (StatusCode::OK, Json(response))
    }

    /// Get metrics endpoint
    async fn get_metrics(State(state): State<Arc<ApiServerState>>) -> impl IntoResponse {
        let metrics = state.metrics.read().await.clone();

        let response = ApiResponse {
            success: true,
            data: Some(metrics),
            error: None,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            request_id: Self::generate_request_id(),
        };

        (StatusCode::OK, Json(response))
    }

    /// Get block by number
    async fn get_block(
        State(state): State<Arc<ApiServerState>>,
        Path(block_number): Path<u64>,
    ) -> impl IntoResponse {
        let block_info = BlockInfo {
            block_hash: format!("block_hash_{}", block_number),
            block_number,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            transactions_count: 100,
            total_amount: 1000000,
            miner_address: "miner_address".to_string(),
            difficulty: 1000000,
            parent_hash: format!("parent_hash_{}", block_number - 1),
        };

        let response = ApiResponse {
            success: true,
            data: Some(block_info),
            error: None,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            request_id: Self::generate_request_id(),
        };

        (StatusCode::OK, Json(response))
    }

    /// Get latest block
    async fn get_latest_block(State(state): State<Arc<ApiServerState>>) -> impl IntoResponse {
        let block_info = BlockInfo {
            block_hash: "latest_block_hash".to_string(),
            block_number: 12345,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            transactions_count: 150,
            total_amount: 1500000,
            miner_address: "latest_miner".to_string(),
            difficulty: 1200000,
            parent_hash: "previous_block_hash".to_string(),
        };

        let response = ApiResponse {
            success: true,
            data: Some(block_info),
            error: None,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            request_id: Self::generate_request_id(),
        };

        (StatusCode::OK, Json(response))
    }

    /// Submit transaction
    async fn submit_transaction(
        State(state): State<Arc<ApiServerState>>,
        Json(transaction): Json<TransactionRequest>,
    ) -> impl IntoResponse {
        // Validate transaction
        if transaction.amount == 0 {
            let response = ApiResponse::<TransactionResponse> {
                success: false,
                data: None,
                error: Some("Invalid transaction amount".to_string()),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                request_id: Self::generate_request_id(),
            };
            return (StatusCode::BAD_REQUEST, Json(response));
        }

        let transaction_response = TransactionResponse {
            transaction_hash: format!("tx_hash_{}", SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()),
            status: "pending".to_string(),
            block_number: None,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            fee: transaction.fee,
        };

        let response = ApiResponse {
            success: true,
            data: Some(transaction_response),
            error: None,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            request_id: Self::generate_request_id(),
        };

        (StatusCode::OK, Json(response))
    }

    /// Get transaction by hash
    async fn get_transaction(
        State(state): State<Arc<ApiServerState>>,
        Path(tx_hash): Path<String>,
    ) -> impl IntoResponse {
        let transaction_response = TransactionResponse {
            transaction_hash: tx_hash.clone(),
            status: "confirmed".to_string(),
            block_number: Some(12345),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            fee: 1000,
        };

        let response = ApiResponse {
            success: true,
            data: Some(transaction_response),
            error: None,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            request_id: Self::generate_request_id(),
        };

        (StatusCode::OK, Json(response))
    }

    /// Get address balance
    async fn get_balance(
        State(state): State<Arc<ApiServerState>>,
        Path(address): Path<String>,
    ) -> impl IntoResponse {
        let balance_data = json!({
            "address": address,
            "balance": 1000000,
            "currency": "IPPAN",
            "last_updated": SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        });

        let response = ApiResponse {
            success: true,
            data: Some(balance_data),
            error: None,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            request_id: Self::generate_request_id(),
        };

        (StatusCode::OK, Json(response))
    }

    /// Get address transactions
    async fn get_address_transactions(
        State(state): State<Arc<ApiServerState>>,
        Path(address): Path<String>,
        Query(params): Query<HashMap<String, String>>,
    ) -> impl IntoResponse {
        let limit = params.get("limit").and_then(|s| s.parse::<u32>().ok()).unwrap_or(10);
        let offset = params.get("offset").and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);

        let transactions = vec![
            TransactionResponse {
                transaction_hash: "tx_hash_1".to_string(),
                status: "confirmed".to_string(),
                block_number: Some(12344),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() - 3600,
                fee: 1000,
            },
            TransactionResponse {
                transaction_hash: "tx_hash_2".to_string(),
                status: "confirmed".to_string(),
                block_number: Some(12345),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                fee: 1500,
            },
        ];

        let response = ApiResponse {
            success: true,
            data: Some(json!({
                "address": address,
                "transactions": transactions,
                "total": 2,
                "limit": limit,
                "offset": offset,
            })),
            error: None,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            request_id: Self::generate_request_id(),
        };

        (StatusCode::OK, Json(response))
    }

    /// Get network peers
    async fn get_peers(State(state): State<Arc<ApiServerState>>) -> impl IntoResponse {
        let peers = vec![
            PeerInfo {
                node_id: "peer_1".to_string(),
                address: "127.0.0.1:8081".to_string(),
                status: "connected".to_string(),
                last_seen: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                latency_ms: Some(50),
                trust_score: 0.9,
            },
            PeerInfo {
                node_id: "peer_2".to_string(),
                address: "127.0.0.1:8082".to_string(),
                status: "connected".to_string(),
                last_seen: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                latency_ms: Some(75),
                trust_score: 0.8,
            },
        ];

        let response = ApiResponse {
            success: true,
            data: Some(peers),
            error: None,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            request_id: Self::generate_request_id(),
        };

        (StatusCode::OK, Json(response))
    }

    /// Get specific peer
    async fn get_peer(
        State(state): State<Arc<ApiServerState>>,
        Path(node_id): Path<String>,
    ) -> impl IntoResponse {
        let peer = PeerInfo {
            node_id: node_id.clone(),
            address: "127.0.0.1:8081".to_string(),
            status: "connected".to_string(),
            last_seen: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            latency_ms: Some(50),
            trust_score: 0.9,
        };

        let response = ApiResponse {
            success: true,
            data: Some(peer),
            error: None,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            request_id: Self::generate_request_id(),
        };

        (StatusCode::OK, Json(response))
    }

    /// Get network statistics
    async fn get_network_stats(State(state): State<Arc<ApiServerState>>) -> impl IntoResponse {
        let stats = json!({
            "total_peers": 50,
            "connected_peers": 45,
            "queued_messages": 100,
            "avg_latency_ms": 75,
            "max_peers": 100,
            "message_timeout_ms": 30000,
        });

        let response = ApiResponse {
            success: true,
            data: Some(stats),
            error: None,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            request_id: Self::generate_request_id(),
        };

        (StatusCode::OK, Json(response))
    }

    /// Get storage nodes
    async fn get_storage_nodes(State(state): State<Arc<ApiServerState>>) -> impl IntoResponse {
        let nodes = vec![
            StorageNodeInfo {
                node_id: "storage_node_1".to_string(),
                address: "127.0.0.1:8081".to_string(),
                status: "online".to_string(),
                capacity_bytes: 1_000_000_000_000,
                used_bytes: 100_000_000_000,
                available_bytes: 900_000_000_000,
                replication_factor: 3,
            },
            StorageNodeInfo {
                node_id: "storage_node_2".to_string(),
                address: "127.0.0.1:8082".to_string(),
                status: "online".to_string(),
                capacity_bytes: 1_000_000_000_000,
                used_bytes: 150_000_000_000,
                available_bytes: 850_000_000_000,
                replication_factor: 3,
            },
        ];

        let response = ApiResponse {
            success: true,
            data: Some(nodes),
            error: None,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            request_id: Self::generate_request_id(),
        };

        (StatusCode::OK, Json(response))
    }

    /// Get storage data
    async fn get_storage_data(
        State(state): State<Arc<ApiServerState>>,
        Path(data_id): Path<String>,
    ) -> impl IntoResponse {
        let data_info = json!({
            "data_id": data_id,
            "size_bytes": 1024,
            "shard_count": 2,
            "replication_factor": 3,
            "encrypted": true,
            "created_at": SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            "data": "base64_encoded_data_here",
        });

        let response = ApiResponse {
            success: true,
            data: Some(data_info),
            error: None,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            request_id: Self::generate_request_id(),
        };

        (StatusCode::OK, Json(response))
    }

    /// Store data
    async fn store_data(
        State(state): State<Arc<ApiServerState>>,
        Json(request): Json<HashMap<String, Value>>,
    ) -> impl IntoResponse {
        let data_id = request.get("data_id")
            .and_then(|v| v.as_str())
            .unwrap_or("auto_generated_id");

        let result = json!({
            "data_id": data_id,
            "shard_ids": vec!["shard_1", "shard_2"],
            "size_bytes": 1024,
            "status": "stored",
            "timestamp": SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        });

        let response = ApiResponse {
            success: true,
            data: Some(result),
            error: None,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            request_id: Self::generate_request_id(),
        };

        (StatusCode::OK, Json(response))
    }

    /// Delete storage data
    async fn delete_storage_data(
        State(state): State<Arc<ApiServerState>>,
        Path(data_id): Path<String>,
    ) -> impl IntoResponse {
        let result = json!({
            "data_id": data_id,
            "status": "deleted",
            "timestamp": SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        });

        let response = ApiResponse {
            success: true,
            data: Some(result),
            error: None,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            request_id: Self::generate_request_id(),
        };

        (StatusCode::OK, Json(response))
    }

    /// Get storage metrics
    async fn get_storage_metrics(State(state): State<Arc<ApiServerState>>) -> impl IntoResponse {
        let metrics = json!({
            "total_capacity_bytes": 2_000_000_000_000,
            "used_capacity_bytes": 250_000_000_000,
            "available_capacity_bytes": 1_750_000_000_000,
            "total_shards": 1000,
            "replicated_shards": 950,
            "failed_shards": 5,
            "read_operations_per_sec": 150.5,
            "write_operations_per_sec": 75.2,
            "average_latency_ms": 25.0,
            "encryption_overhead_percent": 5.0,
            "compression_ratio": 0.8,
        });

        let response = ApiResponse {
            success: true,
            data: Some(metrics),
            error: None,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            request_id: Self::generate_request_id(),
        };

        (StatusCode::OK, Json(response))
    }

    /// Get validators
    async fn get_validators(State(state): State<Arc<ApiServerState>>) -> impl IntoResponse {
        let validators = json!({
            "validators": vec![
                {
                    "node_id": "validator_1",
                    "address": "127.0.0.1:8081",
                    "stake": 1000000,
                    "status": "active",
                    "performance_score": 0.95,
                },
                {
                    "node_id": "validator_2",
                    "address": "127.0.0.1:8082",
                    "stake": 800000,
                    "status": "active",
                    "performance_score": 0.88,
                }
            ],
            "total_validators": 2,
            "total_stake": 1800000,
        });

        let response = ApiResponse {
            success: true,
            data: Some(validators),
            error: None,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            request_id: Self::generate_request_id(),
        };

        (StatusCode::OK, Json(response))
    }

    /// Get consensus round
    async fn get_consensus_round(
        State(state): State<Arc<ApiServerState>>,
        Path(round_number): Path<u64>,
    ) -> impl IntoResponse {
        let round_info = json!({
            "round_number": round_number,
            "validator_set": vec!["validator_1", "validator_2"],
            "timestamp": SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            "status": "completed",
            "block_hash": format!("block_hash_{}", round_number),
        });

        let response = ApiResponse {
            success: true,
            data: Some(round_info),
            error: None,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            request_id: Self::generate_request_id(),
        };

        (StatusCode::OK, Json(response))
    }

    /// Get consensus statistics
    async fn get_consensus_stats(State(state): State<Arc<ApiServerState>>) -> impl IntoResponse {
        let stats = json!({
            "current_round": 12345,
            "total_rounds": 12345,
            "average_round_time_ms": 5000,
            "active_validators": 10,
            "total_validators": 15,
            "consensus_rate": 0.95,
        });

        let response = ApiResponse {
            success: true,
            data: Some(stats),
            error: None,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            request_id: Self::generate_request_id(),
        };

        (StatusCode::OK, Json(response))
    }

    /// Get configuration
    async fn get_config(State(state): State<Arc<ApiServerState>>) -> impl IntoResponse {
        let config = json!({
            "host": state.config.host,
            "port": state.config.port,
            "max_request_size": state.config.max_request_size,
            "rate_limit_requests": state.config.rate_limit_requests,
            "rate_limit_window_seconds": state.config.rate_limit_window_seconds,
            "enable_cors": state.config.enable_cors,
            "enable_metrics": state.config.enable_metrics,
        });

        let response = ApiResponse {
            success: true,
            data: Some(config),
            error: None,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            request_id: Self::generate_request_id(),
        };

        (StatusCode::OK, Json(response))
    }

    /// Update configuration
    async fn update_config(
        State(state): State<Arc<ApiServerState>>,
        Json(config): Json<HashMap<String, Value>>,
    ) -> impl IntoResponse {
        let result = json!({
            "status": "updated",
            "timestamp": SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        });

        let response = ApiResponse {
            success: true,
            data: Some(result),
            error: None,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            request_id: Self::generate_request_id(),
        };

        (StatusCode::OK, Json(response))
    }

    /// Restart node
    async fn restart_node(State(state): State<Arc<ApiServerState>>) -> impl IntoResponse {
        let result = json!({
            "status": "restarting",
            "timestamp": SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        });

        let response = ApiResponse {
            success: true,
            data: Some(result),
            error: None,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            request_id: Self::generate_request_id(),
        };

        (StatusCode::OK, Json(response))
    }

    /// Get logs
    async fn get_logs(
        State(state): State<Arc<ApiServerState>>,
        Query(params): Query<HashMap<String, String>>,
    ) -> impl IntoResponse {
        let level = params.get("level").unwrap_or(&"info".to_string());
        let limit = params.get("limit").and_then(|s| s.parse::<u32>().ok()).unwrap_or(100);

        let logs = vec![
            json!({
                "timestamp": SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                "level": level,
                "message": "API request processed",
                "request_id": "req_123",
            }),
            json!({
                "timestamp": SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() - 60,
                "level": "info",
                "message": "Node started successfully",
                "request_id": null,
            }),
        ];

        let response = ApiResponse {
            success: true,
            data: Some(json!({
                "logs": logs,
                "total": logs.len(),
                "level": level,
                "limit": limit,
            })),
            error: None,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            request_id: Self::generate_request_id(),
        };

        (StatusCode::OK, Json(response))
    }

    /// Generate request ID
    fn generate_request_id() -> String {
        let mut hasher = Sha256::new();
        hasher.update(format!("{}", SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()));
        format!("req_{:x}", hasher.finalize())
    }

    /// Check rate limiting
    async fn check_rate_limit(
        state: &Arc<ApiServerState>,
        client_id: &str,
    ) -> Result<(), ApiError> {
        let mut rate_limits = state.rate_limits.write().await;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if let Some(rate_limit) = rate_limits.get_mut(client_id) {
            if now - rate_limit.window_start >= rate_limit.window_seconds {
                // Reset window
                rate_limit.requests = 1;
                rate_limit.window_start = now;
            } else if rate_limit.requests >= rate_limit.limit {
                return Err(ApiError::RateLimitExceeded);
            } else {
                rate_limit.requests += 1;
            }
        } else {
            // Create new rate limit entry
            rate_limits.insert(client_id.to_string(), RateLimitInfo {
                requests: 1,
                window_start: now,
                limit: state.config.rate_limit_requests,
                window_seconds: state.config.rate_limit_window_seconds,
            });
        }

        Ok(())
    }

    /// Update metrics
    async fn update_metrics(state: &Arc<ApiServerState>, success: bool, response_time_ms: u64) {
        let mut metrics = state.metrics.write().await;
        metrics.total_requests += 1;
        
        if success {
            metrics.successful_requests += 1;
        } else {
            metrics.failed_requests += 1;
        }

        // Update average response time
        let total_time = metrics.average_response_time_ms * (metrics.total_requests - 1) as f64;
        metrics.average_response_time_ms = (total_time + response_time_ms as f64) / metrics.total_requests as f64;

        metrics.last_request_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }
}

impl Default for ApiServerConfig {
    fn default() -> Self {
        ApiServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            max_request_size: 10 * 1024 * 1024, // 10MB
            rate_limit_requests: 100,
            rate_limit_window_seconds: 60,
            enable_cors: true,
            enable_metrics: true,
            api_key: None,
            jwt_secret: None,
        }
    }
}

impl Default for ApiMetrics {
    fn default() -> Self {
        ApiMetrics {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            average_response_time_ms: 0.0,
            requests_per_minute: 0.0,
            active_connections: 0,
            last_request_time: 0,
        }
    }
} 