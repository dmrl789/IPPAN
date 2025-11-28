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

pub mod ipndht;
pub mod libp2p_network;
pub mod parallel_gossip;

pub use libp2p::Multiaddr;
pub use libp2p_network::{
    Libp2pCommand, Libp2pConfig, Libp2pEvent, Libp2pNetwork, DEFAULT_GOSSIP_TOPICS,
};

pub use ipndht::{IpnDhtService, Libp2pFileDhtService, Libp2pHandleDhtService};
pub use parallel_gossip::{
    DagVertexAnnouncement, GossipConfig, GossipError, GossipMessage, GossipMetricsSnapshot,
    GossipPayload, GossipTopic, ParallelGossipNetwork,
};

use anyhow::{anyhow, Result};
use igd::aio::search_gateway;
use igd::SearchOptions;
use ippan_types::{ippan_time_init, ippan_time_now, Block, Transaction};
use parking_lot::{Mutex, RwLock};
use rand::{rngs::StdRng, Rng, SeedableRng};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{interval, sleep};
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

/// Chaos testing controls for intentionally unstable networking.
#[derive(Debug, Clone, Default)]
pub struct ChaosConfig {
    /// Probability (0-10000 => 0-100.00%) to drop an outbound message before it is sent.
    pub drop_outbound_prob: u16,
    /// Probability (0-10000 => 0-100.00%) to drop an inbound message before it is processed.
    pub drop_inbound_prob: u16,
    /// Minimum artificial latency to inject before delivering a message (milliseconds).
    pub extra_latency_ms_min: u64,
    /// Maximum artificial latency to inject before delivering a message (milliseconds).
    pub extra_latency_ms_max: u64,
}

impl ChaosConfig {
    fn normalized(&self) -> Self {
        let mut normalized = self.clone();
        if normalized.extra_latency_ms_max < normalized.extra_latency_ms_min {
            normalized.extra_latency_ms_max = normalized.extra_latency_ms_min;
        }
        normalized
    }
}

/// P2P network configuration
#[derive(Debug, Clone)]
pub struct P2PConfig {
    pub listen_address: String,
    pub max_peers: usize,
    pub peer_discovery_interval: Duration,
    pub message_timeout: Duration,
    pub retry_attempts: usize,
    pub dht: DhtConfig,
    pub chaos: ChaosConfig,
    pub limits: P2PLimits,
}

impl Default for P2PConfig {
    fn default() -> Self {
        Self {
            listen_address: "http://0.0.0.0:9000".to_string(),
            max_peers: 50,
            peer_discovery_interval: Duration::from_secs(30),
            message_timeout: Duration::from_secs(10),
            retry_attempts: 3,
            dht: DhtConfig::default(),
            chaos: ChaosConfig::default(),
            limits: P2PLimits::default(),
        }
    }
}

/// Limits used to guard against oversized or spammy inbound traffic.
#[derive(Debug, Clone)]
pub struct P2PLimits {
    /// Maximum permitted encoded message size (in bytes). `0` disables the guard.
    pub max_message_bytes: usize,
    /// Maximum number of messages accepted per peer before triggering cooldown. `0` disables the guard.
    pub max_messages_per_peer: u64,
    /// Maximum number of messages accepted globally before applying backpressure. `0` disables the guard.
    pub global_message_limit: u64,
    /// Time window for applying the global message cap. The counter resets when the window elapses.
    pub global_message_window: Duration,
    /// Cooldown duration for peers that exceed their allowance.
    pub peer_cooldown: Duration,
}

impl Default for P2PLimits {
    fn default() -> Self {
        Self {
            max_message_bytes: 512 * 1024,
            max_messages_per_peer: 2048,
            global_message_limit: 32_768,
            global_message_window: Duration::from_secs(60),
            peer_cooldown: Duration::from_secs(30),
        }
    }
}

/// Dedicated configuration for HTTP-level DHT announcements.
#[derive(Debug, Clone)]
pub struct DhtConfig {
    pub bootstrap_peers: Vec<String>,
    pub public_host: Option<String>,
    pub enable_upnp: bool,
    pub external_ip_services: Vec<String>,
    pub announce_interval: Duration,
}

