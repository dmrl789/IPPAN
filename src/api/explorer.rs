//! Explorer interface module
//! 
//! Provides blockchain explorer interface for the IPPAN node.

use crate::{api::ApiState, error::IppanError, Result};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::{error, info, warn};
use crate::node::IppanNode;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use tokio::sync::RwLock;

/// Explorer interface
pub struct ExplorerInterface {
    /// Explorer configuration
    config: crate::api::ApiConfig,
    /// API state
    state: ApiState,
    /// Request counter
    request_count: Arc<AtomicU64>,
    /// Explorer handle
    explorer_handle: Option<tokio::task::JoinHandle<()>>,
}

impl ExplorerInterface {
    /// Create a new explorer interface
    pub fn new(config: crate::api::ApiConfig, state: ApiState) -> Self {
        Self {
            config,
            state,
            request_count: Arc::new(AtomicU64::new(0)),
            explorer_handle: None,
        }
    }
    
    /// Start the explorer interface
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting explorer interface");
        
        // Start explorer in background task
        let state = self.state.clone();
        let request_count = self.request_count.clone();
        
        let explorer_handle = tokio::spawn(async move {
            if let Err(e) = Self::run_explorer_loop(state, request_count).await {
                error!("Explorer error: {}", e);
            }
        });
        
        self.explorer_handle = Some(explorer_handle);
        
        info!("Explorer interface started successfully");
        Ok(())
    }
    
    /// Stop the explorer interface
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(handle) = self.explorer_handle.take() {
            handle.abort();
            if let Err(e) = handle.await {
                if !e.is_cancelled() {
                    warn!("Explorer shutdown error: {}", e);
                }
            }
        }
        
        info!("Explorer interface stopped");
        Ok(())
    }
    
    /// Run explorer loop
    async fn run_explorer_loop(state: ApiState, request_count: Arc<AtomicU64>) -> Result<()> {
        loop {
            // In a real implementation, you'd handle explorer requests
            // For now, we'll simulate explorer activity
            
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            
            // Simulate request processing
            request_count.fetch_add(1, Ordering::Relaxed);
        }
    }
    
    /// Get request count
    pub async fn get_request_count(&self) -> Result<u64> {
        Ok(self.request_count.load(Ordering::Relaxed))
    }
    
    /// Get explorer statistics
    pub async fn get_stats(&self) -> Result<ExplorerStats> {
        Ok(ExplorerStats {
            request_count: self.request_count.load(Ordering::Relaxed),
            enabled: self.config.enable_explorer,
        })
    }
}

/// Explorer statistics
#[derive(Debug, Clone)]
pub struct ExplorerStats {
    /// Total requests served
    pub request_count: u64,
    /// Explorer enabled
    pub enabled: bool,
}

/// Block information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockInfo {
    /// Block hash
    pub hash: String,
    /// Block height
    pub height: u64,
    /// Previous block hash
    pub previous_hash: String,
    /// Timestamp
    pub timestamp: u64,
    /// Transaction count
    pub transaction_count: u32,
    /// Block size
    pub size: u64,
    /// Validator
    pub validator: String,
    /// Block reward
    pub reward: u64,
}

/// Transaction information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionInfo {
    /// Transaction hash
    pub hash: String,
    /// Block hash
    pub block_hash: String,
    /// Block height
    pub block_height: u64,
    /// Transaction type
    pub tx_type: TransactionType,
    /// From address
    pub from: String,
    /// To address
    pub to: Option<String>,
    /// Amount
    pub amount: u64,
    /// Fee
    pub fee: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Status
    pub status: TransactionStatus,
}

/// Transaction type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionType {
    /// Payment transaction
    Payment,
    /// Staking transaction
    Staking,
    /// Unstaking transaction
    Unstaking,
    /// Domain registration
    DomainRegistration,
    /// Storage transaction
    Storage,
    /// Reward transaction
    Reward,
}

/// Transaction status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionStatus {
    /// Pending
    Pending,
    /// Confirmed
    Confirmed,
    /// Failed
    Failed,
}

