use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::RwLock;
use tokio::sync::broadcast;
use tokio::time::timeout;

use ippan_types::{Block, BlockId, IppanTimeMicros, Transaction};

const ALL_TOPICS: [GossipTopic; 3] = [
    GossipTopic::Transactions,
    GossipTopic::Blocks,
    GossipTopic::DagVertices,
];

/// Configuration for [`ParallelGossipNetwork`].
#[derive(Debug, Clone)]
pub struct GossipConfig {
    /// Capacity of the broadcast channels backing each topic.
    pub channel_capacity: usize,
    /// Maximum time allowed when publishing with [`publish_with_timeout`].
    pub publish_timeout: Duration,
}

impl Default for GossipConfig {
    fn default() -> Self {
        Self {
            channel_capacity: 512,
            publish_timeout: Duration::from_millis(200),
        }
    }
}

/// Topics used by the parallel gossip network.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GossipTopic {
    Transactions,
    Blocks,
    DagVertices,
}

/// Structured payload carried by [`GossipMessage`].
#[derive(Debug, Clone)]
pub enum GossipPayload {
    Transaction(Transaction),
    Block(Block),
    DagMetadata(DagVertexAnnouncement),
}

/// Lightweight announcement used to describe DAG vertices without shipping the
/// full block payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DagVertexAnnouncement {
    pub block_id: BlockId,
    pub parent_ids: Vec<BlockId>,
    pub round: u64,
    pub width_hint: usize,
}

/// Message published on the gossip network.
#[derive(Debug, Clone)]
pub struct GossipMessage {
    pub topic: GossipTopic,
    pub payload: GossipPayload,
    pub timestamp_us: u64,
}

impl GossipMessage {
    pub fn new(topic: GossipTopic, payload: GossipPayload) -> Self {
        Self {
            topic,
            payload,
            timestamp_us: IppanTimeMicros::now().0,
        }
    }
}

/// Errors that can occur while using the parallel gossip network.
#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum GossipError {
    #[error("topic not registered")]
    UnknownTopic,
    #[error("publish timeout reached")]
    PublishTimeout,
}

#[derive(Debug)]
struct GossipMetrics {
    published: AtomicU64,
    dropped: AtomicU64,
    subscribers: AtomicU64,
}

impl Default for GossipMetrics {
    fn default() -> Self {
        Self {
            published: AtomicU64::new(0),
            dropped: AtomicU64::new(0),
            subscribers: AtomicU64::new(0),
        }
    }
}

impl GossipMetrics {
    fn on_publish(&self, delivered: usize) {
        if delivered > 0 {
            self.published
                .fetch_add(delivered as u64, Ordering::Relaxed);
        } else {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn on_subscribe(&self) {
        self.subscribers.fetch_add(1, Ordering::Relaxed);
    }

    fn snapshot(&self) -> GossipMetricsSnapshot {
        GossipMetricsSnapshot {
            published: self.published.load(Ordering::Relaxed),
            dropped: self.dropped.load(Ordering::Relaxed),
            subscribers: self.subscribers.load(Ordering::Relaxed),
        }
    }
}

/// Read-only metrics snapshot for external consumers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GossipMetricsSnapshot {
    pub published: u64,
    pub dropped: u64,
    pub subscribers: u64,
}

/// QUIC-inspired gossip fabric built atop Tokio broadcast channels. The
/// abstraction keeps the rest of the codebase agnostic to the transport layer
/// while providing observability hooks.
#[derive(Debug)]
pub struct ParallelGossipNetwork {
    config: GossipConfig,
    channels: RwLock<HashMap<GossipTopic, broadcast::Sender<GossipMessage>>>,
    metrics: Arc<GossipMetrics>,
}

impl ParallelGossipNetwork {
    pub fn new(config: GossipConfig) -> Self {
        let metrics = Arc::new(GossipMetrics::default());
        let channels = RwLock::new(HashMap::new());
        let network = Self {
            config,
            channels,
            metrics,
        };
        network.initialise_topics();
        network
    }

    fn initialise_topics(&self) {
        let mut guard = self.channels.write();
        for topic in ALL_TOPICS {
            guard.entry(topic).or_insert_with(|| {
                let (sender, _receiver) = broadcast::channel(self.config.channel_capacity);
                sender
            });
        }
    }

    fn ensure_topic(
        &self,
        topic: GossipTopic,
    ) -> Result<broadcast::Sender<GossipMessage>, GossipError> {
        if let Some(sender) = self.channels.read().get(&topic) {
            return Ok(sender.clone());
        }

        let mut guard = self.channels.write();
        let sender = guard.entry(topic).or_insert_with(|| {
            let (sender, _receiver) = broadcast::channel(self.config.channel_capacity);
            sender
        });
        Ok(sender.clone())
    }

    pub fn subscribe(
        &self,
        topic: GossipTopic,
    ) -> Result<broadcast::Receiver<GossipMessage>, GossipError> {
        let sender = self.ensure_topic(topic)?;
        self.metrics.on_subscribe();
        Ok(sender.subscribe())
    }

