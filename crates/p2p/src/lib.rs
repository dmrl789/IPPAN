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
use tokio::task::JoinHandle;
use tokio::time::interval;
use tracing::{debug, info, warn};
use url::Url;

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
    peer_count: Arc<RwLock<usize>>,
    is_running: Arc<RwLock<bool>>,
    local_peer_id: String,
    listen_address: String,
    announce_address: Arc<RwLock<String>>,
    incoming_sender: mpsc::UnboundedSender<NetworkEvent>,
    incoming_receiver: Arc<Mutex<Option<mpsc::UnboundedReceiver<NetworkEvent>>>>,
    discovery_task: Arc<Mutex<Option<JoinHandle<()>>>>,
    announce_task: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl HttpP2PNetwork {
    pub fn new(config: P2PConfig, listen_address: String) -> Result<Self> {
        let listen_address = normalize_peer(&listen_address)?;
        let http_timeout = if config.message_timeout.is_zero() {
            Duration::from_secs(10)
        } else {
            config.message_timeout
        };

        let client = Client::builder().timeout(http_timeout).build()?;
        let (incoming_sender, incoming_receiver) = mpsc::unbounded_channel();

        let announce_address = config
            .public_host
            .clone()
            .map(|addr| ensure_http_scheme(&addr))
            .and_then(|addr| normalize_peer(&addr).ok())
            .unwrap_or_else(|| listen_address.clone());

        Ok(Self {
            config,
            client,
            peers: Arc::new(RwLock::new(HashSet::new())),
            peer_metadata: Arc::new(RwLock::new(HashMap::new())),
            peer_count: Arc::new(RwLock::new(0)),
            is_running: Arc::new(RwLock::new(false)),
            local_peer_id: derive_peer_id(&listen_address),
            listen_address,
            announce_address: Arc::new(RwLock::new(announce_address)),
            incoming_sender,
            incoming_receiver: Arc::new(Mutex::new(Some(incoming_receiver))),
            discovery_task: Arc::new(Mutex::new(None)),
            announce_task: Arc::new(Mutex::new(None)),
        })
    }

    pub fn get_local_peer_id(&self) -> String {
        self.local_peer_id.clone()
    }

    pub fn get_peer_count(&self) -> usize {
        *self.peer_count.read()
    }

    pub fn get_peers(&self) -> Vec<String> {
        let mut peers: Vec<String> = self.peers.read().iter().cloned().collect();
        peers.sort();
        peers
    }

    pub fn get_peer_metadata(&self) -> Vec<PeerInfo> {
        let peers: HashSet<String> = self.peers.read().iter().cloned().collect();
        self.peer_metadata
            .read()
            .values()
            .map(|record| PeerInfo {
                id: record
                    .peer_id
                    .clone()
                    .unwrap_or_else(|| derive_peer_id(&record.address)),
                address: record.address.clone(),
                last_seen: record.last_seen,
                is_connected: peers.contains(&record.address),
            })
            .collect()
    }

    pub fn take_incoming_events(&self) -> Option<mpsc::UnboundedReceiver<NetworkEvent>> {
        self.incoming_receiver.lock().take()
    }

    pub async fn start(&mut self) -> Result<()> {
        {
            let mut running = self.is_running.write();
            if *running {
                return Ok(());
            }
            *running = true;
        }

        for peer in self.config.bootstrap_peers.clone() {
            if let Err(err) = self.add_peer(peer).await {
                warn!("Failed to add bootstrap peer: {err}");
            }
        }

        self.spawn_discovery_loop();
        self.spawn_announce_loop();

        info!("HTTP P2P network started");
        Ok(())
    }

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
        if let Some(handle) = self.announce_task.lock().take() {
            handle.abort();
        }

        info!("HTTP P2P network stopped");
        Ok(())
    }

    pub async fn add_peer(&self, peer: String) -> Result<()> {
        let peer = normalize_peer(&peer)?;
        if peer == self.listen_address {
            return Ok(());
        }

        let mut was_new = false;
        {
            let mut peers = self.peers.write();
            // Enforce max_peers limit before accepting new peers
            if peers.len() >= self.config.max_peers && !peers.contains(&peer) {
                debug!(
                    "Peer limit reached ({}), rejecting new peer: {}",
                    self.config.max_peers, peer
                );
                return Ok(());
            }
            if peers.insert(peer.clone()) {
                *self.peer_count.write() = peers.len();
                was_new = true;
            }
        }

        if was_new {
            {
                let mut metadata = self.peer_metadata.write();
                metadata
                    .entry(peer.clone())
                    .or_insert_with(|| PeerRecord::new(peer.clone()));
            }

            if let Err(err) = self.send_peer_info(&peer).await {
                debug!("Failed to announce to {}: {}", peer, err);
            }
        }

        Ok(())
    }

    pub fn remove_peer(&self, peer: &str) {
        if let Ok(peer) = normalize_peer(peer) {
            let mut peers = self.peers.write();
            if peers.remove(&peer) {
                *self.peer_count.write() = peers.len();
            }
            self.peer_metadata.write().remove(&peer);
        }
    }

    pub async fn process_incoming_message(
        &self,
        from: &str,
        message: NetworkMessage,
    ) -> Result<()> {
        let from = normalize_peer(from)?;
        self.add_peer(from.clone()).await?;
        self.touch_peer(&from);

        match &message {
            NetworkMessage::PeerInfo {
                peer_id,
                addresses,
                time_us,
            } => {
                self.record_peer_metadata(&from, peer_id, addresses, *time_us);
            }
            NetworkMessage::PeerDiscovery { peers } => {
                for peer in peers.clone() {
                    if let Err(err) = self.add_peer(peer).await {
                        debug!("Failed to add discovered peer: {err}");
                    }
                }
            }
            _ => {}
        }

        let event = match message.clone() {
            NetworkMessage::Block(block) => NetworkEvent::Block {
                from: from.clone(),
                block,
            },
            NetworkMessage::Transaction(transaction) => NetworkEvent::Transaction {
                from: from.clone(),
                transaction,
            },
            NetworkMessage::PeerInfo {
                peer_id,
                addresses,
                time_us,
            } => NetworkEvent::PeerInfo {
                from: from.clone(),
                peer_id,
                addresses,
                time_us,
            },
            NetworkMessage::PeerDiscovery { peers } => NetworkEvent::PeerDiscovery {
                from: from.clone(),
                peers,
            },
            NetworkMessage::BlockRequest { hash } => NetworkEvent::BlockRequest {
                from: from.clone(),
                hash,
            },
            NetworkMessage::BlockResponse(block) => NetworkEvent::BlockResponse {
                from: from.clone(),
                block,
            },
        };

        self.incoming_sender.send(event)?;
        Ok(())
    }

    fn spawn_discovery_loop(&mut self) {
        let peers = self.peers.clone();
        let metadata = self.peer_metadata.clone();
        let peer_count = self.peer_count.clone();
        let incoming = self.incoming_sender.clone();
        let is_running = self.is_running.clone();
        let client = self.client.clone();
        let listen_address = self.listen_address.clone();
        let max_peers = self.config.max_peers;
        let interval_duration = self
            .config
            .peer_discovery_interval
            .max(Duration::from_millis(100));

        *self.discovery_task.lock() = Some(tokio::spawn(async move {
            let mut ticker = interval(interval_duration);
            loop {
                ticker.tick().await;
                if !*is_running.read() {
                    break;
                }

                let known_peers: Vec<String> = peers.read().iter().cloned().collect();
                for peer in known_peers {
                    if peer == listen_address {
                        continue;
                    }

                    match fetch_peer_list(&client, &peer).await {
                        Ok(list) => {
                            let mut newly_discovered = Vec::new();
                            for candidate in list {
                                if candidate == listen_address {
                                    continue;
                                }
                                let Ok(candidate) = normalize_peer(&candidate) else {
                                    continue;
                                };

                                let mut added = false;
                                {
                                    let mut guard = peers.write();
                                    // Enforce max_peers limit in discovery loop
                                    if guard.len() < max_peers && guard.insert(candidate.clone()) {
                                        *peer_count.write() = guard.len();
                                        added = true;
                                    }
                                }

                                if added {
                                    {
                                        let mut meta_guard = metadata.write();
                                        meta_guard
                                            .entry(candidate.clone())
                                            .or_insert_with(|| PeerRecord::new(candidate.clone()))
                                            .last_seen = ippan_time_now();
                                    }
                                    newly_discovered.push(candidate.clone());
                                }
                            }

                            if !newly_discovered.is_empty() {
                                let _ = incoming.send(NetworkEvent::PeerDiscovery {
                                    from: peer.clone(),
                                    peers: newly_discovered,
                                });
                            }
                        }
                        Err(err) => {
                            debug!("Failed to fetch peer list from {}: {}", peer, err);
                        }
                    }
                }
            }
        }));
    }

    fn spawn_announce_loop(&mut self) {
        let peers = self.peers.clone();
        let client = self.client.clone();
        let is_running = self.is_running.clone();
        let peer_id = self.local_peer_id.clone();
        let announce_address = self.announce_address.clone();
        let listen_address = self.listen_address.clone();
        let interval_duration = self
            .config
            .peer_announce_interval
            .max(Duration::from_secs(5));

        *self.announce_task.lock() = Some(tokio::spawn(async move {
            let mut ticker = interval(interval_duration);
            loop {
                ticker.tick().await;
                if !*is_running.read() {
                    break;
                }

                let addresses = {
                    let announce = announce_address.read().clone();
                    vec![announce]
                };

                let message = NetworkMessage::PeerInfo {
                    peer_id: peer_id.clone(),
                    addresses: addresses.clone(),
                    time_us: Some(ippan_time_now()),
                };

                let snapshot: Vec<String> = peers.read().iter().cloned().collect();
                for peer in snapshot {
                    if peer == listen_address {
                        continue;
                    }
                    if let Err(err) = post_message(&client, &peer, &message).await {
                        debug!("Failed to announce to {}: {}", peer, err);
                    }
                }
            }
        }));
    }

    async fn send_peer_info(&self, peer: &str) -> Result<()> {
        let address = self.announce_address.read().clone();
        let message = NetworkMessage::PeerInfo {
            peer_id: self.local_peer_id.clone(),
            addresses: vec![address],
            time_us: Some(ippan_time_now()),
        };
        post_message(&self.client, peer, &message).await
    }

    fn touch_peer(&self, peer: &str) {
        let mut metadata = self.peer_metadata.write();
        metadata
            .entry(peer.to_string())
            .or_insert_with(|| PeerRecord::new(peer.to_string()))
            .last_seen = ippan_time_now();
    }

    fn record_peer_metadata(
        &self,
        observed: &str,
        peer_id: &str,
        addresses: &[String],
        announced_time: Option<u64>,
    ) {
        let normalized_addresses: Vec<String> = addresses
            .iter()
            .filter_map(|addr| normalize_peer(addr).ok())
            .collect();

        let mut metadata = self.peer_metadata.write();
        let mut update_record = |address: String| {
            let entry = metadata
                .entry(address.clone())
                .or_insert_with(|| PeerRecord::new(address.clone()));
            entry.peer_id = Some(peer_id.to_string());
            entry.last_seen = announced_time.unwrap_or_else(ippan_time_now);
            entry.last_announced_us = announced_time;
            entry.observed_address = Some(observed.to_string());
            for advertised in addresses {
                if !entry.advertised_addresses.contains(advertised) {
                    entry.advertised_addresses.push(advertised.clone());
                }
            }
        };

        update_record(observed.to_string());
        for address in normalized_addresses {
            update_record(address);
        }
    }
}

