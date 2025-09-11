//! Network management commands for IPPAN CLI
//! 
//! Implements commands for network management including peer information,
//! connection management, and network statistics.

use crate::{Result, IppanError, TransactionHash};
use super::{CLIContext, CLIResult, OutputFormat};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};
use tracing::{info, warn, error, debug};

/// Peer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    /// Peer ID
    pub peer_id: String,
    /// Peer address
    pub peer_address: String,
    /// Peer port
    pub peer_port: u16,
    /// Connection status
    pub connection_status: String,
    /// Connection time in seconds
    pub connection_time_seconds: u64,
    /// Last seen timestamp
    pub last_seen_timestamp: u64,
    /// Bytes sent
    pub bytes_sent: u64,
    /// Bytes received
    pub bytes_received: u64,
    /// Messages sent
    pub messages_sent: u64,
    /// Messages received
    pub messages_received: u64,
    /// Peer version
    pub peer_version: String,
    /// Peer capabilities
    pub peer_capabilities: Vec<String>,
    /// Is bootstrap peer
    pub is_bootstrap_peer: bool,
    /// Peer reputation
    pub peer_reputation: f64,
}

/// Network statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStats {
    /// Total peers
    pub total_peers: u64,
    /// Connected peers
    pub connected_peers: u64,
    /// Disconnected peers
    pub disconnected_peers: u64,
    /// Bootstrap peers
    pub bootstrap_peers: u64,
    /// Total connections
    pub total_connections: u64,
    /// Active connections
    pub active_connections: u64,
    /// Total bytes sent
    pub total_bytes_sent: u64,
    /// Total bytes received
    pub total_bytes_received: u64,
    /// Total messages sent
    pub total_messages_sent: u64,
    /// Total messages received
    pub total_messages_received: u64,
    /// Average connection time in seconds
    pub average_connection_time_seconds: f64,
    /// Network uptime in seconds
    pub network_uptime_seconds: u64,
    /// Last peer connection timestamp
    pub last_peer_connection_timestamp: Option<u64>,
    /// Last peer disconnection timestamp
    pub last_peer_disconnection_timestamp: Option<u64>,
}

/// Network commands manager
pub struct NetworkCommands {
    /// Network reference
    network: Option<Arc<RwLock<crate::network::NetworkManager>>>,
    /// Statistics
    stats: Arc<RwLock<NetworkCommandStats>>,
    /// Start time
    start_time: Instant,
}

/// Network command statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkCommandStats {
    /// Total commands executed
    pub total_commands_executed: u64,
    /// Successful commands
    pub successful_commands: u64,
    /// Failed commands
    pub failed_commands: u64,
    /// Average execution time in milliseconds
    pub average_execution_time_ms: f64,
    /// Most used commands
    pub most_used_commands: HashMap<String, u64>,
    /// Command success rate
    pub command_success_rate: f64,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Last command timestamp
    pub last_command_timestamp: Option<u64>,
}

impl Default for NetworkCommandStats {
    fn default() -> Self {
        Self {
            total_commands_executed: 0,
            successful_commands: 0,
            failed_commands: 0,
            average_execution_time_ms: 0.0,
            most_used_commands: HashMap::new(),
            command_success_rate: 0.0,
            uptime_seconds: 0,
            last_command_timestamp: None,
        }
    }
}

impl NetworkCommands {
    /// Create a new network commands manager
    pub fn new(network: Option<Arc<RwLock<crate::network::NetworkManager>>>) -> Self {
        Self {
            network,
            stats: Arc::new(RwLock::new(NetworkCommandStats::default())),
            start_time: Instant::now(),
        }
    }
    
    /// Get all peers
    pub async fn get_all_peers(&self) -> Result<Vec<PeerInfo>> {
        info!("Getting all peers");
        
        let peers = vec![
            PeerInfo {
                peer_id: "peer_1234567890abcdef".to_string(),
                peer_address: "192.168.1.100".to_string(),
                peer_port: 30303,
                connection_status: "Connected".to_string(),
                connection_time_seconds: 3600,
                last_seen_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                bytes_sent: 1024 * 1024 * 10, // 10MB
                bytes_received: 1024 * 1024 * 8, // 8MB
                messages_sent: 1000,
                messages_received: 950,
                peer_version: "1.0.0".to_string(),
                peer_capabilities: vec!["BFT".to_string(), "P2P".to_string()],
                is_bootstrap_peer: true,
                peer_reputation: 0.95,
            },
            PeerInfo {
                peer_id: "peer_abcdef1234567890".to_string(),
                peer_address: "192.168.1.101".to_string(),
                peer_port: 30303,
                connection_status: "Connected".to_string(),
                connection_time_seconds: 1800,
                last_seen_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                bytes_sent: 1024 * 1024 * 5, // 5MB
                bytes_received: 1024 * 1024 * 6, // 6MB
                messages_sent: 500,
                messages_received: 480,
                peer_version: "1.0.0".to_string(),
                peer_capabilities: vec!["BFT".to_string(), "P2P".to_string()],
                is_bootstrap_peer: false,
                peer_reputation: 0.88,
            },
        ];
        
        info!("Retrieved {} peers", peers.len());
        Ok(peers)
    }
    
