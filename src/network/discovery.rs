//! Network discovery module
//! 
//! Handles peer discovery and connection management.

use crate::{error::IppanError, NodeId, Result};
use libp2p::{PeerId, Multiaddr};
use std::collections::{HashMap, HashSet};
use tokio::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Discovery manager for finding and connecting to peers
pub struct DiscoveryManager {
    /// Known peers
    known_peers: HashMap<PeerId, PeerInfo>,
    /// Bootstrap peers
    bootstrap_peers: HashSet<PeerId>,
    /// Discovery configuration
    config: DiscoveryConfig,
    /// Last discovery attempt
    last_discovery: Option<Instant>,
    /// Discovery interval
    discovery_interval: Duration,
}

/// Peer information for discovery
#[derive(Debug, Clone)]
pub struct PeerInfo {
    /// Peer ID
    pub peer_id: PeerId,
    /// Node ID (IPPAN specific)
    pub node_id: NodeId,
    /// Multiaddrs
    pub addrs: Vec<Multiaddr>,
    /// Last seen timestamp
    pub last_seen: chrono::DateTime<chrono::Utc>,
    /// Connection attempts
    pub connection_attempts: u32,
    /// Last connection attempt
    pub last_attempt: Option<chrono::DateTime<chrono::Utc>>,
    /// Peer score (for ranking)
    pub score: f64,
}

/// Discovery configuration
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    /// Maximum known peers
    pub max_known_peers: usize,
    /// Discovery interval
    pub discovery_interval: Duration,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Maximum connection attempts
    pub max_connection_attempts: u32,
    /// Retry delay
    pub retry_delay: Duration,
    /// Bootstrap peers
    pub bootstrap_peers: Vec<Multiaddr>,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            max_known_peers: 1000,
            discovery_interval: Duration::from_secs(60),
            connection_timeout: Duration::from_secs(30),
            max_connection_attempts: 3,
            retry_delay: Duration::from_secs(300), // 5 minutes
            bootstrap_peers: vec![],
        }
    }
}

impl DiscoveryManager {
    /// Create a new discovery manager
    pub fn new(config: DiscoveryConfig) -> Self {
        let mut manager = Self {
            known_peers: HashMap::new(),
            bootstrap_peers: HashSet::new(),
            config,
            last_discovery: None,
            discovery_interval: Duration::from_secs(60),
        };
        
        // Add bootstrap peers
        for addr in &manager.config.bootstrap_peers {
            if let Ok(peer_id) = addr.extract_peer_id() {
                manager.bootstrap_peers.insert(peer_id);
            }
        }
        
        manager
    }
    
    /// Add a discovered peer
    pub fn add_peer(&mut self, peer_id: PeerId, node_id: NodeId, addrs: Vec<Multiaddr>) {
        let now = chrono::Utc::now();
        
        let peer_info = PeerInfo {
            peer_id,
            node_id,
            addrs,
            last_seen: now,
            connection_attempts: 0,
            last_attempt: None,
            score: 1.0,
        };
        
        self.known_peers.insert(peer_id, peer_info);
        
        // Limit the number of known peers
        if self.known_peers.len() > self.config.max_known_peers {
            self.prune_peers();
        }
    }
    
    /// Update peer information
    pub fn update_peer(&mut self, peer_id: &PeerId, addrs: Vec<Multiaddr>) {
        if let Some(peer_info) = self.known_peers.get_mut(peer_id) {
            peer_info.addrs = addrs;
            peer_info.last_seen = chrono::Utc::now();
            peer_info.score += 0.1; // Increase score for successful updates
        }
    }
    
    /// Mark connection attempt
    pub fn mark_connection_attempt(&mut self, peer_id: &PeerId, success: bool) {
        if let Some(peer_info) = self.known_peers.get_mut(peer_id) {
            peer_info.last_attempt = Some(chrono::Utc::now());
            
            if success {
                peer_info.connection_attempts = 0;
                peer_info.score += 0.5;
            } else {
                peer_info.connection_attempts += 1;
                peer_info.score -= 0.2;
            }
        }
    }
    
    /// Get peers to connect to
    pub fn get_peers_to_connect(&self, max_peers: usize) -> Vec<PeerInfo> {
        let mut candidates: Vec<_> = self.known_peers.values()
            .filter(|peer| {
                // Filter out peers with too many failed attempts
                peer.connection_attempts < self.config.max_connection_attempts &&
                // Filter out recently attempted peers
                peer.last_attempt.map_or(true, |last| {
                    chrono::Utc::now().signed_duration_since(last) > 
                    chrono::Duration::from_std(self.config.retry_delay).unwrap()
                })
            })
            .cloned()
            .collect();
        
        // Sort by score (highest first)
        candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        
        candidates.into_iter().take(max_peers).collect()
    }
    
    /// Get bootstrap peers
    pub fn get_bootstrap_peers(&self) -> Vec<PeerInfo> {
        self.known_peers.values()
            .filter(|peer| self.bootstrap_peers.contains(&peer.peer_id))
            .cloned()
            .collect()
    }
    
    /// Remove a peer
    pub fn remove_peer(&mut self, peer_id: &PeerId) {
        self.known_peers.remove(peer_id);
        self.bootstrap_peers.remove(peer_id);
    }
    
    /// Prune old or low-scoring peers
    fn prune_peers(&mut self) {
        let now = chrono::Utc::now();
        let cutoff = now - chrono::Duration::hours(24); // Remove peers not seen in 24 hours
        
        self.known_peers.retain(|_, peer| {
            peer.last_seen > cutoff && peer.score > 0.0
        });
        
        // If still too many, remove lowest scoring peers
        if self.known_peers.len() > self.config.max_known_peers {
            let mut peers: Vec<_> = self.known_peers.drain().collect();
            peers.sort_by(|(_, a), (_, b)| a.score.partial_cmp(&b.score).unwrap());
            
            // Keep the highest scoring peers
            let keep_count = self.config.max_known_peers;
            for (peer_id, peer_info) in peers.into_iter().rev().take(keep_count) {
                self.known_peers.insert(peer_id, peer_info);
            }
        }
    }
    
