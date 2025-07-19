//! Proof broadcast for zk-STARK roundchain
//!
//! Broadcasts: (round_header, zk_proof, merkle_root) to all peers
//! Includes retry/fallback mechanism for partial sync

use crate::Result;
use super::{RoundHeader, ZkStarkProof, RoundAggregation, MerkleTree};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error};

/// Broadcast configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastConfig {
    /// Enable proof broadcasting
    pub enable_broadcasting: bool,
    /// Maximum broadcast payload size in bytes
    pub max_payload_size: usize,
    /// Broadcast timeout in milliseconds
    pub broadcast_timeout_ms: u64,
    /// Number of retry attempts
    pub retry_attempts: u32,
    /// Retry delay in milliseconds
    pub retry_delay_ms: u64,
    /// Enable fallback sync mode
    pub enable_fallback_sync: bool,
    /// Target propagation latency in milliseconds (sub-second)
    pub target_propagation_latency_ms: u64,
}

impl Default for BroadcastConfig {
    fn default() -> Self {
        Self {
            enable_broadcasting: true,
            max_payload_size: 200_000, // 200 KB max
            broadcast_timeout_ms: 500, // 500ms timeout
            retry_attempts: 3,
            retry_delay_ms: 100,
            enable_fallback_sync: true,
            target_propagation_latency_ms: 180, // 180ms for intercontinental
        }
    }
}

/// Broadcast message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BroadcastMessage {
    /// Round aggregation with zk-STARK proof
    RoundAggregation(RoundAggregation),
    /// Round header only (for lightweight sync)
    RoundHeader(RoundHeader),
    /// zk-STARK proof only (for verification)
    ZkProof(ZkStarkProof),
    /// Merkle tree for transaction inclusion proofs
    MerkleTree(MerkleTree),
    /// Fallback sync request
    FallbackSyncRequest(u64), // round number
    /// Fallback sync response
    FallbackSyncResponse(RoundAggregation),
}

/// Broadcast statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastStats {
    /// Round number
    pub round_number: u64,
    /// Broadcast time in milliseconds
    pub broadcast_time_ms: u64,
    /// Propagation latency in milliseconds
    pub propagation_latency_ms: u64,
    /// Number of peers reached
    pub peers_reached: u32,
    /// Total peers
    pub total_peers: u32,
    /// Success status
    pub success: bool,
    /// Error message if failed
    pub error_message: Option<String>,
}

/// Peer information for broadcasting
#[derive(Debug, Clone)]
pub struct Peer {
    /// Peer ID
    pub id: [u8; 32],
    /// Peer address
    pub address: String,
    /// Peer latency in milliseconds
    pub latency_ms: u64,
    /// Whether peer is online
    pub online: bool,
    /// Last successful broadcast
    pub last_successful_broadcast: Option<u64>,
}

/// Proof broadcaster for zk-STARK rounds
#[derive(Debug)]
pub struct ProofBroadcaster {
    /// Configuration
    config: BroadcastConfig,
    /// Connected peers
    peers: Arc<RwLock<HashMap<[u8; 32], Peer>>>,
    /// Broadcast statistics
    broadcast_stats: Arc<RwLock<HashMap<u64, BroadcastStats>>>,
    /// Pending broadcasts
    pending_broadcasts: Arc<RwLock<HashMap<u64, RoundAggregation>>>,
}

