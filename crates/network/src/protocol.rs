//! Network protocol implementation for IPPAN
//!
//! Defines the communication protocol, message types, and handlers
//! for the IPPAN P2P network.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Network protocol version
pub const PROTOCOL_VERSION: u32 = 1;

/// Message types for the network protocol
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageType {
    // Handshake messages
    Handshake,
    HandshakeAck,
    
    // Block messages
    BlockAnnouncement,
    BlockRequest,
    BlockResponse,
    
    // Transaction messages
    TransactionAnnouncement,
    TransactionRequest,
    TransactionResponse,
    
    // Peer management
    PeerInfo,
    PeerList,
    PeerDiscovery,
    
    // Consensus messages
    ConsensusMessage,
    RoundProposal,
    RoundVote,
    
    // Keep-alive
    Ping,
    Pong,
    
    // Error
    Error,
}

/// Network message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMessage {
    pub version: u32,
    pub message_type: MessageType,
    pub sender_id: String,
    pub recipient_id: Option<String>,
    pub timestamp: u64,
    pub payload: Vec<u8>,
    pub signature: Option<Vec<u8>>,
}

impl NetworkMessage {
    /// Create a new network message
    pub fn new(
        message_type: MessageType,
        sender_id: String,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            message_type,
            sender_id,
            recipient_id: None,
            timestamp: chrono::Utc::now().timestamp() as u64,
            payload,
            signature: None,
        }
    }

    /// Create a directed message
    pub fn directed(
        message_type: MessageType,
        sender_id: String,
        recipient_id: String,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            message_type,
            sender_id,
            recipient_id: Some(recipient_id),
            timestamp: chrono::Utc::now().timestamp() as u64,
            payload,
            signature: None,
        }
    }

    /// Sign the message
    pub fn sign(&mut self, private_key: &[u8]) -> Result<()> {
        // In a real implementation, this would use proper cryptographic signing
        // For now, we'll just create a placeholder signature
        self.signature = Some(vec![0u8; 64]); // Placeholder for Ed25519 signature
        Ok(())
    }

    /// Verify the message signature
    pub fn verify_signature(&self, public_key: &[u8]) -> bool {
        // In a real implementation, this would verify the Ed25519 signature
        // For now, we'll just return true for testing
        self.signature.is_some()
    }

    /// Serialize the message
    pub fn serialize(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow!("Failed to serialize message: {}", e))
    }

    /// Deserialize a message
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        bincode::deserialize(data).map_err(|e| anyhow!("Failed to deserialize message: {}", e))
    }
}

/// Message handler trait
pub trait MessageHandler: Send + Sync {
    /// Handle an incoming message
    async fn handle_message(&self, message: NetworkMessage) -> Result<()>;
    
    /// Get the message types this handler can process
    fn supported_message_types(&self) -> Vec<MessageType>;
}

/// Protocol error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProtocolError {
    InvalidVersion,
    InvalidMessageType,
    InvalidSignature,
    InvalidPayload,
    HandlerNotFound,
    SerializationError,
    NetworkError,
    Timeout,
    PeerNotFound,
    RateLimited,
}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProtocolError::InvalidVersion => write!(f, "Invalid protocol version"),
            ProtocolError::InvalidMessageType => write!(f, "Invalid message type"),
            ProtocolError::InvalidSignature => write!(f, "Invalid message signature"),
            ProtocolError::InvalidPayload => write!(f, "Invalid message payload"),
            ProtocolError::HandlerNotFound => write!(f, "No handler found for message type"),
            ProtocolError::SerializationError => write!(f, "Message serialization error"),
            ProtocolError::NetworkError => write!(f, "Network communication error"),
            ProtocolError::Timeout => write!(f, "Message processing timeout"),
            ProtocolError::PeerNotFound => write!(f, "Peer not found"),
            ProtocolError::RateLimited => write!(f, "Rate limited"),
        }
    }
}

impl std::error::Error for ProtocolError {}