    /// Check if discovery is due
    pub fn should_discover(&self) -> bool {
        self.last_discovery.map_or(true, |last| {
            last.elapsed() >= self.discovery_interval
        })
    }
    
    /// Mark discovery attempt
    pub fn mark_discovery_attempt(&mut self) {
        self.last_discovery = Some(Instant::now());
    }
    
    /// Get discovery statistics
    pub fn get_stats(&self) -> DiscoveryStats {
        let total_peers = self.known_peers.len();
        let bootstrap_peers = self.bootstrap_peers.len();
        let connectable_peers = self.known_peers.values()
            .filter(|peer| peer.connection_attempts < self.config.max_connection_attempts)
            .count();
        
        DiscoveryStats {
            total_peers,
            bootstrap_peers,
            connectable_peers,
            max_known_peers: self.config.max_known_peers,
        }
    }
    
    /// Get peer by ID
    pub fn get_peer(&self, peer_id: &PeerId) -> Option<&PeerInfo> {
        self.known_peers.get(peer_id)
    }
    
    /// Get all known peers
    pub fn get_all_peers(&self) -> &HashMap<PeerId, PeerInfo> {
        &self.known_peers
    }
    
    /// Add bootstrap peer
    pub fn add_bootstrap_peer(&mut self, peer_id: PeerId) {
        self.bootstrap_peers.insert(peer_id);
    }
    
    /// Remove bootstrap peer
    pub fn remove_bootstrap_peer(&mut self, peer_id: &PeerId) {
        self.bootstrap_peers.remove(peer_id);
    }
}

/// Discovery statistics
#[derive(Debug, Clone)]
pub struct DiscoveryStats {
    /// Total known peers
    pub total_peers: usize,
    /// Bootstrap peers
    pub bootstrap_peers: usize,
    /// Connectable peers
    pub connectable_peers: usize,
    /// Maximum known peers
    pub max_known_peers: usize,
}

/// Peer discovery service
pub struct PeerDiscoveryService {
    /// Discovery manager
    discovery_manager: DiscoveryManager,
    /// Network manager reference
    network_manager: Option<Box<dyn NetworkManagerTrait>>,
}

/// Trait for network manager operations
pub trait NetworkManagerTrait {
    fn dial_peer(&mut self, peer_id: PeerId, addr: Multiaddr) -> Result<()>;
    fn bootstrap(&mut self) -> Result<()>;
}

impl PeerDiscoveryService {
    /// Create a new peer discovery service
    pub fn new(config: DiscoveryConfig) -> Self {
        Self {
            discovery_manager: DiscoveryManager::new(config),
            network_manager: None,
        }
    }
    
    /// Set network manager
    pub fn set_network_manager(&mut self, network_manager: Box<dyn NetworkManagerTrait>) {
        self.network_manager = Some(network_manager);
    }
    
    /// Start discovery service
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting peer discovery service");
        
        // Initial bootstrap
        if let Some(ref mut network_manager) = self.network_manager {
            network_manager.bootstrap()?;
        }
        
        // Start discovery loop
        self.run_discovery_loop().await?;
        
        Ok(())
    }
    
    /// Run discovery loop
    async fn run_discovery_loop(&mut self) -> Result<()> {
        let mut interval = tokio::time::interval(self.discovery_manager.config.discovery_interval);
        
        loop {
            interval.tick().await;
            
            if self.discovery_manager.should_discover() {
                self.perform_discovery().await?;
                self.discovery_manager.mark_discovery_attempt();
            }
            
            // Attempt connections to discovered peers
            self.attempt_connections().await?;
        }
    }
    
    /// Perform peer discovery
    async fn perform_discovery(&mut self) -> Result<()> {
        debug!("Performing peer discovery");
        
        // Bootstrap Kademlia
        if let Some(ref mut network_manager) = self.network_manager {
            network_manager.bootstrap()?;
        }
        
        // In a real implementation, you'd also:
        // - Query DHT for peers
        // - Exchange peer lists with connected peers
        // - Use MDNS for local discovery
        
        Ok(())
    }
    
    /// Attempt connections to discovered peers
    async fn attempt_connections(&mut self) -> Result<()> {
        let peers_to_connect = self.discovery_manager.get_peers_to_connect(5);
        
        for peer_info in peers_to_connect {
            if let Some(ref mut network_manager) = self.network_manager {
                for addr in &peer_info.addrs {
                    match network_manager.dial_peer(peer_info.peer_id, addr.clone()) {
                        Ok(_) => {
                            self.discovery_manager.mark_connection_attempt(&peer_info.peer_id, true);
                            break; // Successfully connected, try next peer
                        }
                        Err(e) => {
                            warn!("Failed to connect to {} at {}: {}", peer_info.peer_id, addr, e);
                            self.discovery_manager.mark_connection_attempt(&peer_info.peer_id, false);
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Get discovery manager
    pub fn discovery_manager(&self) -> &DiscoveryManager {
        &self.discovery_manager
    }
    
    /// Get mutable discovery manager
    pub fn discovery_manager_mut(&mut self) -> &mut DiscoveryManager {
        &mut self.discovery_manager
    }
}

impl std::fmt::Display for PeerInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Peer({}, score: {:.2}, attempts: {}, last_seen: {})",
            self.peer_id,
            self.score,
            self.connection_attempts,
            self.last_seen.format("%Y-%m-%d %H:%M:%S")
        )
    }
}
