//! Tests for IPPAN Network Protocol

use ippan::network::protocol::{
    NetworkProtocolManager, NetworkConfig, NetworkMessage, Peer, ConnectionStatus,
    BlockProposal, BlockVote, ConsensusRound, PeerDiscovery, PeerHandshake,
    PingMessage, PongMessage, HeartbeatMessage, MessagePriority
};
use std::net::{SocketAddr, IpAddr, Ipv4Addr};
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::test]
async fn test_network_protocol_manager_creation() {
    let config = NetworkConfig::default();
    let manager = NetworkProtocolManager::new("test_node", config).await;

    assert!(manager.is_ok());
    
    let manager = manager.unwrap();
    let stats = manager.get_network_stats().await;
    
    assert_eq!(stats.total_peers, 0);
    assert_eq!(stats.connected_peers, 0);
    assert_eq!(stats.queued_messages, 0);
    assert_eq!(stats.max_peers, 50);
    assert_eq!(stats.message_timeout_ms, 30000);
}

#[tokio::test]
async fn test_peer_management() {
    let config = NetworkConfig::default();
    let manager = NetworkProtocolManager::new("test_node", config).await.unwrap();

    // Create test peer
    let peer = Peer {
        node_id: "peer1".to_string(),
        public_key: "test_public_key".to_string(),
        address: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8081),
        capabilities: vec!["consensus".to_string(), "validation".to_string()],
        version: "1.0.0".to_string(),
        last_seen: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        connection_status: ConnectionStatus::Connected,
        latency_ms: Some(50),
        trust_score: 0.8,
        is_validator: true,
    };

    // Add peer
    assert!(manager.add_peer(peer.clone()).await.is_ok());

    // Check peer was added
    let stats = manager.get_network_stats().await;
    assert_eq!(stats.total_peers, 1);
    assert_eq!(stats.connected_peers, 1);

    // Get peer
    let retrieved_peer = manager.get_peer("peer1").await;
    assert!(retrieved_peer.is_some());
    
    let retrieved_peer = retrieved_peer.unwrap();
    assert_eq!(retrieved_peer.node_id, "peer1");
    assert_eq!(retrieved_peer.public_key, "test_public_key");
    assert_eq!(retrieved_peer.connection_status, ConnectionStatus::Connected);
    assert_eq!(retrieved_peer.latency_ms, Some(50));
    assert_eq!(retrieved_peer.trust_score, 0.8);
    assert!(retrieved_peer.is_validator);

    // Get all peers
    let all_peers = manager.get_all_peers().await;
    assert_eq!(all_peers.len(), 1);
    assert_eq!(all_peers[0].node_id, "peer1");

    // Remove peer
    assert!(manager.remove_peer("peer1").await.is_ok());

    // Check peer was removed
    let stats = manager.get_network_stats().await;
    assert_eq!(stats.total_peers, 0);
    assert_eq!(stats.connected_peers, 0);

    // Try to remove non-existent peer
    assert!(manager.remove_peer("non_existent").await.is_err());
}

#[tokio::test]
async fn test_message_sending() {
    let config = NetworkConfig::default();
    let manager = NetworkProtocolManager::new("test_node", config).await.unwrap();

    // Add a peer first
    let peer = Peer {
        node_id: "peer1".to_string(),
        public_key: "test_public_key".to_string(),
        address: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8081),
        capabilities: vec!["consensus".to_string()],
        version: "1.0.0".to_string(),
        last_seen: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        connection_status: ConnectionStatus::Connected,
        latency_ms: None,
        trust_score: 0.8,
        is_validator: false,
    };
    manager.add_peer(peer).await.unwrap();

    // Send ping message
    let ping = PingMessage {
        node_id: "test_node".to_string(),
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        signature: "test_signature".to_string(),
    };

    assert!(manager.send_message(
        "peer1",
        NetworkMessage::Ping(ping),
        MessagePriority::Low
    ).await.is_ok());

    // Check message was queued
    let stats = manager.get_network_stats().await;
    assert_eq!(stats.queued_messages, 1);
}