/// Address information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressInfo {
    /// Address
    pub address: String,
    /// Balance
    pub balance: u64,
    /// Staked amount
    pub staked_amount: u64,
    /// Total received
    pub total_received: u64,
    /// Total sent
    pub total_sent: u64,
    /// Transaction count
    pub transaction_count: u64,
    /// First seen
    pub first_seen: u64,
    /// Last seen
    pub last_seen: u64,
}

/// Domain information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainInfo {
    /// Domain name
    pub name: String,
    /// Owner address
    pub owner: String,
    /// Registration date
    pub registration_date: u64,
    /// Expiration date
    pub expiration_date: u64,
    /// Registration fee
    pub registration_fee: u64,
    /// Status
    pub status: DomainStatus,
}

/// Domain status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DomainStatus {
    /// Active
    Active,
    /// Expired
    Expired,
    /// Pending renewal
    PendingRenewal,
}

/// Storage file information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageFileInfo {
    /// File hash
    pub hash: String,
    /// File name
    pub name: String,
    /// File size
    pub size: u64,
    /// Uploader address
    pub uploader: String,
    /// Upload timestamp
    pub upload_timestamp: u64,
    /// Replication factor
    pub replication_factor: u32,
    /// Storage nodes
    pub storage_nodes: Vec<String>,
    /// Status
    pub status: StorageFileStatus,
}

/// Storage file status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageFileStatus {
    /// Available
    Available,
    /// Partially available
    PartiallyAvailable,
    /// Unavailable
    Unavailable,
}

/// Explorer handler
pub struct ExplorerHandler {
    /// API state
    state: ApiState,
}

impl ExplorerHandler {
    /// Create a new explorer handler
    pub fn new(state: ApiState) -> Self {
        Self { state }
    }
    
    /// Get block information
    pub async fn get_block_info(&self, height: u64) -> Result<Option<BlockInfo>> {
        let node = self.state.node.read().await;
        
        // In a real implementation, you'd query the blockchain
        // For now, we'll return simulated data
        
        let block_info = BlockInfo {
            hash: format!("block_hash_{}", height),
            height,
            previous_hash: if height > 0 { format!("block_hash_{}", height - 1) } else { "genesis".to_string() },
            timestamp: chrono::Utc::now().timestamp() as u64,
            transaction_count: 10,
            size: 1024,
            validator: "validator_address".to_string(),
            reward: 1000000000, // 10 IPN
        };
        
        Ok(Some(block_info))
    }
    
    /// Get transaction information
    pub async fn get_transaction_info(&self, hash: &str) -> Result<Option<TransactionInfo>> {
        // In a real implementation, you'd query the blockchain
        // For now, we'll return simulated data
        
        let tx_info = TransactionInfo {
            hash: hash.to_string(),
            block_hash: "block_hash_123".to_string(),
            block_height: 123,
            tx_type: TransactionType::Payment,
            from: "sender_address".to_string(),
            to: Some("recipient_address".to_string()),
            amount: 5000000000, // 50 IPN
            fee: 10000000, // 0.1 IPN
            timestamp: chrono::Utc::now().timestamp() as u64,
            status: TransactionStatus::Confirmed,
        };
        
        Ok(Some(tx_info))
    }
    
    /// Get address information
    pub async fn get_address_info(&self, address: &str) -> Result<Option<AddressInfo>> {
        // In a real implementation, you'd query the blockchain
        // For now, we'll return simulated data
        
        let address_info = AddressInfo {
            address: address.to_string(),
            balance: 10000000000, // 100 IPN
            staked_amount: 5000000000, // 50 IPN
            total_received: 20000000000, // 200 IPN
            total_sent: 5000000000, // 50 IPN
            transaction_count: 25,
            first_seen: chrono::Utc::now().timestamp() as u64 - 86400, // 1 day ago
            last_seen: chrono::Utc::now().timestamp() as u64,
        };
        
        Ok(Some(address_info))
    }
    