impl Default for DhtConfig {
    fn default() -> Self {
        Self {
            bootstrap_peers: Vec::new(),
            public_host: None,
            enable_upnp: false,
            external_ip_services: vec![
                "https://api.ipify.org".to_string(),
                "https://ifconfig.me/ip".to_string(),
            ],
            announce_interval: Duration::from_secs(60),
        }
    }
}

impl DhtConfig {
    fn announce_override(&self, listen_address: &str) -> Option<String> {
        let candidate = self.public_host.as_ref()?;
        build_candidate_address(listen_address, candidate)
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
    global_message_count: Arc<AtomicU64>,
    global_window_start: Arc<AtomicU64>,
    peer_message_counts: Arc<Mutex<HashMap<String, u64>>>,
    peer_cooldowns: Arc<Mutex<HashMap<String, u64>>>,
    local_peer_id: String,
    listen_address: String,
    announce_address: Arc<RwLock<String>>,
    incoming_sender: mpsc::UnboundedSender<NetworkEvent>,
    incoming_receiver: Arc<Mutex<Option<mpsc::UnboundedReceiver<NetworkEvent>>>>,
    discovery_task: Arc<Mutex<Option<JoinHandle<()>>>>,
    announce_task: Arc<Mutex<Option<JoinHandle<()>>>>,
    chaos: ChaosHandle,
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
            .dht
            .announce_override(&listen_address)
            .unwrap_or_else(|| listen_address.clone());

        let chaos = ChaosHandle::new(config.chaos.clone(), &listen_address);

        Ok(Self {
            config,
            client,
            peers: Arc::new(RwLock::new(HashSet::new())),
            peer_metadata: Arc::new(RwLock::new(HashMap::new())),
            peer_count: Arc::new(RwLock::new(0)),
            is_running: Arc::new(RwLock::new(false)),
            global_message_count: Arc::new(AtomicU64::new(0)),
            global_window_start: Arc::new(AtomicU64::new(ippan_time_now())),
            peer_message_counts: Arc::new(Mutex::new(HashMap::new())),
            peer_cooldowns: Arc::new(Mutex::new(HashMap::new())),
            local_peer_id: derive_peer_id(&listen_address),
            listen_address,
            announce_address: Arc::new(RwLock::new(announce_address)),
            incoming_sender,
            incoming_receiver: Arc::new(Mutex::new(Some(incoming_receiver))),
            discovery_task: Arc::new(Mutex::new(None)),
            announce_task: Arc::new(Mutex::new(None)),
            chaos,
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

        if let Some(address) = self.configure_announce_address().await {
            *self.announce_address.write() = address;
        }

        for peer in self.config.dht.bootstrap_peers.clone() {
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
        self.enforce_inbound_limits(&from, &message)?;
        self.add_peer(from.clone()).await?;
        self.touch_peer(&from);

        if self.chaos.should_drop_inbound() {
            debug!(target: "p2p::chaos", from = %from, "Dropping inbound message due to chaos configuration");
            return Ok(());
        }

        if let Some(delay) = self.chaos.latency_delay() {
            sleep(delay).await;
        }

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

    fn enforce_inbound_limits(&self, peer: &str, message: &NetworkMessage) -> Result<()> {
        let size = message_size_bytes(message)?;
        let limits = &self.config.limits;

        if limits.max_message_bytes > 0 && size > limits.max_message_bytes {
            return Err(anyhow!(
                "message from {peer} exceeds max size ({} > {})",
                size,
                limits.max_message_bytes
            ));
        }

        let now = ippan_time_now();
        if self.peer_in_cooldown(peer, now) {
            return Err(anyhow!("peer {peer} is in cooldown after exceeding limits"));
        }

        if !self.consume_message_budget(peer, now) {
            self.activate_cooldown(peer, now);
            return Err(anyhow!("peer {peer} exceeded message allowance"));
        }

        Ok(())
    }

    fn consume_message_budget(&self, peer: &str, now_us: u64) -> bool {
        let limits = &self.config.limits;

        if limits.global_message_limit > 0 {
            let window_us: u64 = limits
                .global_message_window
                .as_micros()
                .min(u128::from(u64::MAX)) as u64;

            // Check and reset window if it has elapsed
            if window_us > 0 {
                let mut start = self.global_window_start.load(Ordering::Relaxed);
                loop {
                    let elapsed = now_us.saturating_sub(start);
                    if elapsed >= window_us {
                        // Window has elapsed, try to reset
                        match self.global_window_start.compare_exchange(
                            start,
                            now_us,
                            Ordering::Relaxed,
                            Ordering::Relaxed,
                        ) {
                            Ok(_) => {
                                // Successfully reset the window, also reset the count
                                self.global_message_count.store(0, Ordering::Relaxed);
                                break;
                            }
                            Err(updated) => {
                                // Another thread reset it, use the updated value
                                start = updated;
                            }
                        }
                    } else {
                        // Window hasn't elapsed yet, continue with current count
                        break;
                    }
                }
            }

            // Now check the limit and increment atomically
            let mut current = self.global_message_count.load(Ordering::Relaxed);
            loop {
                // Check limit BEFORE attempting to increment
                if current >= limits.global_message_limit {
                    return false;
                }

                // Try to increment atomically
                match self.global_message_count.compare_exchange(
                    current,
                    current + 1,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(updated) => {
                        // Count was updated by another thread, retry with new value
                        current = updated;
                    }
                }
            }
        }

        if limits.max_messages_per_peer == 0 {
            return true;
        }

        let mut counts = self.peer_message_counts.lock();
        let entry = counts.entry(peer.to_string()).or_insert(0);
        if *entry >= limits.max_messages_per_peer {
            return false;
        }
        *entry = entry.saturating_add(1);
        true
    }

    fn activate_cooldown(&self, peer: &str, now_us: u64) {
        let deadline = cooldown_deadline(now_us, self.config.limits.peer_cooldown);
        self.peer_cooldowns
            .lock()
            .insert(peer.to_string(), deadline);
    }

    fn peer_in_cooldown(&self, peer: &str, now_us: u64) -> bool {
        let mut cooldowns = self.peer_cooldowns.lock();
        if let Some(expiry) = cooldowns.get(peer) {
            if *expiry <= now_us {
                cooldowns.remove(peer);
                self.peer_message_counts.lock().remove(peer);
                return false;
            }
            return true;
        }

        false
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
        let chaos = self.chaos.clone();
        let interval_duration = self
            .config
            .dht
            .announce_interval
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
                    if let Err(err) =
                        post_message_with_chaos(&client, &peer, &message, &chaos).await
                    {
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
        post_message_with_chaos(&self.client, peer, &message, &self.chaos).await
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

impl HttpP2PNetwork {
    async fn configure_announce_address(&self) -> Option<String> {
        if let Some(explicit) = self.config.dht.announce_override(&self.listen_address) {
            return Some(explicit);
        }

        if let Some(host) = self.detect_external_ip_via_upnp().await {
            if let Some(address) = build_candidate_address(&self.listen_address, &host) {
                return Some(address);
            }
        }

        if let Some(host) = self.detect_external_ip_via_services().await {
            if let Some(address) = build_candidate_address(&self.listen_address, &host) {
                return Some(address);
            }
        }

        None
    }

    async fn detect_external_ip_via_upnp(&self) -> Option<String> {
        if !self.config.dht.enable_upnp {
            return None;
        }

        match search_gateway(SearchOptions::default()).await {
            Ok(gateway) => match gateway.get_external_ip().await {
                Ok(ip) => {
                    debug!("Discovered external IP via UPnP: {}", ip);
                    Some(ip.to_string())
                }
                Err(err) => {
                    debug!("Failed to fetch external IP via UPnP: {}", err);
                    None
                }
            },
            Err(err) => {
                debug!("UPnP gateway discovery failed: {}", err);
                None
            }
        }
    }

    async fn detect_external_ip_via_services(&self) -> Option<String> {
        if self.config.dht.external_ip_services.is_empty() {
            return None;
        }

        for endpoint in &self.config.dht.external_ip_services {
            let trimmed = endpoint.trim();
            if trimmed.is_empty() {
                continue;
            }

            match self.client.get(trimmed).send().await {
                Ok(response) => {
                    if !response.status().is_success() {
                        continue;
                    }
                    if let Ok(body) = response.text().await {
                        let candidate = body.trim();
                        if candidate.is_empty() {
                            continue;
                        }
                        if candidate.parse::<IpAddr>().is_ok() {
                            debug!("Detected external IP via {}: {}", trimmed, candidate);
                            return Some(candidate.to_string());
                        }
                    }
                }
                Err(err) => {
                    debug!("Failed to query {} for external IP: {}", trimmed, err);
                }
            }
        }

        None
    }
}

fn build_candidate_address(listen_address: &str, host: &str) -> Option<String> {
    if host.contains("://") {
        return normalize_peer(host).ok();
    }

    let base = Url::parse(listen_address).ok()?;
    let scheme = base.scheme();
    let port = base.port_or_known_default();
    let formatted_host = if host.contains(':') {
        host.to_string()
    } else if let Some(port) = port {
        format!("{host}:{port}")
    } else {
        host.to_string()
    };

    let candidate = format!("{scheme}://{formatted_host}");
    normalize_peer(&candidate).ok()
}

fn cooldown_deadline(now_us: u64, duration: Duration) -> u64 {
    let extra = duration.as_micros().min(u64::MAX as u128) as u64;
    now_us.saturating_add(extra)
}

fn message_size_bytes(message: &NetworkMessage) -> Result<usize, serde_json::Error> {
    serde_json::to_vec(message).map(|bytes| bytes.len())
}

fn derive_peer_id(address: &str) -> String {
    let hash = blake3::hash(address.as_bytes());
    format!("peer-{}", &hash.to_hex()[..16])
}

fn ensure_http_scheme(address: &str) -> String {
    if address.starts_with("http://") || address.starts_with("https://") {
        address.to_string()
    } else {
        format!("http://{address}")
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
    let url = format!("{peer}/p2p/peers");
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

    let url = format!("{peer}{endpoint}");
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

async fn post_message_with_chaos(
    client: &Client,
    peer: &str,
    message: &NetworkMessage,
    chaos: &ChaosHandle,
) -> Result<()> {
    if chaos.should_drop_outbound() {
        debug!(target: "p2p::chaos", peer = %peer, "Dropping outbound message due to chaos configuration");
        return Ok(());
    }

    if let Some(delay) = chaos.latency_delay() {
        sleep(delay).await;
    }

    post_message(client, peer, message).await
}

#[derive(Clone)]
struct ChaosHandle {
    config: ChaosConfig,
    rng: Arc<Mutex<StdRng>>,
}

impl ChaosHandle {
    fn new(config: ChaosConfig, listen_address: &str) -> Self {
        let normalized = config.normalized();
        let mut seed = [0u8; 32];
        seed.copy_from_slice(blake3::hash(listen_address.as_bytes()).as_bytes());
        let rng = StdRng::from_seed(seed);
        Self {
            config: normalized,
            rng: Arc::new(Mutex::new(rng)),
        }
    }

    fn should_drop_outbound(&self) -> bool {
        self.sample_drop(self.config.drop_outbound_prob)
    }

    fn should_drop_inbound(&self) -> bool {
        self.sample_drop(self.config.drop_inbound_prob)
    }

    fn sample_drop(&self, probability: u16) -> bool {
        if probability == 0 {
            return false;
        }

        let mut rng = self.rng.lock();
        let roll: u16 = rng.gen_range(0..=9999);
        roll < probability
    }

    fn latency_delay(&self) -> Option<Duration> {
        let min = self.config.extra_latency_ms_min;
        let max = self
            .config
            .extra_latency_ms_max
            .max(self.config.extra_latency_ms_min);

        if max == 0 && min == 0 {
            return None;
        }

        let delay_ms = if max == min {
            max
        } else {
            let mut rng = self.rng.lock();
            rng.gen_range(min..=max)
        };

        if delay_ms == 0 {
            None
        } else {
            Some(Duration::from_millis(delay_ms))
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

    #[tokio::test]
    async fn test_configure_announce_address_prefers_public_host() {
        let config = P2PConfig {
            listen_address: "http://127.0.0.1:9101".into(),
            dht: DhtConfig {
                public_host: Some("https://example.com:9101".into()),
                external_ip_services: Vec::new(),
                enable_upnp: false,
                ..Default::default()
            },
            ..Default::default()
        };

        let network = HttpP2PNetwork::new(config, "http://127.0.0.1:9101".into()).expect("network");
        let override_addr = network.configure_announce_address().await;
        assert_eq!(override_addr.unwrap(), "https://example.com:9101");
    }

    #[test]
    fn test_network_message_serialization() {
        let block = Block::new(vec![[0u8; 32]], vec![], 1, [1u8; 32]);
        let message = NetworkMessage::Block(block);
        let serialized = serde_json::to_vec(&message).unwrap();
        let deserialized: NetworkMessage = serde_json::from_slice(&serialized).unwrap();
        assert!(matches!(deserialized, NetworkMessage::Block(_)));
    }

    #[test]
    fn malformed_network_message_is_rejected() {
        let malformed = br#"{" + not valid json"#;
        let decoded = serde_json::from_slice::<NetworkMessage>(malformed);
        assert!(decoded.is_err());
    }

    #[tokio::test]
    async fn peer_limit_prevents_unbounded_growth() {
        let config = P2PConfig {
            max_peers: 2,
            ..Default::default()
        };
        let network = HttpP2PNetwork::new(config, "http://127.0.0.1:9200".into()).expect("network");

        network
            .add_peer("http://127.0.0.1:9201".into())
            .await
            .expect("peer 1");
        network
            .add_peer("http://127.0.0.1:9202".into())
            .await
            .expect("peer 2");
        network
            .add_peer("http://127.0.0.1:9203".into())
            .await
            .expect("peer 3 exceeds cap");

        assert_eq!(network.get_peer_count(), 2);

        network.remove_peer("http://127.0.0.1:9201");
        network.remove_peer("http://127.0.0.1:9202");
        let metadata = network.get_peer_metadata();
        assert!(metadata.len() <= 1);
    }

    #[tokio::test]
    async fn process_incoming_message_rejects_empty_peer() {
        let config = P2PConfig::default();
        let network = HttpP2PNetwork::new(config, "http://127.0.0.1:9300".into()).expect("network");

        let result = network
            .process_incoming_message("   ", NetworkMessage::PeerDiscovery { peers: vec![] })
            .await;

        assert!(result.is_err());
        assert_eq!(network.get_peer_count(), 0);
    }

    #[tokio::test]
    async fn spammy_malformed_peer_does_not_poison_state() {
        let config = P2PConfig::default();
        let network = HttpP2PNetwork::new(config, "http://127.0.0.1:9310".into()).expect("network");

        let valid_peer = "http://127.0.0.1:9311";
        network.add_peer(valid_peer.into()).await.expect("peer add");

        for _ in 0..5 {
            let result = network
                .process_incoming_message(
                    "this is not a url",
                    NetworkMessage::PeerDiscovery {
                        peers: vec!["%%%".into()],
                    },
                )
                .await;
            assert!(result.is_err());
        }

        assert!(network.get_peer_count() >= 1);

        let announce = network
            .process_incoming_message(
                valid_peer,
                NetworkMessage::PeerInfo {
                    peer_id: "peer-a".into(),
                    addresses: vec![valid_peer.into()],
                    time_us: Some(1),
                },
            )
            .await;
        assert!(announce.is_ok());

        let metadata = network.get_peer_metadata();
        assert!(metadata.iter().any(|p| p.address == valid_peer));
    }

    #[tokio::test]
    async fn oversized_message_is_rejected_before_peer_is_added() {
        let mut config = P2PConfig::default();
        config.limits.max_message_bytes = 32;
        let network = HttpP2PNetwork::new(config, "http://127.0.0.1:9315".into()).expect("network");

        let message = NetworkMessage::PeerInfo {
            peer_id: "peer-oversized".into(),
            addresses: vec!["http://127.0.0.1:9315/with/a/very/long/address".into()],
            time_us: Some(1),
        };

        let result = network
            .process_incoming_message("http://127.0.0.1:9316", message)
            .await;

        assert!(result.is_err());
        assert_eq!(network.get_peer_count(), 0);
    }

    #[tokio::test]
    async fn per_peer_limit_triggers_cooldown() {
        let mut config = P2PConfig::default();
        config.limits.max_messages_per_peer = 2;
        config.limits.global_message_limit = 10;
        config.limits.peer_cooldown = Duration::from_millis(25);
        let network = HttpP2PNetwork::new(config, "http://127.0.0.1:9325".into()).expect("network");

        let peer = "http://127.0.0.1:9326";
        let message = NetworkMessage::PeerDiscovery {
            peers: vec![peer.into()],
        };

        assert!(network
            .process_incoming_message(peer, message.clone())
            .await
            .is_ok());
        assert!(network
            .process_incoming_message(peer, message.clone())
            .await
            .is_ok());

        let rejected = network
            .process_incoming_message(peer, message.clone())
            .await;
        assert!(rejected.is_err());

        tokio::time::sleep(Duration::from_millis(30)).await;

        let after_cooldown = network.process_incoming_message(peer, message).await;
        assert!(after_cooldown.is_ok());
    }

    #[tokio::test]
    async fn global_limit_caps_total_inbound_messages() {
        let mut config = P2PConfig::default();
        config.limits.global_message_limit = 2;
        let network = HttpP2PNetwork::new(config, "http://127.0.0.1:9330".into()).expect("network");

        let message = NetworkMessage::PeerDiscovery { peers: vec![] };

        assert!(network
            .process_incoming_message("http://127.0.0.1:9331", message.clone())
            .await
            .is_ok());
        assert!(network
            .process_incoming_message("http://127.0.0.1:9332", message.clone())
            .await
            .is_ok());

        let rejected = network
            .process_incoming_message("http://127.0.0.1:9333", message)
            .await;
        assert!(rejected.is_err());
    }

    #[tokio::test]
    async fn global_limit_resets_after_window_elapses() {
        ippan_time_init();
        let mut config = P2PConfig::default();
        config.limits.global_message_limit = 2;
        config.limits.global_message_window = Duration::from_millis(20);
        let network = HttpP2PNetwork::new(config, "http://127.0.0.1:9334".into()).expect("network");

        let message = NetworkMessage::PeerDiscovery { peers: vec![] };

        // First two messages should succeed (limit = 2)
        assert!(network
            .process_incoming_message("http://127.0.0.1:9335", message.clone())
            .await
            .is_ok());
        assert!(network
            .process_incoming_message("http://127.0.0.1:9336", message.clone())
            .await
            .is_ok());

        // Third message should fail (limit exceeded) - use a window of 0 to disable window reset
        // and ensure the limit is enforced immediately
        let mut config_no_window = P2PConfig::default();
        config_no_window.limits.global_message_limit = 2;
        config_no_window.limits.global_message_window = Duration::from_millis(0); // Disable window
        let network_no_window = HttpP2PNetwork::new(config_no_window, "http://127.0.0.1:9339".into()).expect("network");
        
        assert!(network_no_window
            .process_incoming_message("http://127.0.0.1:9340", message.clone())
            .await
            .is_ok());
        assert!(network_no_window
            .process_incoming_message("http://127.0.0.1:9341", message.clone())
            .await
            .is_ok());
        assert!(network_no_window
            .process_incoming_message("http://127.0.0.1:9342", message.clone())
            .await
            .is_err());

        // Now test window reset with the original network
        // Wait for window to elapse (window is 20ms, wait 30ms to ensure it elapses)
        tokio::time::sleep(Duration::from_millis(30)).await;

        // After window elapses, limit should reset and message should succeed
        assert!(network
            .process_incoming_message("http://127.0.0.1:9338", message)
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn rapid_connect_disconnect_does_not_leak_peers() {
        let config = P2PConfig::default();
        let network = HttpP2PNetwork::new(config, "http://127.0.0.1:9320".into()).expect("network");

        for i in 0..20 {
            let peer = format!("http://127.0.0.1:94{i:02}");
            let _ = network.add_peer(peer.clone()).await;
            network.remove_peer(&peer);
        }

        assert_eq!(network.get_peer_count(), 0);
        assert!(network.get_peer_metadata().is_empty());
    }
}
