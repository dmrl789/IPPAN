use anyhow::{anyhow, Result};
use ippan_types::{Block, Transaction};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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
    announce_address: Arc<parking_lot::RwLock<String>>,
    upnp_mapping_active: Arc<parking_lot::RwLock<bool>>,
}

impl HttpP2PNetwork {
    /// Create a new HTTP P2P network
    pub fn new(config: P2PConfig, local_address: String) -> Result<Self> {
        let client = Client::builder().timeout(config.message_timeout).build()?;

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
            listen_address: local_address.clone(),
            announce_address: Arc::new(parking_lot::RwLock::new(local_address)),
            upnp_mapping_active: Arc::new(parking_lot::RwLock::new(false)),
        })
    }

    /// Start the P2P network
    pub async fn start(&mut self) -> Result<()> {
        *self.is_running.write() = true;
        info!("Starting HTTP P2P network on {}", self.listen_address);

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
            let announce_address = self.announce_address.clone();
            let peer_count = self.peer_count.clone();

            tokio::spawn(async move {
                let mut interval = interval(config.peer_discovery_interval);

                loop {
                    if !*is_running.read() {
                        break;
                    }

                    interval.tick().await;

                    let local_address = {
                        let raw = announce_address.read().clone();
                        match Self::canonicalize_address(&raw) {
                            Ok(address) => address,
                            Err(err) => {
                                debug!("Failed to canonicalize announce address {}: {}", raw, err);
                                raw
                            }
                        }
                    };

                    Self::discover_peers(&client, &peers, &local_address, &peer_count).await;
                }
            })
        };

        info!("HTTP P2P network started");
        Ok(())
    }

    /// Stop the P2P network
    pub async fn stop(&self) -> Result<()> {
        *self.is_running.write() = false;
        info!("Stopping HTTP P2P network");

        // Wait a bit for the network loops to finish
        sleep(Duration::from_millis(100)).await;

        info!("HTTP P2P network stopped");
        Ok(())
    }

    /// Add a peer to the network
    pub async fn add_peer(&self, peer_address: String) -> Result<()> {
        let canonical = Self::canonicalize_address(&peer_address)?;

        let inserted = {
            let mut peers = self.peers.write();

            if peers.len() >= self.config.max_peers {
                warn!(
                    "Peer limit reached ({}). Skipping peer {}",
                    self.config.max_peers, canonical
                );
                return Ok(());
            }

            peers.insert(canonical.clone())
        };

        if inserted {
            self.update_peer_count();
            info!("Added peer: {}", canonical);
        }

        Ok(())
    }

    /// Remove a peer from the network
    pub fn remove_peer(&self, peer_address: &str) {
        match Self::canonicalize_address(peer_address) {
            Ok(canonical) => {
                {
                    let mut peers = self.peers.write();
                    if !peers.remove(&canonical) {
                        return;
                    }
                }

                self.update_peer_count();
                info!("Removed peer: {}", canonical);
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

    /// Get local peer ID
    pub fn get_local_peer_id(&self) -> String {
        self.local_peer_id.clone()
    }

    /// Get listening address
    pub fn get_listening_address(&self) -> String {
        self.listen_address.clone()
    }

    /// Get the public announce address for this node
    pub fn get_announce_address(&self) -> String {
        let raw = self.announce_address.read().clone();
        Self::canonicalize_address(&raw).unwrap_or(raw)
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

    fn canonicalize_address(address: &str) -> Result<String> {
        let trimmed = address.trim();

        if trimmed.is_empty() {
            return Err(anyhow!("Peer address cannot be empty"));
        }

        let normalized = if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            trimmed.to_string()
        } else {
            format!("http://{}", trimmed)
        };

        let mut url = Url::parse(&normalized)
            .map_err(|err| anyhow!("Invalid peer address {}: {}", trimmed, err))?;

        if url.scheme() != "http" && url.scheme() != "https" {
            return Err(anyhow!(
                "Unsupported peer scheme '{}'. Use http or https.",
                url.scheme()
            ));
        }

        if url.host_str().is_none() {
            return Err(anyhow!("Peer address {} is missing a host", trimmed));
        }

        url.set_path("");
        url.set_query(None);
        url.set_fragment(None);

        let mut canonical = url.to_string();
        if canonical.ends_with('/') {
            canonical = canonical.trim_end_matches('/').to_string();
        }

        Ok(canonical)
    }

    /// Handle outgoing messages by sending them to all peers
    async fn handle_outgoing_message(
        client: &Client,
        peers: &Arc<parking_lot::RwLock<HashSet<String>>>,
        config: &P2PConfig,
        announce_address: &Arc<parking_lot::RwLock<String>>,
        message: NetworkMessage,
    ) {
        let peer_list = peers.read().clone();
        let local_address_raw = announce_address.read().clone();
        let local_address =
            Self::canonicalize_address(&local_address_raw).unwrap_or(local_address_raw);

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

    /// Discover new peers by asking existing peers
    async fn discover_peers(
        client: &Client,
        peers: &Arc<parking_lot::RwLock<HashSet<String>>>,
        local_address: &str,
        peer_count: &Arc<parking_lot::RwLock<usize>>,
    ) {
        let peer_list = peers.read().clone();

        for peer_address in peer_list {
            if peer_address == local_address {
                continue;
            }

            let client = client.clone();
            let peers = peers.clone();
            let local_address = local_address.to_string();
            let peer_count = peer_count.clone();

            tokio::spawn(async move {
                if let Err(e) = Self::request_peers_from_peer(
                    &client,
                    &peer_address,
                    &peers,
                    &local_address,
                    &peer_count,
                )
                .await
                {
                    debug!("Failed to discover peers from {}: {}", peer_address, e);
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
        peer_count: &Arc<parking_lot::RwLock<usize>>,
    ) -> Result<()> {
        let url = format!("{}/p2p/peers", peer_address.trim_end_matches('/'));

        let response = client.get(&url).send().await?;
        if response.status().is_success() {
            let peer_list: Vec<String> = response.json().await?;

            // Add new peers (excluding ourselves)
            let mut current_peers = peers.write();
            let mut updated = false;
            for peer in peer_list {
                match Self::canonicalize_address(&peer) {
                    Ok(canonical) => {
                        if canonical != local_address && !current_peers.contains(&canonical) {
                            current_peers.insert(canonical.clone());
                            info!("Discovered new peer: {}", canonical);
                            updated = true;
                        }
                    }
                    Err(err) => {
                        debug!("Discarding invalid peer address {}: {}", peer, err);
                    }
                }
            }

            let count = current_peers.len();
            drop(current_peers);

            if updated {
                *peer_count.write() = count;
            }
        }

        Ok(())
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
}
