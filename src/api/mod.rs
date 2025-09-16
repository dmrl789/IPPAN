//! API module for IPPAN
//! 
//! Handles HTTP API, CLI, and explorer interfaces

pub mod cli;
pub mod crosschain;
pub mod dns_cli;
pub mod user_cli; // NEW - User-facing transaction CLI
pub mod http;
pub mod simple_http;
pub mod real_rest_api; // NEW - Real REST API implementation
pub mod real_mode; // NEW - Real-mode API implementation
// pub mod v1; // NEW - REST API v1 endpoints (temporarily disabled due to Axum compatibility issues)
// pub mod explorer;

use crate::node::IppanNode;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// API layer that provides HTTP, CLI, and explorer interfaces
pub struct ApiLayer {
    node: Arc<RwLock<Option<Arc<RwLock<IppanNode>>>>>,
    http_server: Option<http::HttpServer>,
    simple_http_server: Option<simple_http::SimpleHttpServer>,
    real_mode_server: Option<real_mode::RealModeServer>,
    // TODO: Re-enable when modules are ready
    // explorer: Option<explorer::ExplorerApi>,
}

impl Default for ApiLayer {
    fn default() -> Self {
        Self {
            node: Arc::new(RwLock::new(None)), // Will be properly initialized later
            http_server: None,
            simple_http_server: None,
            real_mode_server: None,
        }
    }
}

impl ApiLayer {
    pub fn new(node: Arc<RwLock<Option<Arc<RwLock<IppanNode>>>>>) -> Self {
        Self {
            node,
            http_server: None,
            simple_http_server: None,
            real_mode_server: None,
            // TODO: Re-enable when modules are ready
            // explorer: None,
        }
    }

    /// Start all API services
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Starting API layer...");
        
        // REAL MODE: Panic if API starts without node reference
        let node_clone = Arc::clone(&self.node);
        let node_guard = node_clone.read().await;
        if node_guard.is_none() {
            panic!("API started without node reference - REAL_MODE_REQUIRED=true");
        }
        
        // Start simple HTTP server (bind all interfaces for external access)
        let addr = "0.0.0.0:3000".parse().unwrap();
        
        // Extract the node from the Option wrapper (now guaranteed to exist)
        let node_for_server = {
            let node_arc = node_guard.as_ref().unwrap();
            Arc::clone(node_arc)
        };
        self.real_mode_server = Some(real_mode::RealModeServer::new(node_for_server, addr));
        
        // Start the server in a separate task
        let server = self.real_mode_server.as_ref().unwrap();
        let server_clone = real_mode::RealModeServer::new(Arc::clone(&server.node), server.addr);
        
        tokio::spawn(async move {
            if let Err(e) = server_clone.start().await {
                log::error!("Real-mode HTTP server error: {}", e);
            }
        });
        
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
        let node_guard = self.node.read().await;
        let uptime = if let Some(node_arc) = node_guard.as_ref() {
            let inner_node = node_arc.read().await;
            inner_node.get_uptime()
        } else {
            Duration::from_secs(0) // Default uptime if node is not initialized
        };
        
        NodeStatus {
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime,
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