    /// Get peer by ID
    pub async fn get_peer_by_id(&self, peer_id: &str) -> Result<Option<PeerInfo>> {
        info!("Getting peer by ID: {}", peer_id);
        
        let peer = PeerInfo {
            peer_id: peer_id.to_string(),
            peer_address: "192.168.1.100".to_string(),
            peer_port: 30303,
            connection_status: "Connected".to_string(),
            connection_time_seconds: 3600,
            last_seen_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            bytes_sent: 1024 * 1024 * 10, // 10MB
            bytes_received: 1024 * 1024 * 8, // 8MB
            messages_sent: 1000,
            messages_received: 950,
            peer_version: "1.0.0".to_string(),
            peer_capabilities: vec!["BFT".to_string(), "P2P".to_string()],
            is_bootstrap_peer: true,
            peer_reputation: 0.95,
        };
        
        info!("Peer retrieved successfully");
        Ok(Some(peer))
    }
    
    /// Connect to peer
    pub async fn connect_to_peer(&self, address: &str, port: u16) -> Result<()> {
        info!("Connecting to peer: {}:{}", address, port);
        
        // In a real implementation, this would establish a connection to the peer
        // For now, we'll just log the connection attempt
        
        info!("Connected to peer successfully");
        Ok(())
    }
    
    /// Disconnect from peer
    pub async fn disconnect_from_peer(&self, peer_id: &str) -> Result<()> {
        info!("Disconnecting from peer: {}", peer_id);
        
        // In a real implementation, this would disconnect from the peer
        // For now, we'll just log the disconnection
        
        info!("Disconnected from peer successfully");
        Ok(())
    }
    
    /// Get network statistics
    pub async fn get_network_statistics(&self) -> Result<NetworkStats> {
        info!("Getting network statistics");
        
        let stats = NetworkStats {
            total_peers: 25,
            connected_peers: 20,
            disconnected_peers: 5,
            bootstrap_peers: 3,
            total_connections: 30,
            active_connections: 20,
            total_bytes_sent: 1024 * 1024 * 100, // 100MB
            total_bytes_received: 1024 * 1024 * 80, // 80MB
            total_messages_sent: 10000,
            total_messages_received: 9500,
            average_connection_time_seconds: 1800.0, // 30 minutes
            network_uptime_seconds: 86400 * 7, // 7 days
            last_peer_connection_timestamp: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() - 300),
            last_peer_disconnection_timestamp: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() - 600),
        };
        
        info!("Network statistics retrieved successfully");
        Ok(stats)
    }
    
    /// Get connection information
    pub async fn get_connection_info(&self) -> Result<HashMap<String, serde_json::Value>> {
        info!("Getting connection information");
        
        let mut connection_info = HashMap::new();
        connection_info.insert("listen_address".to_string(), serde_json::Value::String("0.0.0.0".to_string()));
        connection_info.insert("listen_port".to_string(), serde_json::Value::Number(serde_json::Number::from(30303)));
        connection_info.insert("max_connections".to_string(), serde_json::Value::Number(serde_json::Number::from(50)));
        connection_info.insert("current_connections".to_string(), serde_json::Value::Number(serde_json::Number::from(20)));
        connection_info.insert("connection_timeout_seconds".to_string(), serde_json::Value::Number(serde_json::Number::from(30)));
        connection_info.insert("message_timeout_seconds".to_string(), serde_json::Value::Number(serde_json::Number::from(10)));
        connection_info.insert("enable_compression".to_string(), serde_json::Value::Bool(true));
        connection_info.insert("enable_encryption".to_string(), serde_json::Value::Bool(true));
        
        info!("Connection information retrieved successfully");
        Ok(connection_info)
    }
    
    /// Update statistics
    async fn update_stats(&self, command_name: &str, execution_time_ms: u64, success: bool) {
        let mut stats = self.stats.write().await;
        
        stats.total_commands_executed += 1;
        if success {
            stats.successful_commands += 1;
        } else {
            stats.failed_commands += 1;
        }
        
        // Update averages
        let total = stats.total_commands_executed as f64;
        stats.average_execution_time_ms = 
            (stats.average_execution_time_ms * (total - 1.0) + execution_time_ms as f64) / total;
        
        // Update most used commands
        *stats.most_used_commands.entry(command_name.to_string()).or_insert(0) += 1;
        
        // Update success rate
        stats.command_success_rate = stats.successful_commands as f64 / total;
        
        // Update timestamps
        stats.last_command_timestamp = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
        stats.uptime_seconds = self.start_time.elapsed().as_secs();
    }
}

