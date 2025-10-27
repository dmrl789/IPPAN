//! Connection management for IPPAN network
//!
//! Provides robust connection handling, reconnection logic, and connection pooling
//! for the IPPAN P2P network.

use anyhow::{anyhow, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::time::{interval, sleep, timeout};
use tracing::{debug, error, info, warn};

use crate::peers::Peer;

/// Connection state for a peer
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

/// Connection configuration
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    pub max_connections: usize,
    pub connection_timeout: Duration,
    pub keep_alive_interval: Duration,
    pub reconnect_interval: Duration,
    pub max_reconnect_attempts: usize,
    pub read_timeout: Duration,
    pub write_timeout: Duration,
    pub buffer_size: usize,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            max_connections: 100,
            connection_timeout: Duration::from_secs(10),
            keep_alive_interval: Duration::from_secs(30),
            reconnect_interval: Duration::from_secs(5),
            max_reconnect_attempts: 5,
            read_timeout: Duration::from_secs(30),
            write_timeout: Duration::from_secs(10),
            buffer_size: 64 * 1024, // 64KB
        }
    }
}

/// Active connection information
#[derive(Debug, Clone)]
pub struct ActiveConnection {
    pub peer: Peer,
    pub state: ConnectionState,
    pub connected_at: Instant,
    pub last_activity: Instant,
    pub reconnect_attempts: usize,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub message_count: u64,
}

impl ActiveConnection {
    pub fn new(peer: Peer) -> Self {
        let now = Instant::now();
        Self {
            peer,
            state: ConnectionState::Disconnected,
            connected_at: now,
            last_activity: now,
            reconnect_attempts: 0,
            bytes_sent: 0,
            bytes_received: 0,
            message_count: 0,
        }
    }

    pub fn is_healthy(&self, config: &ConnectionConfig) -> bool {
        self.state == ConnectionState::Connected
            && self.last_activity.elapsed() < config.keep_alive_interval * 2
    }

    pub fn should_reconnect(&self, config: &ConnectionConfig) -> bool {
        self.state == ConnectionState::Disconnected
            && self.reconnect_attempts < config.max_reconnect_attempts
    }
}

/// Connection manager for handling peer connections
pub struct ConnectionManager {
    config: ConnectionConfig,
    connections: Arc<RwLock<HashMap<String, ActiveConnection>>>,
    listener: Option<TcpListener>,
    message_sender: mpsc::UnboundedSender<NetworkMessage>,
    message_receiver: Option<mpsc::UnboundedReceiver<NetworkMessage>>,
    is_running: Arc<RwLock<bool>>,
}

/// Network message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    Connect { peer_id: String, address: String },
    Disconnect { peer_id: String },
    Data { peer_id: String, data: Vec<u8> },
    KeepAlive { peer_id: String },
    Error { peer_id: String, error: String },
}

