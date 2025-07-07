//! P2P communication module
//! 
//! Handles peer-to-peer communication protocols and message handling.

use crate::{error::IppanError, NodeId, Result};
use libp2p::{
    core::{upgrade, InboundUpgrade, OutboundUpgrade},
    swarm::{NegotiatedSubstream, StreamProtocol},
    PeerId,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// P2P protocol identifier
pub const IPPAN_PROTOCOL: &str = "/ippan/1.0.0";

/// P2P message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum P2pMessage {
    /// Ping message
    Ping(PingMessage),
    /// Pong response
    Pong(PongMessage),
    /// Block announcement
    BlockAnnouncement(BlockAnnouncement),
    /// Transaction announcement
    TransactionAnnouncement(TransactionAnnouncement),
    /// Storage request
    StorageRequest(StorageRequest),
    /// Storage response
    StorageResponse(StorageResponse),
    /// Peer list exchange
    PeerList(PeerList),
    /// Consensus message
    Consensus(ConsensusMessage),
}

/// Ping message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingMessage {
    /// Sender node ID
    pub sender: NodeId,
    /// Timestamp
    pub timestamp: u64,
    /// Nonce for response matching
    pub nonce: u64,
}

/// Pong response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PongMessage {
    /// Responder node ID
    pub responder: NodeId,
    /// Original ping timestamp
    pub ping_timestamp: u64,
    /// Response timestamp
    pub pong_timestamp: u64,
    /// Original nonce
    pub nonce: u64,
}

/// Block announcement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockAnnouncement {
    /// Block hash
    pub block_hash: [u8; 32],
    /// Block height
    pub height: u64,
    /// Sender node ID
    pub sender: NodeId,
    /// Timestamp
    pub timestamp: u64,
}

/// Transaction announcement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionAnnouncement {
    /// Transaction hash
    pub tx_hash: [u8; 32],
    /// Transaction type
    pub tx_type: TransactionType,
    /// Sender node ID
    pub sender: NodeId,
    /// Timestamp
    pub timestamp: u64,
}

/// Transaction type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionType {
    /// Payment transaction
    Payment,
    /// Staking transaction
    Staking,
    /// Domain registration
    DomainRegistration,
    /// Storage transaction
    Storage,
}

/// Storage request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageRequest {
    /// File hash
    pub file_hash: [u8; 32],
    /// Shard index
    pub shard_index: u32,
    /// Requesting node ID
    pub requester: NodeId,
    /// Request ID
    pub request_id: u64,
}

/// Storage response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageResponse {
    /// File hash
    pub file_hash: [u8; 32],
    /// Shard index
    pub shard_index: u32,
    /// Shard data
    pub shard_data: Vec<u8>,
    /// Proof of storage
    pub proof: Vec<u8>,
    /// Response ID
    pub response_id: u64,
}

/// Peer list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerList {
    /// Sender node ID
    pub sender: NodeId,
    /// Peer entries
    pub peers: Vec<PeerEntry>,
    /// Timestamp
    pub timestamp: u64,
}

/// Peer entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerEntry {
    /// Peer ID
    pub peer_id: PeerId,
    /// Node ID
    pub node_id: NodeId,
    /// Multiaddrs
    pub addrs: Vec<String>,
    /// Last seen timestamp
    pub last_seen: u64,
}

/// Consensus message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusMessage {
    /// Message type
    pub msg_type: ConsensusMessageType,
    /// Sender node ID
    pub sender: NodeId,
    /// Round number
    pub round: u64,
    /// Payload
    pub payload: Vec<u8>,
    /// Timestamp
    pub timestamp: u64,
}

/// Consensus message type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsensusMessageType {
    /// Block proposal
    BlockProposal,
    /// Block vote
    BlockVote,
    /// Round change
    RoundChange,
    /// View change
    ViewChange,
}

/// P2P manager
pub struct P2pManager {
    /// Local node ID
    local_node_id: NodeId,
    /// Connected peers
    connected_peers: HashMap<PeerId, PeerConnection>,
    /// Message handlers
    message_handlers: HashMap<P2pMessageType, Box<dyn MessageHandler>>,
    /// Event sender
    event_sender: mpsc::UnboundedSender<P2pEvent>,
    /// Event receiver
    event_receiver: mpsc::UnboundedReceiver<P2pEvent>,
    /// Configuration
    config: P2pConfig,
}

/// Peer connection
#[derive(Debug, Clone)]
pub struct PeerConnection {
    /// Peer ID
    pub peer_id: PeerId,
    /// Node ID
    pub node_id: NodeId,
    /// Connection state
    pub state: ConnectionState,
    /// Last ping time
    pub last_ping: Option<u64>,
    /// Last pong time
    pub last_pong: Option<u64>,
    /// Latency in milliseconds
    pub latency: Option<u64>,
    /// Message count
    pub message_count: u64,
}

/// Connection state
#[derive(Debug, Clone)]
pub enum ConnectionState {
    /// Connecting
    Connecting,
    /// Connected
    Connected,
    /// Disconnected
    Disconnected,
}