#[tokio::test]
async fn test_message_broadcasting() {
    let config = NetworkConfig::default();
    let manager = NetworkProtocolManager::new("test_node", config).await.unwrap();

    // Add multiple peers
    for i in 1..=3 {
        let peer = Peer {
            node_id: format!("peer{}", i),
            public_key: format!("public_key_{}", i),
            address: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080 + i),
            capabilities: vec!["consensus".to_string()],
            version: "1.0.0".to_string(),
            last_seen: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            connection_status: ConnectionStatus::Connected,
            latency_ms: None,
            trust_score: 0.8,
            is_validator: false,
        };
        manager.add_peer(peer).await.unwrap();
    }

    // Broadcast heartbeat message
    let heartbeat = HeartbeatMessage {
        node_id: "test_node".to_string(),
        status: "healthy".to_string(),
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        signature: "test_signature".to_string(),
    };

    assert!(manager.broadcast_message(
        NetworkMessage::Heartbeat(heartbeat),
        MessagePriority::Low
    ).await.is_ok());

    // Check messages were queued for all peers
    let stats = manager.get_network_stats().await;
    assert_eq!(stats.queued_messages, 3);
}

#[tokio::test]
async fn test_consensus_messages() {
    let config = NetworkConfig::default();
    let manager = NetworkProtocolManager::new("test_node", config).await.unwrap();

    // Add peer
    let peer = Peer {
        node_id: "validator1".to_string(),
        public_key: "validator_public_key".to_string(),
        address: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8081),
        capabilities: vec!["consensus".to_string()],
        version: "1.0.0".to_string(),
        last_seen: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        connection_status: ConnectionStatus::Connected,
        latency_ms: None,
        trust_score: 0.9,
        is_validator: true,
    };
    manager.add_peer(peer).await.unwrap();

    // Send block proposal
    let block_proposal = BlockProposal {
        block_hash: "block_hash_123".to_string(),
        proposer_id: "test_node".to_string(),
        round: 1,
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        signature: "proposal_signature".to_string(),
        data: vec![1, 2, 3, 4, 5],
    };

    assert!(manager.send_message(
        "validator1",
        NetworkMessage::BlockProposal(block_proposal),
        MessagePriority::Critical
    ).await.is_ok());

    // Send block vote
    let block_vote = BlockVote {
        block_hash: "block_hash_123".to_string(),
        voter_id: "test_node".to_string(),
        round: 1,
        is_approval: true,
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        signature: "vote_signature".to_string(),
    };

    assert!(manager.send_message(
        "validator1",
        NetworkMessage::BlockVote(block_vote),
        MessagePriority::Critical
    ).await.is_ok());

    // Send consensus round
    let consensus_round = ConsensusRound {
        round_number: 1,
        validator_set: vec!["validator1".to_string(), "validator2".to_string()],
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        signature: "consensus_signature".to_string(),
    };

    assert!(manager.broadcast_message(
        NetworkMessage::ConsensusRound(consensus_round),
        MessagePriority::Critical
    ).await.is_ok());
}

#[tokio::test]
async fn test_peer_discovery() {
    let config = NetworkConfig::default();
    let manager = NetworkProtocolManager::new("test_node", config).await.unwrap();

    // Send peer discovery
    assert!(manager.discover_peers().await.is_ok());

    // Check discovery message was broadcast
    let stats = manager.get_network_stats().await;
    assert_eq!(stats.queued_messages, 0); // No peers to broadcast to initially
}

