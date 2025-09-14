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
pub mod libp2p_network;
pub mod real_p2p; // NEW - Real P2P network implementation
pub mod discovery;
pub mod nat;
pub mod relay;
pub mod protocol;
pub mod security;

use p2p::P2PNetwork;
use libp2p_network::LibP2PNetwork;
use discovery::DiscoveryService;

pub struct NetworkManager {
    pub config: NetworkConfig,
    /// Real P2P network using libp2p
    pub libp2p_network: Arc<RwLock<LibP2PNetwork>>,
    /// Legacy P2P network (for backward compatibility)
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

impl Default for NetworkManager {
    fn default() -> Self {
        Self {
            config: NetworkConfig::default(),
            libp2p_network: Arc::new(RwLock::new(LibP2PNetwork::default())),
            p2p: Arc::new(RwLock::new(P2PNetwork::default())),
            discovery: Arc::new(RwLock::new(DiscoveryService::default())),
            nat: Arc::new(RwLock::new(nat::NATService::default())),
            relay: Arc::new(RwLock::new(relay::RelayService::default())),
            protocols: Arc::new(RwLock::new(protocol::ProtocolManager::default())),
            running: false,
        }
    }
}

impl NetworkManager {
    /// Create a new network manager
    pub async fn new(config: NetworkConfig) -> Result<Self> {
        // Parse listen address from config
        let listen_addr = if config.listen_addr.starts_with("/ip4/") {
            // Parse libp2p multiaddr format
            let parts: Vec<&str> = config.listen_addr.split('/').collect();
            if parts.len() >= 4 {
                let ip = parts[2];
                let port = parts[4].parse::<u16>().unwrap_or(8080);
                SocketAddr::new(ip.parse().unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))), port)
            } else {
                SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)), 8080)
            }
        } else {
            // Parse standard socket address format
            config.listen_addr.parse::<SocketAddr>().unwrap_or_else(|_| {
                SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)), 8080)
            })
        };

        // Initialize real libp2p network
        let libp2p_network = Arc::new(RwLock::new(
            LibP2PNetwork::new(listen_addr, config.bootstrap_nodes.clone()).await?
        ));
        
        // Initialize legacy P2P network for backward compatibility
        let node_id = [0u8; 32]; // TODO: Get from wallet
        let node_address = "127.0.0.1".to_string();
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
            libp2p_network,
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
        
        // Start real libp2p network
        let mut libp2p_network = self.libp2p_network.write().await;
        libp2p_network.start().await?;
        drop(libp2p_network);
        
        // Attempt to dial bootstrap peers
        for addr in self.config.bootstrap_nodes.clone() {
            // Support both multiaddr and host:port formats
            if addr.starts_with("/ip4/") {
                // Parse libp2p multiaddr: /ip4/<ip>/tcp/<port>
                let parts: Vec<&str> = addr.split('/').collect();
                if parts.len() >= 5 {
                    let ip = parts[2].to_string();
                    let port = parts[4].parse::<u16>().unwrap_or(8080);
                    if let Err(e) = self.connect_to_peer(ip, port).await {
                        log::warn!("Failed to dial bootstrap peer {}: {}", addr, e);
                    } else {
                        log::info!("Dialing bootstrap peer {}", addr);
                    }
                } else {
                    log::warn!("Invalid bootstrap multiaddr format: {}", addr);
                }
            } else if let Ok(sock) = addr.parse::<std::net::SocketAddr>() {
                let ip = sock.ip().to_string();
                let port = sock.port();
                if let Err(e) = self.connect_to_peer(ip, port).await {
                    log::warn!("Failed to dial bootstrap peer {}: {}", addr, e);
                } else {
                    log::info!("Dialing bootstrap peer {}", addr);
                }
            } else {
                log::warn!("Unrecognized bootstrap address format: {}", addr);
            }
        }
        
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
        
        // Stop real libp2p network
        let mut libp2p_network = self.libp2p_network.write().await;
        libp2p_network.stop().await?;
        drop(libp2p_network);
        
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
        
        self.running = false;
        log::info!("Network subsystem stopped");
        Ok(())
    }

    /// Run the network event loop (should be called in a separate task)
    pub async fn run_event_loop(&mut self) -> Result<()> {
        log::info!("Starting network event loop...");
        
        let mut libp2p_network = self.libp2p_network.write().await;
        libp2p_network.run_event_loop().await?;
        
        log::info!("Network event loop stopped");
        Ok(())
    }

    /// Connect to a peer
    pub async fn connect_to_peer(&self, address: String, port: u16) -> Result<()> {
        let multiaddr = format!("/ip4/{}/tcp/{}", address, port);
        let multiaddr: libp2p::Multiaddr = multiaddr.parse()
            .map_err(|e| crate::error::IppanError::Network(format!("Invalid peer address: {}", e)))?;
        
        let mut libp2p_network = self.libp2p_network.write().await;
        libp2p_network.connect_to_peer(multiaddr).await?;
        
        log::info!("Connecting to peer {}:{}", address, port);
        Ok(())
    }

    /// Get network statistics
    pub async fn get_network_stats(&self) -> NetworkStats {
        let libp2p_network = self.libp2p_network.read().await;
        let libp2p_stats = libp2p_network.get_network_stats().await;
        
        NetworkStats {
            active_connections: libp2p_stats.connected_peers,
            known_peers: libp2p_stats.known_peers,
            reachable_peers: libp2p_stats.connected_peers,
            total_peers: libp2p_stats.known_peers,
        }
    }

    /// Broadcast a message to all peers
    pub async fn broadcast_message(&self, message: p2p::P2PMessage) -> Result<()> {
        let mut libp2p_network = self.libp2p_network.write().await;
        libp2p_network.broadcast_message(message).await?;
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