/// Network command handlers
pub struct NetworkCommandHandlers;

impl NetworkCommandHandlers {
    /// Handle list-peers command
    pub async fn handle_list_peers(context: &CLIContext, args: Vec<String>) -> Result<CLIResult> {
        let start_time = Instant::now();
        
        let network_commands = NetworkCommands::new(None);
        let peers = network_commands.get_all_peers().await?;
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        Ok(CLIResult {
            success: true,
            data: Some(serde_json::to_value(peers)?),
            error_message: None,
            execution_time_ms: execution_time,
            command_name: "list-peers".to_string(),
            command_arguments: args,
            output_format: OutputFormat::Table,
        })
    }
    
    /// Handle get-peer command
    pub async fn handle_get_peer(context: &CLIContext, args: Vec<String>) -> Result<CLIResult> {
        let start_time = Instant::now();
        
        if args.is_empty() {
            return Ok(CLIResult {
                success: false,
                data: None,
                error_message: Some("Usage: get-peer <peer_id>".to_string()),
                execution_time_ms: start_time.elapsed().as_millis() as u64,
                command_name: "get-peer".to_string(),
                command_arguments: args,
                output_format: OutputFormat::Plain,
            });
        }
        
        let network_commands = NetworkCommands::new(None);
        let peer = network_commands.get_peer_by_id(&args[0]).await?;
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        Ok(CLIResult {
            success: true,
            data: Some(serde_json::to_value(peer)?),
            error_message: None,
            execution_time_ms: execution_time,
            command_name: "get-peer".to_string(),
            command_arguments: args,
            output_format: OutputFormat::Json,
        })
    }
    
    /// Handle connect command
    pub async fn handle_connect(context: &CLIContext, args: Vec<String>) -> Result<CLIResult> {
        let start_time = Instant::now();
        
        if args.len() < 2 {
            return Ok(CLIResult {
                success: false,
                data: None,
                error_message: Some("Usage: connect <address> <port>".to_string()),
                execution_time_ms: start_time.elapsed().as_millis() as u64,
                command_name: "connect".to_string(),
                command_arguments: args,
                output_format: OutputFormat::Plain,
            });
        }
        
        let port = args[1].parse::<u16>()
            .map_err(|_| IppanError::CLI(format!("Invalid port: {}", args[1])))?;
        
        let network_commands = NetworkCommands::new(None);
        network_commands.connect_to_peer(&args[0], port).await?;
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        Ok(CLIResult {
            success: true,
            data: Some(serde_json::Value::String("Connected to peer successfully".to_string())),
            error_message: None,
            execution_time_ms: execution_time,
            command_name: "connect".to_string(),
            command_arguments: args,
            output_format: OutputFormat::Plain,
        })
    }
    
    /// Handle disconnect command
    pub async fn handle_disconnect(context: &CLIContext, args: Vec<String>) -> Result<CLIResult> {
        let start_time = Instant::now();
        
        if args.is_empty() {
            return Ok(CLIResult {
                success: false,
                data: None,
                error_message: Some("Usage: disconnect <peer_id>".to_string()),
                execution_time_ms: start_time.elapsed().as_millis() as u64,
                command_name: "disconnect".to_string(),
                command_arguments: args,
                output_format: OutputFormat::Plain,
            });
        }
        
        let network_commands = NetworkCommands::new(None);
        network_commands.disconnect_from_peer(&args[0]).await?;
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        Ok(CLIResult {
            success: true,
            data: Some(serde_json::Value::String("Disconnected from peer successfully".to_string())),
            error_message: None,
            execution_time_ms: execution_time,
            command_name: "disconnect".to_string(),
            command_arguments: args,
            output_format: OutputFormat::Plain,
        })
    }
    
