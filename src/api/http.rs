//! HTTP server module
//! 
//! Provides HTTP API endpoints for the IPPAN node.

use crate::node::IppanNode;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// HTTP server for IPPAN node API
pub struct HttpServer {
    node: Arc<RwLock<IppanNode>>,
    server: Option<axum::Server<axum::extract::DefaultBodyLimit, axum::routing::IntoMakeService<Router>>>,
    bind_addr: String,
}

impl HttpServer {
    pub fn new(node: Arc<RwLock<IppanNode>>) -> Self {
        Self {
            node,
            server: None,
            bind_addr: "127.0.0.1:8080".to_string(),
        }
    }

    /// Start the HTTP server
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Starting HTTP server on {}", self.bind_addr);
        
        let app = self.create_router();
        let listener = tokio::net::TcpListener::bind(&self.bind_addr).await?;
        
        self.server = Some(axum::Server::from_tcp(listener.into_std()?)?
            .serve(app.into_make_service()));
        
        log::info!("HTTP server started successfully");
        Ok(())
    }

    /// Stop the HTTP server
    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(server) = self.server.take() {
            server.await?;
        }
        Ok(())
    }

    /// Create the API router
    fn create_router(&self) -> Router {
        let node = Arc::clone(&self.node);
        
        Router::new()
            // Health and status endpoints
            .route("/health", get(Self::health_check))
            .route("/status", get(Self::get_status))
            .route("/version", get(Self::get_version))
            
            // Node information
            .route("/node/info", get(Self::get_node_info))
            .route("/node/peers", get(Self::get_peers))
            .route("/node/uptime", get(Self::get_uptime))
            
            // Consensus endpoints
            .route("/consensus/round", get(Self::get_current_round))
            .route("/consensus/blocks", get(Self::get_recent_blocks))
            .route("/consensus/validators", get(Self::get_validators))
            
            // Storage endpoints
            .route("/storage/usage", get(Self::get_storage_usage))
            .route("/storage/files", get(Self::get_stored_files))
            .route("/storage/upload", post(Self::upload_file))
            .route("/storage/download/:hash", get(Self::download_file))
            
            // Wallet endpoints
            .route("/wallet/balance", get(Self::get_balance))
            .route("/wallet/addresses", get(Self::get_addresses))
            .route("/wallet/send", post(Self::send_payment))
            .route("/wallet/transactions", get(Self::get_transactions))
            
            // DHT endpoints
            .route("/dht/keys", get(Self::get_dht_keys))
            .route("/dht/get/:key", get(Self::get_dht_value))
            .route("/dht/put", post(Self::put_dht_value))
            
            // Network endpoints
            .route("/network/stats", get(Self::get_network_stats))
            .route("/network/connect", post(Self::connect_peer))
            
            // Global Fund endpoints
            .route("/global-fund/stats", get(Self::get_global_fund_stats))
            .route("/global-fund/balance", get(Self::get_global_fund_balance))
            .route("/global-fund/distribute", post(Self::distribute_global_fund))
            
            // M2M Payment endpoints
            .route("/m2m/channels", get(Self::get_m2m_channels))
            .route("/m2m/channels", post(Self::create_m2m_channel))
            .route("/m2m/channels/:channel_id", get(Self::get_m2m_channel))
            .route("/m2m/payments", post(Self::process_m2m_payment))
            .route("/m2m/statistics", get(Self::get_m2m_statistics))
            
            .with_state(node)
    }

    // Health check endpoint
    async fn health_check() -> Json<ApiResponse<String>> {
        Json(ApiResponse::success("OK".to_string()))
    }

    // Get API version
    async fn get_version() -> Json<ApiResponse<VersionInfo>> {
        Json(ApiResponse::success(VersionInfo {
            version: env!("CARGO_PKG_VERSION").to_string(),
            api_version: "v1".to_string(),
        }))
    }

    // Get node status
    async fn get_status(State(node): State<Arc<RwLock<IppanNode>>>) -> Json<ApiResponse<NodeStatus>> {
        let node = node.read().await;
        let status = NodeStatus {
            node_id: format!("{:?}", node.node_id()),
            peer_id: node.peer_id().to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime: node.get_uptime(),
            consensus_round: node.consensus.get_current_round(),
            storage_usage: node.storage.get_usage(),
            network_peers: node.network.get_peer_count(),
            wallet_balance: node.wallet.get_balance(),
            dht_keys: node.dht.get_key_count(),
        };
        Json(ApiResponse::success(status))
    }

    // Get node information
    async fn get_node_info(State(node): State<Arc<RwLock<IppanNode>>>) -> Json<ApiResponse<NodeInfo>> {
        let node = node.read().await;
        let info = NodeInfo {
            node_id: format!("{:?}", node.node_id()),
            peer_id: node.peer_id().to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime: node.get_uptime(),
            connected_peers: node.network.get_peer_count(),
            storage_used: node.storage.get_usage().used_bytes,
            storage_capacity: node.storage.get_usage().total_bytes,
        };
        Json(ApiResponse::success(info))
    }

    // Get connected peers
    async fn get_peers(State(node): State<Arc<RwLock<IppanNode>>>) -> Json<ApiResponse<Vec<PeerInfo>>> {
        let node = node.read().await;
        let peers = node.network.get_peers().iter().map(|peer| PeerInfo {
            peer_id: peer.peer_id.to_string(),
            address: peer.address.to_string(),
            last_seen: peer.last_seen,
        }).collect();
        Json(ApiResponse::success(peers))
    }

    // Get node uptime
    async fn get_uptime(State(node): State<Arc<RwLock<IppanNode>>>) -> Json<ApiResponse<UptimeInfo>> {
        let node = node.read().await;
        let uptime = node.get_uptime();
        Json(ApiResponse::success(UptimeInfo {
            uptime_seconds: uptime.as_secs(),
            uptime_formatted: format!("{}d {}h {}m {}s", 
                uptime.as_secs() / 86400,
                (uptime.as_secs() % 86400) / 3600,
                (uptime.as_secs() % 3600) / 60,
                uptime.as_secs() % 60),
        }))
    }

    // Get current consensus round
    async fn get_current_round(State(node): State<Arc<RwLock<IppanNode>>>) -> Json<ApiResponse<ConsensusInfo>> {
        let node = node.read().await;
        let consensus = &node.consensus;
        Json(ApiResponse::success(ConsensusInfo {
            current_round: consensus.get_current_round(),
            is_validator: consensus.is_validator(),
            stake_amount: consensus.get_stake_amount(),
        }))
    }

    // Get recent blocks
    async fn get_recent_blocks(State(node): State<Arc<RwLock<IppanNode>>>) -> Json<ApiResponse<Vec<BlockInfo>>> {
        let node = node.read().await;
        let blocks = node.consensus.get_recent_blocks().iter().map(|block| BlockInfo {
            hash: format!("{:?}", block.hash()),
            round: block.round(),
            timestamp: block.timestamp(),
            transaction_count: block.transactions().len(),
        }).collect();
        Json(ApiResponse::success(blocks))
    }

    // Get validators
    async fn get_validators(State(node): State<Arc<RwLock<IppanNode>>>) -> Json<ApiResponse<Vec<ValidatorInfo>>> {
        let node = node.read().await;
        let validators = node.consensus.get_validators().iter().map(|validator| ValidatorInfo {
            node_id: format!("{:?}", validator.node_id),
            stake_amount: validator.stake_amount,
            is_active: validator.is_active,
        }).collect();
        Json(ApiResponse::success(validators))
    }

    // Get storage usage
    async fn get_storage_usage(State(node): State<Arc<RwLock<IppanNode>>>) -> Json<ApiResponse<StorageUsage>> {
        let node = node.read().await;
        Json(ApiResponse::success(node.storage.get_usage()))
    }

    // Get stored files
    async fn get_stored_files(State(node): State<Arc<RwLock<IppanNode>>>) -> Json<ApiResponse<Vec<FileInfo>>> {
        let node = node.read().await;
        let files = node.storage.get_files().iter().map(|file| FileInfo {
            hash: format!("{:?}", file.hash),
            size: file.size,
            uploaded_at: file.uploaded_at,
            shard_count: file.shard_count,
        }).collect();
        Json(ApiResponse::success(files))
    }

    // Upload file
    async fn upload_file(
        State(node): State<Arc<RwLock<IppanNode>>>,
        Json(request): Json<UploadRequest>,
    ) -> Json<ApiResponse<UploadResponse>> {
        let mut node = node.write().await;
        match node.storage.store_file(&request.data, request.filename).await {
            Ok(hash) => Json(ApiResponse::success(UploadResponse {
                hash: format!("{:?}", hash),
                size: request.data.len() as u64,
            })),
            Err(e) => Json(ApiResponse::error(format!("Upload failed: {}", e))),
        }
    }

    // Download file
    async fn download_file(
        State(node): State<Arc<RwLock<IppanNode>>>,
        Path(hash): Path<String>,
    ) -> Json<ApiResponse<DownloadResponse>> {
        let node = node.read().await;
        // Implementation would decode hash and retrieve file
        Json(ApiResponse::success(DownloadResponse {
            hash: hash,
            data: vec![], // Placeholder
            size: 0,
        }))
    }

    // Get wallet balance
    async fn get_balance(State(node): State<Arc<RwLock<IppanNode>>>) -> Json<ApiResponse<BalanceInfo>> {
        let node = node.read().await;
        Json(ApiResponse::success(BalanceInfo {
            balance: node.wallet.get_balance(),
            staked_amount: node.wallet.get_staked_amount(),
        }))
    }

    // Get wallet addresses
    async fn get_addresses(State(node): State<Arc<RwLock<IppanNode>>>) -> Json<ApiResponse<Vec<String>>> {
        let node = node.read().await;
        Json(ApiResponse::success(node.wallet.get_addresses()))
    }

    // Send payment
    async fn send_payment(
        State(node): State<Arc<RwLock<IppanNode>>>,
        Json(request): Json<PaymentRequest>,
    ) -> Json<ApiResponse<PaymentResponse>> {
        let mut node = node.write().await;
        match node.wallet.send_payment(&request.to_address, request.amount).await {
            Ok(tx_hash) => Json(ApiResponse::success(PaymentResponse {
                transaction_hash: format!("{:?}", tx_hash),
                amount: request.amount,
            })),
            Err(e) => Json(ApiResponse::error(format!("Payment failed: {}", e))),
        }
    }

    // Get transactions
    async fn get_transactions(State(node): State<Arc<RwLock<IppanNode>>>) -> Json<ApiResponse<Vec<TransactionInfo>>> {
        let node = node.read().await;
        let transactions = node.wallet.get_transactions().iter().map(|tx| TransactionInfo {
            hash: format!("{:?}", tx.hash),
            amount: tx.amount,
            to_address: tx.to_address.clone(),
            timestamp: tx.timestamp,
        }).collect();
        Json(ApiResponse::success(transactions))
    }

    // Get DHT keys
    async fn get_dht_keys(State(node): State<Arc<RwLock<IppanNode>>>) -> Json<ApiResponse<Vec<String>>> {
        let node = node.read().await;
        Json(ApiResponse::success(node.dht.get_keys()))
    }

    // Get DHT value
    async fn get_dht_value(
        State(node): State<Arc<RwLock<IppanNode>>>,
        Path(key): Path<String>,
    ) -> Json<ApiResponse<DhtValue>> {
        let node = node.read().await;
        match node.dht.get(&key) {
            Some(value) => Json(ApiResponse::success(DhtValue {
                key: key,
                value: value.clone(),
            })),
            None => Json(ApiResponse::error("Key not found".to_string())),
        }
    }

    // Put DHT value
    async fn put_dht_value(
        State(node): State<Arc<RwLock<IppanNode>>>,
        Json(request): Json<DhtPutRequest>,
    ) -> Json<ApiResponse<String>> {
        let mut node = node.write().await;
        match node.dht.put(&request.key, &request.value).await {
            Ok(_) => Json(ApiResponse::success("Value stored successfully".to_string())),
            Err(e) => Json(ApiResponse::error(format!("Failed to store value: {}", e))),
        }
    }

    // Get network stats
    async fn get_network_stats(State(node): State<Arc<RwLock<IppanNode>>>) -> Json<ApiResponse<NetworkStats>> {
        let node = node.read().await;
        Json(ApiResponse::success(NetworkStats {
            peer_count: node.network.get_peer_count(),
            total_nodes: node.network.get_total_nodes(),
            active_nodes: node.network.get_active_nodes(),
        }))
    }

    // Connect to a peer
    async fn connect_peer(
        State(node): State<Arc<RwLock<IppanNode>>>,
        Json(request): Json<ConnectRequest>,
    ) -> Json<ApiResponse<String>> {
        let mut node = node.write().await;
        match node.network.connect_peer(&request.address).await {
            Ok(_) => Json(ApiResponse::success("Connected successfully".to_string())),
            Err(e) => Json(ApiResponse::error(format!("Failed to connect: {}", e))),
        }
    }

    // Global Fund endpoints
    async fn get_global_fund_stats(State(node): State<Arc<RwLock<IppanNode>>>) -> Json<ApiResponse<GlobalFundStats>> {
        let node = node.read().await;
        match node.get_global_fund_stats().await {
            Ok(stats) => Json(ApiResponse::success(GlobalFundStats {
                total_funds_ever: stats.total_funds_ever,
                total_distributed: stats.total_distributed,
                current_balance: stats.current_balance,
                total_distributions: stats.total_distributions,
                total_nodes_rewarded: stats.total_nodes_rewarded,
                average_distribution: stats.average_distribution,
            })),
            Err(e) => Json(ApiResponse::error(format!("Failed to get global fund stats: {}", e))),
        }
    }

    async fn get_global_fund_balance(State(node): State<Arc<RwLock<IppanNode>>>) -> Json<ApiResponse<u64>> {
        let node = node.read().await;
        match node.get_global_fund_balance().await {
            Ok(balance) => Json(ApiResponse::success(balance)),
            Err(e) => Json(ApiResponse::error(format!("Failed to get global fund balance: {}", e))),
        }
    }

    async fn distribute_global_fund(State(node): State<Arc<RwLock<IppanNode>>>) -> Json<ApiResponse<GlobalFundDistribution>> {
        let node = node.read().await;
        match node.perform_weekly_distribution().await {
            Ok(distribution) => Json(ApiResponse::success(GlobalFundDistribution {
                week: distribution.week,
                total_distributed: distribution.total_distributed,
                eligible_nodes: distribution.eligible_nodes,
                timestamp: distribution.timestamp,
                node_rewards: distribution.node_rewards,
            })),
            Err(e) => Json(ApiResponse::error(format!("Failed to distribute global fund: {}", e))),
        }
    }

    // M2M Payment endpoints
    async fn get_m2m_channels(State(node): State<Arc<RwLock<IppanNode>>>) -> Json<ApiResponse<Vec<M2MChannelInfo>>> {
        let node = node.read().await;
        // For now, return empty list - in a real implementation, you'd get channels for the current node
        Json(ApiResponse::success(Vec::new()))
    }

    async fn create_m2m_channel(
        State(node): State<Arc<RwLock<IppanNode>>>,
        Json(request): Json<CreateM2MChannelRequest>,
    ) -> Json<ApiResponse<M2MChannelInfo>> {
        let node = node.read().await;
        match node.create_m2m_payment_channel(
            request.sender,
            request.recipient,
            request.deposit_amount,
            request.timeout_hours,
        ).await {
            Ok(channel) => Json(ApiResponse::success(M2MChannelInfo {
                channel_id: channel.channel_id,
                sender: channel.sender,
                recipient: channel.recipient,
                total_deposit: channel.total_deposit,
                available_balance: channel.available_balance,
                state: format!("{:?}", channel.state),
                created_at: channel.created_at,
                timeout: channel.timeout,
            })),
            Err(e) => Json(ApiResponse::error(format!("Failed to create M2M channel: {}", e))),
        }
    }

    async fn get_m2m_channel(
        State(node): State<Arc<RwLock<IppanNode>>>,
        Path(channel_id): Path<String>,
    ) -> Json<ApiResponse<M2MChannelInfo>> {
        let node = node.read().await;
        match node.get_m2m_payment_channel(&channel_id).await {
            Ok(Some(channel)) => Json(ApiResponse::success(M2MChannelInfo {
                channel_id: channel.channel_id,
                sender: channel.sender,
                recipient: channel.recipient,
                total_deposit: channel.total_deposit,
                available_balance: channel.available_balance,
                state: format!("{:?}", channel.state),
                created_at: channel.created_at,
                timeout: channel.timeout,
            })),
            Ok(None) => Json(ApiResponse::error("Channel not found".to_string())),
            Err(e) => Json(ApiResponse::error(format!("Failed to get M2M channel: {}", e))),
        }
    }

    async fn process_m2m_payment(
        State(node): State<Arc<RwLock<IppanNode>>>,
        Json(request): Json<M2MPaymentRequest>,
    ) -> Json<ApiResponse<M2MPaymentInfo>> {
        let node = node.read().await;
        match node.process_m2m_micro_payment(
            &request.channel_id,
            request.amount,
            request.tx_type,
        ).await {
            Ok(tx) => Json(ApiResponse::success(M2MPaymentInfo {
                tx_id: tx.tx_id,
                channel_id: tx.channel_id,
                amount: tx.amount,
                fee_amount: tx.fee_amount,
                timestamp: tx.timestamp,
                tx_type: format!("{:?}", tx.tx_type),
            })),
            Err(e) => Json(ApiResponse::error(format!("Failed to process M2M payment: {}", e))),
        }
    }

    async fn get_m2m_statistics(State(node): State<Arc<RwLock<IppanNode>>>) -> Json<ApiResponse<M2MStatistics>> {
        let node = node.read().await;
        match node.get_m2m_statistics().await {
            Ok(stats) => Json(ApiResponse::success(M2MStatistics {
                total_channels: stats.total_channels,
                open_channels: stats.open_channels,
                total_transactions: stats.total_transactions,
                total_volume: stats.total_volume,
                total_fees: stats.total_fees,
                average_transaction_size: stats.average_transaction_size,
            })),
            Err(e) => Json(ApiResponse::error(format!("Failed to get M2M statistics: {}", e))),
        }
    }
}