    /// Get domain information
    pub async fn get_domain_info(&self, domain: &str) -> Result<Option<DomainInfo>> {
        // In a real implementation, you'd query the domain registry
        // For now, we'll return simulated data
        
        let domain_info = DomainInfo {
            name: domain.to_string(),
            owner: "owner_address".to_string(),
            registration_date: chrono::Utc::now().timestamp() as u64 - 86400 * 30, // 30 days ago
            expiration_date: chrono::Utc::now().timestamp() as u64 + 86400 * 335, // 335 days from now
            registration_fee: 1000000000, // 10 IPN
            status: DomainStatus::Active,
        };
        
        Ok(Some(domain_info))
    }
    
    /// Get storage file information
    pub async fn get_storage_file_info(&self, hash: &str) -> Result<Option<StorageFileInfo>> {
        // In a real implementation, you'd query the storage system
        // For now, we'll return simulated data
        
        let file_info = StorageFileInfo {
            hash: hash.to_string(),
            name: "example.txt".to_string(),
            size: 1024 * 1024, // 1MB
            uploader: "uploader_address".to_string(),
            upload_timestamp: chrono::Utc::now().timestamp() as u64 - 3600, // 1 hour ago
            replication_factor: 3,
            storage_nodes: vec![
                "node1".to_string(),
                "node2".to_string(),
                "node3".to_string(),
            ],
            status: StorageFileStatus::Available,
        };
        
        Ok(Some(file_info))
    }
    
    /// Search blocks
    pub async fn search_blocks(&self, query: &str) -> Result<Vec<BlockInfo>> {
        // In a real implementation, you'd search the blockchain
        // For now, we'll return simulated data
        
        let blocks = vec![
            BlockInfo {
                hash: format!("block_hash_{}", query),
                height: query.parse().unwrap_or(0),
                previous_hash: "previous_hash".to_string(),
                timestamp: chrono::Utc::now().timestamp() as u64,
                transaction_count: 15,
                size: 2048,
                validator: "validator_address".to_string(),
                reward: 1000000000,
            }
        ];
        
        Ok(blocks)
    }
    
    /// Search transactions
    pub async fn search_transactions(&self, query: &str) -> Result<Vec<TransactionInfo>> {
        // In a real implementation, you'd search the blockchain
        // For now, we'll return simulated data
        
        let transactions = vec![
            TransactionInfo {
                hash: query.to_string(),
                block_hash: "block_hash_123".to_string(),
                block_height: 123,
                tx_type: TransactionType::Payment,
                from: "sender_address".to_string(),
                to: Some("recipient_address".to_string()),
                amount: 1000000000,
                fee: 10000000,
                timestamp: chrono::Utc::now().timestamp() as u64,
                status: TransactionStatus::Confirmed,
            }
        ];
        
        Ok(transactions)
    }
    
    /// Get recent blocks
    pub async fn get_recent_blocks(&self, limit: usize) -> Result<Vec<BlockInfo>> {
        let node = self.state.node.read().await;
        let current_height = node.block_height();
        
        let mut blocks = Vec::new();
        for i in 0..limit {
            if current_height >= i as u64 {
                let height = current_height - i as u64;
                if let Some(block) = self.get_block_info(height).await? {
                    blocks.push(block);
                }
            }
        }
        
        Ok(blocks)
    }
    
    /// Get recent transactions
    pub async fn get_recent_transactions(&self, limit: usize) -> Result<Vec<TransactionInfo>> {
        // In a real implementation, you'd query recent transactions
        // For now, we'll return simulated data
        
        let mut transactions = Vec::new();
        for i in 0..limit {
            transactions.push(TransactionInfo {
                hash: format!("tx_hash_{}", i),
                block_hash: format!("block_hash_{}", i),
                block_height: i as u64,
                tx_type: TransactionType::Payment,
                from: format!("sender_{}", i),
                to: Some(format!("recipient_{}", i)),
                amount: 1000000000,
                fee: 10000000,
                timestamp: chrono::Utc::now().timestamp() as u64 - i as u64 * 60,
                status: TransactionStatus::Confirmed,
            });
        }
        
        Ok(transactions)
    }
}