fn derive_peer_id(address: &str) -> String {
    let hash = blake3::hash(address.as_bytes());
    format!("peer-{}", &hash.to_hex()[..16])
}

fn ensure_http_scheme(address: &str) -> String {
    if address.starts_with("http://") || address.starts_with("https://") {
        address.to_string()
    } else {
        format!("http://{}", address)
    }
}

fn normalize_peer(address: &str) -> Result<String> {
    let trimmed = address.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("peer address cannot be empty"));
    }

    let candidate = ensure_http_scheme(trimmed);
    let mut url = Url::parse(&candidate)?;
    url.set_path("");
    url.set_query(None);
    url.set_fragment(None);
    let mut normalized = url.to_string();
    while normalized.ends_with('/') {
        normalized.pop();
    }
    Ok(normalized)
}

async fn fetch_peer_list(client: &Client, peer: &str) -> Result<Vec<String>> {
    let url = format!("{}/p2p/peers", peer);
    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        return Err(anyhow!(
            "peer {} responded with status {}",
            peer,
            response.status()
        ));
    }
    Ok(response.json().await?)
}

async fn post_message(client: &Client, peer: &str, message: &NetworkMessage) -> Result<()> {
    let endpoint = match message {
        NetworkMessage::Block(_) => "/p2p/blocks",
        NetworkMessage::Transaction(_) => "/p2p/transactions",
        NetworkMessage::PeerInfo { .. } => "/p2p/peer-info",
        NetworkMessage::PeerDiscovery { .. } => "/p2p/peer-discovery",
        NetworkMessage::BlockRequest { .. } => "/p2p/block-request",
        NetworkMessage::BlockResponse { .. } => "/p2p/block-response",
    };

    let url = format!("{}{}", peer, endpoint);
    let response = client.post(url).json(message).send().await?;
    if !response.status().is_success() {
        return Err(anyhow!(
            "peer {} returned error status {} for {}",
            peer,
            response.status(),
            endpoint
        ));
    }
    Ok(())
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
