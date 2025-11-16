//! Peer discovery service for IPPAN network
//!
//! Handles automatic peer discovery, peer exchange, and network topology management.

use anyhow::Result;
use ippan_types::{format_ratio, RatioMicros, RATIO_SCALE};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::interval;
use tracing::{debug, info, warn};

use crate::peers::Peer;

/// Discovery configuration
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    pub discovery_interval: Duration,
    pub peer_exchange_interval: Duration,
    pub max_peers: usize,
    pub min_peers: usize,
    pub bootstrap_peers: Vec<String>,
    pub discovery_timeout: Duration,
    pub peer_ttl: Duration,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            discovery_interval: Duration::from_secs(30),
            peer_exchange_interval: Duration::from_secs(60),
            max_peers: 50,
            min_peers: 5,
            bootstrap_peers: vec![],
            discovery_timeout: Duration::from_secs(10),
            peer_ttl: Duration::from_secs(3600), // 1 hour
        }
    }
}

/// Peer discovery service for IPPAN network.
///
/// Handles automatic peer discovery, peer exchange, and network topology management.
/// Currently operates independently from DHT-based discovery; future enhancements
/// will integrate with Kademlia DHT for distributed peer routing.
///
/// **Current capabilities:**
/// - Bootstrap peer management
/// - Peer exchange protocol
/// - Stale peer cleanup
/// - Reputation tracking
///
/// **Planned enhancements (see `docs/ipndht/ipndht_hardening_plan.md`):**
/// - DNS seed resolution
/// - Cold-start recovery with peer cache
/// - Minimum peer validation (2+ nodes)
/// - DHT-based peer advertising
pub struct PeerDiscovery {
    config: DiscoveryConfig,
    known_peers: Arc<RwLock<HashMap<String, DiscoveredPeer>>>,
    active_peers: Arc<RwLock<HashSet<String>>>,
    discovery_sender: mpsc::UnboundedSender<DiscoveryMessage>,
    discovery_receiver: Option<mpsc::UnboundedReceiver<DiscoveryMessage>>,
    is_running: Arc<std::sync::atomic::AtomicBool>,
}

/// Discovered peer information
#[derive(Debug, Clone)]
pub struct DiscoveredPeer {
    pub peer: Peer,
    pub discovered_at: Instant,
    pub last_seen: Instant,
    pub connection_attempts: usize,
    pub is_connected: bool,
    pub reputation_score_micros: RatioMicros,
    pub capabilities: Vec<String>,
}

impl DiscoveredPeer {
    pub fn new(peer: Peer) -> Self {
        let now = Instant::now();
        Self {
            peer,
            discovered_at: now,
            last_seen: now,
            connection_attempts: 0,
            is_connected: false,
            reputation_score_micros: RATIO_SCALE / 2, // Start with neutral reputation
            capabilities: vec![],
        }
    }

    pub fn is_stale(&self, config: &DiscoveryConfig) -> bool {
        self.last_seen.elapsed() > config.peer_ttl
    }

    pub fn should_retry_connection(&self) -> bool {
        !self.is_connected && self.connection_attempts < 3
    }
}

/// Discovery message types
#[derive(Debug, Clone)]
pub enum DiscoveryMessage {
    DiscoverPeers,
    PeerFound { peer: Peer },
    PeerLost { peer_id: String },
    PeerConnected { peer_id: String },
    PeerDisconnected { peer_id: String },
    ExchangePeers { peer_id: String },
    UpdateReputation { peer_id: String, score_micros: RatioMicros },
}

