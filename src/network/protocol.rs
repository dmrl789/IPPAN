//! Protocol manager for IPPAN network

use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use chrono::{DateTime, Utc};

/// Protocol message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProtocolMessage {
    /// Block protocol
    Block(BlockMessage),
    /// Transaction protocol
    Transaction(TransactionMessage),
    /// Consensus protocol
    Consensus(ConsensusMessage),
    /// Storage protocol
    Storage(StorageMessage),
    /// Domain protocol
    Domain(DomainMessage),
}

/// Block protocol message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockMessage {
    /// Message type
    pub msg_type: BlockMessageType,
    /// Block data
    pub block_data: Option<Vec<u8>>,
    /// Block hash
    pub block_hash: Option<[u8; 32]>,
    /// Block height
    pub height: Option<u64>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Block message type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlockMessageType {
    /// Block announcement
    Announcement,
    /// Block request
    Request,
    /// Block response
    Response,
    /// Block validation
    Validation,
}

/// Transaction protocol message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionMessage {
    /// Message type
    pub msg_type: TransactionMessageType,
    /// Transaction data
    pub tx_data: Option<Vec<u8>>,
    /// Transaction hash
    pub tx_hash: Option<[u8; 32]>,
    /// Transaction type
    pub tx_type: Option<String>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Transaction message type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionMessageType {
    /// Transaction announcement
    Announcement,
    /// Transaction request
    Request,
    /// Transaction response
    Response,
    /// Transaction validation
    Validation,
}

/// Consensus protocol message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusMessage {
    /// Message type
    pub msg_type: ConsensusMessageType,
    /// Round number
    pub round: u64,
    /// Phase
    pub phase: String,
    /// Payload
    pub payload: Option<Vec<u8>>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
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

/// Storage protocol message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageMessage {
    /// Message type
    pub msg_type: StorageMessageType,
    /// File hash
    pub file_hash: Option<[u8; 32]>,
    /// Shard index
    pub shard_index: Option<u32>,
    /// Shard data
    pub shard_data: Option<Vec<u8>>,
    /// Proof
    pub proof: Option<Vec<u8>>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Storage message type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageMessageType {
    /// Storage request
    Request,
    /// Storage response
    Response,
    /// Storage proof
    Proof,
    /// Storage challenge
    Challenge,
}

/// Domain protocol message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainMessage {
    /// Message type
    pub msg_type: DomainMessageType,
    /// Domain name
    pub domain_name: Option<String>,
    /// Domain data
    pub domain_data: Option<Vec<u8>>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Domain message type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DomainMessageType {
    /// Domain registration
    Registration,
    /// Domain renewal
    Renewal,
    /// Domain transfer
    Transfer,
    /// Domain query
    Query,
}

/// Protocol handler trait
pub trait ProtocolHandler: Send + Sync {
    /// Handle protocol message
    fn handle_message(&self, message: ProtocolMessage) -> Result<()>;
    
    /// Get protocol name
    fn protocol_name(&self) -> &str;
}

/// Protocol manager
pub struct ProtocolManager {
    /// Registered protocol handlers
    handlers: Arc<RwLock<HashMap<String, Box<dyn ProtocolHandler>>>>,
    /// Message sender
    message_sender: mpsc::Sender<ProtocolMessage>,
    /// Message receiver
    _message_receiver: Option<mpsc::Receiver<ProtocolMessage>>,
    /// Running flag
    running: bool,
}