// API Response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: u64,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: chrono::Utc::now().timestamp() as u64,
        }
    }
    
    pub fn error(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            timestamp: chrono::Utc::now().timestamp() as u64,
        }
    }
}

// Request/Response types
#[derive(Debug, Deserialize)]
pub struct UploadRequest {
    pub filename: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub hash: String,
    pub size: u64,
}

#[derive(Debug, Serialize)]
pub struct DownloadResponse {
    pub hash: String,
    pub data: Vec<u8>,
    pub size: u64,
}

#[derive(Debug, Deserialize)]
pub struct PaymentRequest {
    pub to_address: String,
    pub amount: u64,
}

#[derive(Debug, Serialize)]
pub struct PaymentResponse {
    pub transaction_hash: String,
    pub amount: u64,
}

#[derive(Debug, Deserialize)]
pub struct DhtPutRequest {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Serialize)]
pub struct DhtValue {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct ConnectRequest {
    pub address: String,
}

// Info types
#[derive(Debug, Serialize)]
pub struct VersionInfo {
    pub version: String,
    pub api_version: String,
}

#[derive(Debug, Serialize)]
pub struct NodeStatus {
    pub node_id: String,
    pub peer_id: String,
    pub version: String,
    pub uptime: std::time::Duration,
    pub consensus_round: u64,
    pub storage_usage: crate::api::StorageUsage,
    pub network_peers: usize,
    pub wallet_balance: u64,
    pub dht_keys: usize,
}

#[derive(Debug, Serialize)]
pub struct NodeInfo {
    pub node_id: String,
    pub peer_id: String,
    pub version: String,
    pub uptime: std::time::Duration,
    pub connected_peers: usize,
    pub storage_used: u64,
    pub storage_capacity: u64,
}

#[derive(Debug, Serialize)]
pub struct PeerInfo {
    pub peer_id: String,
    pub address: String,
    pub last_seen: std::time::SystemTime,
}

#[derive(Debug, Serialize)]
pub struct UptimeInfo {
    pub uptime_seconds: u64,
    pub uptime_formatted: String,
}

#[derive(Debug, Serialize)]
pub struct ConsensusInfo {
    pub current_round: u64,
    pub is_validator: bool,
    pub stake_amount: u64,
}

#[derive(Debug, Serialize)]
pub struct BlockInfo {
    pub hash: String,
    pub round: u64,
    pub timestamp: u64,
    pub transaction_count: usize,
}

#[derive(Debug, Serialize)]
pub struct ValidatorInfo {
    pub node_id: String,
    pub stake_amount: u64,
    pub is_active: bool,
}

#[derive(Debug, Serialize)]
pub struct FileInfo {
    pub hash: String,
    pub size: u64,
    pub uploaded_at: std::time::SystemTime,
    pub shard_count: usize,
}

#[derive(Debug, Serialize)]
pub struct BalanceInfo {
    pub balance: u64,
    pub staked_amount: u64,
}

#[derive(Debug, Serialize)]
pub struct TransactionInfo {
    pub hash: String,
    pub amount: u64,
    pub to_address: String,
    pub timestamp: u64,
}

#[derive(Debug, Serialize)]
pub struct NetworkStats {
    pub peer_count: usize,
    pub total_nodes: usize,
    pub active_nodes: usize,
}

// Global Fund data structures
#[derive(Debug, Serialize)]
pub struct GlobalFundStats {
    pub total_funds_ever: u64,
    pub total_distributed: u64,
    pub current_balance: u64,
    pub total_distributions: u32,
    pub total_nodes_rewarded: u32,
    pub average_distribution: u64,
}

#[derive(Debug, Serialize)]
pub struct GlobalFundDistribution {
    pub week: u64,
    pub total_distributed: u64,
    pub eligible_nodes: u32,
    pub timestamp: u64,
    pub node_rewards: std::collections::HashMap<String, u64>,
}

// M2M Payment data structures
#[derive(Debug, Serialize)]
pub struct M2MChannelInfo {
    pub channel_id: String,
    pub sender: String,
    pub recipient: String,
    pub total_deposit: u64,
    pub available_balance: u64,
    pub state: String,
    pub created_at: u64,
    pub timeout: u64,
}

#[derive(Debug, Serialize)]
pub struct M2MPaymentInfo {
    pub tx_id: String,
    pub channel_id: String,
    pub amount: u64,
    pub fee_amount: u64,
    pub timestamp: u64,
    pub tx_type: String,
}

#[derive(Debug, Serialize)]
pub struct M2MStatistics {
    pub total_channels: usize,
    pub open_channels: usize,
    pub total_transactions: usize,
    pub total_volume: u64,
    pub total_fees: u64,
    pub average_transaction_size: u64,
}

#[derive(Debug, Deserialize)]
pub struct CreateM2MChannelRequest {
    pub sender: String,
    pub recipient: String,
    pub deposit_amount: u64,
    pub timeout_hours: u64,
}

#[derive(Debug, Deserialize)]
pub struct M2MPaymentRequest {
    pub channel_id: String,
    pub amount: u64,
    pub tx_type: crate::wallet::m2m_payments::MicroTransactionType,
}