impl ProofBroadcaster {
    /// Create a new proof broadcaster
    pub fn new(config: BroadcastConfig) -> Self {
        Self {
            config,
            peers: Arc::new(RwLock::new(HashMap::new())),
            broadcast_stats: Arc::new(RwLock::new(HashMap::new())),
            pending_broadcasts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a peer for broadcasting
    pub async fn add_peer(&self, peer: Peer) {
        let mut peers = self.peers.write().await;
        let peer_id = peer.id;
        peers.insert(peer_id, peer);
        
        debug!("Added peer for broadcasting: {:?}", peer_id);
    }

    /// Remove a peer from broadcasting
    pub async fn remove_peer(&self, peer_id: &[u8; 32]) {
        let mut peers = self.peers.write().await;
        peers.remove(peer_id);
        
        debug!("Removed peer from broadcasting: {:?}", peer_id);
    }

    /// Broadcast round aggregation to all peers
    pub async fn broadcast_round_aggregation(&self, aggregation: RoundAggregation) -> Result<()> {
        if !self.config.enable_broadcasting {
            return Ok(());
        }

        let round_number = aggregation.header.round_number;
        info!("Broadcasting round aggregation {} to peers", round_number);

        let start_time = std::time::Instant::now();

        // Store pending broadcast
        {
            let mut pending = self.pending_broadcasts.write().await;
            pending.insert(round_number, aggregation.clone());
        }

        // Get all peers
        let peers = self.peers.read().await;
        let total_peers = peers.len() as u32;
        let mut peers_reached = 0;

        // Broadcast to each peer
        for (peer_id, peer) in peers.iter() {
            if !peer.online {
                continue;
            }

            match self.broadcast_to_peer(peer, &aggregation).await {
                Ok(_) => {
                    peers_reached += 1;
                    debug!("Successfully broadcast to peer: {:?}", peer_id);
                }
                Err(e) => {
                    warn!("Failed to broadcast to peer {:?}: {}", peer_id, e);
                }
            }
        }

        let broadcast_time = start_time.elapsed().as_millis() as u64;

        // Record statistics
        self.record_broadcast_stats(
            round_number,
            broadcast_time,
            peers_reached,
            total_peers,
            peers_reached > 0,
            None,
        ).await;

        info!(
            "Broadcast completed: {} ms, {}/{} peers reached",
            broadcast_time,
            peers_reached,
            total_peers
        );

        Ok(())
    }

    /// Broadcast to a specific peer with retry logic
    async fn broadcast_to_peer(&self, peer: &Peer, aggregation: &RoundAggregation) -> Result<()> {
        let mut attempts = 0;
        let max_attempts = self.config.retry_attempts;

        while attempts < max_attempts {
            match self.send_to_peer(peer, aggregation).await {
                Ok(_) => return Ok(()),
                Err(e) => {
                    attempts += 1;
                    if attempts < max_attempts {
                        warn!(
                            "Broadcast attempt {} failed for peer {:?}: {}, retrying...",
                            attempts,
                            peer.id,
                            e
                        );
                        
                        // Wait before retry
                        tokio::time::sleep(
                            std::time::Duration::from_millis(self.config.retry_delay_ms)
                        ).await;
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        Err(crate::IppanError::Network(
            format!("Failed to broadcast to peer after {} attempts", max_attempts)
        ))
    }

    /// Send aggregation to a specific peer
    async fn send_to_peer(&self, peer: &Peer, aggregation: &RoundAggregation) -> Result<()> {
        // TODO: Implement actual network sending
        // For now, simulate network transmission
        
        // Check payload size
        let payload_size = self.calculate_payload_size(aggregation);
        if payload_size > self.config.max_payload_size {
            return Err(crate::IppanError::Validation(
                format!("Payload size {} exceeds maximum {}", payload_size, self.config.max_payload_size)
            ));
        }

        // Simulate network latency
        let latency = peer.latency_ms;
        tokio::time::sleep(std::time::Duration::from_millis(latency)).await;

        // Simulate occasional network failures
        if rand::random::<f32>() < 0.1 { // 10% failure rate
            return Err(crate::IppanError::Network("Simulated network failure".to_string()));
        }

        debug!("Sent aggregation to peer {:?} (latency: {} ms)", peer.id, latency);
        Ok(())
    }

    /// Calculate payload size for aggregation
    fn calculate_payload_size(&self, aggregation: &RoundAggregation) -> usize {
        // Estimate size of round aggregation
        let header_size = std::mem::size_of::<RoundHeader>();
        let proof_size = aggregation.zk_proof.proof_size;
        let merkle_size = aggregation.merkle_tree.nodes.len() * 32; // 32 bytes per hash
        let tx_hashes_size = aggregation.transaction_hashes.len() * 32;
        
        header_size + proof_size + merkle_size + tx_hashes_size
    }

    /// Broadcast lightweight round header only
    pub async fn broadcast_round_header(&self, header: RoundHeader) -> Result<()> {
        if !self.config.enable_broadcasting {
            return Ok(());
        }

        info!("Broadcasting lightweight round header {}", header.round_number);

        let peers = self.peers.read().await;
        let mut success_count = 0;

        for (peer_id, peer) in peers.iter() {
            if !peer.online {
                continue;
            }

            match self.send_header_to_peer(peer, &header).await {
                Ok(_) => {
                    success_count += 1;
                    debug!("Successfully sent header to peer: {:?}", peer_id);
                }
                Err(e) => {
                    warn!("Failed to send header to peer {:?}: {}", peer_id, e);
                }
            }
        }

        info!("Header broadcast completed: {}/{} peers", success_count, peers.len());
        Ok(())
    }

    /// Send header to a specific peer
    async fn send_header_to_peer(&self, peer: &Peer, header: &RoundHeader) -> Result<()> {
        // TODO: Implement actual header sending
        // Simulate network transmission
        tokio::time::sleep(std::time::Duration::from_millis(peer.latency_ms)).await;
        Ok(())
    }

    /// Handle fallback sync request
    pub async fn handle_fallback_sync_request(&self, round_number: u64) -> Result<Option<RoundAggregation>> {
        if !self.config.enable_fallback_sync {
            return Ok(None);
        }

        info!("Handling fallback sync request for round {}", round_number);

        // Check if we have the aggregation locally
        let pending = self.pending_broadcasts.read().await;
        if let Some(aggregation) = pending.get(&round_number) {
            debug!("Found aggregation for round {} in pending broadcasts", round_number);
            return Ok(Some(aggregation.clone()));
        }

        // TODO: Check local storage for historical aggregations
        warn!("No aggregation found for round {} in fallback sync", round_number);
        Ok(None)
    }

    /// Request fallback sync from peers
    pub async fn request_fallback_sync(&self, round_number: u64) -> Result<Option<RoundAggregation>> {
        if !self.config.enable_fallback_sync {
            return Ok(None);
        }

        info!("Requesting fallback sync for round {}", round_number);

        let peers = self.peers.read().await;
        
        for (peer_id, peer) in peers.iter() {
            if !peer.online {
                continue;
            }

            match self.request_sync_from_peer(peer, round_number).await {
                Ok(Some(aggregation)) => {
                    info!("Received fallback sync from peer {:?} for round {}", peer_id, round_number);
                    return Ok(Some(aggregation));
                }
                Ok(None) => {
                    debug!("Peer {:?} does not have round {}", peer_id, round_number);
                }
                Err(e) => {
                    warn!("Failed to request sync from peer {:?}: {}", peer_id, e);
                }
            }
        }

        warn!("No peer could provide fallback sync for round {}", round_number);
        Ok(None)
    }

    /// Request sync from a specific peer
    async fn request_sync_from_peer(&self, peer: &Peer, round_number: u64) -> Result<Option<RoundAggregation>> {
        // TODO: Implement actual sync request
        // Simulate network request
        tokio::time::sleep(std::time::Duration::from_millis(peer.latency_ms)).await;
        
        // Simulate peer response (random success)
        if rand::random::<f32>() < 0.3 { // 30% chance of having the data
            // Return a placeholder aggregation
            let header = RoundHeader::new(round_number, [0u8; 32], [0u8; 32], 0, [0u8; 32]);
            let proof = ZkStarkProof {
                proof_data: vec![],
                proof_size: 0,
                proving_time_ms: 0,
                verification_time_ms: 0,
                round_number,
                transaction_count: 0,
            };
            
            let aggregation = RoundAggregation {
                header,
                zk_proof: proof,
                transaction_hashes: vec![],
                merkle_tree: MerkleTree::new(vec![]),
            };
            
            return Ok(Some(aggregation));
        }
        
        Ok(None)
    }

    /// Record broadcast statistics
    async fn record_broadcast_stats(
        &self,
        round_number: u64,
        broadcast_time: u64,
        peers_reached: u32,
        total_peers: u32,
        success: bool,
        error_message: Option<String>,
    ) {
        let stats = BroadcastStats {
            round_number,
            broadcast_time_ms: broadcast_time,
            propagation_latency_ms: broadcast_time, // Simplified
            peers_reached,
            total_peers,
            success,
            error_message,
        };

        let mut broadcast_stats = self.broadcast_stats.write().await;
        broadcast_stats.insert(round_number, stats);
    }

    /// Get broadcast statistics for a round
    pub async fn get_broadcast_stats(&self, round_number: u64) -> Option<BroadcastStats> {
        let stats = self.broadcast_stats.read().await;
        stats.get(&round_number).cloned()
    }

    /// Get all broadcast statistics
    pub async fn get_all_broadcast_stats(&self) -> Vec<BroadcastStats> {
        let stats = self.broadcast_stats.read().await;
        stats.values().cloned().collect()
    }

    /// Get peer count
    pub async fn get_peer_count(&self) -> usize {
        let peers = self.peers.read().await;
        peers.len()
    }

    /// Get online peer count
    pub async fn get_online_peer_count(&self) -> usize {
        let peers = self.peers.read().await;
        peers.values().filter(|p| p.online).count()
    }

    /// Update peer latency
    pub async fn update_peer_latency(&self, peer_id: &[u8; 32], latency_ms: u64) {
        let mut peers = self.peers.write().await;
        if let Some(peer) = peers.get_mut(peer_id) {
            peer.latency_ms = latency_ms;
            debug!("Updated peer {:?} latency to {} ms", peer_id, latency_ms);
        }
    }

    /// Update peer online status
    pub async fn update_peer_status(&self, peer_id: &[u8; 32], online: bool) {
        let mut peers = self.peers.write().await;
        if let Some(peer) = peers.get_mut(peer_id) {
            peer.online = online;
            debug!("Updated peer {:?} status to online: {}", peer_id, online);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::{blockdag::Transaction, hashtimer::HashTimer};

    #[tokio::test]
    async fn test_broadcaster_creation() {
        let config = BroadcastConfig::default();
        let broadcaster = ProofBroadcaster::new(config);
        
        assert!(broadcaster.config.enable_broadcasting);
        assert_eq!(broadcaster.config.target_propagation_latency_ms, 180);
    }

    #[tokio::test]
    async fn test_peer_management() {
        let config = BroadcastConfig::default();
        let broadcaster = ProofBroadcaster::new(config);
        
        let peer = Peer {
            id: [1u8; 32],
            address: "127.0.0.1:8080".to_string(),
            latency_ms: 50,
            online: true,
            last_successful_broadcast: None,
        };
        
        broadcaster.add_peer(peer).await;
        assert_eq!(broadcaster.get_peer_count().await, 1);
        assert_eq!(broadcaster.get_online_peer_count().await, 1);
    }

    #[tokio::test]
    async fn test_round_header_broadcast() {
        let config = BroadcastConfig::default();
        let broadcaster = ProofBroadcaster::new(config);
        
        let header = RoundHeader::new(1, [1u8; 32], [2u8; 32], 1234567890, [3u8; 32]);
        
        // Should succeed even with no peers
        broadcaster.broadcast_round_header(header).await.unwrap();
    }
} 