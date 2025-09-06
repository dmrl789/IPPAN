//! API module for IPPAN
//! 
//! Handles HTTP API, CLI, and explorer interfaces

pub mod cli;
pub mod crosschain;
pub mod dns_cli;
pub mod user_cli; // NEW - User-facing transaction CLI
pub mod http;
// pub mod v1; // NEW - REST API v1 endpoints (temporarily disabled due to Axum compatibility issues)
// pub mod explorer;

use crate::node::IppanNode;
use std::sync::Arc;
use tokio::sync::RwLock;

/// API layer that provides HTTP, CLI, and explorer interfaces
pub struct ApiLayer {
    node: Arc<RwLock<IppanNode>>,
    http_server: Option<http::HttpServer>,
    // TODO: Re-enable when modules are ready
    // explorer: Option<explorer::ExplorerApi>,
}

impl ApiLayer {
    pub fn new(node: Arc<RwLock<IppanNode>>) -> Self {
        Self {
            node,
            http_server: None,
            // TODO: Re-enable when modules are ready
            // explorer: None,
        }
    }

    /// Start all API services
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Starting API layer...");
        
        // Start HTTP server
        let node_clone = Arc::clone(&self.node);
        self.http_server = Some(http::HttpServer::new(node_clone));
        self.http_server.as_mut().unwrap().start().await?;
        
        // TODO: Re-enable when modules are ready
        // Start explorer API
        // let node_clone = Arc::clone(&self.node);
        // self.explorer = Some(explorer::ExplorerApi::new(node_clone));
        // self.explorer.as_mut().unwrap().start().await?;
        
        log::info!("API layer started successfully");
        Ok(())
    }

    /// Stop all API services
    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Stopping API layer...");
        
        if let Some(mut server) = self.http_server.take() {
            server.stop().await?;
        }
        
        // TODO: Re-enable when modules are ready
        // if let Some(mut explorer) = self.explorer.take() {
        //     explorer.stop().await?;
        // }
        
        log::info!("API layer stopped");
        Ok(())
    }

    /// Get node status for API responses
    pub async fn get_node_status(&self) -> NodeStatus {
        let node = self.node.read().await;
        NodeStatus {
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime: node.get_uptime(),
            consensus_round: 0, // TODO: Implement consensus round access
            storage_usage: StorageUsage {
                used_bytes: 0,
                total_bytes: 0,
                shard_count: 0,
            }, // TODO: Implement storage usage
            network_peers: 0, // TODO: Implement peer count
            wallet_balance: 0, // TODO: Implement wallet balance
            dht_keys: 0, // TODO: Implement DHT key count
        }
    }
}

/// Node status information for API responses
#[derive(Debug, serde::Serialize)]
pub struct NodeStatus {
    pub version: String,
    pub uptime: std::time::Duration,
    pub consensus_round: u64,
    pub storage_usage: StorageUsage,
    pub network_peers: usize,
    pub wallet_balance: u64,
    pub dht_keys: usize,
}

#[derive(Debug, serde::Serialize)]
pub struct StorageUsage {
    pub used_bytes: u64,
    pub total_bytes: u64,
    pub shard_count: usize,
}