/// Explorer API for blockchain exploration and analytics
pub struct ExplorerApi {
    node: Arc<RwLock<IppanNode>>,
    server: Option<axum::Server<axum::extract::DefaultBodyLimit, axum::routing::IntoMakeService<Router>>>,
    bind_addr: String,
}

impl ExplorerApi {
    pub fn new(node: Arc<RwLock<IppanNode>>) -> Self {
        Self {
            node,
            server: None,
            bind_addr: "127.0.0.1:8081".to_string(),
        }
    }

    /// Start the explorer API server
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Starting Explorer API on {}", self.bind_addr);
        
        let app = self.create_router();
        let listener = tokio::net::TcpListener::bind(&self.bind_addr).await?;
        
        self.server = Some(axum::Server::from_tcp(listener.into_std()?)?
            .serve(app.into_make_service()));
        
        log::info!("Explorer API started successfully");
        Ok(())
    }

    /// Stop the explorer API server
    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(server) = self.server.take() {
            server.await?;
        }
        Ok(())
    }

    /// Create the explorer router
    fn create_router(&self) -> Router {
        let node = Arc::clone(&self.node);
        
        Router::new()
            // Blockchain exploration
            .route("/blocks", get(Self::get_blocks))
            .route("/blocks/:hash", get(Self::get_block))
            .route("/blocks/latest", get(Self::get_latest_block))
            .route("/blocks/round/:round", get(Self::get_blocks_by_round))
            
            // Transaction exploration
            .route("/transactions", get(Self::get_transactions))
            .route("/transactions/:hash", get(Self::get_transaction))
            .route("/transactions/address/:address", get(Self::get_transactions_by_address))
            
            // Network analytics
            .route("/network/stats", get(Self::get_network_stats))
            .route("/network/validators", get(Self::get_network_validators))
            .route("/network/peers", get(Self::get_network_peers))
            
            // Storage analytics
            .route("/storage/stats", get(Self::get_storage_stats))
            .route("/storage/files", get(Self::get_storage_files))
            .route("/storage/files/:hash", get(Self::get_storage_file))
            
            // DHT analytics
            .route("/dht/stats", get(Self::get_dht_stats))
            .route("/dht/keys", get(Self::get_dht_keys))
            .route("/dht/keys/:key", get(Self::get_dht_key))
            
            // Economic analytics
            .route("/economics/supply", get(Self::get_token_supply))
            .route("/economics/fees", get(Self::get_fee_stats))
            .route("/economics/staking", get(Self::get_staking_stats))
            
            // Search functionality
            .route("/search", get(Self::search))
            
            .with_state(node)
    }

    // Get blocks with pagination
    async fn get_blocks(
        State(node): State<Arc<RwLock<IppanNode>>>,
        Query(params): Query<BlockQueryParams>,
    ) -> Json<ApiResponse<BlockListResponse>> {
        let node = node.read().await;
        let blocks = node.consensus.get_blocks(
            params.page.unwrap_or(0),
            params.limit.unwrap_or(20)
        );
        
        let block_infos = blocks.iter().map(|block| BlockInfo {
            hash: format!("{:?}", block.hash()),
            round: block.round(),
            timestamp: block.timestamp(),
            transaction_count: block.transactions().len(),
            validator: format!("{:?}", block.validator()),
            size: block.size(),
        }).collect();
        
        Json(ApiResponse::success(BlockListResponse {
            blocks: block_infos,
            total: node.consensus.get_total_blocks(),
            page: params.page.unwrap_or(0),
            limit: params.limit.unwrap_or(20),
        }))
    }

    // Get specific block by hash
    async fn get_block(
        State(node): State<Arc<RwLock<IppanNode>>>,
        Path(hash): Path<String>,
    ) -> Json<ApiResponse<DetailedBlockInfo>> {
        let node = node.read().await;
        // Implementation would decode hash and find block
        let block_info = DetailedBlockInfo {
            hash: hash,
            round: 0,
            timestamp: 0,
            transactions: vec![],
            validator: "".to_string(),
            size: 0,
            parent_hashes: vec![],
            merkle_root: "".to_string(),
        };
        Json(ApiResponse::success(block_info))
    }

    // Get latest block
    async fn get_latest_block(
        State(node): State<Arc<RwLock<IppanNode>>>,
    ) -> Json<ApiResponse<BlockInfo>> {
        let node = node.read().await;
        let latest_block = node.consensus.get_latest_block();
        let block_info = BlockInfo {
            hash: format!("{:?}", latest_block.hash()),
            round: latest_block.round(),
            timestamp: latest_block.timestamp(),
            transaction_count: latest_block.transactions().len(),
            validator: format!("{:?}", latest_block.validator()),
            size: latest_block.size(),
        };
        Json(ApiResponse::success(block_info))
    }

    // Get blocks by round
    async fn get_blocks_by_round(
        State(node): State<Arc<RwLock<IppanNode>>>,
        Path(round): Path<u64>,
    ) -> Json<ApiResponse<Vec<BlockInfo>>> {
        let node = node.read().await;
        let blocks = node.consensus.get_blocks_by_round(round);
        let block_infos = blocks.iter().map(|block| BlockInfo {
            hash: format!("{:?}", block.hash()),
            round: block.round(),
            timestamp: block.timestamp(),
            transaction_count: block.transactions().len(),
            validator: format!("{:?}", block.validator()),
            size: block.size(),
        }).collect();
        Json(ApiResponse::success(block_infos))
    }

    // Get transactions with pagination
    async fn get_transactions(
        State(node): State<Arc<RwLock<IppanNode>>>,
        Query(params): Query<TransactionQueryParams>,
    ) -> Json<ApiResponse<TransactionListResponse>> {
        let node = node.read().await;
        let transactions = node.wallet.get_transactions();
        
        let tx_infos = transactions.iter().map(|tx| TransactionInfo {
            hash: format!("{:?}", tx.hash),
            from_address: tx.from_address.clone(),
            to_address: tx.to_address.clone(),
            amount: tx.amount,
            fee: tx.fee,
            timestamp: tx.timestamp,
            status: tx.status.clone(),
        }).collect();
        
        Json(ApiResponse::success(TransactionListResponse {
            transactions: tx_infos,
            total: transactions.len(),
            page: params.page.unwrap_or(0),
            limit: params.limit.unwrap_or(20),
        }))
    }

    // Get specific transaction
    async fn get_transaction(
        State(node): State<Arc<RwLock<IppanNode>>>,
        Path(hash): Path<String>,
    ) -> Json<ApiResponse<DetailedTransactionInfo>> {
        let node = node.read().await;
        // Implementation would decode hash and find transaction
        let tx_info = DetailedTransactionInfo {
            hash: hash,
            from_address: "".to_string(),
            to_address: "".to_string(),
            amount: 0,
            fee: 0,
            timestamp: 0,
            status: "".to_string(),
            block_hash: "".to_string(),
            block_round: 0,
        };
        Json(ApiResponse::success(tx_info))
    }

    // Get transactions by address
    async fn get_transactions_by_address(
        State(node): State<Arc<RwLock<IppanNode>>>,
        Path(address): Path<String>,
    ) -> Json<ApiResponse<Vec<TransactionInfo>>> {
        let node = node.read().await;
        let transactions = node.wallet.get_transactions_by_address(&address);
        let tx_infos = transactions.iter().map(|tx| TransactionInfo {
            hash: format!("{:?}", tx.hash),
            from_address: tx.from_address.clone(),
            to_address: tx.to_address.clone(),
            amount: tx.amount,
            fee: tx.fee,
            timestamp: tx.timestamp,
            status: tx.status.clone(),
        }).collect();
        Json(ApiResponse::success(tx_infos))
    }

    // Get network statistics
    async fn get_network_stats(
        State(node): State<Arc<RwLock<IppanNode>>>,
    ) -> Json<ApiResponse<NetworkStats>> {
        let node = node.read().await;
        let stats = NetworkStats {
            total_nodes: node.network.get_total_nodes(),
            active_nodes: node.network.get_active_nodes(),
            connected_peers: node.network.get_peer_count(),
            total_stake: node.consensus.get_total_stake(),
            active_validators: node.consensus.get_active_validator_count(),
            current_round: node.consensus.get_current_round(),
            block_height: node.consensus.get_block_height(),
        };
        Json(ApiResponse::success(stats))
    }

    // Get network validators
    async fn get_network_validators(
        State(node): State<Arc<RwLock<IppanNode>>>,
    ) -> Json<ApiResponse<Vec<ValidatorInfo>>> {
        let node = node.read().await;
        let validators = node.consensus.get_validators();
        let validator_infos = validators.iter().map(|validator| ValidatorInfo {
            node_id: format!("{:?}", validator.node_id),
            address: validator.address.clone(),
            stake_amount: validator.stake_amount,
            is_active: validator.is_active,
            uptime: validator.uptime,
            total_blocks: validator.total_blocks,
        }).collect();
        Json(ApiResponse::success(validator_infos))
    }

    // Get network peers
    async fn get_network_peers(
        State(node): State<Arc<RwLock<IppanNode>>>,
    ) -> Json<ApiResponse<Vec<PeerInfo>>> {
        let node = node.read().await;
        let peers = node.network.get_peers();
        let peer_infos = peers.iter().map(|peer| PeerInfo {
            peer_id: peer.peer_id.to_string(),
            address: peer.address.to_string(),
            last_seen: peer.last_seen,
            is_validator: peer.is_validator,
            stake_amount: peer.stake_amount,
        }).collect();
        Json(ApiResponse::success(peer_infos))
    }

    // Get storage statistics
    async fn get_storage_stats(
        State(node): State<Arc<RwLock<IppanNode>>>,
    ) -> Json<ApiResponse<StorageStats>> {
        let node = node.read().await;
        let usage = node.storage.get_usage();
        let stats = StorageStats {
            total_files: node.storage.get_file_count(),
            total_size: usage.used_bytes,
            total_capacity: usage.total_bytes,
            shard_count: usage.shard_count,
            replication_factor: node.storage.get_replication_factor(),
            active_shards: node.storage.get_active_shard_count(),
        };
        Json(ApiResponse::success(stats))
    }

    // Get storage files
    async fn get_storage_files(
        State(node): State<Arc<RwLock<IppanNode>>>,
    ) -> Json<ApiResponse<Vec<FileInfo>>> {
        let node = node.read().await;
        let files = node.storage.get_files();
        let file_infos = files.iter().map(|file| FileInfo {
            hash: format!("{:?}", file.hash),
            filename: file.filename.clone(),
            size: file.size,
            uploaded_at: file.uploaded_at,
            shard_count: file.shard_count,
            replication_factor: file.replication_factor,
        }).collect();
        Json(ApiResponse::success(file_infos))
    }

    // Get specific storage file
    async fn get_storage_file(
        State(node): State<Arc<RwLock<IppanNode>>>,
        Path(hash): Path<String>,
    ) -> Json<ApiResponse<DetailedFileInfo>> {
        let node = node.read().await;
        // Implementation would decode hash and find file
        let file_info = DetailedFileInfo {
            hash: hash,
            filename: "".to_string(),
            size: 0,
            uploaded_at: std::time::SystemTime::now(),
            shard_count: 0,
            replication_factor: 0,
            shard_locations: vec![],
            merkle_root: "".to_string(),
        };
        Json(ApiResponse::success(file_info))
    }

    // Get DHT statistics
    async fn get_dht_stats(
        State(node): State<Arc<RwLock<IppanNode>>>,
    ) -> Json<ApiResponse<DhtStats>> {
        let node = node.read().await;
        let stats = DhtStats {
            total_keys: node.dht.get_key_count(),
            total_values: node.dht.get_value_count(),
            replication_factor: node.dht.get_replication_factor(),
            active_nodes: node.dht.get_active_node_count(),
        };
        Json(ApiResponse::success(stats))
    }

    // Get DHT keys
    async fn get_dht_keys(
        State(node): State<Arc<RwLock<IppanNode>>>,
    ) -> Json<ApiResponse<Vec<String>>> {
        let node = node.read().await;
        Json(ApiResponse::success(node.dht.get_keys()))
    }

    // Get DHT key value
    async fn get_dht_key(
        State(node): State<Arc<RwLock<IppanNode>>>,
        Path(key): Path<String>,
    ) -> Json<ApiResponse<DhtKeyInfo>> {
        let node = node.read().await;
        match node.dht.get(&key) {
            Some(value) => {
                let key_info = DhtKeyInfo {
                    key: key,
                    value: value,
                    last_updated: std::time::SystemTime::now(),
                    replication_count: 1,
                };
                Json(ApiResponse::success(key_info))
            }
            None => Json(ApiResponse::error("Key not found".to_string())),
        }
    }

    // Get token supply information
    async fn get_token_supply(
        State(node): State<Arc<RwLock<IppanNode>>>,
    ) -> Json<ApiResponse<TokenSupplyInfo>> {
        let node = node.read().await;
        let supply_info = TokenSupplyInfo {
            total_supply: 21_000_000_000_000, // 21M IPN in smallest units
            circulating_supply: node.wallet.get_total_circulating_supply(),
            staked_supply: node.wallet.get_total_staked_supply(),
            burned_supply: node.wallet.get_total_burned_supply(),
        };
        Json(ApiResponse::success(supply_info))
    }

    // Get fee statistics
    async fn get_fee_stats(
        State(node): State<Arc<RwLock<IppanNode>>>,
    ) -> Json<ApiResponse<FeeStats>> {
        let node = node.read().await;
        let fee_stats = FeeStats {
            total_fees_collected: node.wallet.get_total_fees_collected(),
            fees_this_round: node.wallet.get_fees_this_round(),
            average_fee_per_transaction: node.wallet.get_average_fee_per_transaction(),
            fee_rate_percentage: 1.0, // 1% as per PRD
        };
        Json(ApiResponse::success(fee_stats))
    }

    // Get staking statistics
    async fn get_staking_stats(
        State(node): State<Arc<RwLock<IppanNode>>>,
    ) -> Json<ApiResponse<StakingStats>> {
        let node = node.read().await;
        let staking_stats = StakingStats {
            total_staked: node.wallet.get_total_staked_supply(),
            total_stakers: node.wallet.get_total_stakers(),
            average_stake: node.wallet.get_average_stake(),
            minimum_stake: 10_000_000, // 10 IPN in smallest units
            maximum_stake: 100_000_000, // 100 IPN in smallest units
        };
        Json(ApiResponse::success(staking_stats))
    }

    // Search functionality
    async fn search(
        State(node): State<Arc<RwLock<IppanNode>>>,
        Query(params): Query<SearchParams>,
    ) -> Json<ApiResponse<SearchResults>> {
        let node = node.read().await;
        let query = params.q.unwrap_or_default();
        
        let results = SearchResults {
            blocks: vec![], // Implementation would search blocks
            transactions: vec![], // Implementation would search transactions
            files: vec![], // Implementation would search files
            addresses: vec![], // Implementation would search addresses
            query: query,
        };
        Json(ApiResponse::success(results))
    }
}

