//! API module for IPPAN
//! 
//! Handles HTTP API, CLI, and explorer interfaces

pub mod cli;
pub mod explorer;
pub mod http;

use crate::{error::IppanError, Result};
use axum::{
    routing::{get, post},
    Router,
    extract::State,
    response::Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// API server (stub implementation)
pub struct ApiServer {
    config: ApiConfig,
}

/// API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub listen_addr: String,
    pub enable_cors: bool,
    pub rate_limit: u32,
    pub enable_auth: bool,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            listen_addr: "127.0.0.1:8080".to_string(),
            enable_cors: true,
            rate_limit: 1000,
            enable_auth: false,
        }
    }
}

/// API state
#[derive(Clone)]
pub struct ApiState {
    /// Node reference
    pub node: Arc<RwLock<crate::node::IppanNode>>,
    /// API configuration
    pub config: ApiConfig,
}

impl ApiServer {
    pub async fn new(config: ApiConfig) -> Result<Self> {
        Ok(Self { config })
    }
    
    pub async fn start(&self) -> Result<()> {
        Ok(())
    }
    
    pub async fn stop(&self) -> Result<()> {
        Ok(())
    }
}

/// API statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiStats {
    /// HTTP requests served
    pub http_requests: u64,
    /// CLI commands executed
    pub cli_commands: u64,
    /// Explorer requests served
    pub explorer_requests: u64,
    /// Active connections
    pub active_connections: u64,
    /// Server uptime
    pub uptime: std::time::Duration,
}

/// API response wrapper
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    /// Success status
    pub success: bool,
    /// Response data
    pub data: Option<T>,
    /// Error message
    pub error: Option<String>,
    /// Timestamp
    pub timestamp: u64,
}

impl<T> ApiResponse<T> {
    /// Create a successful response
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: chrono::Utc::now().timestamp() as u64,
        }
    }
    
    /// Create an error response
    pub fn error(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            timestamp: chrono::Utc::now().timestamp() as u64,
        }
    }
}

/// Node information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// Node ID
    pub node_id: String,
    /// Peer ID
    pub peer_id: String,
    /// Version
    pub version: String,
    /// Uptime
    pub uptime: u64,
    /// Connected peers
    pub connected_peers: u32,
    /// Storage used
    pub storage_used: u64,
    /// Storage capacity
    pub storage_capacity: u64,
}

/// Network information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    /// Total nodes
    pub total_nodes: u32,
    /// Active nodes
    pub active_nodes: u32,
    /// Network hash rate
    pub hash_rate: f64,
    /// Block height
    pub block_height: u64,
    /// Last block hash
    pub last_block_hash: String,
}

/// Storage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageInfo {
    /// Total files stored
    pub total_files: u64,
    /// Total storage used
    pub storage_used: u64,
    /// Storage capacity
    pub storage_capacity: u64,
    /// Replication factor
    pub replication_factor: u32,
    /// Active shards
    pub active_shards: u32,
}

/// Consensus information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusInfo {
    /// Current round
    pub current_round: u64,
    /// Validator status
    pub is_validator: bool,
    /// Stake amount
    pub stake_amount: u64,
    /// Block proposals
    pub block_proposals: u64,
    /// Block votes
    pub block_votes: u64,
}

/// Wallet information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletInfo {
    /// Balance
    pub balance: u64,
    /// Staked amount
    pub staked_amount: u64,
    /// Total transactions
    pub total_transactions: u64,
    /// Addresses
    pub addresses: Vec<String>,
}

/// API endpoints
pub mod endpoints {
    use super::*;
    
    /// Get node information
    pub async fn get_node_info(State(state): State<ApiState>) -> Result<Json<ApiResponse<NodeInfo>>, StatusCode> {
        let node = state.node.read().await;
        
        let node_info = NodeInfo {
            node_id: format!("{:?}", node.node_id()),
            peer_id: node.peer_id().to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime: node.uptime().as_secs(),
            connected_peers: node.connected_peers_count() as u32,
            storage_used: node.storage_used(),
            storage_capacity: node.storage_capacity(),
        };
        
        Ok(Json(ApiResponse::success(node_info)))
    }
    
    /// Get network information
    pub async fn get_network_info(State(state): State<ApiState>) -> Result<Json<ApiResponse<NetworkInfo>>, StatusCode> {
        let node = state.node.read().await;
        
        let network_info = NetworkInfo {
            total_nodes: node.total_nodes() as u32,
            active_nodes: node.active_nodes() as u32,
            hash_rate: node.hash_rate(),
            block_height: node.block_height(),
            last_block_hash: format!("{:?}", node.last_block_hash()),
        };
        
        Ok(Json(ApiResponse::success(network_info)))
    }
    
    /// Get storage information
    pub async fn get_storage_info(State(state): State<ApiState>) -> Result<Json<ApiResponse<StorageInfo>>, StatusCode> {
        let node = state.node.read().await;
        
        let storage_info = StorageInfo {
            total_files: node.total_files(),
            storage_used: node.storage_used(),
            storage_capacity: node.storage_capacity(),
            replication_factor: node.replication_factor(),
            active_shards: node.active_shards(),
        };
        
        Ok(Json(ApiResponse::success(storage_info)))
    }
    
    /// Get consensus information
    pub async fn get_consensus_info(State(state): State<ApiState>) -> Result<Json<ApiResponse<ConsensusInfo>>, StatusCode> {
        let node = state.node.read().await;
        
        let consensus_info = ConsensusInfo {
            current_round: node.current_round(),
            is_validator: node.is_validator(),
            stake_amount: node.stake_amount(),
            block_proposals: node.block_proposals(),
            block_votes: node.block_votes(),
        };
        
        Ok(Json(ApiResponse::success(consensus_info)))
    }
    
    /// Get wallet information
    pub async fn get_wallet_info(State(state): State<ApiState>) -> Result<Json<ApiResponse<WalletInfo>>, StatusCode> {
        let node = state.node.read().await;
        
        let wallet_info = WalletInfo {
            balance: node.wallet_balance(),
            staked_amount: node.staked_amount(),
            total_transactions: node.total_transactions(),
            addresses: node.wallet_addresses(),
        };
        
        Ok(Json(ApiResponse::success(wallet_info)))
    }
    
    /// Health check endpoint
    pub async fn health_check() -> Result<Json<ApiResponse<String>>, StatusCode> {
        Ok(Json(ApiResponse::success("OK".to_string())))
    }
}

/// Create API router
pub fn create_router(state: ApiState) -> Router {
    Router::new()
        .route("/health", get(endpoints::health_check))
        .route("/node/info", get(endpoints::get_node_info))
        .route("/network/info", get(endpoints::get_network_info))
        .route("/storage/info", get(endpoints::get_storage_info))
        .route("/consensus/info", get(endpoints::get_consensus_info))
        .route("/wallet/info", get(endpoints::get_wallet_info))
        .with_state(state)
}
