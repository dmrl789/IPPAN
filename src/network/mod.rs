//! Network subsystem for IPPAN
//!
//! Handles P2P networking, peer discovery, and message routing.

use crate::config::NetworkConfig;
use crate::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::net::SocketAddr;
use serde::Serialize;

pub mod p2p;
pub mod discovery;
pub mod nat;
pub mod relay;
pub mod protocol;
pub mod security;

use p2p::P2PNetwork;
use discovery::DiscoveryService;

pub struct NetworkManager {
    pub config: NetworkConfig,
    /// P2P network
    pub p2p: Arc<RwLock<P2PNetwork>>,
    /// Discovery service
    pub discovery: Arc<RwLock<DiscoveryService>>,
    /// NAT traversal
    pub nat: Arc<RwLock<nat::NATService>>,
    /// Relay service
    pub relay: Arc<RwLock<relay::RelayService>>,
    /// Protocol handlers
    pub protocols: Arc<RwLock<protocol::ProtocolManager>>,
    running: bool,
}

impl NetworkManager {
    /// Create a new network manager
    pub async fn new(config: NetworkConfig) -> Result<Self> {
        // Initialize P2P network
        let node_id = [0u8; 32]; // TODO: Get from wallet
        let node_address = "127.0.0.1".to_string();
        let listen_addr = SocketAddr::new(
            std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
            8080, // Default port
        );
        
        let p2p = Arc::new(RwLock::new(
            P2PNetwork::new(node_id, node_address, listen_addr).await?
        ));
        
        // Initialize discovery service
        let bootstrap_peers: Vec<SocketAddr> = config.bootstrap_nodes.iter()
            .filter_map(|addr| addr.parse::<SocketAddr>().ok())
            .collect();
        let discovery = Arc::new(RwLock::new(
            DiscoveryService::new(node_id, bootstrap_peers).await?
        ));
        
        // Initialize NAT service
        let nat = Arc::new(RwLock::new(nat::NATService::new()));
        
        // Initialize relay service
        let relay = Arc::new(RwLock::new(relay::RelayService::new()));
        
        // Initialize protocol manager
        let protocols = Arc::new(RwLock::new(protocol::ProtocolManager::new()));
        
        Ok(Self {
            config,
            p2p,
            discovery,
            nat,
            relay,
            protocols,
            running: false,
        })
    }

    /// Start the network subsystem
    pub async fn start(&mut self) -> Result<()> {
        log::info!("Starting network subsystem...");
        
        // Start P2P network (legacy - will be replaced with secure version)
        // let mut p2p = self.p2p.write().await;
        // p2p.start().await?;
        
        // Start discovery service
        let mut discovery = self.discovery.write().await;
        discovery.start().await?;
        
        // Start NAT service
        let mut nat = self.nat.write().await;
        nat.start().await?;
        
        // Start relay service
        let mut relay = self.relay.write().await;
        relay.start().await?;
        
        // Start protocol manager
        let mut protocols = self.protocols.write().await;
        protocols.start().await?;
        
        self.running = true;
        log::info!("Network subsystem started");
        Ok(())
    }

    /// Stop the network subsystem
    pub async fn stop(&mut self) -> Result<()> {
        log::info!("Stopping network subsystem...");
        
        // Stop protocol manager
        let mut protocols = self.protocols.write().await;
        protocols.stop().await?;
        
        // Stop relay service
        let mut relay = self.relay.write().await;
        relay.stop().await?;
        
        // Stop NAT service
        let mut nat = self.nat.write().await;
        nat.stop().await?;
        
        // Stop discovery service
        let mut discovery = self.discovery.write().await;
        discovery.stop().await?;
        
        // Stop P2P network (legacy - will be replaced with secure version)
        // let mut p2p = self.p2p.write().await;
        // p2p.stop().await?;
        
        self.running = false;
        log::info!("Network subsystem stopped");
        Ok(())
    }

    /// Connect to a peer
    pub async fn connect_to_peer(&self, address: String, port: u16) -> Result<()> {
        // Legacy implementation - will be replaced with secure version
        log::info!("Connecting to peer {}:{} (legacy implementation)", address, port);
        Ok(())
    }

    /// Get network statistics
    pub async fn get_network_stats(&self) -> NetworkStats {
        let discovery = self.discovery.read().await;
        
        let active_connections = 0; // Legacy - will be updated with secure version
        let known_peers = discovery.get_peer_count().await;
        let reachable_peers = discovery.get_reachable_peers().await.len();
        
        NetworkStats {
            active_connections,
            known_peers,
            reachable_peers,
            total_peers: known_peers,
        }
    }

    /// Broadcast a message to all peers
    pub async fn broadcast_message(&self, message: p2p::P2PMessage) -> Result<()> {
        // Legacy implementation - will be replaced with secure version
        log::info!("Broadcasting message (legacy implementation): {:?}", message);
        Ok(())
    }

    /// Get known peers
    pub async fn get_known_peers(&self) -> Vec<discovery::PeerInfo> {
        let discovery = self.discovery.read().await;
        discovery.get_known_peers().await
    }
}

/// Network statistics
#[derive(Debug, Serialize)]
pub struct NetworkStats {
    pub active_connections: usize,
    pub known_peers: usize,
    pub reachable_peers: usize,
    pub total_peers: usize,
}