// API Response wrapper (same as http.rs)
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

// Query parameters
#[derive(Debug, Deserialize)]
pub struct BlockQueryParams {
    pub page: Option<u64>,
    pub limit: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct TransactionQueryParams {
    pub page: Option<u64>,
    pub limit: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub q: Option<String>,
}

// Response types
#[derive(Debug, Serialize)]
pub struct BlockListResponse {
    pub blocks: Vec<BlockInfo>,
    pub total: u64,
    pub page: u64,
    pub limit: u64,
}

#[derive(Debug, Serialize)]
pub struct TransactionListResponse {
    pub transactions: Vec<TransactionInfo>,
    pub total: usize,
    pub page: u64,
    pub limit: u64,
}

#[derive(Debug, Serialize)]
pub struct BlockInfo {
    pub hash: String,
    pub round: u64,
    pub timestamp: u64,
    pub transaction_count: usize,
    pub validator: String,
    pub size: u64,
}

#[derive(Debug, Serialize)]
pub struct DetailedBlockInfo {
    pub hash: String,
    pub round: u64,
    pub timestamp: u64,
    pub transactions: Vec<TransactionInfo>,
    pub validator: String,
    pub size: u64,
    pub parent_hashes: Vec<String>,
    pub merkle_root: String,
}

#[derive(Debug, Serialize)]
pub struct TransactionInfo {
    pub hash: String,
    pub from_address: String,
    pub to_address: String,
    pub amount: u64,
    pub fee: u64,
    pub timestamp: u64,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct DetailedTransactionInfo {
    pub hash: String,
    pub from_address: String,
    pub to_address: String,
    pub amount: u64,
    pub fee: u64,
    pub timestamp: u64,
    pub status: String,
    pub block_hash: String,
    pub block_round: u64,
}

#[derive(Debug, Serialize)]
pub struct NetworkStats {
    pub total_nodes: usize,
    pub active_nodes: usize,
    pub connected_peers: usize,
    pub total_stake: u64,
    pub active_validators: usize,
    pub current_round: u64,
    pub block_height: u64,
}

#[derive(Debug, Serialize)]
pub struct ValidatorInfo {
    pub node_id: String,
    pub address: String,
    pub stake_amount: u64,
    pub is_active: bool,
    pub uptime: std::time::Duration,
    pub total_blocks: u64,
}

#[derive(Debug, Serialize)]
pub struct PeerInfo {
    pub peer_id: String,
    pub address: String,
    pub last_seen: std::time::SystemTime,
    pub is_validator: bool,
    pub stake_amount: u64,
}

#[derive(Debug, Serialize)]
pub struct StorageStats {
    pub total_files: u64,
    pub total_size: u64,
    pub total_capacity: u64,
    pub shard_count: usize,
    pub replication_factor: u32,
    pub active_shards: usize,
}

#[derive(Debug, Serialize)]
pub struct FileInfo {
    pub hash: String,
    pub filename: String,
    pub size: u64,
    pub uploaded_at: std::time::SystemTime,
    pub shard_count: usize,
    pub replication_factor: u32,
}

#[derive(Debug, Serialize)]
pub struct DetailedFileInfo {
    pub hash: String,
    pub filename: String,
    pub size: u64,
    pub uploaded_at: std::time::SystemTime,
    pub shard_count: usize,
    pub replication_factor: u32,
    pub shard_locations: Vec<String>,
    pub merkle_root: String,
}

#[derive(Debug, Serialize)]
pub struct DhtStats {
    pub total_keys: usize,
    pub total_values: usize,
    pub replication_factor: u32,
    pub active_nodes: usize,
}

#[derive(Debug, Serialize)]
pub struct DhtKeyInfo {
    pub key: String,
    pub value: String,
    pub last_updated: std::time::SystemTime,
    pub replication_count: u32,
}

#[derive(Debug, Serialize)]
pub struct TokenSupplyInfo {
    pub total_supply: u64,
    pub circulating_supply: u64,
    pub staked_supply: u64,
    pub burned_supply: u64,
}

#[derive(Debug, Serialize)]
pub struct FeeStats {
    pub total_fees_collected: u64,
    pub fees_this_round: u64,
    pub average_fee_per_transaction: u64,
    pub fee_rate_percentage: f64,
}

#[derive(Debug, Serialize)]
pub struct StakingStats {
    pub total_staked: u64,
    pub total_stakers: usize,
    pub average_stake: u64,
    pub minimum_stake: u64,
    pub maximum_stake: u64,
}

#[derive(Debug, Serialize)]
pub struct SearchResults {
    pub blocks: Vec<BlockInfo>,
    pub transactions: Vec<TransactionInfo>,
    pub files: Vec<FileInfo>,
    pub addresses: Vec<String>,
    pub query: String,
}
