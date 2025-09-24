use anyhow::{anyhow, Result};
use ippan_types::{Block, Transaction};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::net::{IpAddr, SocketAddrV4};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::{interval, sleep};
use tracing::{debug, info, warn};
use url::Url;

use igd::aio::search_gateway;
use igd::PortMappingProtocol;
use local_ip_address::local_ip;

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
    peers: Arc<parking_lot::RwLock<HashSet<String>>>,
    message_sender: mpsc::UnboundedSender<NetworkMessage>,
    message_receiver: Option<mpsc::UnboundedReceiver<NetworkMessage>>,
    peer_count: Arc<parking_lot::RwLock<usize>>,
    is_running: Arc<parking_lot::RwLock<bool>>,
    local_peer_id: String,
    listen_address: String,
    listen_port: u16,
    announce_address: Arc<parking_lot::RwLock<String>>,
    upnp_mapping_active: Arc<parking_lot::RwLock<bool>>,
}

impl HttpP2PNetwork {
    /// Create a new HTTP P2P network
    pub fn new(config: P2PConfig, local_address: String) -> Result<Self> {
        let client = Client::builder().timeout(config.message_timeout).build()?;

        let listen_address = Self::canonicalize_address(&local_address)?;
        let listen_port = Self::extract_port(&listen_address)?;

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

        Ok(Self {
            config,
            client,
            peers: Arc::new(parking_lot::RwLock::new(HashSet::new())),
            message_sender,
            message_receiver: Some(message_receiver),
            peer_count: Arc::new(parking_lot::RwLock::new(0)),
            is_running: Arc::new(parking_lot::RwLock::new(false)),
            local_peer_id,
            listen_address: listen_address.clone(),
            listen_port,
            announce_address: Arc::new(parking_lot::RwLock::new(listen_address)),
            upnp_mapping_active: Arc::new(parking_lot::RwLock::new(false)),
        })
    }

    /// Start the P2P network
    pub async fn start(&mut self) -> Result<()> {
        if *self.is_running.read() {
            return Ok(());
        }

        *self.is_running.write() = true;
        info!("Starting HTTP P2P network on {}", self.listen_address);

        Self::refresh_announce_address_inner(
            &self.client,
            &self.config,
            &self.listen_address,
            self.listen_port,
            &self.announce_address,
            &self.upnp_mapping_active,
        )
        .await?;

        // Add bootstrap peers
        for peer in self.config.bootstrap_peers.clone() {
            if let Err(error) = self.add_peer(peer).await {
                warn!("Failed to add bootstrap peer: {}", error);
            }
        }

        // Start the message processing loop
        let is_running = self.is_running.clone();
        let peers = self.peers.clone();
        let client = self.client.clone();
        let config = self.config.clone();
        let announce_address = self.announce_address.clone();
        let mut message_receiver = self.message_receiver.take().unwrap();

        tokio::spawn(async move {
            loop {
                if !*is_running.read() {
                    break;
                }

                match message_receiver.recv().await {
                    Some(message) => {
                        let local_address = announce_address.read().clone();
                        HttpP2PNetwork::handle_outgoing_message(
                            &client,
                            &peers,
                            &config,
                            &local_address,
                            message,
                        )
                        .await;
                    }
                    None => {
                        // Channel closed; give the loop a chance to exit
                        sleep(Duration::from_millis(10)).await;
                    }
                }
            }
        });

        // Start peer discovery loop
        let discovery_is_running = self.is_running.clone();
        let discovery_peers = self.peers.clone();
        let discovery_client = self.client.clone();
        let discovery_interval = self.config.peer_discovery_interval;
        let discovery_announce = self.announce_address.clone();

        tokio::spawn(async move {
            let mut ticker = interval(discovery_interval);
            loop {
                ticker.tick().await;
                if !*discovery_is_running.read() {
                    break;
                }

                let local_address = discovery_announce.read().clone();
                HttpP2PNetwork::discover_peers(&discovery_client, &discovery_peers, &local_address)
                    .await;
            }
        });

        // Periodically announce ourselves and refresh public address information
        let announce_is_running = self.is_running.clone();
        let announce_sender = self.message_sender.clone();
        let announce_peer_id = self.local_peer_id.clone();
        let announce_address = self.announce_address.clone();
        let announce_client = self.client.clone();
        let announce_config = self.config.clone();
        let announce_interval = self.config.peer_announce_interval;
        let announce_upnp = self.upnp_mapping_active.clone();
        let listen_address = self.listen_address.clone();
        let listen_port = self.listen_port;

        tokio::spawn(async move {
            let mut ticker = interval(announce_interval);
            loop {
                ticker.tick().await;
                if !*announce_is_running.read() {
                    break;
                }

                if let Err(error) = HttpP2PNetwork::refresh_announce_address_inner(
                    &announce_client,
                    &announce_config,
                    &listen_address,
                    listen_port,
                    &announce_address,
                    &announce_upnp,
                )
                .await
                {
                    warn!("Failed to refresh announce address: {}", error);
                }

                let address = announce_address.read().clone();
                let message = NetworkMessage::PeerInfo {
                    peer_id: announce_peer_id.clone(),
                    addresses: vec![address],
                };

                if let Err(error) = announce_sender.send(message) {
                    warn!("Failed to broadcast peer info: {}", error);
                }
            }
        });

        info!("HTTP P2P network started");
        Ok(())
    }

