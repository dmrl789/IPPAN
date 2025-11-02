pub mod parallel_gossip;
pub use parallel_gossip::{
    DagVertexAnnouncement, GossipConfig, GossipError, GossipMessage, GossipMetricsSnapshot,
    GossipPayload, GossipTopic, ParallelGossipNetwork,
};

use anyhow::{anyhow, Result};
use igd::aio::search_gateway;
use igd::PortMappingProtocol;
use ippan_types::{ippan_time_now, Block, Transaction};
use local_ip_address::local_ip;
use parking_lot::{Mutex, RwLock};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, SocketAddrV4};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
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

/// Network message types
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

/// Peer information
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

/// HTTP-based P2P network manager
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
}

impl HttpP2PNetwork {
    /// Create a new HTTP P2P network
    pub fn new(config: P2PConfig, local_address: String) -> Result<Self> {
        let client = Client::builder().timeout(config.message_timeout).build()?;

        // Validate and normalize the listen address
        let canonical_listen = Self::canonicalize_address(&local_address)?;

        // Generate a simple peer ID
        let local_peer_id = format!(
            "ippan-peer-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );

        // Create message channels
        let (message_sender, message_receiver) = mpsc::unbounded_channel();
        let (incoming_sender, incoming_receiver) = mpsc::unbounded_channel();

        Ok(Self {
            config,
            client,
            peers: Arc::new(RwLock::new(HashSet::new())),
            peer_metadata: Arc::new(RwLock::new(HashMap::new())),
            message_sender,
            message_receiver: Some(message_receiver),
            peer_count: Arc::new(RwLock::new(0)),
            is_running: Arc::new(RwLock::new(false)),
            local_peer_id,
            listen_address: canonical_listen.clone(),
            local_address: canonical_listen.clone(),
            announce_address: Arc::new(RwLock::new(canonical_listen)),
            upnp_mapping_active: Arc::new(RwLock::new(false)),
            incoming_sender,
            incoming_receiver: Arc::new(Mutex::new(Some(incoming_receiver))),
        })
    }

    /// Start the P2P network
    pub async fn start(&mut self) -> Result<()> {
        *self.is_running.write() = true;
        self.setup_connectivity().await?;

        let announce_address = self.announce_address.read().clone();
        info!(
            "Starting HTTP P2P network on {} (announcing {})",
            self.listen_address, announce_address
        );

        // Add bootstrap peers
        for peer in &self.config.bootstrap_peers {
            self.add_peer(peer.clone()).await?;
        }

        // Start the message processing loop
        let _message_handle = {
            let is_running = self.is_running.clone();
            let peers = self.peers.clone();
            let client = self.client.clone();
            let config = self.config.clone();
            let announce_address = self.announce_address.clone();
            let local_address = self.local_address.clone();
            let mut message_receiver = self.message_receiver.take().unwrap();

            tokio::spawn(async move {
                loop {
                    if !*is_running.read() {
                        break;
                    }

                    // Process outgoing messages
                    if let Some(msg) = message_receiver.recv().await {
                        Self::handle_outgoing_message(
                            &client,
                            &peers,
                            &config,
                            &announce_address,
                            &local_address,
                            msg,
                        )
                        .await;
                    }

                    // Small delay to prevent busy waiting
                    sleep(Duration::from_millis(10)).await;
                }
            })
        };

        // Start peer discovery loop
        let _discovery_handle = {
            let is_running = self.is_running.clone();
            let peers = self.peers.clone();
            let client = self.client.clone();
            let config = self.config.clone();
            let local_address = self.local_address.clone();
            let announce_address = self.announce_address.clone();
            let peer_count = self.peer_count.clone();
            let peer_metadata = self.peer_metadata.clone();
            let incoming_sender = self.incoming_sender.clone();

            tokio::spawn(async move {
                let mut interval = interval(config.peer_discovery_interval);

                loop {
                    if !*is_running.read() {
                        break;
                    }

                    interval.tick().await;
                    Self::discover_peers(
                        &client,
                        &peers,
                        &peer_metadata,
                        incoming_sender.clone(),
                        &local_address,
                        &announce_address,
                        &peer_count,
                    )
                    .await;
                }
            })
        };

        // Periodically announce our public addresses to peers
        let _announce_handle = {
            let is_running = self.is_running.clone();
            let sender = self.message_sender.clone();
            let config = self.config.clone();
            let local_peer_id = self.local_peer_id.clone();
            let announce_address = self.announce_address.clone();
            let local_address = self.local_address.clone();

            tokio::spawn(async move {
                let mut interval = interval(config.peer_announce_interval);
                loop {
                    if !*is_running.read() {
                        break;
                    }

                    interval.tick().await;
                    let mut addresses = vec![announce_address.read().clone()];
                    if !addresses.contains(&local_address) {
                        addresses.push(local_address.clone());
                    }

                    if let Err(e) = sender.send(NetworkMessage::PeerInfo {
                        peer_id: local_peer_id.clone(),
                        addresses,
                        time_us: Some(ippan_time_now()),
                    }) {
                        warn!("Failed to enqueue peer announcement: {}", e);
                        break;
                    }
                }
            })
        };

        // Send an initial announcement so bootstrap peers learn our address quickly
        let initial_addresses = self.local_advertised_addresses();
        self.message_sender.send(NetworkMessage::PeerInfo {
            peer_id: self.local_peer_id.clone(),
            addresses: initial_addresses,
            time_us: Some(ippan_time_now()),
        })?;

        info!("HTTP P2P network started");
        Ok(())
    }

    /// Take ownership of the incoming event stream. Returns `None` if already taken.
    pub fn take_incoming_events(&self) -> Option<mpsc::UnboundedReceiver<NetworkEvent>> {
        let mut guard = self.incoming_receiver.lock();
        guard.take()
    }

    fn emit_event(&self, event: NetworkEvent) {
        if let Err(err) = self.incoming_sender.send(event) {
            debug!("dropping inbound network event: {}", err);
        }
    }

    fn touch_peer(
        &self,
        address: &str,
        peer_id: Option<&str>,
        last_seen: Option<u64>,
        observed_address: Option<&str>,
        advertised_addresses: Option<&[String]>,
        announced_at: Option<u64>,
    ) {
        let now = ippan_time_now();
        let mut metadata = self.peer_metadata.write();
        let entry = metadata
            .entry(address.to_string())
            .or_insert_with(|| PeerRecord::new(address.to_string()));

        if let Some(id) = peer_id {
            entry.peer_id = Some(id.to_string());
        }

        entry.last_seen = last_seen.unwrap_or(now);

        if let Some(observed) = observed_address {
            entry.observed_address = Some(observed.to_string());
        }

        if let Some(addresses) = advertised_addresses {
            entry.advertised_addresses = addresses.to_vec();
        }

        if let Some(time) = announced_at {
            entry.last_announced_us = Some(time);
        }
    }

    fn record_peer_seen(&self, address: &str) {
        if let Ok(canonical) = Self::canonicalize_address(address) {
            if self.peers.read().contains(&canonical) {
                self.touch_peer(&canonical, None, Some(ippan_time_now()), None, None, None);
            }
        }
    }

    async fn handle_peer_info_message(
        &self,
        from: &str,
        peer_id: String,
        mut addresses: Vec<String>,
        time_us: Option<u64>,
    ) -> Result<()> {
        if addresses.is_empty() {
            addresses.push(from.to_string());
        }

        let last_seen = time_us.unwrap_or_else(ippan_time_now);

        for address in &addresses {
            if let Err(err) = self.add_peer(address.clone()).await {
                debug!("skip invalid announced peer {}: {}", address, err);
            }
        }

        let canonical_from = Self::canonicalize_address(from).unwrap_or_else(|_| from.to_string());

        for address in &addresses {
            self.touch_peer(
                address,
                Some(&peer_id),
                Some(last_seen),
                Some(&canonical_from),
                Some(&addresses),
                time_us,
            );
        }

        self.emit_event(NetworkEvent::PeerInfo {
            from: canonical_from,
            peer_id,
            addresses,
            time_us,
        });

        Ok(())
    }

    async fn handle_peer_discovery_message(&self, from: &str, peers: Vec<String>) -> Result<()> {
        let canonical_from = Self::canonicalize_address(from).unwrap_or_else(|_| from.to_string());

        for peer in peers.iter() {
            if let Err(err) = self.add_peer(peer.clone()).await {
                debug!(
                    "skip invalid discovered peer {} from {}: {}",
                    peer, from, err
                );
            } else {
                self.record_peer_seen(peer);
            }
        }

        self.emit_event(NetworkEvent::PeerDiscovery {
            from: canonical_from,
            peers,
        });

        Ok(())
    }

    /// Process an inbound network message coming from a peer.
    ///
    /// Returns an optional `NetworkMessage` that should be sent back to the
    /// caller (e.g. when responding to a block request). Most message types are
    /// handled asynchronously and therefore return `None`.
    pub async fn process_incoming_message(
        &self,
        from: &str,
        message: NetworkMessage,
    ) -> Result<Option<NetworkMessage>> {
        self.record_peer_seen(from);

        match message {
            NetworkMessage::Block(block) => {
                let canonical_from =
                    Self::canonicalize_address(from).unwrap_or_else(|_| from.to_string());
                self.emit_event(NetworkEvent::Block {
                    from: canonical_from,
                    block,
                });
                Ok(None)
            }
            NetworkMessage::Transaction(transaction) => {
                let canonical_from =
                    Self::canonicalize_address(from).unwrap_or_else(|_| from.to_string());
                self.emit_event(NetworkEvent::Transaction {
                    from: canonical_from,
                    transaction,
                });
                Ok(None)
            }
            NetworkMessage::BlockRequest { hash } => {
                let canonical_from =
                    Self::canonicalize_address(from).unwrap_or_else(|_| from.to_string());
                self.emit_event(NetworkEvent::BlockRequest {
                    from: canonical_from,
                    hash,
                });
                Ok(None)
            }
            NetworkMessage::BlockResponse(block) => {
                let canonical_from =
                    Self::canonicalize_address(from).unwrap_or_else(|_| from.to_string());
                self.emit_event(NetworkEvent::BlockResponse {
                    from: canonical_from,
                    block,
                });
                Ok(None)
            }
            NetworkMessage::PeerInfo {
                peer_id,
                addresses,
                time_us,
            } => {
                self.handle_peer_info_message(from, peer_id, addresses, time_us)
                    .await?;
                Ok(None)
            }
            NetworkMessage::PeerDiscovery { peers } => {
                self.handle_peer_discovery_message(from, peers).await?;
                Ok(None)
            }
        }
    }

    /// Stop the P2P network
    pub async fn stop(&self) -> Result<()> {
        *self.is_running.write() = false;
        info!("Stopping HTTP P2P network");

        if self.config.enable_upnp {
            self.teardown_upnp_mapping().await;
        }

        // Wait a bit for the network loops to finish
        sleep(Duration::from_millis(100)).await;

        info!("HTTP P2P network stopped");
        Ok(())
    }

    /// Add a peer to the network
    pub async fn add_peer(&self, peer_address: String) -> Result<()> {
        let canonical = Self::canonicalize_address(&peer_address)?;

        self.touch_peer(&canonical, None, Some(ippan_time_now()), None, None, None);

        let inserted = {
            let mut peers = self.peers.write();
            if peers.contains(&canonical) {
                false
            } else if peers.len() >= self.config.max_peers {
                warn!(
                    "Peer limit reached ({}). Skipping peer {}",
                    self.config.max_peers, canonical
                );
                false
            } else {
                peers.insert(canonical.clone())
            }
        };

        if inserted {
            self.update_peer_count();
            info!("Added peer: {}", canonical);

            // Share our addresses with the newly added peer
            let addresses = self.local_advertised_addresses();
            let client = self.client.clone();
            let config = self.config.clone();
            if let Err(e) = Self::send_message_to_peer(
                &client,
                &canonical,
                &NetworkMessage::PeerInfo {
                    peer_id: self.local_peer_id.clone(),
                    addresses,
                    time_us: Some(ippan_time_now()),
                },
                &config,
            )
            .await
            {
                warn!("Failed to send peer info to {}: {}", canonical, e);
            }
        }

        Ok(())
    }

    /// Remove a peer from the network
    pub fn remove_peer(&self, peer_address: &str) {
        match Self::canonicalize_address(peer_address) {
            Ok(canonical) => {
                let removed = {
                    let mut peers = self.peers.write();
                    peers.remove(&canonical)
                };

                if removed {
                    self.update_peer_count();
                    info!("Removed peer: {}", canonical);
                    self.peer_metadata.write().remove(&canonical);
                }
            }
            Err(e) => {
                warn!(
                    "Failed to canonicalize peer {} for removal: {}",
                    peer_address, e
                );
            }
        }
    }

    /// Get message sender for sending messages
    pub fn get_message_sender(&self) -> mpsc::UnboundedSender<NetworkMessage> {
        self.message_sender.clone()
    }

    /// Get current peer count
    pub fn get_peer_count(&self) -> usize {
        *self.peer_count.read()
    }

    fn update_peer_count(&self) {
        let count = self.peers.read().len();
        *self.peer_count.write() = count;
    }

    /// Get local peer ID
    pub fn get_local_peer_id(&self) -> String {
        self.local_peer_id.clone()
    }

    /// Get listening address
    pub fn get_listening_address(&self) -> String {
        self.listen_address.clone()
    }

    /// Get list of peers
    pub fn get_peers(&self) -> Vec<String> {
        self.peers.read().iter().cloned().collect()
    }

    /// Get rich peer metadata snapshot
    pub fn get_peer_metadata(&self) -> Vec<PeerInfo> {
        let peers_set = self.peers.read().clone();
        self.peer_metadata
            .read()
            .values()
            .map(|record| PeerInfo {
                id: record.peer_id.clone().unwrap_or_default(),
                address: record.address.clone(),
                last_seen: record.last_seen,
                is_connected: peers_set.contains(&record.address),
            })
            .collect()
    }

    /// Get the address advertised to other peers
    pub fn get_announce_address(&self) -> String {
        self.announce_address.read().clone()
    }

    /// Queue a peer info announcement for the network
    pub fn announce_self(&self) -> Result<()> {
        let address = self.get_announce_address();
        let message = NetworkMessage::PeerInfo {
            peer_id: self.local_peer_id.clone(),
            addresses: vec![address],
            time_us: Some(ippan_time_now()),
        };
        self.message_sender.send(message)?;
        Ok(())
    }

    /// Broadcast a block to all peers
    pub async fn broadcast_block(&self, block: Block) -> Result<()> {
        let message = NetworkMessage::Block(block);
        self.message_sender.send(message)?;
        Ok(())
    }

    /// Broadcast a transaction to all peers
    pub async fn broadcast_transaction(&self, tx: Transaction) -> Result<()> {
        let message = NetworkMessage::Transaction(tx);
        self.message_sender.send(message)?;
        Ok(())
    }

    /// Request a block from peers
    pub async fn request_block(&self, hash: [u8; 32]) -> Result<()> {
        let message = NetworkMessage::BlockRequest { hash };
        self.message_sender.send(message)?;
        Ok(())
    }

    /// Request a block from a specific peer and wait for response
    pub async fn request_block_from_peer(
        &self,
        hash: [u8; 32],
        peer_address: &str,
    ) -> Result<Option<Block>> {
        let client = reqwest::Client::new();
        let message = NetworkMessage::BlockRequest { hash };

        let url = format!("{}/p2p/block-request", peer_address.trim_end_matches('/'));

        match client.post(&url).json(&message).send().await {
            Ok(response) if response.status().is_success() => {
                let response_text = response
                    .text()
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to read response: {}", e))?;

                let network_message: NetworkMessage = serde_json::from_str(&response_text)
                    .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;

                match network_message {
                    NetworkMessage::BlockResponse(block) => {
                        debug!("Received block response for hash: {}", hex::encode(hash));
                        Ok(Some(block))
                    }
                    _ => {
                        warn!("Unexpected response type for block request");
                        Ok(None)
                    }
                }
            }
            Ok(response) if response.status() == 404 => {
                debug!("Block not found on peer {}", peer_address);
                Ok(None)
            }
            Ok(response) => {
                warn!(
                    "Unexpected status code {} from peer {}",
                    response.status(),
                    peer_address
                );
                Ok(None)
            }
            Err(e) => {
                warn!("Failed to request block from peer {}: {}", peer_address, e);
                Ok(None)
            }
        }
    }

    fn local_advertised_addresses(&self) -> Vec<String> {
        let mut addresses = vec![self.announce_address.read().clone()];
        if !addresses.contains(&self.local_address) {
            addresses.push(self.local_address.clone());
        }
        addresses
    }

    async fn setup_connectivity(&mut self) -> Result<()> {
        let listen_url = Url::parse(&self.listen_address)?;
        let scheme = listen_url.scheme().to_string();
        let port = listen_url
            .port_or_known_default()
            .ok_or_else(|| anyhow!("listen address is missing a port"))?;

        if let Some(address) = self
            .determine_public_address(&scheme, port)
            .await?
            .or_else(|| self.local_network_address(&scheme, port))
        {
            *self.announce_address.write() = address.clone();
            info!("Announcing peer address: {}", address);
        } else {
            warn!(
                "Falling back to listen address for announcements: {}",
                self.listen_address
            );
            *self.announce_address.write() = self.listen_address.clone();
        }

        Ok(())
    }

    async fn determine_public_address(
        &mut self,
        scheme: &str,
        port: u16,
    ) -> Result<Option<String>> {
        if let Some(public_host) = self.config.public_host.as_ref() {
            let address = self.build_public_host_address(scheme, port, public_host)?;
            return Ok(Some(address));
        }

        if self.config.enable_upnp {
            match self.try_setup_upnp(port, scheme).await {
                Ok(Some(address)) => return Ok(Some(address)),
                Ok(None) => {}
                Err(e) => warn!("UPnP setup failed: {}", e),
            }
        }

        if let Some(address) = self.fetch_external_ip(scheme, port).await {
            return Ok(Some(address));
        }

        Ok(None)
    }

    fn build_public_host_address(
        &self,
        scheme: &str,
        port: u16,
        public_host: &str,
    ) -> Result<String> {
        if let Ok(url) = Url::parse(public_host) {
            let host = url
                .host_str()
                .ok_or_else(|| anyhow!("public host URL missing host"))?;
            let port = url.port().unwrap_or(port);
            let scheme = url.scheme().to_string();
            return Self::build_address(&scheme, host, port);
        }

        Self::build_address(scheme, public_host, port)
    }

    async fn try_setup_upnp(&self, port: u16, scheme: &str) -> Result<Option<String>> {
        let gateway = match search_gateway(igd::SearchOptions::default()).await {
            Ok(gateway) => gateway,
            Err(e) => {
                debug!("No UPnP gateway detected: {}", e);
                return Ok(None);
            }
        };

        let local_ip = match local_ip() {
            Ok(IpAddr::V4(ipv4)) => ipv4,
            Ok(other) => {
                debug!("UPnP requires IPv4 local address, found {}", other);
                return Ok(None);
            }
            Err(e) => {
                debug!("Failed to determine local IP for UPnP: {}", e);
                return Ok(None);
            }
        };

        let socket = SocketAddrV4::new(local_ip, port);
        match gateway
            .add_port(PortMappingProtocol::TCP, port, socket, 0, "ippan-node")
            .await
        {
            Ok(()) => match gateway.get_external_ip().await {
                Ok(public_ip) => {
                    *self.upnp_mapping_active.write() = true;
                    let address = Self::build_address(scheme, &public_ip.to_string(), port)?;
                    info!("Established UPnP port mapping: {} -> {}", address, socket);
                    Ok(Some(address))
                }
                Err(e) => {
                    warn!("Failed to retrieve external IP via UPnP: {}", e);
                    Ok(None)
                }
            },
            Err(e) => {
                debug!("Failed to create UPnP port mapping: {}", e);
                Ok(None)
            }
        }
    }

    async fn teardown_upnp_mapping(&self) {
        if !*self.upnp_mapping_active.read() {
            return;
        }

        let listen_port = match Url::parse(&self.listen_address)
            .ok()
            .and_then(|url| url.port_or_known_default())
        {
            Some(port) => port,
            None => return,
        };

        match search_gateway(igd::SearchOptions::default()).await {
            Ok(gateway) => {
                match gateway
                    .remove_port(PortMappingProtocol::TCP, listen_port)
                    .await
                {
                    Ok(()) => info!("Removed UPnP port mapping on {}", listen_port),
                    Err(e) => warn!("Failed to remove UPnP port mapping: {}", e),
                }
            }
            Err(e) => debug!("Failed to contact UPnP gateway for removal: {}", e),
        }

        *self.upnp_mapping_active.write() = false;
    }

    async fn fetch_external_ip(&self, scheme: &str, port: u16) -> Option<String> {
        for service in &self.config.external_ip_services {
            match self.client.get(service).send().await {
                Ok(response) => {
                    if !response.status().is_success() {
                        debug!(
                            "External IP service {} returned status {}",
                            service,
                            response.status()
                        );
                        continue;
                    }

                    match response.text().await {
                        Ok(body) => {
                            let ip_str = body.trim();
                            if ip_str.is_empty() {
                                debug!("External IP service {} returned empty body", service);
                                continue;
                            }

                            if ip_str.parse::<IpAddr>().is_err() {
                                debug!(
                                    "External IP service {} returned invalid IP: {}",
                                    service, ip_str
                                );
                                continue;
                            }

                            match Self::build_address(scheme, ip_str, port) {
                                Ok(address) => return Some(address),
                                Err(e) => {
                                    debug!(
                                        "Failed to build announce address from {}: {}",
                                        ip_str, e
                                    );
                                }
                            }
                        }
                        Err(e) => debug!(
                            "Failed to read response from external IP service {}: {}",
                            service, e
                        ),
                    }
                }
                Err(e) => debug!("External IP service {} request failed: {}", service, e),
            }
        }

        None
    }

    fn local_network_address(&self, scheme: &str, port: u16) -> Option<String> {
        match local_ip() {
            Ok(ip) => Self::build_address(scheme, &ip.to_string(), port).ok(),
            Err(e) => {
                debug!("Failed to determine local network IP: {}", e);
                None
            }
        }
    }

    async fn handle_outgoing_message(
        client: &Client,
        peers: &Arc<RwLock<HashSet<String>>>,
        config: &P2PConfig,
        announce_address: &Arc<RwLock<String>>,
        local_address: &str,
        message: NetworkMessage,
    ) {
        let peer_list = peers.read().clone();
        if peer_list.is_empty() {
            return;
        }

        let announce_address = announce_address.read().clone();
        let mut self_addresses = vec![local_address.to_string()];
        if !self_addresses.contains(&announce_address) {
            self_addresses.push(announce_address);
        }

        for peer_address in peer_list {
            if self_addresses.contains(&peer_address) {
                continue;
            }

            let client = client.clone();
            let message = message.clone();
            let config = config.clone();

            tokio::spawn(async move {
                if let Err(e) =
                    Self::send_message_to_peer(&client, &peer_address, &message, &config).await
                {
                    warn!("Failed to send message to peer {}: {}", peer_address, e);
                }
            });
        }
    }

    async fn send_message_to_peer(
        client: &Client,
        peer_address: &str,
        message: &NetworkMessage,
        config: &P2PConfig,
    ) -> Result<()> {
        let endpoint = match message {
            NetworkMessage::Block(_) => "/p2p/blocks",
            NetworkMessage::Transaction(_) => "/p2p/transactions",
            NetworkMessage::BlockRequest { .. } => "/p2p/block-request",
            NetworkMessage::BlockResponse(_) => "/p2p/block-response",
            NetworkMessage::PeerInfo { .. } => "/p2p/peer-info",
            NetworkMessage::PeerDiscovery { .. } => "/p2p/peer-discovery",
        };

        let url = format!("{peer_address}{endpoint}");

        for attempt in 1..=config.retry_attempts {
            match client.post(&url).json(message).send().await {
                Ok(response) if response.status().is_success() => {
                    debug!("Successfully sent message to peer {}", peer_address);
                    return Ok(());
                }
                Ok(response) => {
                    if attempt == config.retry_attempts {
                        return Err(P2PError::Peer(format!(
                            "Peer {} returned status {}",
                            peer_address,
                            response.status()
                        ))
                        .into());
                    }
                    debug!(
                        "Peer {} responded with status {} (attempt {}/{})",
                        peer_address,
                        response.status(),
                        attempt,
                        config.retry_attempts
                    );
                }
                Err(e) => {
                    if attempt == config.retry_attempts {
                        return Err(P2PError::Peer(format!(
                            "Failed to send to {} after {} attempts: {}",
                            peer_address, config.retry_attempts, e
                        ))
                        .into());
                    }
                    debug!(
                        "Attempt {} failed for peer {}: {}",
                        attempt, peer_address, e
                    );
                }
            }

            sleep(Duration::from_millis(100 * attempt as u64)).await;
        }

        Ok(())
    }

    async fn discover_peers(
        client: &Client,
        peers: &Arc<RwLock<HashSet<String>>>,
        peer_metadata: &Arc<RwLock<HashMap<String, PeerRecord>>>,
        incoming_sender: mpsc::UnboundedSender<NetworkEvent>,
        local_address: &str,
        announce_address: &Arc<RwLock<String>>,
        peer_count: &Arc<RwLock<usize>>,
    ) {
        let peer_list = peers.read().clone();
        if peer_list.is_empty() {
            return;
        }

        for peer_address in peer_list {
            let client = client.clone();
            let peers = peers.clone();
            let peer_metadata = peer_metadata.clone();
            let local_address = local_address.to_string();
            let announce_address = announce_address.clone();
            let peer_count = peer_count.clone();
            let incoming_sender = incoming_sender.clone();

            tokio::spawn(async move {
                let announced = announce_address.read().clone();
                if let Err(e) = Self::request_peers_from_peer(
                    &client,
                    &peer_address,
                    &peers,
                    &peer_metadata,
                    incoming_sender.clone(),
                    &local_address,
                    &announced,
                    &peer_count,
                )
                .await
                {
                    debug!("Failed to discover peers from {}: {}", peer_address, e);
                }
            });
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn request_peers_from_peer(
        client: &Client,
        peer_address: &str,
        peers: &Arc<RwLock<HashSet<String>>>,
        peer_metadata: &Arc<RwLock<HashMap<String, PeerRecord>>>,
        incoming_sender: mpsc::UnboundedSender<NetworkEvent>,
        local_address: &str,
        announce_address: &str,
        peer_count: &Arc<RwLock<usize>>,
    ) -> Result<()> {
        let url = format!("{peer_address}/p2p/peers");

        let response = client.get(&url).send().await?;
        if response.status().is_success() {
            let peer_list: Vec<String> = response.json().await?;

            let mut current_peers = peers.write();
            let mut discovered = Vec::new();
            for peer in peer_list {
                match Self::canonicalize_address(&peer) {
                    Ok(canonical) => {
                        if canonical != local_address && canonical != announce_address {
                            let inserted = current_peers.insert(canonical.clone());

                            {
                                let mut records = peer_metadata.write();
                                let entry = records
                                    .entry(canonical.clone())
                                    .or_insert_with(|| PeerRecord::new(canonical.clone()));
                                entry.last_seen = ippan_time_now();
                            }

                            if inserted {
                                info!("Discovered new peer: {}", canonical);
                                discovered.push(canonical.clone());
                            }
                        }
                    }
                    Err(e) => debug!("Invalid peer address {} from {}: {}", peer, peer_address, e),
                }
            }

            *peer_count.write() = current_peers.len();

            if !discovered.is_empty() {
                let _ = incoming_sender.send(NetworkEvent::PeerDiscovery {
                    from: peer_address.to_string(),
                    peers: discovered,
                });
            }
        }

        Ok(())
    }

    fn canonicalize_address(address: &str) -> Result<String> {
        let candidate = if address.contains("://") {
            address.to_string()
        } else {
            format!("http://{address}")
        };

        let url = Url::parse(&candidate)?;
        let scheme = url.scheme().to_string();
        let host = url
            .host_str()
            .ok_or_else(|| anyhow!("peer address missing host"))?
            .to_string();
        let port = url
            .port_or_known_default()
            .ok_or_else(|| anyhow!("peer address missing port"))?;

        Self::build_address(&scheme, &host, port)
    }

    fn build_address(scheme: &str, host: &str, port: u16) -> Result<String> {
        // Wrap IPv6 hosts with [] if missing
        let formatted_host = if host.contains(':') && !host.starts_with('[') && !host.ends_with(']')
        {
            format!("[{host}]")
        } else {
            host.to_string()
        };

        let mut url = Url::parse(&format!("{scheme}://{formatted_host}:{port}/"))?;
        url.set_path("");
        url.set_query(None);
        url.set_fragment(None);
        let formatted = url.to_string();
        Ok(formatted.trim_end_matches('/').to_string())
    }
}

#[cfg(all(test, feature = "enable-tests"))]
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

        assert!(
            matches!(deserialized, NetworkMessage::Block(_)),
            "Expected Block message, got: {:?}",
            deserialized
        );
    }

    #[tokio::test]
    async fn test_peer_management() {
        let config = P2PConfig::default();
        let network = HttpP2PNetwork::new(config, "http://localhost:9000".to_string()).unwrap();

        // Test adding peers
        network
            .add_peer("http://localhost:9001".to_string())
            .await
            .unwrap();
        network
            .add_peer("http://localhost:9002".to_string())
            .await
            .unwrap();

        assert_eq!(network.get_peer_count(), 2);
        assert_eq!(network.get_peers().len(), 2);

        // Test removing peers
        network.remove_peer("http://localhost:9001");
        assert_eq!(network.get_peer_count(), 1);
    }

    #[tokio::test]
    async fn test_process_incoming_block_event() {
        let config = P2PConfig::default();
        let network = HttpP2PNetwork::new(config, "http://localhost:9000".to_string()).unwrap();
        let mut events = network
            .take_incoming_events()
            .expect("expected event receiver");

        let block = Block::new(vec![[0u8; 32]], vec![], 1, [1u8; 32]);

        network
            .process_incoming_message(
                "http://localhost:9001",
                NetworkMessage::Block(block.clone()),
            )
            .await
            .unwrap();

        match events.recv().await.unwrap() {
            NetworkEvent::Block {
                from,
                block: received,
            } => {
                assert_eq!(from, "http://localhost:9001");
                assert_eq!(received.hash(), block.hash());
            }
            other => panic!("Unexpected event: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_peer_info_updates_metadata() {
        let config = P2PConfig::default();
        let network = HttpP2PNetwork::new(config, "http://localhost:9000".to_string()).unwrap();
        let mut events = network
            .take_incoming_events()
            .expect("expected event receiver");

        network
            .process_incoming_message(
                "http://localhost:9001",
                NetworkMessage::PeerInfo {
                    peer_id: "peer-1".to_string(),
                    addresses: vec!["http://localhost:9001".to_string()],
                    time_us: Some(123_456),
                },
            )
            .await
            .unwrap();

        // Drain event channel to avoid cascading into other tests.
        assert!(matches!(
            events.recv().await.unwrap(),
            NetworkEvent::PeerInfo { .. }
        ));

        let metadata = network.get_peer_metadata();
        assert_eq!(metadata.len(), 1);
        let peer = &metadata[0];
        assert_eq!(peer.id, "peer-1");
        assert_eq!(peer.address, "http://localhost:9001");
        assert_eq!(peer.last_seen, 123_456);
        assert!(peer.is_connected);
    }

    #[tokio::test]
    async fn test_peer_discovery_adds_peer() {
        let config = P2PConfig::default();
        let network = HttpP2PNetwork::new(config, "http://localhost:9000".to_string()).unwrap();
        let mut events = network
            .take_incoming_events()
            .expect("expected event receiver");

        network
            .process_incoming_message(
                "http://localhost:9001",
                NetworkMessage::PeerDiscovery {
                    peers: vec!["http://localhost:9002".to_string()],
                },
            )
            .await
            .unwrap();

        match events.recv().await.unwrap() {
            NetworkEvent::PeerDiscovery { peers, .. } => {
                assert_eq!(peers, vec!["http://localhost:9002".to_string()]);
            }
            other => panic!("Unexpected event: {other:?}"),
        }

        assert!(network
            .get_peers()
            .contains(&"http://localhost:9002".to_string()));
    }
}