#[tokio::test]
async fn test_ping_pong() {
    let config = NetworkConfig::default();
    let manager = NetworkProtocolManager::new("test_node", config).await.unwrap();

    // Add peer
    let peer = Peer {
        node_id: "peer1".to_string(),
        public_key: "test_public_key".to_string(),
        address: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8081),
        capabilities: vec!["consensus".to_string()],
        version: "1.0.0".to_string(),
        last_seen: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        connection_status: ConnectionStatus::Connected,
        latency_ms: None,
        trust_score: 0.8,
        is_validator: false,
    };
    manager.add_peer(peer).await.unwrap();

    // Send ping
    assert!(manager.send_ping("peer1").await.is_ok());

    // Check ping was sent
    let stats = manager.get_network_stats().await;
    assert_eq!(stats.queued_messages, 1);
}

#[tokio::test]
async fn test_heartbeat() {
    let config = NetworkConfig::default();
    let manager = NetworkProtocolManager::new("test_node", config).await.unwrap();

    // Add peers
    for i in 1..=2 {
        let peer = Peer {
            node_id: format!("peer{}", i),
            public_key: format!("public_key_{}", i),
            address: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080 + i),
            capabilities: vec!["consensus".to_string()],
            version: "1.0.0".to_string(),
            last_seen: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            connection_status: ConnectionStatus::Connected,
            latency_ms: None,
            trust_score: 0.8,
            is_validator: false,
        };
        manager.add_peer(peer).await.unwrap();
    }

    // Send heartbeat
    assert!(manager.send_heartbeat().await.is_ok());

    // Check heartbeat was broadcast
    let stats = manager.get_network_stats().await;
    assert_eq!(stats.queued_messages, 2);
}

#[tokio::test]
async fn test_message_handlers() {
    let config = NetworkConfig::default();
    let manager = NetworkProtocolManager::new("test_node", config).await.unwrap();

    // Register message handler
    let mut received_messages = Vec::new();
    let received_messages_clone = Arc::new(RwLock::new(Vec::new()));
    
    let handler = {
        let received_messages = received_messages_clone.clone();
        move |message: NetworkMessage| {
            let received_messages = received_messages.clone();
            tokio::spawn(async move {
                let mut messages = received_messages.write().await;
                messages.push(message);
            });
        }
    };

    manager.register_message_handler("ping", Box::new(handler)).await;

    // Send ping message
    let ping = PingMessage {
        node_id: "test_node".to_string(),
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        signature: "test_signature".to_string(),
    };

    // Note: In a real test, we would need to actually process the message
    // This is a simplified test to verify handler registration
    assert!(manager.send_message(
        "peer1",
        NetworkMessage::Ping(ping),
        MessagePriority::Low
    ).await.is_ok());
}

#[tokio::test]
async fn test_network_config() {
    let config = NetworkConfig {
        listen_port: 9090,
        max_peers: 100,
        message_timeout_ms: 60000,
        heartbeat_interval_ms: 45000,
        discovery_interval_ms: 90000,
        max_message_size_bytes: 2_000_000,
        enable_encryption: true,
        trust_threshold: 0.8,
    };

    let manager = NetworkProtocolManager::new("test_node", config).await.unwrap();
    let stats = manager.get_network_stats().await;

    assert_eq!(stats.max_peers, 100);
    assert_eq!(stats.message_timeout_ms, 60000);
}

#[tokio::test]
async fn test_peer_connection_status() {
    let config = NetworkConfig::default();
    let manager = NetworkProtocolManager::new("test_node", config).await.unwrap();

    // Add peers with different connection statuses
    let connected_peer = Peer {
        node_id: "connected_peer".to_string(),
        public_key: "public_key_1".to_string(),
        address: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8081),
        capabilities: vec!["consensus".to_string()],
        version: "1.0.0".to_string(),
        last_seen: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        connection_status: ConnectionStatus::Connected,
        latency_ms: Some(50),
        trust_score: 0.9,
        is_validator: true,
    };

    let disconnected_peer = Peer {
        node_id: "disconnected_peer".to_string(),
        public_key: "public_key_2".to_string(),
        address: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8082),
        capabilities: vec!["consensus".to_string()],
        version: "1.0.0".to_string(),
        last_seen: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        connection_status: ConnectionStatus::Disconnected,
        latency_ms: None,
        trust_score: 0.5,
        is_validator: false,
    };

    manager.add_peer(connected_peer).await.unwrap();
    manager.add_peer(disconnected_peer).await.unwrap();

    let stats = manager.get_network_stats().await;
    assert_eq!(stats.total_peers, 2);
    assert_eq!(stats.connected_peers, 1);
}