/// Network protocol implementation
pub struct NetworkProtocol {
    handlers: HashMap<MessageType, Arc<dyn MessageHandler>>,
    message_sender: mpsc::UnboundedSender<NetworkMessage>,
    message_receiver: Option<mpsc::UnboundedReceiver<NetworkMessage>>,
    is_running: Arc<std::sync::atomic::AtomicBool>,
}

impl NetworkProtocol {
    /// Create a new network protocol
    pub fn new() -> Self {
        let (message_sender, message_receiver) = mpsc::unbounded_channel();
        
        Self {
            handlers: HashMap::new(),
            message_sender,
            message_receiver: Some(message_receiver),
            is_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Register a message handler
    pub fn register_handler(&mut self, handler: Arc<dyn MessageHandler>) {
        for message_type in handler.supported_message_types() {
            self.handlers.insert(message_type, handler.clone());
        }
    }

    /// Start the protocol
    pub async fn start(&mut self) -> Result<()> {
        self.is_running.store(true, std::sync::atomic::Ordering::SeqCst);
        
        let mut message_receiver = self.message_receiver.take().unwrap();
        let handlers = self.handlers.clone();
        let is_running = self.is_running.clone();

        tokio::spawn(async move {
            while is_running.load(std::sync::atomic::Ordering::SeqCst) {
                if let Some(message) = message_receiver.recv().await {
                    Self::process_message(&handlers, message).await;
                }
            }
        });

        info!("Network protocol started");
        Ok(())
    }

    /// Stop the protocol
    pub async fn stop(&mut self) -> Result<()> {
        self.is_running.store(false, std::sync::atomic::Ordering::SeqCst);
        info!("Network protocol stopped");
        Ok(())
    }

    /// Send a message
    pub async fn send_message(&self, message: NetworkMessage) -> Result<()> {
        self.message_sender.send(message)?;
        Ok(())
    }

    /// Process an incoming message
    async fn process_message(
        handlers: &HashMap<MessageType, Arc<dyn MessageHandler>>,
        message: NetworkMessage,
    ) {
        debug!("Processing message: {:?}", message.message_type);
        
        if let Some(handler) = handlers.get(&message.message_type) {
            if let Err(e) = handler.handle_message(message).await {
                error!("Error handling message: {}", e);
            }
        } else {
            warn!("No handler found for message type: {:?}", message.message_type);
        }
    }
}

/// Default message handlers

/// Handshake handler
pub struct HandshakeHandler {
    node_id: String,
}

impl HandshakeHandler {
    pub fn new(node_id: String) -> Self {
        Self { node_id }
    }
}

#[async_trait::async_trait]
impl MessageHandler for HandshakeHandler {
    async fn handle_message(&self, message: NetworkMessage) -> Result<()> {
        match message.message_type {
            MessageType::Handshake => {
                info!("Received handshake from {}", message.sender_id);
                // Process handshake and send acknowledgment
            }
            MessageType::HandshakeAck => {
                info!("Received handshake acknowledgment from {}", message.sender_id);
                // Process handshake acknowledgment
            }
            _ => {
                return Err(anyhow!("Unsupported message type for handshake handler"));
            }
        }
        Ok(())
    }

    fn supported_message_types(&self) -> Vec<MessageType> {
        vec![MessageType::Handshake, MessageType::HandshakeAck]
    }
}

/// Block handler
pub struct BlockHandler {
    block_storage: Arc<dyn BlockStorage>,
}

pub trait BlockStorage: Send + Sync {
    async fn store_block(&self, block: &[u8]) -> Result<()>;
    async fn get_block(&self, block_id: &str) -> Result<Option<Vec<u8>>>;
}

impl BlockHandler {
    pub fn new(block_storage: Arc<dyn BlockStorage>) -> Self {
        Self { block_storage }
    }
}

#[async_trait::async_trait]
impl MessageHandler for BlockHandler {
    async fn handle_message(&self, message: NetworkMessage) -> Result<()> {
        match message.message_type {
            MessageType::BlockAnnouncement => {
                info!("Received block announcement from {}", message.sender_id);
                // Process block announcement
            }
            MessageType::BlockRequest => {
                info!("Received block request from {}", message.sender_id);
                // Process block request and send response
            }
            MessageType::BlockResponse => {
                info!("Received block response from {}", message.sender_id);
                // Process block response
            }
            _ => {
                return Err(anyhow!("Unsupported message type for block handler"));
            }
        }
        Ok(())
    }

    fn supported_message_types(&self) -> Vec<MessageType> {
        vec![
            MessageType::BlockAnnouncement,
            MessageType::BlockRequest,
            MessageType::BlockResponse,
        ]
    }
}

/// Transaction handler
pub struct TransactionHandler {
    mempool: Arc<dyn MempoolStorage>,
}

pub trait MempoolStorage: Send + Sync {
    async fn add_transaction(&self, transaction: &[u8]) -> Result<()>;
    async fn get_transaction(&self, tx_id: &str) -> Result<Option<Vec<u8>>>;
}

impl TransactionHandler {
    pub fn new(mempool: Arc<dyn MempoolStorage>) -> Self {
        Self { mempool }
    }
}

#[async_trait::async_trait]
impl MessageHandler for TransactionHandler {
    async fn handle_message(&self, message: NetworkMessage) -> Result<()> {
        match message.message_type {
            MessageType::TransactionAnnouncement => {
                info!("Received transaction announcement from {}", message.sender_id);
                // Process transaction announcement
            }
            MessageType::TransactionRequest => {
                info!("Received transaction request from {}", message.sender_id);
                // Process transaction request and send response
            }
            MessageType::TransactionResponse => {
                info!("Received transaction response from {}", message.sender_id);
                // Process transaction response
            }
            _ => {
                return Err(anyhow!("Unsupported message type for transaction handler"));
            }
        }
        Ok(())
    }

    fn supported_message_types(&self) -> Vec<MessageType> {
        vec![
            MessageType::TransactionAnnouncement,
            MessageType::TransactionRequest,
            MessageType::TransactionResponse,
        ]
    }
}

/// Ping handler
pub struct PingHandler;

#[async_trait::async_trait]
impl MessageHandler for PingHandler {
    async fn handle_message(&self, message: NetworkMessage) -> Result<()> {
        match message.message_type {
            MessageType::Ping => {
                debug!("Received ping from {}", message.sender_id);
                // Send pong response
            }
            MessageType::Pong => {
                debug!("Received pong from {}", message.sender_id);
                // Process pong response
            }
            _ => {
                return Err(anyhow!("Unsupported message type for ping handler"));
            }
        }
        Ok(())
    }

    fn supported_message_types(&self) -> Vec<MessageType> {
        vec![MessageType::Ping, MessageType::Pong]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_message_creation() {
        let message = NetworkMessage::new(
            MessageType::Ping,
            "test-sender".to_string(),
            vec![1, 2, 3, 4],
        );
        
        assert_eq!(message.version, PROTOCOL_VERSION);
        assert_eq!(message.message_type, MessageType::Ping);
        assert_eq!(message.sender_id, "test-sender");
        assert!(message.recipient_id.is_none());
    }

    #[test]
    fn test_network_message_serialization() {
        let message = NetworkMessage::new(
            MessageType::Ping,
            "test-sender".to_string(),
            vec![1, 2, 3, 4],
        );
        
        let serialized = message.serialize().unwrap();
        let deserialized = NetworkMessage::deserialize(&serialized).unwrap();
        
        assert_eq!(message.version, deserialized.version);
        assert_eq!(message.message_type, deserialized.message_type);
        assert_eq!(message.sender_id, deserialized.sender_id);
        assert_eq!(message.payload, deserialized.payload);
    }

    #[tokio::test]
    async fn test_network_protocol() {
        let mut protocol = NetworkProtocol::new();
        let handler = Arc::new(HandshakeHandler::new("test-node".to_string()));
        protocol.register_handler(handler);
        
        assert!(protocol.start().await.is_ok());
        assert!(protocol.stop().await.is_ok());
    }
}
