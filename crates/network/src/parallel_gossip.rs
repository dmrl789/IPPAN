use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::{future::join_all, stream, StreamExt};
use ippan_types::{Block, IppanTimeMicros};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio::time::timeout;
use tracing::{debug, warn};
use sha2::{Sha256, Digest};

use crate::peers::PeerDirectory;
use crate::deduplication::MessageDeduplicator;
use crate::reputation::ReputationManager;
use crate::metrics::NetworkMetrics;
use crate::health::HealthMonitor;

const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_millis(200);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[allow(clippy::large_enum_variant)]
pub enum GossipMessage {
    Block(Box<Block>),
    Transactions { id: String, txs: Vec<Value> },
}

/// Parallel gossip relay for block and transaction propagation.
/// Each broadcast batch is sent concurrently to all connected peers.
/// 
/// Production features:
/// - Message deduplication to prevent rebroadcasting
/// - Peer reputation tracking
/// - Network metrics collection
/// - Connection health monitoring
#[derive(Clone)]
pub struct ParallelGossip {
    peers: Arc<RwLock<PeerDirectory>>,
    connect_timeout: Duration,
    deduplicator: Arc<MessageDeduplicator>,
    reputation: Arc<ReputationManager>,
    metrics: Arc<NetworkMetrics>,
    health: Arc<HealthMonitor>,
}

impl ParallelGossip {
    pub fn new(peers: Arc<RwLock<PeerDirectory>>) -> Self {
        Self {
            peers,
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            deduplicator: Arc::new(MessageDeduplicator::default()),
            reputation: Arc::new(ReputationManager::default()),
            metrics: Arc::new(NetworkMetrics::default()),
            health: Arc::new(HealthMonitor::default()),
        }
    }

    pub fn with_timeout(peers: Arc<RwLock<PeerDirectory>>, connect_timeout: Duration) -> Self {
        Self {
            peers,
            connect_timeout,
            deduplicator: Arc::new(MessageDeduplicator::default()),
            reputation: Arc::new(ReputationManager::default()),
            metrics: Arc::new(NetworkMetrics::default()),
            health: Arc::new(HealthMonitor::default()),
        }
    }

    /// Get network metrics snapshot
    pub fn metrics(&self) -> crate::metrics::NetworkMetricsSnapshot {
        self.metrics.snapshot()
    }

    /// Get reputation score for a peer
    pub fn peer_reputation(&self, peer_address: &str) -> crate::reputation::ReputationScore {
        self.reputation.get_score(peer_address)
    }

    /// Get health status for a peer
    pub fn peer_health(&self, peer_address: &str) -> crate::health::PeerHealth {
        self.health.get_health(peer_address)
    }

