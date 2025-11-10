//! IPPAN P2P stack — deterministic peer connectivity, gossip propagation,
//! and metadata tracking for DAG consensus nodes.
//!
//! This crate provides both HTTP-based and libp2p-based networking layers.
//! - **libp2p_network**: Production-grade, decentralized peer discovery and relay network
//!   featuring Kademlia DHT, GossipSub, mDNS, Relay, and DCUtR for NAT traversal.
//! - **parallel_gossip**: Concurrent gossip engine optimized for DAG-based consensus.
//!
//! Features:
//! - Deterministic peer connectivity
//! - Gossip propagation for DAG blocks and transactions
//! - Peer metadata tracking and announcements
//! - HTTP and libp2p integration with NAT/UPnP support
//! - Efficient discovery and message scheduling for consensus
//!
//! Used by IPPAN RPC services, gateway nodes, and consensus layers.

pub mod libp2p_network;
pub mod parallel_gossip;

pub use libp2p_network::{
    Libp2pCommand, Libp2pConfig, Libp2pEvent, Libp2pNetwork, DEFAULT_GOSSIP_TOPICS,
};

pub use parallel_gossip::{
    DagVertexAnnouncement, GossipConfig, GossipError, GossipMessage, GossipMetricsSnapshot,
    GossipPayload, GossipTopic, ParallelGossipNetwork,
};

use anyhow::{anyhow, Result};
use ippan_types::{ippan_time_now, Block, Transaction};
use parking_lot::{Mutex, RwLock};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;
use tracing::{debug, warn};

/// P2P network errors
#[derive(thiserror::Error, Debug)]
pub enum P2PError {
    #[error("Channel error: {0}")]
    Channel(#[from] Box<mpsc::error::SendError<NetworkMessage>>),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("URL error: {0}")]
    Url(#[from] url::ParseError),
    #[error("Peer error: {0}")]
    Peer(String),
}

impl From<mpsc::error::SendError<NetworkMessage>> for P2PError {
    fn from(err: mpsc::error::SendError<NetworkMessage>) -> Self {
        Self::Channel(Box::new(err))
    }
}

/// Network message types exchanged between peers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    Block(Block),
    Transaction(Transaction),
    BlockRequest {
        hash: [u8; 32],
    },
    BlockResponse(Block),
    PeerInfo {
        peer_id: String,
        addresses: Vec<String>,
        #[serde(default)]
        time_us: Option<u64>,
    },
    PeerDiscovery {
        peers: Vec<String>,
    },
}

/// Peer information snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub id: String,
    pub address: String,
    pub last_seen: u64,
    pub is_connected: bool,
}

/// Events emitted by the HTTP P2P network when inbound messages are processed.
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    Block {
        from: String,
        block: Block,
    },
    Transaction {
        from: String,
        transaction: Transaction,
    },
    PeerInfo {
        from: String,
        peer_id: String,
        addresses: Vec<String>,
        time_us: Option<u64>,
    },
    PeerDiscovery {
        from: String,
        peers: Vec<String>,
    },
    BlockRequest {
        from: String,
        hash: [u8; 32],
    },
    BlockResponse {
        from: String,
        block: Block,
    },
}

#[derive(Debug, Clone)]
struct PeerRecord {
    peer_id: Option<String>,
    address: String,
    last_seen: u64,
    last_announced_us: Option<u64>,
    observed_address: Option<String>,
    advertised_addresses: Vec<String>,
}

impl PeerRecord {
    fn new(address: String) -> Self {
        Self {
            peer_id: None,
            address: address.clone(),
            last_seen: ippan_time_now(),
            last_announced_us: None,
            observed_address: None,
            advertised_addresses: Vec::new(),
        }
    }
}

/// P2P network configuration
#[derive(Debug, Clone)]
pub struct P2PConfig {
    pub listen_address: String,
    pub bootstrap_peers: Vec<String>,
    pub max_peers: usize,
    pub peer_discovery_interval: Duration,
    pub message_timeout: Duration,
    pub retry_attempts: usize,
    pub public_host: Option<String>,
    pub enable_upnp: bool,
    pub external_ip_services: Vec<String>,
    pub peer_announce_interval: Duration,
}

