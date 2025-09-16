//! Peer discovery for IPPAN network

use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tokio::time::{interval, Duration};
use chrono::{DateTime, Utc};

/// Peer discovery message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiscoveryMessage {
    /// Peer announcement
    PeerAnnouncement(PeerAnnouncement),
    /// Peer request
    PeerRequest(PeerRequest),
    /// Peer response
    PeerResponse(PeerResponse),
    /// Ping message
    Ping(PingMessage),
    /// Pong response
    Pong(PongMessage),
}

/// Peer announcement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerAnnouncement {
    /// Node ID
    pub node_id: [u8; 32],
    /// Node address
    pub address: String,
    /// Node port
    pub port: u16,
    /// Node features
    pub features: Vec<String>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Signature
    pub signature: Option<Vec<u8>>,
}

/// Peer request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerRequest {
    /// Requesting node ID
    pub node_id: [u8; 32],
    /// Maximum peers to return
    pub max_peers: u32,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Peer response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerResponse {
    /// Responding node ID
    pub node_id: [u8; 32],
    /// List of peers
    pub peers: Vec<PeerInfo>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Ping message for discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingMessage {
    /// Node ID
    pub node_id: [u8; 32],
    /// Nonce
    pub nonce: u64,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Pong response for discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PongMessage {
    /// Node ID
    pub node_id: [u8; 32],
    /// Nonce from ping
    pub nonce: u64,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Peer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    /// Node ID
    pub node_id: [u8; 32],
    /// Address
    pub address: String,
    /// Port
    pub port: u16,
    /// Features
    pub features: Vec<String>,
    /// Last seen
    pub last_seen: DateTime<Utc>,
    /// Peer score
    pub score: f64,
    /// Connection status
    pub status: PeerStatus,
}

/// Peer status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PeerStatus {
    /// Unknown peer
    Unknown,
    /// Peer is reachable
    Reachable,
    /// Peer is unreachable
    Unreachable,
    /// Peer is connected
    Connected,
}

/// Peerstore for managing peer information
#[derive(Debug)]
pub struct Peerstore {
    /// Known peers
    peers: Arc<RwLock<HashMap<String, PeerInfo>>>,
    /// Peer scores
    scores: Arc<RwLock<HashMap<String, f64>>>,
    /// Peer bans
    bans: Arc<RwLock<HashMap<String, DateTime<Utc>>>>,
    /// Maximum peers
    max_peers: usize,
}

impl Peerstore {
    /// Create a new peerstore
    pub fn new(max_peers: usize) -> Self {
        Self {
            peers: Arc::new(RwLock::new(HashMap::new())),
            scores: Arc::new(RwLock::new(HashMap::new())),
            bans: Arc::new(RwLock::new(HashMap::new())),
            max_peers,
        }
    }

    /// Add a peer to the peerstore
    pub async fn add_peer(&self, peer_info: PeerInfo) -> Result<()> {
        let key = format!("{}:{}", peer_info.address, peer_info.port);
        
        let mut peers = self.peers.write().await;
        let mut scores = self.scores.write().await;
        
        if peers.len() < self.max_peers {
            peers.insert(key.clone(), peer_info.clone());
            scores.insert(key, 1.0); // Default score
            log::debug!("Added peer to peerstore: {}:{}", peer_info.address, peer_info.port);
        } else {
            log::warn!("Peerstore full, cannot add new peer");
        }
        
        Ok(())
    }

    /// Remove a peer from the peerstore
    pub async fn remove_peer(&self, address: &str, port: u16) -> Result<()> {
        let key = format!("{}:{}", address, port);
        
        let mut peers = self.peers.write().await;
        let mut scores = self.scores.write().await;
        let mut bans = self.bans.write().await;
        
        peers.remove(&key);
        scores.remove(&key);
        bans.remove(&key);
        
        log::debug!("Removed peer from peerstore: {}:{}", address, port);
        Ok(())
    }

    /// Update peer score
    pub async fn update_peer_score(&self, address: &str, port: u16, score: f64) -> Result<()> {
        let key = format!("{}:{}", address, port);
        let mut scores = self.scores.write().await;
        
        scores.insert(key, score.max(0.0).min(1.0)); // Clamp between 0 and 1
        log::debug!("Updated peer score: {}:{} -> {}", address, port, score);
        
        Ok(())
    }

    /// Ban a peer
    pub async fn ban_peer(&self, address: &str, port: u16, duration_minutes: i64) -> Result<()> {
        let key = format!("{}:{}", address, port);
        let mut bans = self.bans.write().await;
        
        let ban_until = Utc::now() + chrono::Duration::minutes(duration_minutes);
        bans.insert(key, ban_until);
        
        log::info!("Banned peer: {}:{} until {}", address, port, ban_until);
        Ok(())
    }

    /// Check if peer is banned
    pub async fn is_peer_banned(&self, address: &str, port: u16) -> bool {
        let key = format!("{}:{}", address, port);
        let bans = self.bans.read().await;
        
        if let Some(ban_until) = bans.get(&key) {
            if Utc::now() < *ban_until {
                return true;
            }
        }
        
        false
    }

    /// Get peer score
    pub async fn get_peer_score(&self, address: &str, port: u16) -> f64 {
        let key = format!("{}:{}", address, port);
        let scores = self.scores.read().await;
        
        scores.get(&key).copied().unwrap_or(0.0)
    }

    /// Get all peers
    pub async fn get_all_peers(&self) -> Vec<PeerInfo> {
        let peers = self.peers.read().await;
        peers.values().cloned().collect()
    }

    /// Get peers with minimum score
    pub async fn get_peers_with_min_score(&self, min_score: f64) -> Vec<PeerInfo> {
        let peers = self.peers.read().await;
        let scores = self.scores.read().await;
        
        peers.iter()
            .filter(|(key, _)| {
                scores.get(*key).copied().unwrap_or(0.0) >= min_score
            })
            .map(|(_, peer)| peer.clone())
            .collect()
    }

    /// Clean up expired bans
    pub async fn cleanup_expired_bans(&self) {
        let mut bans = self.bans.write().await;
        let now = Utc::now();
        
        bans.retain(|_, ban_until| *ban_until > now);
    }
}

/// Discovery service
#[derive(Debug)]
pub struct DiscoveryService {
    /// Node ID
    node_id: [u8; 32],
    /// Peerstore for managing peers
    peerstore: Arc<Peerstore>,
    /// Bootstrap peers
    bootstrap_peers: Vec<SocketAddr>,
    /// Discovery interval
    discovery_interval: Duration,
    /// Peer timeout
    _peer_timeout: Duration,
    /// Maximum peers
    max_peers: usize,
    /// Message sender
    message_sender: broadcast::Sender<DiscoveryMessage>,
    /// Message receiver
    _message_receiver: broadcast::Receiver<DiscoveryMessage>,
    /// Running flag
    running: bool,
}

impl Default for DiscoveryService {
    fn default() -> Self {
        let (message_sender, _message_receiver) = broadcast::channel(1000);
        Self {
            node_id: [0u8; 32],
            peerstore: Arc::new(Peerstore::new(100)),
            bootstrap_peers: Vec::new(),
            discovery_interval: Duration::from_secs(30),
            _peer_timeout: Duration::from_secs(60),
            max_peers: 100,
            message_sender,
            _message_receiver,
            running: false,
        }
    }
}

impl DiscoveryService {
    /// Create a new discovery service
    pub async fn new(
        node_id: [u8; 32],
        bootstrap_peers: Vec<SocketAddr>,
    ) -> Result<Self> {
        let (message_sender, message_receiver) = broadcast::channel(1000);
        
        Ok(Self {
            node_id,
            peerstore: Arc::new(Peerstore::new(100)), // Initialize with a default max_peers
            bootstrap_peers,
            discovery_interval: Duration::from_secs(60), // 1 minute
            _peer_timeout: Duration::from_secs(300), // 5 minutes
            max_peers: 100,
            message_sender,
            _message_receiver: message_receiver,
            running: false,
        })
    }

    /// Start the discovery service
    pub async fn start(&mut self) -> Result<()> {
        log::info!("Discovery service temporarily disabled");
        return Ok(());
        // log::info!("Starting peer discovery service");
        // self.running = true;
        
        // Start discovery loop
        let discovery_interval = self.discovery_interval;
        let peerstore = self.peerstore.clone();
        let bootstrap_peers = self.bootstrap_peers.clone();
        
        tokio::spawn(async move {
            let mut interval = interval(discovery_interval);
            
            loop {
                interval.tick().await;
                
                // Discover new peers
                Self::discover_peers(&peerstore, &bootstrap_peers).await;
                
                // Clean up stale peers
                Self::cleanup_stale_peers(&peerstore).await;
            }
        });
        
        // Start message processing loop
        let message_sender = self.message_sender.clone();
        let peerstore = self.peerstore.clone();
        
        tokio::spawn(async move {
            let mut message_receiver = message_sender.subscribe();
            
            while let Ok(message) = message_receiver.recv().await {
                Self::handle_discovery_message(message, &peerstore).await;
            }
        });
        
        Ok(())
    }

    /// Stop the discovery service
    pub async fn stop(&mut self) -> Result<()> {
        log::info!("Stopping peer discovery service");
        self.running = false;
        Ok(())
    }

    /// Announce this node to the network
    pub async fn announce_node(&self, address: String, port: u16, features: Vec<String>) -> Result<()> {
        let announcement = PeerAnnouncement {
            node_id: self.node_id,
            address,
            port,
            features,
            timestamp: Utc::now(),
            signature: None, // TODO: Add signature
        };
        
        let message = DiscoveryMessage::PeerAnnouncement(announcement);
        
        // Broadcast announcement to known peers
        self.broadcast_message(message).await?;
        
        Ok(())
    }

    /// Request peers from the network
    pub async fn request_peers(&self, max_peers: u32) -> Result<()> {
        let request = PeerRequest {
            node_id: self.node_id,
            max_peers,
            timestamp: Utc::now(),
        };
        
        let message = DiscoveryMessage::PeerRequest(request);
        
        // Send request to known peers
        self.broadcast_message(message).await?;
        
        Ok(())
    }

    /// Add a peer manually
    pub async fn add_peer(&self, peer_info: PeerInfo) -> Result<()> {
        self.peerstore.add_peer(peer_info).await
    }

    /// Remove a peer
    pub async fn remove_peer(&self, address: &str, port: u16) -> Result<()> {
        self.peerstore.remove_peer(address, port).await
    }

    /// Get known peers
    pub async fn get_known_peers(&self) -> Vec<PeerInfo> {
        self.peerstore.get_all_peers().await
    }

    /// Get reachable peers
    pub async fn get_reachable_peers(&self) -> Vec<PeerInfo> {
        self.peerstore.get_peers_with_min_score(0.5).await // Assuming a minimum score of 0.5 for reachable
    }

    /// Get peer count
    pub async fn get_peer_count(&self) -> usize {
        let peers = self.peerstore.get_all_peers().await;
        peers.len()
    }

    /// Broadcast a discovery message
    async fn broadcast_message(&self, message: DiscoveryMessage) -> Result<()> {
        // TODO: Implement actual broadcasting to peers
        log::debug!("Broadcasting discovery message: {:?}", message);
        Ok(())
    }

    /// Discover new peers
    async fn discover_peers(
        peerstore: &Arc<Peerstore>,
        bootstrap_peers: &[SocketAddr],
    ) {
        // TODO: Implement peer discovery logic
        log::debug!("Discovering new peers...");
        
        // For now, just add bootstrap peers if not already known
        for bootstrap_peer in bootstrap_peers {
            let peer_info = PeerInfo {
                node_id: [0u8; 32], // Unknown node ID
                address: bootstrap_peer.ip().to_string(),
                port: bootstrap_peer.port(),
                features: vec!["bootstrap".to_string()],
                last_seen: Utc::now(),
                score: 0.5,
                status: PeerStatus::Unknown,
            };
            
            if let Err(e) = peerstore.add_peer(peer_info).await {
                log::warn!("Failed to add bootstrap peer: {}", e);
            } else {
                log::info!("Added bootstrap peer: {}:{}", bootstrap_peer.ip(), bootstrap_peer.port());
            }
        }
    }

    /// Clean up stale peers
    async fn cleanup_stale_peers(peerstore: &Arc<Peerstore>) {
        let peers = peerstore.get_all_peers().await;
        let now = Utc::now();
        let timeout = chrono::Duration::minutes(10); // 10 minutes
        
        for peer in peers {
            let time_diff = now - peer.last_seen;
            if time_diff > timeout {
                if let Err(e) = peerstore.remove_peer(&peer.address, peer.port).await {
                    log::warn!("Failed to remove stale peer: {}", e);
                } else {
                    log::debug!("Removed stale peer: {}:{}", peer.address, peer.port);
                }
            }
        }
    }

    /// Handle discovery message
    pub async fn handle_discovery_message(
        message: DiscoveryMessage,
        peerstore: &Arc<Peerstore>,
    ) {
        match message {
            DiscoveryMessage::PeerAnnouncement(announcement) => {
                log::info!("Received peer announcement from: {}:{}", announcement.address, announcement.port);
                
                let peer_info = PeerInfo {
                    node_id: announcement.node_id,
                    address: announcement.address,
                    port: announcement.port,
                    features: announcement.features,
                    last_seen: Utc::now(),
                    score: 1.0,
                    status: PeerStatus::Reachable,
                };
                
                if let Err(e) = peerstore.add_peer(peer_info).await {
                    log::warn!("Failed to add peer from announcement: {}", e);
                }
            }
            
            DiscoveryMessage::PeerRequest(request) => {
                log::debug!("Received peer request from: {:?}", request.node_id);
                
                // TODO: Send peer response with known peers
            }
            
            DiscoveryMessage::PeerResponse(response) => {
                log::debug!("Received peer response with {} peers", response.peers.len());
                
                for peer_info in response.peers {
                    if let Err(e) = peerstore.add_peer(peer_info).await {
                        log::warn!("Failed to add peer from response: {}", e);
                    }
                }
            }
            
            DiscoveryMessage::Ping(ping) => {
                log::debug!("Received ping from: {:?}", ping.node_id);
                
                // TODO: Send pong response
            }
            
            DiscoveryMessage::Pong(pong) => {
                log::debug!("Received pong from: {:?}", pong.node_id);
                
                // Update peer status - this would need to be implemented with a way to identify the peer
                // For now, we'll just log it
                log::debug!("Peer {} is reachable", hex::encode(pong.node_id));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_discovery_service_creation() {
        use std::net::{IpAddr, Ipv4Addr};
        let node_id = [1u8; 32];
        let bootstrap_peers = vec![
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8081),
        ];
        
        let service = DiscoveryService::new(node_id, bootstrap_peers).await.unwrap();
        
        assert_eq!(service.max_peers, 100);
        assert_eq!(service.bootstrap_peers.len(), 2);
    }

    #[tokio::test]
    async fn test_peer_announcement() {
        let announcement = PeerAnnouncement {
            node_id: [1u8; 32],
            address: "127.0.0.1".to_string(),
            port: 8080,
            features: vec!["blocks".to_string(), "transactions".to_string()],
            timestamp: Utc::now(),
            signature: None,
        };
        
        let message = DiscoveryMessage::PeerAnnouncement(announcement);
        let serialized = serde_json::to_vec(&message).unwrap();
        let deserialized: DiscoveryMessage = serde_json::from_slice(&serialized).unwrap();
        
        assert!(matches!(deserialized, DiscoveryMessage::PeerAnnouncement(_)));
    }

    #[tokio::test]
    async fn test_peer_info() {
        let peer_info = PeerInfo {
            node_id: [1u8; 32],
            address: "127.0.0.1".to_string(),
            port: 8080,
            features: vec!["blocks".to_string()],
            last_seen: Utc::now(),
            score: 1.0,
            status: PeerStatus::Reachable,
        };
        
        assert_eq!(peer_info.address, "127.0.0.1");
        assert_eq!(peer_info.port, 8080);
        assert_eq!(peer_info.status, PeerStatus::Reachable);
    }
}