impl ConnectionManager {
    /// Create a new connection manager
    pub fn new(config: ConnectionConfig) -> Self {
        let (message_sender, message_receiver) = mpsc::unbounded_channel();
        
        Self {
            config,
            connections: Arc::new(RwLock::new(HashMap::new())),
            listener: None,
            message_sender,
            message_receiver: Some(message_receiver),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the connection manager
    pub async fn start(&mut self, listen_addr: SocketAddr) -> Result<()> {
        *self.is_running.write() = true;
        
        // Start listening for incoming connections
        self.listener = Some(TcpListener::bind(listen_addr).await?);
        info!("Connection manager listening on {}", listen_addr);

        // Start connection handling tasks
        self.start_connection_handler().await;
        self.start_keep_alive_handler().await;
        self.start_reconnect_handler().await;

        Ok(())
    }

    /// Stop the connection manager
    pub async fn stop(&mut self) -> Result<()> {
        *self.is_running.write() = false;
        
        // Close all connections
        {
            let mut connections = self.connections.write();
            for (_, conn) in connections.iter_mut() {
                conn.state = ConnectionState::Disconnected;
            }
        }

        info!("Connection manager stopped");
        Ok(())
    }

    /// Connect to a peer
    pub async fn connect(&self, peer: Peer) -> Result<()> {
        let peer_id = peer.id.clone().unwrap_or_else(|| peer.address.clone());
        
        // Check if already connected
        {
            let connections = self.connections.read();
            if let Some(conn) = connections.get(&peer_id) {
                if conn.state == ConnectionState::Connected {
                    return Ok(());
                }
            }
        }

        // Add to connections
        {
            let mut connections = self.connections.write();
            connections.insert(peer_id.clone(), ActiveConnection::new(peer.clone()));
        }

        // Send connect message
        self.message_sender.send(NetworkMessage::Connect {
            peer_id: peer_id.clone(),
            address: peer.address.clone(),
        })?;

        Ok(())
    }

    /// Disconnect from a peer
    pub async fn disconnect(&self, peer_id: &str) -> Result<()> {
        {
            let mut connections = self.connections.write();
            if let Some(conn) = connections.get_mut(peer_id) {
                conn.state = ConnectionState::Disconnected;
            }
        }

        self.message_sender.send(NetworkMessage::Disconnect {
            peer_id: peer_id.to_string(),
        })?;

        Ok(())
    }

    /// Send data to a peer
    pub async fn send_data(&self, peer_id: &str, data: Vec<u8>) -> Result<()> {
        self.message_sender.send(NetworkMessage::Data {
            peer_id: peer_id.to_string(),
            data,
        })?;
        Ok(())
    }

    /// Get connection statistics
    pub fn get_stats(&self) -> ConnectionStats {
        let connections = self.connections.read();
        let mut stats = ConnectionStats::default();
        
        for conn in connections.values() {
            stats.total_connections += 1;
            match conn.state {
                ConnectionState::Connected => stats.connected_count += 1,
                ConnectionState::Connecting => stats.connecting_count += 1,
                ConnectionState::Reconnecting => stats.reconnecting_count += 1,
                ConnectionState::Disconnected => stats.disconnected_count += 1,
                ConnectionState::Failed => stats.failed_count += 1,
            }
            stats.total_bytes_sent += conn.bytes_sent;
            stats.total_bytes_received += conn.bytes_received;
            stats.total_messages += conn.message_count;
        }

        stats
    }

    /// Get active connections
    pub fn get_connections(&self) -> Vec<ActiveConnection> {
        self.connections.read().values().cloned().collect()
    }

    /// Start the main connection handler
    async fn start_connection_handler(&self) {
        let connections = self.connections.clone();
        let config = self.config.clone();
        let message_sender = self.message_sender.clone();
        let is_running = self.is_running.clone();
        let mut message_receiver = self.message_receiver.take().unwrap();

        tokio::spawn(async move {
            while *is_running.read() {
                if let Some(message) = message_receiver.recv().await {
                    match message {
                        NetworkMessage::Connect { peer_id, address } => {
                            Self::handle_connect(&connections, &config, &message_sender, &peer_id, &address).await;
                        }
                        NetworkMessage::Disconnect { peer_id } => {
                            Self::handle_disconnect(&connections, &peer_id).await;
                        }
                        NetworkMessage::Data { peer_id, data } => {
                            Self::handle_send_data(&connections, &config, &message_sender, &peer_id, data).await;
                        }
                        NetworkMessage::KeepAlive { peer_id } => {
                            Self::handle_keep_alive(&connections, &peer_id).await;
                        }
                        NetworkMessage::Error { peer_id, error } => {
                            Self::handle_error(&connections, &peer_id, &error).await;
                        }
                    }
                }
            }
        });
    }

    /// Start the keep-alive handler
    async fn start_keep_alive_handler(&self) {
        let connections = self.connections.clone();
        let config = self.config.clone();
        let message_sender = self.message_sender.clone();
        let is_running = self.is_running.clone();

        tokio::spawn(async move {
            let mut interval = interval(config.keep_alive_interval);
            
            while *is_running.read() {
                interval.tick().await;
                
                let keep_alive_peers: Vec<String> = {
                    let conns = connections.read();
                    conns.iter()
                        .filter(|(_, conn)| conn.state == ConnectionState::Connected)
                        .map(|(peer_id, _)| peer_id.clone())
                        .collect()
                };

                for peer_id in keep_alive_peers {
                    if let Err(e) = message_sender.send(NetworkMessage::KeepAlive { peer_id }) {
                        warn!("Failed to send keep-alive: {}", e);
                    }
                }
            }
        });
    }

    /// Start the reconnect handler
    async fn start_reconnect_handler(&self) {
        let connections = self.connections.clone();
        let config = self.config.clone();
        let message_sender = self.message_sender.clone();
        let is_running = self.is_running.clone();

        tokio::spawn(async move {
            let mut interval = interval(config.reconnect_interval);
            
            while *is_running.read() {
                interval.tick().await;
                
                let reconnect_peers: Vec<(String, String)> = {
                    let conns = connections.read();
                    conns.iter()
                        .filter(|(_, conn)| conn.should_reconnect(&config))
                        .map(|(peer_id, conn)| (peer_id.clone(), conn.peer.address.clone()))
                        .collect()
                };

                for (peer_id, address) in reconnect_peers {
                    if let Err(e) = message_sender.send(NetworkMessage::Connect { peer_id, address }) {
                        warn!("Failed to send reconnect message: {}", e);
                    }
                }
            }
        });
    }

    /// Handle connection attempt
    async fn handle_connect(
        connections: &Arc<RwLock<HashMap<String, ActiveConnection>>>,
        config: &ConnectionConfig,
        message_sender: &mpsc::UnboundedSender<NetworkMessage>,
        peer_id: &str,
        address: &str,
    ) {
        // Parse address
        let addr = match address.parse::<SocketAddr>() {
            Ok(addr) => addr,
            Err(e) => {
                error!("Invalid address {}: {}", address, e);
                let _ = message_sender.send(NetworkMessage::Error {
                    peer_id: peer_id.to_string(),
                    error: format!("Invalid address: {}", e),
                });
                return;
            }
        };

        // Update connection state
        {
            let mut conns = connections.write();
            if let Some(conn) = conns.get_mut(peer_id) {
                conn.state = ConnectionState::Connecting;
            }
        }

        // Attempt connection with timeout
        match timeout(config.connection_timeout, TcpStream::connect(addr)).await {
            Ok(Ok(mut stream)) => {
                info!("Connected to peer {} at {}", peer_id, address);
                
                // Update connection state
                {
                    let mut conns = connections.write();
                    if let Some(conn) = conns.get_mut(peer_id) {
                        conn.state = ConnectionState::Connected;
                        conn.connected_at = Instant::now();
                        conn.last_activity = Instant::now();
                        conn.reconnect_attempts = 0;
                    }
                }

                // Start connection handler
                Self::handle_connection_stream(connections, config, message_sender, peer_id, stream).await;
            }
            Ok(Err(e)) => {
                error!("Failed to connect to {}: {}", address, e);
                Self::handle_connection_error(connections, message_sender, peer_id, &e.to_string()).await;
            }
            Err(_) => {
                error!("Connection timeout to {}", address);
                Self::handle_connection_error(connections, message_sender, peer_id, "Connection timeout").await;
            }
        }
    }

    /// Handle connection stream
    async fn handle_connection_stream(
        connections: &Arc<RwLock<HashMap<String, ActiveConnection>>>,
        config: &ConnectionConfig,
        message_sender: &mpsc::UnboundedSender<NetworkMessage>,
        peer_id: &str,
        mut stream: TcpStream,
    ) {
        let mut buffer = vec![0u8; config.buffer_size];
        
        loop {
            // Check if connection is still valid
            {
                let conns = connections.read();
                if let Some(conn) = conns.get(peer_id) {
                    if conn.state != ConnectionState::Connected {
                        break;
                    }
                } else {
                    break;
                }
            }

            // Read with timeout
            match timeout(config.read_timeout, stream.read(&mut buffer)).await {
                Ok(Ok(0)) => {
                    debug!("Peer {} disconnected", peer_id);
                    break;
                }
                Ok(Ok(n)) => {
                    let data = buffer[..n].to_vec();
                    
                    // Update connection stats
                    {
                        let mut conns = connections.write();
                        if let Some(conn) = conns.get_mut(peer_id) {
                            conn.last_activity = Instant::now();
                            conn.bytes_received += n as u64;
                            conn.message_count += 1;
                        }
                    }

                    // Process received data
                    debug!("Received {} bytes from peer {}", n, peer_id);
                }
                Ok(Err(e)) => {
                    error!("Read error from peer {}: {}", peer_id, e);
                    break;
                }
                Err(_) => {
                    debug!("Read timeout from peer {}", peer_id);
                    continue;
                }
            }
        }

        // Mark as disconnected
        {
            let mut conns = connections.write();
            if let Some(conn) = conns.get_mut(peer_id) {
                conn.state = ConnectionState::Disconnected;
            }
        }
    }

    /// Handle disconnection
    async fn handle_disconnect(
        connections: &Arc<RwLock<HashMap<String, ActiveConnection>>>,
        peer_id: &str,
    ) {
        let mut conns = connections.write();
        if let Some(conn) = conns.get_mut(peer_id) {
            conn.state = ConnectionState::Disconnected;
        }
    }

    /// Handle send data
    async fn handle_send_data(
        connections: &Arc<RwLock<HashMap<String, ActiveConnection>>>,
        config: &ConnectionConfig,
        message_sender: &mpsc::UnboundedSender<NetworkMessage>,
        peer_id: &str,
        data: Vec<u8>,
    ) {
        // In a real implementation, this would send data through the connection
        // For now, we'll just update stats
        {
            let mut conns = connections.write();
            if let Some(conn) = conns.get_mut(peer_id) {
                if conn.state == ConnectionState::Connected {
                    conn.bytes_sent += data.len() as u64;
                    conn.message_count += 1;
                    conn.last_activity = Instant::now();
                }
            }
        }
    }

    /// Handle keep-alive
    async fn handle_keep_alive(
        connections: &Arc<RwLock<HashMap<String, ActiveConnection>>>,
        peer_id: &str,
    ) {
        let mut conns = connections.write();
        if let Some(conn) = conns.get_mut(peer_id) {
            conn.last_activity = Instant::now();
        }
    }

    /// Handle connection error
    async fn handle_connection_error(
        connections: &Arc<RwLock<HashMap<String, ActiveConnection>>>,
        message_sender: &mpsc::UnboundedSender<NetworkMessage>,
        peer_id: &str,
        error: &str,
    ) {
        {
            let mut conns = connections.write();
            if let Some(conn) = conns.get_mut(peer_id) {
                conn.state = ConnectionState::Failed;
                conn.reconnect_attempts += 1;
            }
        }

        let _ = message_sender.send(NetworkMessage::Error {
            peer_id: peer_id.to_string(),
            error: error.to_string(),
        });
    }

    /// Handle error
    async fn handle_error(
        connections: &Arc<RwLock<HashMap<String, ActiveConnection>>>,
        peer_id: &str,
        error: &str,
    ) {
        warn!("Connection error for peer {}: {}", peer_id, error);
        
        {
            let mut conns = connections.write();
            if let Some(conn) = conns.get_mut(peer_id) {
                conn.state = ConnectionState::Failed;
                conn.reconnect_attempts += 1;
            }
        }
    }
}

/// Connection statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConnectionStats {
    pub total_connections: usize,
    pub connected_count: usize,
    pub connecting_count: usize,
    pub reconnecting_count: usize,
    pub disconnected_count: usize,
    pub failed_count: usize,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
    pub total_messages: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[tokio::test]
    async fn test_connection_manager_creation() {
        let config = ConnectionConfig::default();
        let manager = ConnectionManager::new(config);
        assert_eq!(manager.get_connections().len(), 0);
    }

    #[tokio::test]
    async fn test_connection_stats() {
        let config = ConnectionConfig::default();
        let manager = ConnectionManager::new(config);
        let stats = manager.get_stats();
        assert_eq!(stats.total_connections, 0);
    }

    #[tokio::test]
    async fn test_peer_connection() {
        let config = ConnectionConfig::default();
        let manager = ConnectionManager::new(config);
        let peer = Peer::with_id("test-peer", "127.0.0.1:8080");
        
        // This would normally connect, but we're just testing the API
        let result = manager.connect(peer).await;
        assert!(result.is_ok());
    }
}
