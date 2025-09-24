use anyhow::{anyhow, Context, Result};
use igd::aio::search_gateway;
use igd::PortMappingProtocol;
use ippan_types::{Block, Transaction};
use local_ip_address::local_ip;
use parking_lot::RwLock;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr, SocketAddrV4};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::{interval, sleep, timeout};
use tracing::{debug, error, info, warn};
use url::Url;

/// P2P network errors
#[derive(thiserror::Error, Debug)]
pub enum P2PError {
    #[error("Channel error: {0}")]
    Channel(#[from] mpsc::error::SendError<NetworkMessage>),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("URL error: {0}")]
    Url(#[from] url::ParseError),
    #[error("Peer error: {0}")]
    Peer(String),
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
    message_sender: mpsc::UnboundedSender<NetworkMessage>,
    message_receiver: Option<mpsc::UnboundedReceiver<NetworkMessage>>,
    peer_count: Arc<RwLock<usize>>,
    is_running: Arc<RwLock<bool>>,
    local_peer_id: String,
    listen_address: String,
    listen_scheme: String,
    listen_port: u16,
    announce_address: Arc<RwLock<String>>,
    upnp_mapping_active: Arc<RwLock<bool>>,
}

impl HttpP2PNetwork {
    /// Create a new HTTP P2P network
    pub fn new(config: P2PConfig, local_address: String) -> Result<Self> {
        let url = Url::parse(&local_address)
            .with_context(|| format!("invalid listen address: {local_address}"))?;
        let listen_scheme = url.scheme().to_string();
        let listen_host = url
            .host_str()
            .ok_or_else(|| anyhow!("listen address is missing host"))?;
        let listen_port = url
            .port_or_known_default()
            .ok_or_else(|| anyhow!("listen address must include a port"))?;

        let client = Client::builder().timeout(config.message_timeout).build()?;

        let local_peer_id = format!(
            "ippan-peer-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        );

        let (message_sender, message_receiver) = mpsc::unbounded_channel();
        let listen_address = format!("{}://{}:{}", listen_scheme, listen_host, listen_port);

        Ok(Self {
            config,
            client,
            peers: Arc::new(RwLock::new(HashSet::new())),
            message_sender,
            message_receiver: Some(message_receiver),
            peer_count: Arc::new(RwLock::new(0)),
            is_running: Arc::new(RwLock::new(false)),
            local_peer_id,
            listen_address: listen_address.clone(),
            listen_scheme,
            listen_port,
            announce_address: Arc::new(RwLock::new(listen_address)),
            upnp_mapping_active: Arc::new(RwLock::new(false)),
        })
    }
    /// Start the P2P network
    pub async fn start(&mut self) -> Result<()> {
        *self.is_running.write() = true;
        info!("Starting HTTP P2P network on {}", self.listen_address);

        self.configure_connectivity().await?;

        for peer in &self.config.bootstrap_peers {
            if let Err(error) = self.add_peer(peer.clone()).await {
                warn!("Failed to add bootstrap peer {}: {}", peer, error);
            }
        }

        let message_loop = {
            let is_running = self.is_running.clone();
            let peers = self.peers.clone();
            let client = self.client.clone();
            let config = self.config.clone();
            let announce_address = self.announce_address.clone();
            let mut message_receiver = self.message_receiver.take().unwrap();

            tokio::spawn(async move {
                while *is_running.read() {
                    match message_receiver.recv().await {
                        Some(msg) => {
                            let local_address = {
                                let guard = announce_address.read();
                                guard.clone()
                            };
                            Self::broadcast_message(&client, &local_address, &peers, &config, msg)
                                .await;
                        }
                        None => break,
                    }

                    sleep(Duration::from_millis(10)).await;
                }
            })
        };

        let discovery_loop = {
            let is_running = self.is_running.clone();
            let peers = self.peers.clone();
            let client = self.client.clone();
            let config = self.config.clone();
            let announce_address = self.announce_address.clone();
            let peer_count = self.peer_count.clone();

            tokio::spawn(async move {
                let mut interval = interval(config.peer_discovery_interval);

                while *is_running.read() {
                    interval.tick().await;
                    let local_address = {
                        let guard = announce_address.read();
                        guard.clone()
                    };

                    Self::discover_peers(&client, &peers, &peer_count, &config, &local_address)
                        .await;
                }
            })
        };

        let announce_loop = {
            let is_running = self.is_running.clone();
            let peers = self.peers.clone();
            let client = self.client.clone();
            let config = self.config.clone();
            let announce_address = self.announce_address.clone();
            let peer_id = self.local_peer_id.clone();

            tokio::spawn(async move {
                let mut interval = interval(config.peer_announce_interval);

                while *is_running.read() {
                    interval.tick().await;

                    let current_address = {
                        let guard = announce_address.read();
                        guard.clone()
                    };

                    let message = NetworkMessage::PeerInfo {
                        peer_id: peer_id.clone(),
                        addresses: vec![current_address.clone()],
                    };

                    Self::broadcast_message(&client, &current_address, &peers, &config, message)
                        .await;
                }
            })
        };

        drop(message_loop);
        drop(discovery_loop);
        drop(announce_loop);

        let _ = self.message_sender.send(NetworkMessage::PeerInfo {
            peer_id: self.local_peer_id.clone(),
            addresses: vec![self.get_announce_address()],
        });

        info!("HTTP P2P network started");
        Ok(())
    }

    /// Stop the P2P network
    pub async fn stop(&self) -> Result<()> {
        *self.is_running.write() = false;
        info!("Stopping HTTP P2P network");

        sleep(Duration::from_millis(100)).await;
        self.cleanup_connectivity().await;

        info!("HTTP P2P network stopped");
        Ok(())
    }

    /// Add a peer to the network
    pub async fn add_peer(&self, peer_address: String) -> Result<()> {
        let canonical = Self::canonicalize_address(&peer_address, &self.listen_scheme)?;
        let self_address = self.get_announce_address();

        if canonical == self_address {
            return Ok(());
        }

        {
            let mut peers = self.peers.write();

            if peers.contains(&canonical) {
                return Ok(());
            }

            if peers.len() >= self.config.max_peers {
                warn!(
                    "Peer limit reached ({}). Skipping peer {}",
                    self.config.max_peers, canonical
                );
                return Ok(());
            }

            peers.insert(canonical.clone());
        }

        self.update_peer_count();
        info!("Added peer: {}", canonical);

        let client = self.client.clone();
        let config = self.config.clone();
        let peer_id = self.local_peer_id.clone();
        let announce_address = self.get_announce_address();
        tokio::spawn(async move {
            let message = NetworkMessage::PeerInfo {
                peer_id,
                addresses: vec![announce_address],
            };

            if let Err(err) =
                Self::send_message_to_peer(&client, &canonical, &message, &config).await
            {
                warn!("Failed to send peer info to {}: {}", canonical, err);
            }
        });

        Ok(())
    }

    /// Remove a peer from the network
    pub fn remove_peer(&self, peer_address: &str) {
        match Self::canonicalize_address(peer_address, &self.listen_scheme) {
            Ok(canonical) => {
                let removed = {
                    let mut peers = self.peers.write();
                    peers.remove(&canonical)
                };

                if removed {
                    self.update_peer_count();
                    info!("Removed peer: {}", canonical);
                }
            }
            Err(e) => warn!(
                "Failed to canonicalize peer {} for removal: {}",
                peer_address, e
            ),
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

    /// Get local peer ID
    pub fn get_local_peer_id(&self) -> String {
        self.local_peer_id.clone()
    }

    /// Get listening address
    pub fn get_listening_address(&self) -> String {
        self.listen_address.clone()
    }

    /// Get announce address shared with peers
    pub fn get_announce_address(&self) -> String {
        self.announce_address.read().clone()
    }

    /// Get list of peers
    pub fn get_peers(&self) -> Vec<String> {
        let mut peers = self.peers.read().iter().cloned().collect::<Vec<_>>();
        peers.sort();
        peers
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
    async fn broadcast_message(
        client: &Client,
        local_address: &str,
        peers: &Arc<RwLock<HashSet<String>>>,
        config: &P2PConfig,
        message: NetworkMessage,
    ) {
        let peer_list = peers.read().clone();

        for peer_address in peer_list {
            if peer_address == local_address {
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

        let url = format!("{}{}", peer_address.trim_end_matches('/'), endpoint);

        for attempt in 1..=config.retry_attempts {
            match client.post(&url).json(message).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        debug!("Successfully sent message to peer {}", peer_address);
                        return Ok(());
                    } else {
                        warn!(
                            "Peer {} returned status: {}",
                            peer_address,
                            response.status()
                        );
                    }
                }
                Err(e) => {
                    if attempt == config.retry_attempts {
                        return Err(P2PError::Peer(format!(
                            "Failed to send to {} after {} attempts: {}",
                            peer_address, config.retry_attempts, e
                        ))
                        .into());
                    }
                    warn!(
                        "Attempt {} failed for peer {}: {}",
                        attempt, peer_address, e
                    );
                    sleep(Duration::from_millis(100 * attempt as u64)).await;
                }
            }
        }

        Ok(())
    }

    async fn discover_peers(
        client: &Client,
        peers: &Arc<RwLock<HashSet<String>>>,
        peer_count: &Arc<RwLock<usize>>,
        config: &P2PConfig,
        local_address: &str,
    ) {
        let peer_list = peers.read().clone();

        for peer_address in peer_list {
            let client = client.clone();
            let peers = peers.clone();
            let peer_count = peer_count.clone();
            let config = config.clone();
            let local_address = local_address.to_string();

            tokio::spawn(async move {
                if let Err(e) = Self::request_peers_from_peer(
                    &client,
                    &peer_address,
                    &peers,
                    &peer_count,
                    &config,
                    &local_address,
                )
                .await
                {
                    debug!("Failed to discover peers from {}: {}", peer_address, e);
                }
            });
        }
    }

    async fn request_peers_from_peer(
        client: &Client,
        peer_address: &str,
        peers: &Arc<RwLock<HashSet<String>>>,
        peer_count: &Arc<RwLock<usize>>,
        config: &P2PConfig,
        local_address: &str,
    ) -> Result<()> {
        let url = format!("{}/p2p/peers", peer_address.trim_end_matches('/'));

        let response = client.get(&url).send().await?;
        if response.status().is_success() {
            let peer_list: Vec<String> = response.json().await?;

            let mut new_peers = false;
            for peer in peer_list {
                if let Ok(canonical) = Self::canonicalize_address(&peer, &config.listen_address) {
                    if canonical != local_address {
                        let inserted = {
                            let mut current = peers.write();
                            if current.len() >= config.max_peers {
                                false
                            } else if current.contains(&canonical) {
                                false
                            } else {
                                current.insert(canonical.clone());
                                true
                            }
                        };

                        if inserted {
                            new_peers = true;
                            info!("Discovered new peer: {}", canonical);
                        }
                    }
                }
            }

            if new_peers {
                let count = peers.read().len();
                *peer_count.write() = count;
            }
        }

        Ok(())
    }

    fn update_peer_count(&self) {
        let count = self.peers.read().len();
        *self.peer_count.write() = count;
    }

    fn canonicalize_address(address: &str, default_scheme: &str) -> Result<String> {
        if address.trim().is_empty() {
            return Err(anyhow!("address cannot be empty"));
        }

        let scheme_hint = if default_scheme.contains("://") {
            Url::parse(default_scheme)
                .map(|url| url.scheme().to_string())
                .unwrap_or_else(|_| "http".to_string())
        } else {
            default_scheme.to_string()
        };

        let normalized = if address.contains("://") {
            Url::parse(address)?
        } else {
            let url = format!("{}://{}", scheme_hint, address);
            Url::parse(&url)?
        };

        let scheme = normalized.scheme();
        if scheme != "http" && scheme != "https" {
            return Err(anyhow!("unsupported scheme: {}", scheme));
        }

        let host = normalized
            .host_str()
            .ok_or_else(|| anyhow!("peer address missing host"))?;
        let port = normalized
            .port_or_known_default()
            .ok_or_else(|| anyhow!("peer address missing port"))?;

        Ok(format!("{}://{}:{}", scheme, host, port))
    }
    async fn configure_connectivity(&self) -> Result<()> {
        let scheme = self.listen_scheme.clone();
        let port = self.listen_port;

        if let Some(public_host) = &self.config.public_host {
            let announce = format!("{}://{}:{}", scheme, public_host, port);
            *self.announce_address.write() = announce.clone();
            info!("Announcing public host {}", announce);
            return Ok(());
        }

        if self.config.enable_upnp {
            match self.try_configure_upnp().await {
                Ok(Some(external_ip)) => {
                    let announce = format!("{}://{}:{}", scheme, external_ip, port);
                    *self.announce_address.write() = announce.clone();
                    info!("UPnP mapped external address {}", announce);
                    return Ok(());
                }
                Ok(None) => {
                    debug!("UPnP not available; falling back to external IP detection");
                }
                Err(err) => {
                    warn!("Failed to configure UPnP: {}", err);
                }
            }
        }

        if let Some(external_ip) = self.detect_external_ip().await {
            let announce = format!("{}://{}:{}", scheme, external_ip, port);
            *self.announce_address.write() = announce.clone();
            info!("Detected external address {}", announce);
            return Ok(());
        }

        if let Ok(local_ip) = local_ip() {
            let announce = format!("{}://{}:{}", scheme, local_ip, port);
            *self.announce_address.write() = announce.clone();
            info!("Using local address {}", announce);
        }

        Ok(())
    }

    async fn cleanup_connectivity(&self) {
        if !self.config.enable_upnp {
            return;
        }

        if !*self.upnp_mapping_active.read() {
            return;
        }

        match timeout(Duration::from_secs(3), search_gateway(Default::default())).await {
            Ok(Ok(gateway)) => {
                if let Err(err) = gateway
                    .remove_port(PortMappingProtocol::TCP, self.listen_port)
                    .await
                {
                    warn!("Failed to remove UPnP port mapping: {}", err);
                } else {
                    info!("Removed UPnP port mapping for {}", self.listen_port);
                    *self.upnp_mapping_active.write() = false;
                }
            }
            Ok(Err(err)) => warn!("Failed to locate gateway for UPnP cleanup: {}", err),
            Err(_) => warn!("UPnP cleanup timed out"),
        }
    }

    async fn try_configure_upnp(&self) -> Result<Option<Ipv4Addr>> {
        let gateway =
            match timeout(Duration::from_secs(3), search_gateway(Default::default())).await {
                Ok(Ok(gateway)) => gateway,
                Ok(Err(err)) => {
                    debug!("UPnP gateway discovery failed: {}", err);
                    return Ok(None);
                }
                Err(_) => {
                    debug!("UPnP gateway discovery timed out");
                    return Ok(None);
                }
            };

        let local_addr = match self.internal_socket_v4() {
            Some(addr) => addr,
            None => {
                debug!("No IPv4 address available for UPnP mapping");
                return Ok(None);
            }
        };

        let external_ip = gateway.get_external_ip().await?;
        gateway
            .add_port(
                PortMappingProtocol::TCP,
                self.listen_port,
                local_addr,
                0,
                "ippan-node",
            )
            .await?;

        *self.upnp_mapping_active.write() = true;
        Ok(Some(external_ip))
    }

    fn internal_socket_v4(&self) -> Option<SocketAddrV4> {
        match local_ip() {
            Ok(IpAddr::V4(ip)) => Some(SocketAddrV4::new(ip, self.listen_port)),
            Ok(IpAddr::V6(_)) => None,
            Err(_) => None,
        }
    }

    async fn detect_external_ip(&self) -> Option<IpAddr> {
        for service in &self.config.external_ip_services {
            match self.client.get(service).send().await {
                Ok(response) => {
                    if !response.status().is_success() {
                        continue;
                    }

                    match response.text().await {
                        Ok(body) => {
                            let text = body.trim();
                            if let Ok(ip) = text.parse::<IpAddr>() {
                                return Some(ip);
                            }
                        }
                        Err(err) => {
                            debug!("Failed to read response from {}: {}", service, err);
                        }
                    }
                }
                Err(err) => {
                    debug!("Failed to query external IP from {}: {}", service, err);
                }
            }
        }

        None
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
        let block = Block::new([0u8; 32], vec![], 1, [1u8; 32]);
        let message = NetworkMessage::Block(block);

        let serialized = serde_json::to_vec(&message).unwrap();
        let deserialized: NetworkMessage = serde_json::from_slice(&serialized).unwrap();

        match deserialized {
            NetworkMessage::Block(_) => {}
            _ => panic!("Expected Block message"),
        }
    }

    #[tokio::test]
    async fn test_peer_management() {
        let config = P2PConfig::default();
        let network = HttpP2PNetwork::new(config, "http://localhost:9000".to_string()).unwrap();

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

        network.remove_peer("http://localhost:9001");
        assert_eq!(network.get_peer_count(), 1);
    }
}