/// P2P message type
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum P2pMessageType {
    /// Ping message
    Ping,
    /// Pong message
    Pong,
    /// Block announcement
    BlockAnnouncement,
    /// Transaction announcement
    TransactionAnnouncement,
    /// Storage request
    StorageRequest,
    /// Storage response
    StorageResponse,
    /// Peer list
    PeerList,
    /// Consensus message
    Consensus,
}

/// P2P events
#[derive(Debug)]
pub enum P2pEvent {
    /// Peer connected
    PeerConnected(PeerId, NodeId),
    /// Peer disconnected
    PeerDisconnected(PeerId),
    /// Message received
    MessageReceived(PeerId, P2pMessage),
    /// Message sent
    MessageSent(PeerId, P2pMessage),
    /// Error occurred
    Error(P2pError),
}

/// P2P errors
#[derive(Debug, thiserror::Error)]
pub enum P2pError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Message serialization failed: {0}")]
    SerializationFailed(String),
    #[error("Message deserialization failed: {0}")]
    DeserializationFailed(String),
    #[error("Protocol error: {0}")]
    ProtocolError(String),
    #[error("Peer not found: {0}")]
    PeerNotFound(PeerId),
}

/// Message handler trait
pub trait MessageHandler: Send + Sync {
    fn handle_message(&self, peer_id: PeerId, message: P2pMessage) -> Result<()>;
}

/// P2P configuration
#[derive(Debug, Clone)]
pub struct P2pConfig {
    /// Protocol identifier
    pub protocol: String,
    /// Ping interval
    pub ping_interval: std::time::Duration,
    /// Connection timeout
    pub connection_timeout: std::time::Duration,
    /// Max message size
    pub max_message_size: usize,
    /// Enable peer exchange
    pub enable_peer_exchange: bool,
}

impl Default for P2pConfig {
    fn default() -> Self {
        Self {
            protocol: IPPAN_PROTOCOL.to_string(),
            ping_interval: std::time::Duration::from_secs(30),
            connection_timeout: std::time::Duration::from_secs(60),
            max_message_size: 1024 * 1024, // 1MB
            enable_peer_exchange: true,
        }
    }
}

impl P2pManager {
    /// Create a new P2P manager
    pub fn new(local_node_id: NodeId, config: P2pConfig) -> Self {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        
        Self {
            local_node_id,
            connected_peers: HashMap::new(),
            message_handlers: HashMap::new(),
            event_sender,
            event_receiver,
            config,
        }
    }
    
    /// Register a message handler
    pub fn register_handler(&mut self, msg_type: P2pMessageType, handler: Box<dyn MessageHandler>) {
        self.message_handlers.insert(msg_type, handler);
    }
    
    /// Start the P2P manager
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting P2P manager");
        
        // Start ping loop
        self.start_ping_loop().await?;
        
        // Start event processing loop
        self.run_event_loop().await?;
        