    /// Compute message hash for deduplication
    fn compute_message_hash(payload: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(payload);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result[..]);
        hash
    }

    pub async fn broadcast_blocks_parallel(&self, blocks: Vec<Block>) {
        if blocks.is_empty() {
            return;
        }

        let peers_snapshot = {
            let peers = self.peers.read().await;
            peers.list_connected()
        };

        if peers_snapshot.is_empty() {
            return;
        }

        let peers_snapshot = Arc::new(peers_snapshot);
        let timeout = self.connect_timeout;

        stream::iter(blocks)
            .for_each_concurrent(None, |block| {
                let peers_snapshot = peers_snapshot.clone();
                async move {
                    let payload = match serde_json::to_vec(&GossipMessage::Block(Box::new(block.clone()))) {
                        Ok(bytes) => Arc::<[u8]>::from(bytes),
                        Err(err) => {
                            warn!(
                                target = "parallel_gossip",
                                error = %err,
                                "failed to serialize block gossip message",
                            );
                            return;
                        }
                    };

                    // Check for duplicate message
                    let message_hash = Self::compute_message_hash(&payload);
                    if !self.deduplicator.check_and_mark(message_hash) {
                        debug!(
                            target = "parallel_gossip",
                            "skipping duplicate block broadcast",
                        );
                        return;
                    }

                    let send_tasks = peers_snapshot.iter().map(|peer| {
                        let addr = peer.address.clone();
                        let payload = payload.clone();
                        let reputation = self.reputation.clone();
                        let metrics = self.metrics.clone();
                        let health = self.health.clone();
                        async move {
                            // Skip banned peers
                            if reputation.should_ban(&addr) {
                                debug!(
                                    target = "parallel_gossip",
                                    address = %addr,
                                    "skipping banned peer",
                                );
                                return;
                            }

                            let start = Instant::now();
                            match Self::send_to_peer(addr.clone(), payload.clone(), timeout).await {
                                Ok(()) => {
                                    metrics.record_message_sent(payload.len());
                                    metrics.record_latency(start.elapsed());
                                    reputation.record_success(&addr);
                                    health.record_success(&addr);
                                }
                                Err(error) => {
                                    warn!(
                                        target = "parallel_gossip",
                                        address = %addr,
                                        error = %error,
                                        "failed to send block gossip",
                                    );
                                    metrics.record_message_failed();
                                    reputation.record_failure(&addr);
                                    health.record_failure(&addr);
                                }
                            }
                        }
                    });

                    join_all(send_tasks).await;
                }
            })
            .await;
    }

    pub async fn broadcast_transactions_parallel<T>(&self, txs: Vec<T>)
    where
        T: Serialize + Send + Sync + Clone,
    {
        if txs.is_empty() {
            return;
        }

        let peers_snapshot = {
            let peers = self.peers.read().await;
            peers.list_connected()
        };

        if peers_snapshot.is_empty() {
            return;
        }

        let peers_snapshot = Arc::new(peers_snapshot);
        let timeout = self.connect_timeout;
        let chunks: Vec<Vec<T>> = txs.chunks(200).map(|chunk| chunk.to_vec()).collect();

        stream::iter(chunks)
            .for_each_concurrent(None, |batch| {
                let peers_snapshot = peers_snapshot.clone();
                async move {
                    let gossip_id = IppanTimeMicros::now().0.to_string();
                    let txs: Vec<Value> = batch
                        .iter()
                        .filter_map(|tx| match serde_json::to_value(tx) {
                            Ok(value) => Some(value),
                            Err(err) => {
                                warn!(
                                    target = "parallel_gossip",
                                    error = %err,
                                    "failed to serialize transaction for gossip",
                                );
                                None
                            }
                        })
                        .collect();

                    if txs.is_empty() {
                        return;
                    }

                    let payload = match serde_json::to_vec(&GossipMessage::Transactions {
                        id: gossip_id.clone(),
                        txs,
                    }) {
                        Ok(bytes) => Arc::<[u8]>::from(bytes),
                        Err(err) => {
                            warn!(
                                target = "parallel_gossip",
                                error = %err,
                                "failed to serialize transaction batch gossip message",
                            );
                            return;
                        }
                    };

                    // Check for duplicate message
                    let message_hash = Self::compute_message_hash(&payload);
                    if !self.deduplicator.check_and_mark(message_hash) {
                        debug!(
                            target = "parallel_gossip",
                            "skipping duplicate transaction batch broadcast",
                        );
                        return;
                    }

                    let send_tasks = peers_snapshot.iter().map(|peer| {
                        let addr = peer.address.clone();
                        let payload = payload.clone();
                        let reputation = self.reputation.clone();
                        let metrics = self.metrics.clone();
                        let health = self.health.clone();
                        async move {
                            // Skip banned peers
                            if reputation.should_ban(&addr) {
                                debug!(
                                    target = "parallel_gossip",
                                    address = %addr,
                                    "skipping banned peer",
                                );
                                return;
                            }

                            let start = Instant::now();
                            match Self::send_to_peer(addr.clone(), payload.clone(), timeout).await {
                                Ok(()) => {
                                    metrics.record_message_sent(payload.len());
                                    metrics.record_latency(start.elapsed());
                                    reputation.record_success(&addr);
                                    health.record_success(&addr);
                                }
                                Err(error) => {
                                    warn!(
                                        target = "parallel_gossip",
                                        address = %addr,
                                        error = %error,
                                        "failed to send transaction gossip",
                                    );
                                    metrics.record_message_failed();
                                    reputation.record_failure(&addr);
                                    health.record_failure(&addr);
                                }
                            }
                        }
                    });

                    join_all(send_tasks).await;
                }
            })
            .await;
    }

    async fn send_to_peer(
        addr: String,
        payload: Arc<[u8]>,
        timeout_duration: Duration,
    ) -> Result<(), String> {
        match timeout(timeout_duration, TcpStream::connect(&addr)).await {
            Ok(Ok(mut stream)) => {
                if let Err(err) = stream.write_all(&payload).await {
                    return Err(format!("write error {err}"));
                }
                Ok(())
            }
            Ok(Err(err)) => Err(format!("connect error {err}")),
            Err(_) => Err("timeout".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peers::PeerDirectory;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn empty_inputs_exit_early() {
        let directory = Arc::new(RwLock::new(PeerDirectory::new()));
        let gossip = ParallelGossip::new(directory);

        gossip.broadcast_blocks_parallel(Vec::new()).await;
        gossip
            .broadcast_transactions_parallel::<serde_json::Value>(Vec::new())
            .await;
    }
}