impl ProtocolManager {
    /// Create a new protocol manager
    pub fn new() -> Self {
        let (message_sender, message_receiver) = mpsc::channel(1000);
        
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
            message_sender,
            _message_receiver: Some(message_receiver),
            running: false,
        }
    }

    /// Start the protocol manager
    pub async fn start(&mut self) -> Result<()> {
        log::info!("Starting protocol manager");
        self.running = true;
        
        // Start message processing loop
        let message_receiver = self._message_receiver.take().ok_or_else(|| {
            crate::error::IppanError::Network("Message receiver already taken".to_string())
        })?;
        let handlers = self.handlers.clone();
        
        tokio::spawn(async move {
            let mut message_receiver = message_receiver;
            
            while let Some(message) = message_receiver.recv().await {
                Self::process_message(message, &handlers).await;
            }
        });
        
        Ok(())
    }

    /// Stop the protocol manager
    pub async fn stop(&mut self) -> Result<()> {
        log::info!("Stopping protocol manager");
        self.running = false;
        Ok(())
    }

    /// Register a protocol handler
    pub async fn register_handler(&self, handler: Box<dyn ProtocolHandler>) -> Result<()> {
        let protocol_name = handler.protocol_name().to_string();
        let mut handlers = self.handlers.write().await;
        
        handlers.insert(protocol_name.clone(), handler);
        log::info!("Registered protocol handler: {}", protocol_name);
        
        Ok(())
    }

    /// Unregister a protocol handler
    pub async fn unregister_handler(&self, protocol_name: &str) -> Result<()> {
        let mut handlers = self.handlers.write().await;
        
        if handlers.remove(protocol_name).is_some() {
            log::info!("Unregistered protocol handler: {}", protocol_name);
        }
        
        Ok(())
    }

    /// Send a protocol message
    pub async fn send_message(&self, message: ProtocolMessage) -> Result<()> {
        if let Err(e) = self.message_sender.send(message).await {
            return Err(crate::error::IppanError::Network(
                format!("Failed to send protocol message: {}", e)
            ));
        }
        
        Ok(())
    }

    /// Get registered protocols
    pub async fn get_registered_protocols(&self) -> Vec<String> {
        let handlers = self.handlers.read().await;
        handlers.keys().cloned().collect()
    }

    /// Process protocol message
    pub async fn process_message(
        message: ProtocolMessage,
        handlers: &Arc<RwLock<HashMap<String, Box<dyn ProtocolHandler>>>>,
    ) {
        let protocol_name = match &message {
            ProtocolMessage::Block(_) => "block",
            ProtocolMessage::Transaction(_) => "transaction",
            ProtocolMessage::Consensus(_) => "consensus",
            ProtocolMessage::Storage(_) => "storage",
            ProtocolMessage::Domain(_) => "domain",
        };
        
        let handlers = handlers.read().await;
        
        if let Some(handler) = handlers.get(protocol_name) {
            if let Err(e) = handler.handle_message(message) {
                log::error!("Failed to handle {} protocol message: {}", protocol_name, e);
            }
        } else {
            log::warn!("No handler registered for protocol: {}", protocol_name);
        }
    }
}

/// Block protocol handler
pub struct BlockProtocolHandler;

impl ProtocolHandler for BlockProtocolHandler {
    fn handle_message(&self, message: ProtocolMessage) -> Result<()> {
        if let ProtocolMessage::Block(block_msg) = message {
            match block_msg.msg_type {
                BlockMessageType::Announcement => {
                    log::info!("Received block announcement: {:?}", block_msg.block_hash);
                    // TODO: Process block announcement
                }
                BlockMessageType::Request => {
                    log::info!("Received block request for height: {:?}", block_msg.height);
                    // TODO: Process block request
                }
                BlockMessageType::Response => {
                    log::info!("Received block response: {:?}", block_msg.block_hash);
                    // TODO: Process block response
                }
                BlockMessageType::Validation => {
                    log::info!("Received block validation: {:?}", block_msg.block_hash);
                    // TODO: Process block validation
                }
            }
        }
        
        Ok(())
    }
    
    fn protocol_name(&self) -> &str {
        "block"
    }
}

/// Transaction protocol handler
pub struct TransactionProtocolHandler;

impl ProtocolHandler for TransactionProtocolHandler {
    fn handle_message(&self, message: ProtocolMessage) -> Result<()> {
        if let ProtocolMessage::Transaction(tx_msg) = message {
            match tx_msg.msg_type {
                TransactionMessageType::Announcement => {
                    log::info!("Received transaction announcement: {:?}", tx_msg.tx_hash);
                    // TODO: Process transaction announcement
                }
                TransactionMessageType::Request => {
                    log::info!("Received transaction request: {:?}", tx_msg.tx_hash);
                    // TODO: Process transaction request
                }
                TransactionMessageType::Response => {
                    log::info!("Received transaction response: {:?}", tx_msg.tx_hash);
                    // TODO: Process transaction response
                }
                TransactionMessageType::Validation => {
                    log::info!("Received transaction validation: {:?}", tx_msg.tx_hash);
                    // TODO: Process transaction validation
                }
            }
        }
        
        Ok(())
    }
    
    fn protocol_name(&self) -> &str {
        "transaction"
    }
}

/// Consensus protocol handler
pub struct ConsensusProtocolHandler;

impl ProtocolHandler for ConsensusProtocolHandler {
    fn handle_message(&self, message: ProtocolMessage) -> Result<()> {
        if let ProtocolMessage::Consensus(consensus_msg) = message {
            match consensus_msg.msg_type {
                ConsensusMessageType::BlockProposal => {
                    log::info!("Received block proposal for round: {}", consensus_msg.round);
                    // TODO: Process block proposal
                }
                ConsensusMessageType::BlockVote => {
                    log::info!("Received block vote for round: {}", consensus_msg.round);
                    // TODO: Process block vote
                }
                ConsensusMessageType::RoundChange => {
                    log::info!("Received round change: {}", consensus_msg.round);
                    // TODO: Process round change
                }
                ConsensusMessageType::ViewChange => {
                    log::info!("Received view change for round: {}", consensus_msg.round);
                    // TODO: Process view change
                }
            }
        }
        
        Ok(())
    }
    