        Ok(())
    }
    
    /// Start ping loop
    async fn start_ping_loop(&mut self) -> Result<()> {
        let mut interval = tokio::time::interval(self.config.ping_interval);
        
        loop {
            interval.tick().await;
            
            for (peer_id, connection) in &mut self.connected_peers {
                if connection.state == ConnectionState::Connected {
                    self.send_ping(*peer_id).await?;
                }
            }
        }
    }
    
    /// Run event processing loop
    async fn run_event_loop(&mut self) -> Result<()> {
        while let Some(event) = self.event_receiver.recv().await {
            self.handle_event(event).await?;
        }
        Ok(())
    }
    
    /// Handle P2P event
    async fn handle_event(&mut self, event: P2pEvent) -> Result<()> {
        match event {
            P2pEvent::PeerConnected(peer_id, node_id) => {
                self.handle_peer_connected(peer_id, node_id).await?;
            }
            P2pEvent::PeerDisconnected(peer_id) => {
                self.handle_peer_disconnected(peer_id).await?;
            }
            P2pEvent::MessageReceived(peer_id, message) => {
                self.handle_message_received(peer_id, message).await?;
            }
            P2pEvent::MessageSent(peer_id, message) => {
                self.handle_message_sent(peer_id, message).await?;
            }
            P2pEvent::Error(error) => {
                self.handle_error(error).await?;
            }
        }
        Ok(())
    }
    
    /// Handle peer connected
    async fn handle_peer_connected(&mut self, peer_id: PeerId, node_id: NodeId) -> Result<()> {
        let connection = PeerConnection {
            peer_id,
            node_id,
            state: ConnectionState::Connected,
            last_ping: None,
            last_pong: None,
            latency: None,
            message_count: 0,
        };
        
        self.connected_peers.insert(peer_id, connection);
        info!("Peer connected: {} (node: {:?})", peer_id, node_id);
        
        Ok(())
    }
    
    /// Handle peer disconnected
    async fn handle_peer_disconnected(&mut self, peer_id: PeerId) -> Result<()> {
        self.connected_peers.remove(&peer_id);
        info!("Peer disconnected: {}", peer_id);
        Ok(())
    }
    
    /// Handle message received
    async fn handle_message_received(&mut self, peer_id: PeerId, message: P2pMessage) -> Result<()> {
        // Update message count
        if let Some(connection) = self.connected_peers.get_mut(&peer_id) {
            connection.message_count += 1;
        }
        
        // Route message to appropriate handler
        let msg_type = self.get_message_type(&message);
        if let Some(handler) = self.message_handlers.get(&msg_type) {
            handler.handle_message(peer_id, message)?;
        } else {
            warn!("No handler registered for message type: {:?}", msg_type);
        }
        
        Ok(())
    }
    
    /// Handle message sent
    async fn handle_message_sent(&mut self, _peer_id: PeerId, _message: P2pMessage) -> Result<()> {
        // Update statistics if needed
        Ok(())
    }
    
    /// Handle error
    async fn handle_error(&mut self, error: P2pError) -> Result<()> {
        error!("P2P error: {}", error);
        Ok(())
    }
    
    /// Send ping to peer
    async fn send_ping(&mut self, peer_id: PeerId) -> Result<()> {
        let timestamp = chrono::Utc::now().timestamp_millis() as u64;
        let nonce = rand::random::<u64>();
        
        let ping = P2pMessage::Ping(PingMessage {
            sender: self.local_node_id,
            timestamp,
            nonce,
        });
        
        self.send_message(peer_id, ping).await?;
        
        // Update last ping time
        if let Some(connection) = self.connected_peers.get_mut(&peer_id) {
            connection.last_ping = Some(timestamp);
        }
        
        Ok(())
    }
    
    /// Send message to peer
    pub async fn send_message(&mut self, peer_id: PeerId, message: P2pMessage) -> Result<()> {
        // In a real implementation, you'd serialize and send the message
        // For now, we'll just log it
        
        debug!("Sending message to {}: {:?}", peer_id, message);
        
        // Emit message sent event
        let _ = self.event_sender.send(P2pEvent::MessageSent(peer_id, message));
        
        Ok(())
    }
    
    /// Broadcast message to all connected peers
    pub async fn broadcast_message(&mut self, message: P2pMessage) -> Result<()> {
        for peer_id in self.connected_peers.keys().cloned().collect::<Vec<_>>() {
            self.send_message(peer_id, message.clone()).await?;
        }
        Ok(())
    }
    
    /// Get message type
    fn get_message_type(&self, message: &P2pMessage) -> P2pMessageType {
        match message {
            P2pMessage::Ping(_) => P2pMessageType::Ping,
            P2pMessage::Pong(_) => P2pMessageType::Pong,
            P2pMessage::BlockAnnouncement(_) => P2pMessageType::BlockAnnouncement,
            P2pMessage::TransactionAnnouncement(_) => P2pMessageType::TransactionAnnouncement,
            P2pMessage::StorageRequest(_) => P2pMessageType::StorageRequest,
            P2pMessage::StorageResponse(_) => P2pMessageType::StorageResponse,
            P2pMessage::PeerList(_) => P2pMessageType::PeerList,
            P2pMessage::Consensus(_) => P2pMessageType::Consensus,
        }
    }
    
    /// Get connected peers
    pub fn connected_peers(&self) -> &HashMap<PeerId, PeerConnection> {
        &self.connected_peers
    }
    
    /// Get peer connection
    pub fn get_peer_connection(&self, peer_id: &PeerId) -> Option<&PeerConnection> {
        self.connected_peers.get(peer_id)
    }
    
    /// Check if peer is connected
    pub fn is_peer_connected(&self, peer_id: &PeerId) -> bool {
        self.connected_peers.contains_key(peer_id)
    }
    
    /// Get P2P statistics
    pub fn get_stats(&self) -> P2pStats {
        let connected_count = self.connected_peers.len();
        let total_messages = self.connected_peers.values()
            .map(|conn| conn.message_count)
            .sum();
        
        P2pStats {
            connected_peers: connected_count,
            total_messages,
            protocol: self.config.protocol.clone(),
        }
    }
}

/// P2P statistics
#[derive(Debug, Clone)]
pub struct P2pStats {
    /// Number of connected peers
    pub connected_peers: usize,
    /// Total messages sent/received
    pub total_messages: u64,
    /// Protocol identifier
    pub protocol: String,
}

/// Default ping handler
pub struct PingHandler;

impl MessageHandler for PingHandler {
    fn handle_message(&self, peer_id: PeerId, message: P2pMessage) -> Result<()> {
        if let P2pMessage::Ping(ping) = message {
            debug!("Received ping from {} with nonce {}", peer_id, ping.nonce);
            
            // In a real implementation, you'd send a pong response
            // For now, we'll just log it
        }
        Ok(())
    }
}

/// Default pong handler
pub struct PongHandler;

impl MessageHandler for PongHandler {
    fn handle_message(&self, peer_id: PeerId, message: P2pMessage) -> Result<()> {
        if let P2pMessage::Pong(pong) = message {
            debug!("Received pong from {} with latency {}ms", 
                peer_id, 
                pong.pong_timestamp - pong.ping_timestamp
            );
        }
        Ok(())
    }
}
