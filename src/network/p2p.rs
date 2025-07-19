//! P2P networking for IPPAN

use crate::Result;
use crate::utils::address::validate_ippan_address;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use chrono::{DateTime, Utc};

/// P2P message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum P2PMessage {
    /// Handshake message
    Handshake(HandshakeMessage),
    /// Ping message
    Ping(PingMessage),
    /// Pong response
    Pong(PongMessage),
    /// Block announcement
    BlockAnnouncement(BlockAnnouncement),
    /// Transaction announcement
    TransactionAnnouncement(TransactionAnnouncement),
    /// Get blocks request
    GetBlocks(GetBlocksRequest),
    /// Block data response
    BlockData(BlockData),
    /// Get peers request
    GetPeers(GetPeersRequest),
    /// Peers response
    PeersResponse(PeersResponse),
}

/// Handshake message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeMessage {
    /// Node ID
    pub node_id: [u8; 32],
    /// Node address
    pub address: String,
    /// Protocol version
    pub version: u32,
    /// Supported features
    pub features: Vec<String>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Ping message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingMessage {
    /// Nonce for response matching
    pub nonce: u64,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Pong response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PongMessage {
    /// Nonce from ping
    pub nonce: u64,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Block announcement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockAnnouncement {
    /// Block hash
    pub block_hash: [u8; 32],
    /// Block height
    pub height: u64,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Transaction announcement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionAnnouncement {
    /// Transaction hash
    pub tx_hash: [u8; 32],
    /// Transaction type
    pub tx_type: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Get blocks request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetBlocksRequest {
    /// Starting height
    pub start_height: u64,
    /// Ending height
    pub end_height: u64,
    /// Maximum blocks to return
    pub max_blocks: u32,
}

/// Block data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockData {
    /// Block hash
    pub block_hash: [u8; 32],
    /// Block height
    pub height: u64,
    /// Block data
    pub data: Vec<u8>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Get peers request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPeersRequest {
    /// Maximum peers to return
    pub max_peers: u32,
}

/// Peers response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeersResponse {
    /// List of peer addresses
    pub peers: Vec<PeerInfo>,
}

/// Peer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    /// Peer address
    pub address: String,
    /// Peer port
    pub port: u16,
    /// Last seen timestamp
    pub last_seen: DateTime<Utc>,
    /// Peer score
    pub score: f64,
}

/// P2P connection
#[derive(Debug)]
pub struct P2PConnection {
    /// Connection ID
    pub id: String,
    /// Remote address
    pub remote_addr: SocketAddr,
    /// Connection state
    pub state: ConnectionState,
    /// Last activity
    pub last_activity: DateTime<Utc>,
    /// Connection score
    pub score: f64,
    /// TCP stream
    pub stream: Option<TcpStream>,
}

/// Connection state
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    /// Connecting
    Connecting,
    /// Connected
    Connected,
    /// Handshaking
    Handshaking,
    /// Ready
    Ready,
    /// Disconnected
    Disconnected,
}

/// P2P network manager
pub struct P2PNetwork {
    /// Node ID
    node_id: [u8; 32],
    /// Node address
    node_address: String,
    /// Listening address
    listen_addr: SocketAddr,
    /// Active connections
    connections: Arc<RwLock<HashMap<String, P2PConnection>>>,
    /// Known peers
    known_peers: Arc<RwLock<HashMap<String, PeerInfo>>>,
    /// Message sender
    message_sender: mpsc::Sender<P2PMessage>,
    /// Message receiver
    message_receiver: mpsc::Receiver<P2PMessage>,
    /// Protocol version
    protocol_version: u32,
    /// Maximum connections
    max_connections: usize,
    /// Connection timeout
    connection_timeout: std::time::Duration,
}

impl P2PNetwork {
    /// Create a new P2P network
    pub async fn new(
        node_id: [u8; 32],
        node_address: String,
        listen_addr: SocketAddr,
    ) -> Result<Self> {
        let (message_sender, message_receiver) = mpsc::channel(1000);
        
        Ok(Self {
            node_id,
            node_address,
            listen_addr,
            connections: Arc::new(RwLock::new(HashMap::new())),
            known_peers: Arc::new(RwLock::new(HashMap::new())),
            message_sender,
            message_receiver,
            protocol_version: 1,
            max_connections: 50,
            connection_timeout: std::time::Duration::from_secs(30),
        })
    }

    /// Start the P2P network
    pub async fn start(&mut self) -> Result<()> {
        log::info!("Starting P2P network on {}", self.listen_addr);
        
        // Start listening for incoming connections
        let listener = TcpListener::bind(self.listen_addr).await?;
        
        // Spawn connection handler
        let connections = self.connections.clone();
        let known_peers = self.known_peers.clone();
        let message_sender = self.message_sender.clone();
        
        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        let conn_id = format!("conn_{}", addr);
                        let mut conn = P2PConnection {
                            id: conn_id.clone(),
                            remote_addr: addr,
                            state: ConnectionState::Connected,
                            last_activity: Utc::now(),
                            score: 0.0,
                            stream: Some(stream),
                        };
                        
                        connections.write().await.insert(conn_id.clone(), conn);
                        
                        // Handle connection
                        let connections_clone = connections.clone();
                        let known_peers_clone = known_peers.clone();
                        let message_sender_clone = message_sender.clone();
                        
                        tokio::spawn(async move {
                            Self::handle_connection(
                                conn_id,
                                connections_clone,
                                known_peers_clone,
                                message_sender_clone,
                            ).await;
                        });
                    }
                    Err(e) => {
                        log::error!("Failed to accept connection: {}", e);
                    }
                }
            }
        });
        
        Ok(())
    }

    /// Stop the P2P network
    pub async fn stop(&mut self) -> Result<()> {
        log::info!("Stopping P2P network");
        
        // Close all connections
        let mut connections = self.connections.write().await;
        connections.clear();
        
        Ok(())
    }

    /// Connect to a peer
    pub async fn connect_to_peer(&mut self, address: String, port: u16) -> Result<()> {
        let addr = format!("{}:{}", address, port);
        let socket_addr = addr.parse::<SocketAddr>().map_err(|e| {
            crate::error::IppanError::Network(format!("Invalid address format: {}", e))
        })?;
        
        match TcpStream::connect(socket_addr).await {
            Ok(stream) => {
                let conn_id = format!("out_{}", addr);
                let conn = P2PConnection {
                    id: conn_id.clone(),
                    remote_addr: socket_addr,
                    state: ConnectionState::Connected,
                    last_activity: Utc::now(),
                    score: 0.0,
                    stream: Some(stream),
                };
                
                self.connections.write().await.insert(conn_id, conn);
                
                // Add to known peers
                let peer_info = PeerInfo {
                    address,
                    port,
                    last_seen: Utc::now(),
                    score: 0.0,
                };
                
                self.known_peers.write().await.insert(addr.clone(), peer_info);
                
                log::info!("Connected to peer: {}", addr);
                Ok(())
            }
            Err(e) => {
                log::error!("Failed to connect to peer {}: {}", addr, e);
                Err(crate::error::IppanError::Network(
                    format!("Failed to connect to peer: {}", e)
                ))
            }
        }
    }

    /// Send a message to a peer
    pub async fn send_message(&self, peer_id: &str, message: P2PMessage) -> Result<()> {
        let connections = self.connections.read().await;
        
        if let Some(conn) = connections.get(peer_id) {
            if let Some(_stream) = conn.stream.as_ref() {
                let message_data = serde_json::to_vec(&message)?;
                // TODO: Implement actual stream writing
                log::debug!("Would send message to peer: {} bytes", message_data.len());
                Ok(())
            } else {
                Err(crate::error::IppanError::Network(
                    "Connection stream not available".to_string()
                ))
            }
        } else {
            Err(crate::error::IppanError::Network(
                format!("Peer not found: {}", peer_id)
            ))
        }
    }

    /// Broadcast a message to all peers
    pub async fn broadcast_message(&self, message: P2PMessage) -> Result<()> {
        let connections = self.connections.read().await;
        
        for (peer_id, _) in connections.iter() {
            if let Err(e) = self.send_message(peer_id, message.clone()).await {
                log::warn!("Failed to send message to peer {}: {}", peer_id, e);
            }
        }
        
        Ok(())
    }

    /// Get active connections
    pub async fn get_active_connections(&self) -> Vec<P2PConnection> {
        let connections = self.connections.read().await;
        connections.values()
            .filter(|conn| conn.state == ConnectionState::Ready)
            .map(|conn| P2PConnection {
                id: conn.id.clone(),
                remote_addr: conn.remote_addr,
                state: conn.state.clone(),
                last_activity: conn.last_activity,
                score: conn.score,
                stream: None, // Don't clone the stream
            })
            .collect()
    }

    /// Get known peers
    pub async fn get_known_peers(&self) -> Vec<PeerInfo> {
        let peers = self.known_peers.read().await;
        peers.values().cloned().collect()
    }

    /// Handle incoming connection
    async fn handle_connection(
        conn_id: String,
        connections: Arc<RwLock<HashMap<String, P2PConnection>>>,
        known_peers: Arc<RwLock<HashMap<String, PeerInfo>>>,
        message_sender: mpsc::Sender<P2PMessage>,
    ) {
        let mut connections = connections.write().await;
        
        if let Some(conn) = connections.get_mut(&conn_id) {
            conn.state = ConnectionState::Handshaking;
            
            // Perform handshake
            let handshake = HandshakeMessage {
                node_id: [0u8; 32], // TODO: Use actual node ID
                address: "127.0.0.1".to_string(),
                version: 1,
                features: vec!["blocks".to_string(), "transactions".to_string()],
                timestamp: Utc::now(),
            };
            
            let message = P2PMessage::Handshake(handshake);
            
            if let Some(mut stream) = conn.stream.as_mut() {
                if let Ok(message_data) = serde_json::to_vec(&message) {
                    if stream.write_all(&message_data).await.is_ok() {
                        conn.state = ConnectionState::Ready;
                        conn.last_activity = Utc::now();
                        
                        // Update peer info
                        let mut peers = known_peers.write().await;
                        let peer_info = PeerInfo {
                            address: conn.remote_addr.ip().to_string(),
                            port: conn.remote_addr.port(),
                            last_seen: Utc::now(),
                            score: 1.0,
                        };
                        peers.insert(conn.remote_addr.to_string(), peer_info);
                    }
                }
            }
        }
    }

    /// Handle incoming message
    async fn handle_message(&self, message: P2PMessage) -> Result<()> {
        match message {
            P2PMessage::Handshake(handshake) => {
                log::info!("Received handshake from peer: {}", handshake.address);
                // TODO: Validate handshake and respond
            }
            P2PMessage::Ping(ping) => {
                let _pong = P2PMessage::Pong(PongMessage {
                    nonce: ping.nonce,
                    timestamp: Utc::now(),
                });
                // TODO: Send pong response
            }
            P2PMessage::BlockAnnouncement(announcement) => {
                log::info!("Received block announcement: {:?}", announcement.block_hash);
                // TODO: Process block announcement
            }
            P2PMessage::TransactionAnnouncement(announcement) => {
                log::info!("Received transaction announcement: {:?}", announcement.tx_hash);
                // TODO: Process transaction announcement
            }
            _ => {
                log::debug!("Received message: {:?}", message);
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_p2p_network_creation() {
        let node_id = [1u8; 32];
        let node_address = "127.0.0.1".to_string();
        let listen_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        
        let network = P2PNetwork::new(node_id, node_address, listen_addr).await.unwrap();
        
        assert_eq!(network.protocol_version, 1);
        assert_eq!(network.max_connections, 50);
    }

    #[tokio::test]
    async fn test_handshake_message() {
        let handshake = HandshakeMessage {
            node_id: [1u8; 32],
            address: "127.0.0.1".to_string(),
            version: 1,
            features: vec!["blocks".to_string()],
            timestamp: Utc::now(),
        };
        
        let message = P2PMessage::Handshake(handshake);
        let serialized = serde_json::to_vec(&message).unwrap();
        let deserialized: P2PMessage = serde_json::from_slice(&serialized).unwrap();
        
        assert!(matches!(deserialized, P2PMessage::Handshake(_)));
    }

    #[tokio::test]
    async fn test_ping_pong_message() {
        let ping = PingMessage {
            nonce: 12345,
            timestamp: Utc::now(),
        };
        
        let pong = PongMessage {
            nonce: 12345,
            timestamp: Utc::now(),
        };
        
        let ping_message = P2PMessage::Ping(ping);
        let pong_message = P2PMessage::Pong(pong);
        
        assert!(matches!(ping_message, P2PMessage::Ping(_)));
        assert!(matches!(pong_message, P2PMessage::Pong(_)));
    }
}