    pub fn publish(&self, message: GossipMessage) -> Result<usize, GossipError> {
        let sender = self.ensure_topic(message.topic)?;
        match sender.send(message) {
            Ok(delivered) => {
                self.metrics.on_publish(delivered);
                Ok(delivered)
            }
            Err(_err) => {
                self.metrics.on_publish(0);
                Ok(0)
            }
        }
    }

    pub async fn publish_with_timeout(&self, message: GossipMessage) -> Result<usize, GossipError> {
        let timeout_duration = self.config.publish_timeout;
        match timeout(timeout_duration, async { self.publish(message) }).await {
            Ok(result) => result,
            Err(_) => Err(GossipError::PublishTimeout),
        }
    }

    pub fn broadcast_block(&self, block: Block) -> Result<usize, GossipError> {
        self.publish(GossipMessage::new(
            GossipTopic::Blocks,
            GossipPayload::Block(block),
        ))
    }

    pub fn broadcast_transaction(&self, tx: Transaction) -> Result<usize, GossipError> {
        self.publish(GossipMessage::new(
            GossipTopic::Transactions,
            GossipPayload::Transaction(tx),
        ))
    }

    pub fn broadcast_dag_vertex(
        &self,
        announcement: DagVertexAnnouncement,
    ) -> Result<usize, GossipError> {
        self.publish(GossipMessage::new(
            GossipTopic::DagVertices,
            GossipPayload::DagMetadata(announcement),
        ))
    }

    pub fn metrics(&self) -> GossipMetricsSnapshot {
        self.metrics.snapshot()
    }
}

#[cfg(all(test, feature = "enable-tests"))]
mod tests {
    use super::*;
    use ippan_types::{Amount, RoundId, ValidatorId};
    use rand::{rngs::StdRng, RngCore, SeedableRng};
    use std::time::Duration;

    fn random_validator(rng: &mut StdRng) -> ValidatorId {
        let mut bytes = [0u8; 32];
        rng.fill_bytes(&mut bytes);
        bytes
    }

    fn build_transaction(rng: &mut StdRng, nonce: u64) -> Transaction {
        let from = random_validator(rng);
        let mut to = random_validator(rng);
        if to == from {
            to[0] ^= 0xAA;
        }
        let mut tx = Transaction::new(from, to, Amount(1), nonce);
        tx.id = tx.hash();
        tx
    }

    fn build_block(rng: &mut StdRng, round: RoundId, parents: Vec<BlockId>) -> Block {
        let tx = build_transaction(rng, round + 11);
        let creator = random_validator(rng);
        Block::new(parents, vec![tx], round, creator)
    }

    #[tokio::test]
    async fn broadcast_block_reaches_subscribers() {
        let network = ParallelGossipNetwork::new(GossipConfig::default());
        let mut rng = StdRng::seed_from_u64(321);
        let block = build_block(&mut rng, 4, Vec::new());

        let mut sub_a = network.subscribe(GossipTopic::Blocks).unwrap();
        let mut sub_b = network.subscribe(GossipTopic::Blocks).unwrap();

        network.broadcast_block(block.clone()).unwrap();

        let received_a = sub_a.recv().await.unwrap();
        let received_b = sub_b.recv().await.unwrap();

        match received_a.payload {
            GossipPayload::Block(inner) => assert_eq!(inner.hash(), block.hash()),
            other => panic!("Expected Block payload, got: {:?}", other),
        }

        match received_b.payload {
            GossipPayload::Block(inner) => assert_eq!(inner.hash(), block.hash()),
            other => panic!("Expected Block payload, got: {:?}", other),
        }

        let metrics = network.metrics();
        assert_eq!(metrics.subscribers, 2);
        assert!(metrics.published >= 2);
    }

    #[tokio::test]
    async fn dag_metadata_round_trip() {
        let network = ParallelGossipNetwork::new(GossipConfig::default());
        let mut subscriber = network.subscribe(GossipTopic::DagVertices).unwrap();

        let announcement = DagVertexAnnouncement {
            block_id: [1u8; 32],
            parent_ids: vec![[2u8; 32], [3u8; 32]],
            round: 12,
            width_hint: 4,
        };

        network.broadcast_dag_vertex(announcement.clone()).unwrap();

        let received = subscriber.recv().await.unwrap();
        match received.payload {
            GossipPayload::DagMetadata(meta) => assert_eq!(meta, announcement),
            other => panic!("Expected DagMetadata payload, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn timeout_wrapper_returns_ok() {
        let config = GossipConfig {
            publish_timeout: Duration::from_micros(1),
            ..Default::default()
        };
        let network = ParallelGossipNetwork::new(config);
        let mut rng = StdRng::seed_from_u64(8080);
        let block = build_block(&mut rng, 5, Vec::new());

        let result = network
            .publish_with_timeout(GossipMessage::new(
                GossipTopic::Blocks,
                GossipPayload::Block(block),
            ))
            .await;

        assert!(result.is_ok());
        let metrics = network.metrics();
        assert_eq!(metrics.dropped, 1);
    }
}