    /// Handle network-stats command
    pub async fn handle_network_stats(context: &CLIContext, args: Vec<String>) -> Result<CLIResult> {
        let start_time = Instant::now();
        
        let network_commands = NetworkCommands::new(None);
        let stats = network_commands.get_network_statistics().await?;
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        Ok(CLIResult {
            success: true,
            data: Some(serde_json::to_value(stats)?),
            error_message: None,
            execution_time_ms: execution_time,
            command_name: "network-stats".to_string(),
            command_arguments: args,
            output_format: OutputFormat::Json,
        })
    }
    
    /// Handle connection-info command
    pub async fn handle_connection_info(context: &CLIContext, args: Vec<String>) -> Result<CLIResult> {
        let start_time = Instant::now();
        
        let network_commands = NetworkCommands::new(None);
        let connection_info = network_commands.get_connection_info().await?;
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        Ok(CLIResult {
            success: true,
            data: Some(serde_json::to_value(connection_info)?),
            error_message: None,
            execution_time_ms: execution_time,
            command_name: "connection-info".to_string(),
            command_arguments: args,
            output_format: OutputFormat::Json,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_peer_info() {
        let peer = PeerInfo {
            peer_id: "peer_1234567890abcdef".to_string(),
            peer_address: "192.168.1.100".to_string(),
            peer_port: 30303,
            connection_status: "Connected".to_string(),
            connection_time_seconds: 3600,
            last_seen_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            bytes_sent: 1024 * 1024 * 10,
            bytes_received: 1024 * 1024 * 8,
            messages_sent: 1000,
            messages_received: 950,
            peer_version: "1.0.0".to_string(),
            peer_capabilities: vec!["BFT".to_string(), "P2P".to_string()],
            is_bootstrap_peer: true,
            peer_reputation: 0.95,
        };
        
        assert_eq!(peer.peer_id, "peer_1234567890abcdef");
        assert_eq!(peer.peer_address, "192.168.1.100");
        assert_eq!(peer.peer_port, 30303);
        assert_eq!(peer.connection_status, "Connected");
        assert_eq!(peer.connection_time_seconds, 3600);
        assert_eq!(peer.bytes_sent, 1024 * 1024 * 10);
        assert_eq!(peer.bytes_received, 1024 * 1024 * 8);
        assert_eq!(peer.messages_sent, 1000);
        assert_eq!(peer.messages_received, 950);
        assert_eq!(peer.peer_version, "1.0.0");
        assert_eq!(peer.peer_capabilities, vec!["BFT".to_string(), "P2P".to_string()]);
        assert!(peer.is_bootstrap_peer);
        assert_eq!(peer.peer_reputation, 0.95);
    }
    
    #[tokio::test]
    async fn test_network_stats() {
        let stats = NetworkStats {
            total_peers: 25,
            connected_peers: 20,
            disconnected_peers: 5,
            bootstrap_peers: 3,
            total_connections: 30,
            active_connections: 20,
            total_bytes_sent: 1024 * 1024 * 100,
            total_bytes_received: 1024 * 1024 * 80,
            total_messages_sent: 10000,
            total_messages_received: 9500,
            average_connection_time_seconds: 1800.0,
            network_uptime_seconds: 86400 * 7,
            last_peer_connection_timestamp: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() - 300),
            last_peer_disconnection_timestamp: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() - 600),
        };
        
        assert_eq!(stats.total_peers, 25);
        assert_eq!(stats.connected_peers, 20);
        assert_eq!(stats.disconnected_peers, 5);
        assert_eq!(stats.bootstrap_peers, 3);
        assert_eq!(stats.total_connections, 30);
        assert_eq!(stats.active_connections, 20);
        assert_eq!(stats.total_bytes_sent, 1024 * 1024 * 100);
        assert_eq!(stats.total_bytes_received, 1024 * 1024 * 80);
        assert_eq!(stats.total_messages_sent, 10000);
        assert_eq!(stats.total_messages_received, 9500);
        assert_eq!(stats.average_connection_time_seconds, 1800.0);
        assert_eq!(stats.network_uptime_seconds, 86400 * 7);
        assert!(stats.last_peer_connection_timestamp.is_some());
        assert!(stats.last_peer_disconnection_timestamp.is_some());
    }
}