    fn protocol_name(&self) -> &str {
        "consensus"
    }
}

/// Storage protocol handler
pub struct StorageProtocolHandler;

impl ProtocolHandler for StorageProtocolHandler {
    fn handle_message(&self, message: ProtocolMessage) -> Result<()> {
        if let ProtocolMessage::Storage(storage_msg) = message {
            match storage_msg.msg_type {
                StorageMessageType::Request => {
                    log::info!("Received storage request: {:?}", storage_msg.file_hash);
                    // TODO: Process storage request
                }
                StorageMessageType::Response => {
                    log::info!("Received storage response: {:?}", storage_msg.file_hash);
                    // TODO: Process storage response
                }
                StorageMessageType::Proof => {
                    log::info!("Received storage proof: {:?}", storage_msg.file_hash);
                    // TODO: Process storage proof
                }
                StorageMessageType::Challenge => {
                    log::info!("Received storage challenge: {:?}", storage_msg.file_hash);
                    // TODO: Process storage challenge
                }
            }
        }
        
        Ok(())
    }
    
    fn protocol_name(&self) -> &str {
        "storage"
    }
}

/// Domain protocol handler
pub struct DomainProtocolHandler;

impl ProtocolHandler for DomainProtocolHandler {
    fn handle_message(&self, message: ProtocolMessage) -> Result<()> {
        if let ProtocolMessage::Domain(domain_msg) = message {
            match domain_msg.msg_type {
                DomainMessageType::Registration => {
                    log::info!("Received domain registration: {:?}", domain_msg.domain_name);
                    // TODO: Process domain registration
                }
                DomainMessageType::Renewal => {
                    log::info!("Received domain renewal: {:?}", domain_msg.domain_name);
                    // TODO: Process domain renewal
                }
                DomainMessageType::Transfer => {
                    log::info!("Received domain transfer: {:?}", domain_msg.domain_name);
                    // TODO: Process domain transfer
                }
                DomainMessageType::Query => {
                    log::info!("Received domain query: {:?}", domain_msg.domain_name);
                    // TODO: Process domain query
                }
            }
        }
        
        Ok(())
    }
    
    fn protocol_name(&self) -> &str {
        "domain"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_protocol_manager_creation() {
        let manager = ProtocolManager::new();
        
        assert!(!manager.running);
    }

    #[tokio::test]
    async fn test_protocol_manager_start_stop() {
        let mut manager = ProtocolManager::new();
        
        manager.start().await.unwrap();
        assert!(manager.running);
        
        manager.stop().await.unwrap();
        assert!(!manager.running);
    }

    #[tokio::test]
    async fn test_protocol_handler_registration() {
        let manager = ProtocolManager::new();
        
        let block_handler = Box::new(BlockProtocolHandler);
        manager.register_handler(block_handler).await.unwrap();
        
        let protocols = manager.get_registered_protocols().await;
        assert!(protocols.contains(&"block".to_string()));
    }

    #[tokio::test]
    async fn test_block_message_serialization() {
        let block_msg = BlockMessage {
            msg_type: BlockMessageType::Announcement,
            block_data: Some(vec![1, 2, 3, 4]),
            block_hash: Some([1u8; 32]),
            height: Some(123),
            timestamp: Utc::now(),
        };
        
        let message = ProtocolMessage::Block(block_msg);
        let serialized = serde_json::to_vec(&message).unwrap();
        let deserialized: ProtocolMessage = serde_json::from_slice(&serialized).unwrap();
        
        assert!(matches!(deserialized, ProtocolMessage::Block(_)));
    }

    #[tokio::test]
    async fn test_transaction_message_serialization() {
        let tx_msg = TransactionMessage {
            msg_type: TransactionMessageType::Announcement,
            tx_data: Some(vec![1, 2, 3, 4]),
            tx_hash: Some([1u8; 32]),
            tx_type: Some("payment".to_string()),
            timestamp: Utc::now(),
        };
        
        let message = ProtocolMessage::Transaction(tx_msg);
        let serialized = serde_json::to_vec(&message).unwrap();
        let deserialized: ProtocolMessage = serde_json::from_slice(&serialized).unwrap();
        
        assert!(matches!(deserialized, ProtocolMessage::Transaction(_)));
    }
} 