impl Default for P2PConfig {
    fn default() -> Self {
        Self {
            listen_address: "http://0.0.0.0:9000".to_string(),
            bootstrap_peers: vec![],
            max_peers: 50,
            peer_discovery_interval: Duration::from_secs(30),
            message_timeout: Duration::from_secs(10),
            retry_attempts: 3,
            public_host: None,
            enable_upnp: false,
            external_ip_services: vec![
                "https://api.ipify.org".to_string(),
                "https://ifconfig.me/ip".to_string(),
            ],
            peer_announce_interval: Duration::from_secs(60),
        }
    }
}

// ---------------------------------------------------------------------------
// HTTP-based P2P Network Implementation
// ---------------------------------------------------------------------------

pub struct HttpP2PNetwork {
    config: P2PConfig,
    client: Client,
    peers: Arc<RwLock<HashSet<String>>>,
    peer_metadata: Arc<RwLock<HashMap<String, PeerRecord>>>,
    message_sender: mpsc::UnboundedSender<NetworkMessage>,
    message_receiver: Option<mpsc::UnboundedReceiver<NetworkMessage>>,
    peer_count: Arc<RwLock<usize>>,
    is_running: Arc<RwLock<bool>>,
    local_peer_id: String,
    listen_address: String,
    local_address: String,
    announce_address: Arc<RwLock<String>>,
    upnp_mapping_active: Arc<RwLock<bool>>,
    incoming_sender: mpsc::UnboundedSender<NetworkEvent>,
    incoming_receiver: Arc<Mutex<Option<mpsc::UnboundedReceiver<NetworkEvent>>>>,
    discovery_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl HttpP2PNetwork {
    /// Create a new HTTP P2P network manager.
    pub fn new(config: P2PConfig, listen_address: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(config.message_timeout)
            .build()
            .map_err(|err| anyhow!("failed to build HTTP client: {err}"))?;

        let (message_sender, message_receiver) = mpsc::unbounded_channel();
        let (incoming_sender, incoming_receiver) = mpsc::unbounded_channel();

        let peers = Arc::new(RwLock::new(HashSet::new()));
        let peer_metadata = Arc::new(RwLock::new(HashMap::new()));
        let peer_count = Arc::new(RwLock::new(0usize));

        // Preload bootstrap peers so that discovery starts from them immediately.
        for peer in &config.bootstrap_peers {
            let _ = Self::add_peer_entry(
                &peers,
                &peer_metadata,
                &peer_count,
                config.max_peers,
                &listen_address,
                peer,
            );
        }

        let announce_address = config
            .public_host
            .clone()
            .unwrap_or_else(|| listen_address.clone());

        Ok(Self {
            config,
            client,
            peers,
            peer_metadata,
            message_sender,
            message_receiver: Some(message_receiver),
            peer_count,
            is_running: Arc::new(RwLock::new(false)),
            local_peer_id: format!("http-{}", listen_address),
            listen_address: listen_address.clone(),
            local_address: listen_address,
            announce_address: Arc::new(RwLock::new(announce_address)),
            upnp_mapping_active: Arc::new(RwLock::new(false)),
            incoming_sender,
            incoming_receiver: Arc::new(Mutex::new(Some(incoming_receiver))),
            discovery_task: Arc::new(Mutex::new(None)),
        })
    }

    /// Start background tasks for peer discovery and message dispatch.
    pub async fn start(&mut self) -> Result<()> {
        if *self.is_running.read() {
            return Ok(());
        }

        if let Some(receiver) = self.message_receiver.take() {
            self.spawn_message_dispatch(receiver);
        }

        *self.is_running.write() = true;
        self.run_discovery_iteration().await.ok();
        let handle = self.spawn_peer_discovery_task();
        *self.discovery_task.lock() = Some(handle);

        Ok(())
    }

    /// Stop the background tasks.
    pub async fn stop(&self) -> Result<()> {
        {
            let mut running = self.is_running.write();
            if !*running {
                return Ok(());
            }
            *running = false;
        }

        if let Some(handle) = self.discovery_task.lock().take() {
            handle.abort();
        }

        Ok(())
    }

    /// Returns the peer ID used to identify the local node.
    pub fn get_local_peer_id(&self) -> String {
        self.local_peer_id.clone()
    }

    /// Return the address the network listens on.
    pub fn listen_address(&self) -> &str {
        &self.listen_address
    }

    /// Expose the current announce address for diagnostics.
    pub fn announce_address(&self) -> String {
        self.announce_address.read().clone()
    }

    /// Indicates whether UPnP port mapping is active.
    pub fn is_upnp_mapping_active(&self) -> bool {
        *self.upnp_mapping_active.read()
    }

    /// Provide a sender for broadcasting network messages.
    pub fn message_sender(&self) -> mpsc::UnboundedSender<NetworkMessage> {
        self.message_sender.clone()
    }

    /// Returns the number of currently connected peers.
    pub fn get_peer_count(&self) -> usize {
        *self.peer_count.read()
    }

    /// Returns a snapshot of the known peer addresses.
    pub fn get_peers(&self) -> Vec<String> {
        self.peers.read().iter().cloned().collect()
    }

    /// Returns metadata describing every known peer.
    pub fn get_peer_metadata(&self) -> Vec<PeerInfo> {
        let peers = self.peers.read();
        self.peer_metadata
            .read()
            .values()
            .map(|record| PeerInfo {
                id: record
                    .peer_id
                    .clone()
                    .unwrap_or_else(|| record.address.clone()),
                address: record.address.clone(),
                last_seen: record.last_seen,
                is_connected: peers.contains(&record.address),
            })
            .collect()
    }

    /// Take the incoming event channel if it is still available.
    pub fn take_incoming_events(&self) -> Option<mpsc::UnboundedReceiver<NetworkEvent>> {
        self.incoming_receiver.lock().take()
    }

    /// Add a peer to the topology.
    pub async fn add_peer(&self, address: String) -> Result<()> {
        Self::add_peer_entry(
            &self.peers,
            &self.peer_metadata,
            &self.peer_count,
            self.config.max_peers,
            &self.local_address,
            &address,
        )?;
        Ok(())
    }

    /// Remove a peer from the topology and clear its metadata.
    pub fn remove_peer(&self, address: &str) {
        Self::remove_peer_entry(&self.peers, &self.peer_metadata, &self.peer_count, address);
    }

    /// Process an inbound message from a remote peer.
    pub async fn process_incoming_message(
        &self,
        from: &str,
        message: NetworkMessage,
    ) -> Result<()> {
        Self::add_peer_entry(
            &self.peers,
            &self.peer_metadata,
            &self.peer_count,
            self.config.max_peers,
            &self.local_address,
            from,
        )?;

        let now = ippan_time_now();
        Self::touch_peer_record(&self.peer_metadata, from, now);

        match message.clone() {
            NetworkMessage::Block(block) => {
                self.dispatch_event(NetworkEvent::Block {
                    from: from.to_string(),
                    block,
                });
            }
            NetworkMessage::Transaction(transaction) => {
                self.dispatch_event(NetworkEvent::Transaction {
                    from: from.to_string(),
                    transaction,
                });
            }
            NetworkMessage::BlockRequest { hash } => {
                self.dispatch_event(NetworkEvent::BlockRequest {
                    from: from.to_string(),
                    hash,
                });
            }
            NetworkMessage::BlockResponse(block) => {
                self.dispatch_event(NetworkEvent::BlockResponse {
                    from: from.to_string(),
                    block,
                });
            }
            NetworkMessage::PeerInfo {
                peer_id,
                addresses,
                time_us,
            } => {
                self.update_peer_record(from, Some(peer_id.clone()), time_us, &addresses);
                for address in &addresses {
                    let _ = Self::add_peer_entry(
                        &self.peers,
                        &self.peer_metadata,
                        &self.peer_count,
                        self.config.max_peers,
                        &self.local_address,
                        address,
                    );
                }
                self.dispatch_event(NetworkEvent::PeerInfo {
                    from: from.to_string(),
                    peer_id,
                    addresses,
                    time_us,
                });
            }
            NetworkMessage::PeerDiscovery { peers } => {
                let mut discovered = Vec::new();
                for peer in &peers {
                    if let Ok(inserted) = Self::add_peer_entry(
                        &self.peers,
                        &self.peer_metadata,
                        &self.peer_count,
                        self.config.max_peers,
                        &self.local_address,
                        peer,
                    ) {
                        if inserted {
                            discovered.push(peer.clone());
                        }
                    }
                }

                if !peers.is_empty() {
                    self.dispatch_event(NetworkEvent::PeerDiscovery {
                        from: from.to_string(),
                        peers: peers.clone(),
                    });
                }

                // Ensure we keep metadata entries for peers discovered through gossip.
                for peer in discovered {
                    Self::touch_peer_record(&self.peer_metadata, &peer, now);
                }
            }
        }

        Ok(())
    }

    fn dispatch_event(&self, event: NetworkEvent) {
        if let Err(err) = self.incoming_sender.send(event.clone()) {
            warn!("failed to dispatch network event {event:?}: {err}");
        }
    }

    fn spawn_message_dispatch(&self, mut receiver: mpsc::UnboundedReceiver<NetworkMessage>) {
        let peers = self.peers.clone();
        let client = self.client.clone();
        let config = self.config.clone();
        tokio::spawn(async move {
            while let Some(message) = receiver.recv().await {
                let targets: Vec<String> = peers.read().iter().cloned().collect();
                for peer in targets {
                    let endpoint = format!("{}/p2p/messages", peer.trim_end_matches('/'));
                    if let Err(err) = client
                        .post(endpoint)
                        .json(&message)
                        .timeout(config.message_timeout)
                        .send()
                        .await
                    {
                        debug!("failed to deliver message to {peer}: {err}");
                    }
                }
            }
        });
    }

    fn spawn_peer_discovery_task(&self) -> tokio::task::JoinHandle<()> {
        let client = self.client.clone();
        let peers = self.peers.clone();
        let peer_metadata = self.peer_metadata.clone();
        let peer_count = self.peer_count.clone();
        let incoming_sender = self.incoming_sender.clone();
        let config = self.config.clone();
        let is_running = self.is_running.clone();
        let local_address = self.local_address.clone();

        tokio::spawn(async move {
            let mut ticker = interval(config.peer_discovery_interval);
            loop {
                ticker.tick().await;
                if !*is_running.read() {
                    break;
                }

                let _ = Self::discover_from_known_peers(
                    &client,
                    &peers,
                    &peer_metadata,
                    &peer_count,
                    &incoming_sender,
                    &config,
                    &local_address,
                )
                .await;
            }
        })
    }

    async fn run_discovery_iteration(&self) -> Result<()> {
        Self::discover_from_known_peers(
            &self.client,
            &self.peers,
            &self.peer_metadata,
            &self.peer_count,
            &self.incoming_sender,
            &self.config,
            &self.local_address,
        )
        .await
    }

    async fn discover_from_known_peers(
        client: &Client,
        peers: &Arc<RwLock<HashSet<String>>>,
        peer_metadata: &Arc<RwLock<HashMap<String, PeerRecord>>>,
        peer_count: &Arc<RwLock<usize>>,
        incoming_sender: &mpsc::UnboundedSender<NetworkEvent>,
        config: &P2PConfig,
        local_address: &str,
    ) -> Result<()> {
        let mut targets: Vec<String> = config.bootstrap_peers.clone();
        for peer in peers.read().iter() {
            if !targets.contains(peer) {
                targets.push(peer.clone());
            }
        }

        for target in targets {
            let endpoint = format!("{}/p2p/peers", target.trim_end_matches('/'));
            match client
                .get(&endpoint)
                .timeout(config.message_timeout)
                .send()
                .await
            {
                Ok(response) => match response.json::<Vec<String>>().await {
                    Ok(discovered) => {
                        if !discovered.is_empty() {
                            let mut newly_added = Vec::new();
                            for peer in &discovered {
                                match Self::add_peer_entry(
                                    peers,
                                    peer_metadata,
                                    peer_count,
                                    config.max_peers,
                                    local_address,
                                    peer,
                                ) {
                                    Ok(inserted) if inserted => newly_added.push(peer.clone()),
                                    Ok(_) => {}
                                    Err(err) => {
                                        debug!(
                                            "failed to add discovered peer {peer} from {target}: {err}"
                                        );
                                    }
                                }
                            }

                            if let Err(err) = incoming_sender.send(NetworkEvent::PeerDiscovery {
                                from: target.clone(),
                                peers: discovered.clone(),
                            }) {
                                warn!("failed to enqueue discovery event from {target}: {err}");
                            }

                            for peer in newly_added {
                                Self::touch_peer_record(peer_metadata, &peer, ippan_time_now());
                            }
                        }
                    }
                    Err(err) => {
                        debug!("failed to decode peer list response from {endpoint}: {err}");
                    }
                },
                Err(err) => {
                    debug!("peer discovery request to {endpoint} failed: {err}");
                }
            }
        }

        Ok(())
    }

    fn add_peer_entry(
        peers: &Arc<RwLock<HashSet<String>>>,
        peer_metadata: &Arc<RwLock<HashMap<String, PeerRecord>>>,
        peer_count: &Arc<RwLock<usize>>,
        max_peers: usize,
        local_address: &str,
        address: &str,
    ) -> Result<bool> {
        if address == local_address {
            return Ok(false);
        }

        let mut guard = peers.write();
        if guard.contains(address) {
            return Ok(false);
        }

        if guard.len() >= max_peers {
            return Err(anyhow!("max peer count reached"));
        }

        guard.insert(address.to_string());
        *peer_count.write() = guard.len();
        drop(guard);

        peer_metadata
            .write()
            .entry(address.to_string())
            .or_insert_with(|| PeerRecord::new(address.to_string()))
            .last_seen = ippan_time_now();

        Ok(true)
    }

    fn remove_peer_entry(
        peers: &Arc<RwLock<HashSet<String>>>,
        peer_metadata: &Arc<RwLock<HashMap<String, PeerRecord>>>,
        peer_count: &Arc<RwLock<usize>>,
        address: &str,
    ) {
        let mut guard = peers.write();
        if guard.remove(address) {
            *peer_count.write() = guard.len();
        }
        drop(guard);

        peer_metadata.write().remove(address);
    }

    fn touch_peer_record(
        peer_metadata: &Arc<RwLock<HashMap<String, PeerRecord>>>,
        address: &str,
        time_us: u64,
    ) {
        let mut metadata = peer_metadata.write();
        let entry = metadata
            .entry(address.to_string())
            .or_insert_with(|| PeerRecord::new(address.to_string()));
        entry.last_seen = time_us;
    }

    fn update_peer_record(
        &self,
        address: &str,
        peer_id: Option<String>,
        announced_time: Option<u64>,
        advertised_addresses: &[String],
    ) {
        let mut metadata = self.peer_metadata.write();
        let record = metadata
            .entry(address.to_string())
            .or_insert_with(|| PeerRecord::new(address.to_string()));
        if let Some(id) = peer_id {
            record.peer_id = Some(id);
        }
        record.last_announced_us = announced_time;
        record.last_seen = announced_time.unwrap_or_else(ippan_time_now);
        if !advertised_addresses.is_empty() {
            record.advertised_addresses = advertised_addresses.to_vec();
            record.observed_address = Some(address.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_http_p2p_network_creation() {
        let config = P2PConfig::default();
        let network = HttpP2PNetwork::new(config, "http://localhost:9000".to_string());
        assert!(network.is_ok());
    }

    #[test]
    fn test_network_message_serialization() {
        let block = Block::new(vec![[0u8; 32]], vec![], 1, [1u8; 32]);
        let message = NetworkMessage::Block(block);
        let serialized = serde_json::to_vec(&message).unwrap();
        let deserialized: NetworkMessage = serde_json::from_slice(&serialized).unwrap();
        assert!(matches!(deserialized, NetworkMessage::Block(_)));
    }
}