/// Discovery service implementation
impl PeerDiscovery {
    /// Create a new peer discovery service
    pub fn new(config: DiscoveryConfig) -> Self {
        let (discovery_sender, discovery_receiver) = mpsc::unbounded_channel();

        Self {
            config,
            known_peers: Arc::new(RwLock::new(HashMap::new())),
            active_peers: Arc::new(RwLock::new(HashSet::new())),
            discovery_sender,
            discovery_receiver: Some(discovery_receiver),
            is_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Start the discovery service
    pub async fn start(&mut self) -> Result<()> {
        self.is_running
            .store(true, std::sync::atomic::Ordering::SeqCst);

        // Start discovery tasks
        self.start_discovery_loop().await;
        self.start_peer_exchange_loop().await;
        self.start_cleanup_loop().await;
        self.start_message_handler().await;

        info!("Peer discovery service started");
        Ok(())
    }

    /// Stop the discovery service
    pub async fn stop(&mut self) -> Result<()> {
        self.is_running
            .store(false, std::sync::atomic::Ordering::SeqCst);
        info!("Peer discovery service stopped");
        Ok(())
    }

    /// Add a bootstrap peer
    pub async fn add_bootstrap_peer(&self, address: String) -> Result<()> {
        let peer = Peer::new(address.clone());
        let _ = self.add_peer(peer).await;
        Ok(())
    }

    /// Add a discovered peer
    pub async fn add_peer(&self, peer: Peer) -> Result<()> {
        let peer_id = peer.id.clone().unwrap_or_else(|| peer.address.clone());

        {
            let mut known_peers = self.known_peers.write();
            known_peers.insert(peer_id.clone(), DiscoveredPeer::new(peer.clone()));
        }

        self.discovery_sender
            .send(DiscoveryMessage::PeerFound { peer })?;
        Ok(())
    }

    /// Remove a peer
    pub async fn remove_peer(&self, peer_id: &str) -> Result<()> {
        {
            let mut known_peers = self.known_peers.write();
            known_peers.remove(peer_id);
        }

        {
            let mut active_peers = self.active_peers.write();
            active_peers.remove(peer_id);
        }

        self.discovery_sender.send(DiscoveryMessage::PeerLost {
            peer_id: peer_id.to_string(),
        })?;
        Ok(())
    }

    /// Get known peers
    pub fn get_known_peers(&self) -> Vec<DiscoveredPeer> {
        self.known_peers.read().values().cloned().collect()
    }

    /// Get active peers
    pub fn get_active_peers(&self) -> Vec<String> {
        self.active_peers.read().iter().cloned().collect()
    }

    /// Get peers for connection
    pub fn get_peers_for_connection(&self) -> Vec<Peer> {
        let known_peers = self.known_peers.read();
        let active_peers = self.active_peers.read();

        known_peers
            .values()
            .filter(|peer| {
                !peer.is_stale(&self.config)
                    && !active_peers.contains(peer.peer.id.as_ref().unwrap_or(&peer.peer.address))
                    && peer.should_retry_connection()
            })
            .map(|peer| peer.peer.clone())
            .collect()
    }

    /// Update peer reputation
    pub async fn update_peer_reputation(
        &self,
        peer_id: &str,
        score_micros: RatioMicros,
    ) -> Result<()> {
        {
            let mut known_peers = self.known_peers.write();
            if let Some(peer) = known_peers.get_mut(peer_id) {
                peer.reputation_score_micros = score_micros.min(RATIO_SCALE);
            }
        }

        self.discovery_sender
            .send(DiscoveryMessage::UpdateReputation {
                peer_id: peer_id.to_string(),
                score_micros: score_micros.min(RATIO_SCALE),
            })?;
        Ok(())
    }

    /// Mark peer as connected
    pub async fn mark_peer_connected(&self, peer_id: &str) -> Result<()> {
        {
            let mut known_peers = self.known_peers.write();
            if let Some(peer) = known_peers.get_mut(peer_id) {
                peer.is_connected = true;
                peer.last_seen = Instant::now();
            }
        }

        {
            let mut active_peers = self.active_peers.write();
            active_peers.insert(peer_id.to_string());
        }

        self.discovery_sender
            .send(DiscoveryMessage::PeerConnected {
                peer_id: peer_id.to_string(),
            })?;
        Ok(())
    }

    /// Mark peer as disconnected
    pub async fn mark_peer_disconnected(&self, peer_id: &str) -> Result<()> {
        {
            let mut known_peers = self.known_peers.write();
            if let Some(peer) = known_peers.get_mut(peer_id) {
                peer.is_connected = false;
                peer.connection_attempts += 1;
            }
        }

        {
            let mut active_peers = self.active_peers.write();
            active_peers.remove(peer_id);
        }

        self.discovery_sender
            .send(DiscoveryMessage::PeerDisconnected {
                peer_id: peer_id.to_string(),
            })?;
        Ok(())
    }

    /// Start the discovery loop
    async fn start_discovery_loop(&self) {
        let config = self.config.clone();
        let discovery_sender = self.discovery_sender.clone();
        let is_running = self.is_running.clone();

        tokio::spawn(async move {
            let mut interval = interval(config.discovery_interval);

            while is_running.load(std::sync::atomic::Ordering::SeqCst) {
                interval.tick().await;

                if let Err(e) = discovery_sender.send(DiscoveryMessage::DiscoverPeers) {
                    warn!("Failed to send discovery message: {}", e);
                }
            }
        });
    }

    /// Start the peer exchange loop
    async fn start_peer_exchange_loop(&self) {
        let config = self.config.clone();
        let _discovery_sender = self.discovery_sender.clone();
        let is_running = self.is_running.clone();

        tokio::spawn(async move {
            let mut interval = interval(config.peer_exchange_interval);

            while is_running.load(std::sync::atomic::Ordering::SeqCst) {
                interval.tick().await;

                // Exchange peers with connected peers
                // This would be implemented with actual peer communication
                debug!("Performing peer exchange");
            }
        });
    }

    /// Start the cleanup loop
    async fn start_cleanup_loop(&self) {
        let config = self.config.clone();
        let known_peers = self.known_peers.clone();
        let is_running = self.is_running.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(300)); // 5 minutes

            while is_running.load(std::sync::atomic::Ordering::SeqCst) {
                interval.tick().await;

                // Remove stale peers
                let mut peers_to_remove = Vec::new();
                {
                    let known_peers_guard = known_peers.read();
                    for (peer_id, peer) in known_peers_guard.iter() {
                        if peer.is_stale(&config) {
                            peers_to_remove.push(peer_id.clone());
                        }
                    }
                }

                if !peers_to_remove.is_empty() {
                    let mut known_peers_guard = known_peers.write();
                    for peer_id in peers_to_remove {
                        known_peers_guard.remove(&peer_id);
                        debug!("Removed stale peer: {}", peer_id);
                    }
                }
            }
        });
    }

    /// Start the message handler
    async fn start_message_handler(&mut self) {
        let mut discovery_receiver = self.discovery_receiver.take().unwrap();
        let is_running = self.is_running.clone();

        tokio::spawn(async move {
            while is_running.load(std::sync::atomic::Ordering::SeqCst) {
                if let Some(message) = discovery_receiver.recv().await {
                    match message {
                        DiscoveryMessage::DiscoverPeers => {
                            Self::handle_discover_peers().await;
                        }
                        DiscoveryMessage::PeerFound { peer } => {
                            debug!("Peer found: {}", peer.address);
                        }
                        DiscoveryMessage::PeerLost { peer_id } => {
                            debug!("Peer lost: {}", peer_id);
                        }
                        DiscoveryMessage::PeerConnected { peer_id } => {
                            debug!("Peer connected: {}", peer_id);
                        }
                        DiscoveryMessage::PeerDisconnected { peer_id } => {
                            debug!("Peer disconnected: {}", peer_id);
                        }
                        DiscoveryMessage::ExchangePeers { peer_id } => {
                            debug!("Exchanging peers with: {}", peer_id);
                        }
                        DiscoveryMessage::UpdateReputation {
                            peer_id,
                            score_micros,
                        } => {
                            debug!(
                                "Updated reputation for {}: {}",
                                peer_id,
                                format_ratio(score_micros)
                            );
                        }
                    }
                }
            }
        });
    }

    /// Handle peer discovery
    async fn handle_discover_peers() {
        // In a real implementation, this would:
        // 1. Query DNS seeds
        // 2. Contact bootstrap peers
        // 3. Use DHT for peer discovery
        // 4. Exchange peer lists with connected peers
        debug!("Discovering new peers");
    }
}