    /// Stop the P2P network
    pub async fn stop(&self) -> Result<()> {
        *self.is_running.write() = false;
        info!("Stopping HTTP P2P network");

        // Tear down any UPnP mapping if we previously created one
        if self.config.enable_upnp && *self.upnp_mapping_active.read() {
            if let Err(error) = Self::remove_upnp_mapping(self.listen_port).await {
                warn!("Failed to remove UPnP port mapping: {}", error);
            }
            *self.upnp_mapping_active.write() = false;
        }

        // Wait a bit for the network loops to finish
        sleep(Duration::from_millis(100)).await;

        info!("HTTP P2P network stopped");
        Ok(())
    }

    /// Add a peer to the network
    pub async fn add_peer(&self, peer_address: String) -> Result<()> {
        let canonical = Self::canonicalize_address(&peer_address)?;

        if canonical == self.get_announce_address() {
            return Ok(());
        }

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
                peers.insert(canonical.clone());
                true
            }
        };

        if !inserted {
            return Ok(());
        }

        self.update_peer_count();
        info!("Added peer: {}", canonical);

        // Announce ourselves to the newly added peer so they learn our public address
        let client = self.client.clone();
        let config = self.config.clone();
        let peer_id = self.local_peer_id.clone();
        let announce_address = self.get_announce_address();

        tokio::spawn(async move {
            let message = NetworkMessage::PeerInfo {
                peer_id,
                addresses: vec![announce_address],
            };

            if let Err(error) =
                HttpP2PNetwork::send_message_to_peer(&client, &canonical, &message, &config).await
            {
                warn!("Failed to announce to peer {}: {}", canonical, error);
            }
        });

        Ok(())
    }

    /// Remove a peer from the network
    pub fn remove_peer(&self, peer_address: &str) {
        match Self::canonicalize_address(peer_address) {
            Ok(canonical) => {
                let removed = self.peers.write().remove(&canonical);
                if removed {
                    self.update_peer_count();
                    info!("Removed peer: {}", canonical);
                }
            }
            Err(error) => {
                warn!(
                    "Failed to canonicalize peer {} for removal: {}",
                    peer_address, error
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

    /// Get local peer ID
    pub fn get_local_peer_id(&self) -> String {
        self.local_peer_id.clone()
    }

    /// Get listening address
    pub fn get_listening_address(&self) -> String {
        self.listen_address.clone()
    }

    /// Get public announce address
    pub fn get_announce_address(&self) -> String {
        self.announce_address.read().clone()
    }

    /// Get list of peers
    pub fn get_peers(&self) -> Vec<String> {
        self.peers.read().iter().cloned().collect()
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

    fn update_peer_count(&self) {
        let count = self.peers.read().len();
        *self.peer_count.write() = count;
    }

    /// Handle outgoing messages by sending them to all peers
    async fn handle_outgoing_message(
        client: &Client,
        peers: &Arc<parking_lot::RwLock<HashSet<String>>>,
        config: &P2PConfig,
        local_address: &str,
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
                if let Err(error) =
                    HttpP2PNetwork::send_message_to_peer(&client, &peer_address, &message, &config)
                        .await
                {
                    warn!("Failed to send message to peer {}: {}", peer_address, error);
                }
            });
        }
    }

    /// Send a message to a specific peer
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

        let url = format!("{}{}", peer_address, endpoint);

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
                Err(error) => {
                    if attempt == config.retry_attempts {
                        return Err(P2PError::Peer(format!(
                            "Failed to send to {} after {} attempts: {}",
                            peer_address, config.retry_attempts, error
                        ))
                        .into());
                    }
                    warn!(
                        "Attempt {} failed for peer {}: {}",
                        attempt, peer_address, error
                    );
                    sleep(Duration::from_millis(100 * attempt as u64)).await;
                }
            }
        }

        Ok(())
    }

    /// Discover new peers by asking existing peers
    async fn discover_peers(
        client: &Client,
        peers: &Arc<parking_lot::RwLock<HashSet<String>>>,
        local_address: &str,
    ) {
        let peer_list = peers.read().clone();

        for peer_address in peer_list {
            let client = client.clone();
            let peers = peers.clone();
            let local_address = local_address.to_string();

            tokio::spawn(async move {
                if let Err(error) = HttpP2PNetwork::request_peers_from_peer(
                    &client,
                    &peer_address,
                    &peers,
                    &local_address,
                )
                .await
                {
                    debug!("Failed to discover peers from {}: {}", peer_address, error);
                }
            });
        }
    }

    /// Request peer list from a specific peer
    async fn request_peers_from_peer(
        client: &Client,
        peer_address: &str,
        peers: &Arc<parking_lot::RwLock<HashSet<String>>>,
        local_address: &str,
    ) -> Result<()> {
        let url = format!("{}/p2p/peers", peer_address);

        let response = client.get(&url).send().await?;
        if response.status().is_success() {
            let peer_list: Vec<String> = response.json().await?;

            let mut current_peers = peers.write();
            for peer in peer_list {
                if peer != local_address && !current_peers.contains(&peer) {
                    current_peers.insert(peer.clone());
                    info!("Discovered new peer: {}", peer);
                }
            }
        }

        Ok(())
    }

    fn extract_port(address: &str) -> Result<u16> {
        let url = Url::parse(address)?;
        url.port()
            .ok_or_else(|| anyhow!("listen address missing port: {}", address))
    }

    fn canonicalize_address(address: &str) -> Result<String> {
        let trimmed = address.trim();
        let candidate = if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            trimmed.to_string()
        } else {
            format!("http://{}", trimmed)
        };

        let mut url = Url::parse(&candidate)?;
        if url.scheme() != "http" {
            return Err(anyhow!(
                "unsupported scheme {} for peer {}",
                url.scheme(),
                address
            ));
        }
        if url.host_str().is_none() {
            return Err(anyhow!("peer address missing host: {}", address));
        }
        if url.port().is_none() {
            return Err(anyhow!("peer address missing port: {}", address));
        }
        url.set_query(None);
        url.set_fragment(None);
        if url.path() == "/" {
            url.set_path("");
        }
        let mut normalized = url.to_string();
        if normalized.ends_with('/') {
            normalized.pop();
        }
        Ok(normalized)
    }

    async fn refresh_announce_address_inner(
        client: &Client,
        config: &P2PConfig,
        listen_address: &str,
        listen_port: u16,
        announce_address: &Arc<parking_lot::RwLock<String>>,
        upnp_mapping_active: &Arc<parking_lot::RwLock<bool>>,
    ) -> Result<()> {
        if let Some(public_host) = &config.public_host {
            if *upnp_mapping_active.read() {
                if let Err(error) = Self::remove_upnp_mapping(listen_port).await {
                    warn!("Failed to remove existing UPnP mapping: {}", error);
                }
                *upnp_mapping_active.write() = false;
            }

            let address = Self::build_public_address(public_host, listen_port)?;
            *announce_address.write() = address;
            return Ok(());
        }

        if config.enable_upnp {
            match Self::try_setup_upnp_mapping(listen_port).await {
                Ok(Some(address)) => {
                    *announce_address.write() = address;
                    *upnp_mapping_active.write() = true;
                    return Ok(());
                }
                Ok(None) => {
                    *upnp_mapping_active.write() = false;
                }
                Err(error) => {
                    *upnp_mapping_active.write() = false;
                    warn!("UPnP mapping failed: {}", error);
                }
            }
        } else if *upnp_mapping_active.read() {
            if let Err(error) = Self::remove_upnp_mapping(listen_port).await {
                warn!("Failed to remove UPnP mapping: {}", error);
            }
            *upnp_mapping_active.write() = false;
        }

        if let Some(address) =
            Self::try_external_ip_lookup(client, listen_port, &config.external_ip_services).await
        {
            *announce_address.write() = address;
            return Ok(());
        }

        let fallback = Self::fallback_local_address(listen_address, listen_port);
        *announce_address.write() = fallback;
        Ok(())
    }

    fn build_public_address(host_or_url: &str, port: u16) -> Result<String> {
        let trimmed = host_or_url.trim();
        let candidate = if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            trimmed.to_string()
        } else {
            format!("http://{}:{}", trimmed, port)
        };

        let mut url = Url::parse(&candidate)?;
        if url.scheme() != "http" {
            return Err(anyhow!(
                "unsupported scheme {} for public host {}",
                url.scheme(),
                host_or_url
            ));
        }
        if url.port().is_none() {
            url.set_port(Some(port))
                .map_err(|_| anyhow!("failed to set port on public host"))?;
        }
        url.set_query(None);
        url.set_fragment(None);
        if url.path() == "/" {
            url.set_path("");
        }
        let mut normalized = url.to_string();
        if normalized.ends_with('/') {
            normalized.pop();
        }
        Ok(normalized)
    }

    async fn try_setup_upnp_mapping(listen_port: u16) -> Result<Option<String>> {
        let gateway = match search_gateway(Default::default()).await {
            Ok(gateway) => gateway,
            Err(error) => {
                debug!("No UPnP gateway found: {}", error);
                return Ok(None);
            }
        };

        let local_ip = match local_ip() {
            Ok(IpAddr::V4(ip)) => ip,
            Ok(IpAddr::V6(_)) => {
                debug!("Local IP is IPv6; UPnP requires IPv4");
                return Ok(None);
            }
            Err(error) => {
                debug!("Failed to determine local IP for UPnP: {}", error);
                return Ok(None);
            }
        };

        let external_ip = match gateway.get_external_ip().await {
            Ok(ip) => ip,
            Err(error) => {
                debug!("Failed to obtain external IP from gateway: {}", error);
                return Ok(None);
            }
        };

        let socket = SocketAddrV4::new(local_ip, listen_port);
        gateway
            .add_port(
                PortMappingProtocol::TCP,
                listen_port,
                socket,
                0,
                "ippan-node",
            )
            .await
            .map_err(|error| anyhow!("failed to add UPnP port mapping: {}", error))?;

        info!(
            "UPnP mapped local {} to external {}:{}",
            socket, external_ip, listen_port
        );

        Ok(Some(format!("http://{}:{}", external_ip, listen_port)))
    }

    async fn remove_upnp_mapping(listen_port: u16) -> Result<()> {
        let gateway = search_gateway(Default::default())
            .await
            .map_err(|error| anyhow!("failed to locate gateway: {}", error))?;
        gateway
            .remove_port(PortMappingProtocol::TCP, listen_port)
            .await
            .map_err(|error| anyhow!("failed to remove port mapping: {}", error))
    }

    async fn try_external_ip_lookup(
        client: &Client,
        listen_port: u16,
        services: &[String],
    ) -> Option<String> {
        for service in services {
            let url = if service.starts_with("http://") || service.starts_with("https://") {
                service.clone()
            } else {
                format!("https://{}", service)
            };

            match client.get(&url).send().await {
                Ok(response) => match response.text().await {
                    Ok(text) => {
                        let trimmed = text.trim();
                        if let Ok(ip) = trimmed.parse::<IpAddr>() {
                            if let IpAddr::V4(ipv4) = ip {
                                return Some(format!("http://{}:{}", ipv4, listen_port));
                            }
                        }
                    }
                    Err(error) => {
                        debug!("Failed to read response from {}: {}", url, error);
                    }
                },
                Err(error) => {
                    debug!("Failed to query external IP from {}: {}", url, error);
                }
            }
        }

        None
    }

    fn fallback_local_address(listen_address: &str, listen_port: u16) -> String {
        if let Ok(ip) = local_ip() {
            if let IpAddr::V4(ipv4) = ip {
                return format!("http://{}:{}", ipv4, listen_port);
            }
        }

        if let Ok(url) = Url::parse(listen_address) {
            if let Some(host) = url.host_str() {
                if host != "0.0.0.0" && host != "::" {
                    return format!("http://{}:{}", host, listen_port);
                }
            }
        }

        listen_address.to_string()
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