#[tokio::test]
async fn test_message_priority() {
    let config = NetworkConfig::default();
    let manager = NetworkProtocolManager::new("test_node", config).await.unwrap();

    // Add peer
    let peer = Peer {
        node_id: "peer1".to_string(),
        public_key: "test_public_key".to_string(),
        address: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8081),
        capabilities: vec!["consensus".to_string()],
        version: "1.0.0".to_string(),
        last_seen: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        connection_status: ConnectionStatus::Connected,
        latency_ms: None,
        trust_score: 0.8,
        is_validator: false,
    };
    manager.add_peer(peer).await.unwrap();

    // Send messages with different priorities
    let ping = PingMessage {
        node_id: "test_node".to_string(),
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        signature: "test_signature".to_string(),
    };

    assert!(manager.send_message(
        "peer1",
        NetworkMessage::Ping(ping),
        MessagePriority::Low
    ).await.is_ok());

    let block_proposal = BlockProposal {
        block_hash: "block_hash_123".to_string(),
        proposer_id: "test_node".to_string(),
        round: 1,
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        signature: "proposal_signature".to_string(),
        data: vec![1, 2, 3, 4, 5],
    };

    assert!(manager.send_message(
        "peer1",
        NetworkMessage::BlockProposal(block_proposal),
        MessagePriority::Critical
    ).await.is_ok());

    // Check messages were queued
    let stats = manager.get_network_stats().await;
    assert_eq!(stats.queued_messages, 2);
}

#[tokio::test]
async fn test_network_cleanup() {
    let config = NetworkConfig::default();
    let manager = NetworkProtocolManager::new("test_node", config).await.unwrap();

    // Add peer
    let peer = Peer {
        node_id: "peer1".to_string(),
        public_key: "test_public_key".to_string(),
        address: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8081),
        capabilities: vec!["consensus".to_string()],
        version: "1.0.0".to_string(),
        last_seen: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        connection_status: ConnectionStatus::Connected,
        latency_ms: None,
        trust_score: 0.8,
        is_validator: false,
    };
    manager.add_peer(peer).await.unwrap();

    // Send some messages
    let ping = PingMessage {
        node_id: "test_node".to_string(),
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        signature: "test_signature".to_string(),
    };

    manager.send_message(
        "peer1",
        NetworkMessage::Ping(ping),
        MessagePriority::Low
    ).await.unwrap();

    // Check initial state
    let stats = manager.get_network_stats().await;
    assert_eq!(stats.total_peers, 1);
    assert_eq!(stats.queued_messages, 1);

    // Clean up old data
    manager.cleanup_old_data(1).await; // 1 second max age

    // Check state after cleanup (should still have recent data)
    let stats = manager.get_network_stats().await;
    assert_eq!(stats.total_peers, 1);
    assert_eq!(stats.queued_messages, 1);
}

#[tokio::test]
async fn test_network_serialization() {
    // Test message serialization
    let ping = PingMessage {
        node_id: "test_node".to_string(),
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        signature: "test_signature".to_string(),
    };

    let network_message = NetworkMessage::Ping(ping);

    // Test serialization
    let serialized = serde_json::to_string(&network_message).unwrap();
    let deserialized: NetworkMessage = serde_json::from_str(&serialized).unwrap();

    // Verify deserialized message is correct
    match deserialized {
        NetworkMessage::Ping(ping_msg) => {
            assert_eq!(ping_msg.node_id, "test_node");
            assert!(!ping_msg.signature.is_empty());
        }
        _ => panic!("Expected Ping message"),
    }
} 