/// Type alias for DiscoveryService
pub type DiscoveryService = PeerDiscovery;

/// Discovery service statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiscoveryStats {
    pub total_known_peers: usize,
    pub active_peers: usize,
    pub connected_peers: usize,
    pub discovery_attempts: u64,
    pub successful_discoveries: u64,
    pub peer_exchanges: u64,
}

impl PeerDiscovery {
    /// Get discovery statistics
    pub fn get_stats(&self) -> DiscoveryStats {
        let known_peers = self.known_peers.read();
        let active_peers = self.active_peers.read();

        let connected_peers = known_peers
            .values()
            .filter(|peer| peer.is_connected)
            .count();

        DiscoveryStats {
            total_known_peers: known_peers.len(),
            active_peers: active_peers.len(),
            connected_peers,
            discovery_attempts: 0, // Would be tracked in real implementation
            successful_discoveries: 0,
            peer_exchanges: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_peer_discovery_creation() {
        let config = DiscoveryConfig::default();
        let discovery = PeerDiscovery::new(config);
        assert_eq!(discovery.get_known_peers().len(), 0);
    }

    #[tokio::test]
    async fn test_add_peer() {
        let config = DiscoveryConfig::default();
        let discovery = PeerDiscovery::new(config);
        let peer = Peer::new("127.0.0.1:8080".to_string());

        assert!(discovery.add_peer(peer).await.is_ok());
        assert_eq!(discovery.get_known_peers().len(), 1);
    }

    #[tokio::test]
    async fn test_peer_reputation() {
        let config = DiscoveryConfig::default();
        let discovery = PeerDiscovery::new(config);
        let peer = Peer::with_id("test-peer", "127.0.0.1:8080");

        discovery.add_peer(peer).await.unwrap();
        assert!(discovery
            .update_peer_reputation("test-peer", RATIO_SCALE * 8 / 10)
            .await
            .is_ok());
    }
}
