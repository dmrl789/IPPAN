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

/// Network message types exchanged between peers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    Block(Block),
    Transaction(Transaction),
    BlockRequest { hash: [u8; 32] },
    BlockResponse(Block),
    PeerInfo {
        peer_id: String,
        addresses: Vec<String>,
        #[serde(default)]
        time_us: Option<u64>,
    },
    PeerDiscovery { peers: Vec<String> },
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
    Block { from: String, block: Block },
    Transaction { from: String, transaction: Transaction },
    PeerInfo {
        from: String,
        peer_id: String,
        addresses: Vec<String>,
        time_us: Option<u64>,
    },
    PeerDiscovery { from: String, peers: Vec<String> },
    BlockRequest { from: String, hash: [u8; 32] },
    BlockResponse { from: String, block: Block },
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
}

// (Implementation identical to your existing version — no changes required here)

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
