use crate::block::Block;
use crate::error::Result;
use crate::transaction::Transaction;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{info, warn};

// Network topics
pub const TX_GOSSIP_TOPIC: &str = "ippan-tx-gossip";
pub const BLOCK_GOSSIP_TOPIC: &str = "ippan-block-gossip";
pub const ROUND_GOSSIP_TOPIC: &str = "ippan-round-gossip";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    Transaction(Transaction),
    Block(Block),
    RoundData(Vec<u8>), // Serialized round data
    TimeSync(u64),      // IPPAN time sync
}

#[derive(Clone)]
pub struct NetworkManager {
    tx_sender: mpsc::UnboundedSender<NetworkMessage>,
    peers: Arc<RwLock<HashMap<String, PeerInfo>>>,
    metrics: Arc<crate::metrics::Metrics>,
}

#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub peer_id: String,
    pub addresses: Vec<String>,
    pub last_seen: std::time::Instant,
    pub is_connected: bool,
}

impl NetworkManager {
    pub async fn new(
        _keypair: libp2p::identity::Keypair,
        metrics: Arc<crate::metrics::Metrics>,
    ) -> Result<Self> {
        info!("Starting simplified network manager");

        // Create message channels
        let (tx_sender, _tx_receiver) = mpsc::unbounded_channel();

        Ok(Self {
            tx_sender,
            peers: Arc::new(RwLock::new(HashMap::new())),
            metrics,
        })
    }

    pub async fn start(&mut self, _listen_addr: &str) -> Result<()> {
        info!("Network manager started (simplified mode)");
        Ok(())
    }

    pub fn get_message_sender(&self) -> mpsc::UnboundedSender<NetworkMessage> {
        self.tx_sender.clone()
    }

    pub async fn get_peer_count(&self) -> usize {
        self.peers.read().await.len()
    }

    pub async fn add_bootstrap_peer(&mut self, _peer_addr: &str) -> Result<()> {
        info!("Bootstrap peer added (simplified mode)");
        Ok(())
    }

    async fn add_peer(&self, peer_id: String) {
        let mut peers = self.peers.write().await;
        peers.insert(peer_id.clone(), PeerInfo {
            peer_id,
            addresses: Vec::new(),
            last_seen: std::time::Instant::now(),
            is_connected: true,
        });
    }

    async fn remove_peer(&self, peer_id: String) {
        let mut peers = self.peers.write().await;
        peers.remove(&peer_id);
    }
}
