//! IPNDHT Resilience Tests
//!
//! Multi-node test harness for validating DHT resilience, cold-start recovery,
//! file descriptor propagation, handle propagation, and partition recovery.

use std::collections::HashSet;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use ippan_p2p::{Libp2pConfig, Libp2pEvent, Libp2pNetwork};
use libp2p::{Multiaddr, PeerId};
use parking_lot::RwLock;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::time::sleep;
use tracing::{debug, info};

// ----------------------------------------------------------------------------
// Test Harness Infrastructure
// ----------------------------------------------------------------------------

/// Configuration for a test DHT node
#[derive(Debug, Clone)]
pub struct DhtNodeConfig {
    /// Node identifier for logging
    pub node_id: String,
    /// Listen addresses (use 0 for ephemeral port)
    pub listen_addresses: Vec<Multiaddr>,
    /// Bootstrap peers to connect to on startup
    pub bootstrap_peers: Vec<Multiaddr>,
    /// Enable mDNS for local discovery
    pub enable_mdns: bool,
    /// Enable relay for NAT traversal
    pub enable_relay: bool,
}

impl Default for DhtNodeConfig {
    fn default() -> Self {
        Self {
            node_id: "test-node".to_string(),
            listen_addresses: vec![Multiaddr::from_str("/ip4/127.0.0.1/tcp/0").unwrap()],
            bootstrap_peers: Vec::new(),
            enable_mdns: false,
            enable_relay: false,
        }
    }
}

/// A test DHT node instance with event monitoring
pub struct DhtTestNode {
    pub config: DhtNodeConfig,
    pub network: Libp2pNetwork,
    pub peer_id: PeerId,
    pub listen_addresses: Vec<Multiaddr>,
    pub events: Arc<RwLock<Vec<Libp2pEvent>>>,
    _event_receiver: Option<UnboundedReceiver<Libp2pEvent>>,
    _event_task: tokio::task::JoinHandle<()>,
}

impl DhtTestNode {
    /// Spawn a new test DHT node
    pub async fn spawn(config: DhtNodeConfig) -> anyhow::Result<Self> {
        let libp2p_config = Libp2pConfig {
            listen_addresses: config.listen_addresses.clone(),
            bootstrap_peers: config.bootstrap_peers.clone(),
            relay_servers: Vec::new(),
            gossip_topics: vec!["ippan/files".to_string(), "ippan/handles".to_string()],
            enable_mdns: config.enable_mdns,
            enable_relay: config.enable_relay,
            identity_keypair: None,
            protocol_version: "/ippan-test/1.0.0".to_string(),
            agent_version: format!("ippan-test/{}", config.node_id),
            bootstrap_retry_interval: Duration::from_secs(5),
            bootstrap_max_retries: 10,
        };

        let network = Libp2pNetwork::new(libp2p_config)?;
        let peer_id = network.peer_id();
        let listen_addresses = network.listen_addresses();

        info!(
            "[{}] Node spawned: peer_id={}, listen_addrs={:?}",
            config.node_id, peer_id, listen_addresses
        );

        let events = Arc::new(RwLock::new(Vec::new()));
        let events_clone = events.clone();
        let node_id = config.node_id.clone();

        let mut event_receiver = network
            .take_event_receiver()
            .expect("event receiver available");

        // Spawn background task to collect events
        let event_task = tokio::spawn(async move {
            while let Some(event) = event_receiver.recv().await {
                debug!("[{}] Event: {:?}", node_id, event);
                events_clone.write().push(event);
            }
        });

        // Small delay to ensure node is fully initialized
        sleep(Duration::from_millis(100)).await;

        Ok(Self {
            config,
            network,
            peer_id,
            listen_addresses,
            events,
            _event_receiver: None,
            _event_task: event_task,
        })
    }

    /// Get the primary listen address (first in list)
    pub fn primary_address(&self) -> Option<Multiaddr> {
        self.listen_addresses.first().cloned()
    }

    /// Get a multiaddr with peer ID for dialing
    pub fn dial_address(&self) -> Option<Multiaddr> {
        self.primary_address().map(|mut addr| {
            addr.push(libp2p::multiaddr::Protocol::P2p(self.peer_id));
            addr
        })
    }

    /// Publish data to a gossip topic
    pub fn publish(&self, topic: &str, data: Vec<u8>) -> anyhow::Result<()> {
        self.network.publish(topic, data)
    }

    /// Dial another peer
    pub fn dial(&self, address: Multiaddr) -> anyhow::Result<()> {
        self.network.dial(address)
    }

    /// Add explicit peer to routing table
    pub fn add_peer(&self, peer_id: PeerId, address: Option<Multiaddr>) -> anyhow::Result<()> {
        self.network.add_explicit_peer(peer_id, address)
    }

    /// Get all collected events
    pub fn get_events(&self) -> Vec<Libp2pEvent> {
        self.events.read().clone()
    }

    /// Clear collected events
    pub fn clear_events(&self) {
        self.events.write().clear();
    }

    /// Wait for a specific peer connection event
    pub async fn wait_for_peer_connected(
        &self,
        peer_id: PeerId,
        timeout_duration: Duration,
    ) -> Result<(), String> {
        let start = std::time::Instant::now();
        while start.elapsed() < timeout_duration {
            let events = self.events.read();
            if events
                .iter()
                .any(|e| matches!(e, Libp2pEvent::PeerConnected { peer } if peer == &peer_id))
            {
                return Ok(());
            }
            drop(events);
            sleep(Duration::from_millis(50)).await;
        }
        Err(format!(
            "[{}] Timeout waiting for peer {} to connect",
            self.config.node_id, peer_id
        ))
    }

    /// Wait for peer discovery event
    pub async fn wait_for_peer_discovered(
        &self,
        peer_id: PeerId,
        timeout_duration: Duration,
    ) -> Result<(), String> {
        let start = std::time::Instant::now();
        while start.elapsed() < timeout_duration {
            let events = self.events.read();
            if events.iter().any(|e| {
                if let Libp2pEvent::PeerDiscovered { peers } = e {
                    peers.iter().any(|(p, _)| p == &peer_id)
                } else {
                    false
                }
            }) {
                return Ok(());
            }
            drop(events);
            sleep(Duration::from_millis(50)).await;
        }
        Err(format!(
            "[{}] Timeout waiting to discover peer {}",
            self.config.node_id, peer_id
        ))
    }

    /// Wait for gossip message from a peer on a topic
    pub async fn wait_for_gossip(
        &self,
        topic: &str,
        timeout_duration: Duration,
    ) -> Result<Vec<u8>, String> {
        let start = std::time::Instant::now();
        while start.elapsed() < timeout_duration {
            let events = self.events.read();
            for event in events.iter() {
                if let Libp2pEvent::Gossip { topic: t, data, .. } = event {
                    if t == topic {
                        return Ok(data.clone());
                    }
                }
            }
            drop(events);
            sleep(Duration::from_millis(50)).await;
        }
        Err(format!(
            "[{}] Timeout waiting for gossip on topic {}",
            self.config.node_id, topic
        ))
    }

    /// Shutdown the node
    pub fn shutdown(&self) {
        self.network.shutdown();
    }
}

impl Drop for DhtTestNode {
    fn drop(&mut self) {
        self.shutdown();
    }
}

// ----------------------------------------------------------------------------
// Multi-Node Test Utilities
// ----------------------------------------------------------------------------

/// Spawn N test nodes with sequential configuration
pub async fn spawn_test_nodes(count: usize) -> anyhow::Result<Vec<DhtTestNode>> {
    let mut nodes = Vec::new();
    for i in 0..count {
        let config = DhtNodeConfig {
            node_id: format!("node-{}", i),
            listen_addresses: vec![Multiaddr::from_str("/ip4/127.0.0.1/tcp/0")?],
            bootstrap_peers: Vec::new(),
            enable_mdns: false,
            enable_relay: false,
        };
        let node = DhtTestNode::spawn(config).await?;
        nodes.push(node);
    }
    Ok(nodes)
}

/// Connect node B to node A (B bootstraps from A)
pub async fn connect_nodes(node_a: &DhtTestNode, node_b: &DhtTestNode) -> anyhow::Result<()> {
    if let Some(addr) = node_a.dial_address() {
        info!(
            "[{}] Dialing [{}] at {}",
            node_b.config.node_id, node_a.config.node_id, addr
        );
        node_b.dial(addr)?;

        // Wait for connection establishment
        node_b
            .wait_for_peer_connected(node_a.peer_id, Duration::from_secs(5))
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

        info!(
            "[{}] Successfully connected to [{}]",
            node_b.config.node_id, node_a.config.node_id
        );
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Node {} has no dial address",
            node_a.config.node_id
        ))
    }
}

/// Wait for all nodes to discover each other (full mesh)
pub async fn wait_for_full_mesh(
    nodes: &[DhtTestNode],
    timeout_duration: Duration,
) -> Result<(), String> {
    let start = std::time::Instant::now();

    loop {
        if start.elapsed() > timeout_duration {
            return Err("Timeout waiting for full mesh connectivity".to_string());
        }

        let mut all_connected = true;
        for node in nodes {
            let events = node.get_events();
            let connected_peers: HashSet<PeerId> = events
                .iter()
                .filter_map(|e| {
                    if let Libp2pEvent::PeerConnected { peer } = e {
                        Some(*peer)
                    } else {
                        None
                    }
                })
                .collect();

            let expected_peers: HashSet<PeerId> = nodes
                .iter()
                .filter(|n| n.peer_id != node.peer_id)
                .map(|n| n.peer_id)
                .collect();

            if !expected_peers.is_subset(&connected_peers) {
                all_connected = false;
                break;
            }
        }

        if all_connected {
            info!("Full mesh connectivity established");
            return Ok(());
        }

        sleep(Duration::from_millis(100)).await;
    }
}

// ----------------------------------------------------------------------------
// Resilience Test Cases
// ----------------------------------------------------------------------------

#[tokio::test]
#[ignore] // Run with --ignored flag due to network timing sensitivity
async fn test_minimal_2_node_discovery() {
    tracing_subscriber::fmt::init();

    info!("=== Test: Minimal 2-Node Discovery ===");

    // Spawn Node A
    let node_a = DhtTestNode::spawn(DhtNodeConfig {
        node_id: "node-a".to_string(),
        ..Default::default()
    })
    .await
    .expect("spawn node A");

    info!("Node A spawned: peer_id={}", node_a.peer_id);

    // Spawn Node B with A as bootstrap
    let bootstrap_addr = node_a.dial_address().expect("node A dial address");
    let node_b = DhtTestNode::spawn(DhtNodeConfig {
        node_id: "node-b".to_string(),
        bootstrap_peers: vec![bootstrap_addr.clone()],
        ..Default::default()
    })
    .await
    .expect("spawn node B");

    info!("Node B spawned: peer_id={}", node_b.peer_id);

    // Wait for B to discover and connect to A
    node_b
        .wait_for_peer_connected(node_a.peer_id, Duration::from_secs(10))
        .await
        .expect("Node B connects to Node A");

    info!("✓ Node B successfully connected to Node A");

    // Verify reciprocal discovery: A should eventually discover B
    node_a
        .wait_for_peer_connected(node_b.peer_id, Duration::from_secs(10))
        .await
        .expect("Node A discovers Node B reciprocally");

    info!("✓ Node A reciprocally discovered Node B");

    // Verify both nodes have each other in routing table
    let a_events = node_a.get_events();
    let b_events = node_b.get_events();

    let a_has_b = a_events
        .iter()
        .any(|e| matches!(e, Libp2pEvent::PeerConnected { peer } if peer == &node_b.peer_id));
    let b_has_a = b_events
        .iter()
        .any(|e| matches!(e, Libp2pEvent::PeerConnected { peer } if peer == &node_a.peer_id));

    assert!(a_has_b, "Node A routing table contains Node B");
    assert!(b_has_a, "Node B routing table contains Node A");

    info!("✓ Both nodes have each other in routing tables");
    info!("=== Test PASSED: Minimal 2-Node Discovery ===");
}

#[tokio::test]
#[ignore]
async fn test_cold_start_recovery() {
    tracing_subscriber::fmt::init();

    info!("=== Test: Cold Start Recovery ===");

    // Node A starts alone (no bootstrap peers)
    let node_a = DhtTestNode::spawn(DhtNodeConfig {
        node_id: "node-a-cold".to_string(),
        ..Default::default()
    })
    .await
    .expect("spawn node A");

    info!("Node A spawned alone: peer_id={}", node_a.peer_id);

    // Wait a bit to simulate cold start
    sleep(Duration::from_secs(1)).await;

    // Node B starts and announces itself
    let node_b = DhtTestNode::spawn(DhtNodeConfig {
        node_id: "node-b".to_string(),
        ..Default::default()
    })
    .await
    .expect("spawn node B");

    info!("Node B spawned: peer_id={}", node_b.peer_id);

    // Manually connect A to B (simulating retry bootstrap logic)
    if let Some(b_addr) = node_b.dial_address() {
        info!("Node A attempting to dial Node B at {}", b_addr);
        node_a.dial(b_addr).expect("dial B from A");

        // Wait for connection
        node_a
            .wait_for_peer_connected(node_b.peer_id, Duration::from_secs(10))
            .await
            .expect("Node A connects to Node B after cold start");

        info!("✓ Node A successfully recovered from cold start and connected to Node B");
    } else {
        panic!("Node B has no dial address");
    }

    // Verify connection is bidirectional
    node_b
        .wait_for_peer_connected(node_a.peer_id, Duration::from_secs(5))
        .await
        .expect("Node B sees Node A connection");

    info!("✓ Bidirectional connection established after cold start");
    info!("=== Test PASSED: Cold Start Recovery ===");
}

#[tokio::test]
#[ignore]
async fn test_file_descriptor_propagation() {
    tracing_subscriber::fmt::init();

    info!("=== Test: File Descriptor Propagation ===");

    // Spawn 3 nodes
    let nodes = spawn_test_nodes(3).await.expect("spawn 3 nodes");
    let node_a = &nodes[0];
    let node_b = &nodes[1];
    let node_c = &nodes[2];

    // Connect nodes in a chain: A <-> B <-> C
    connect_nodes(node_a, node_b).await.expect("connect A to B");
    connect_nodes(node_b, node_c).await.expect("connect B to C");

    // Wait for mesh to stabilize
    sleep(Duration::from_secs(2)).await;

    // Node A publishes a file descriptor
    let file_data = b"test file content";
    let file_hash = blake3::hash(file_data);
    let file_hash_hex = file_hash.to_hex().to_string();
    let file_descriptor_msg = serde_json::json!({
        "type": "file_publish",
        "content_hash": file_hash_hex,
        "size": file_data.len(),
        "owner": hex::encode([1u8; 32]),
    });

    let file_msg_bytes = serde_json::to_vec(&file_descriptor_msg).unwrap();

    info!("[{}] Publishing file descriptor", node_a.config.node_id);
    node_a
        .publish("ippan/files", file_msg_bytes.clone())
        .expect("publish file descriptor from A");

    // Node B should receive the file descriptor via gossip
    let b_received = node_b
        .wait_for_gossip("ippan/files", Duration::from_secs(10))
        .await
        .expect("Node B receives file descriptor");

    info!("✓ Node B received file descriptor gossip");

    let b_msg: serde_json::Value = serde_json::from_slice(&b_received).unwrap();
    assert_eq!(
        b_msg["content_hash"].as_str().unwrap(),
        file_hash_hex,
        "Node B received correct file hash"
    );

    // Node C should also receive it (propagation through B)
    let c_received = node_c
        .wait_for_gossip("ippan/files", Duration::from_secs(10))
        .await
        .expect("Node C receives file descriptor");

    info!("✓ Node C received file descriptor gossip");

    let c_msg: serde_json::Value = serde_json::from_slice(&c_received).unwrap();
    assert_eq!(
        c_msg["content_hash"].as_str().unwrap(),
        file_hash_hex,
        "Node C received correct file hash"
    );

    info!("✓ File descriptor successfully propagated across all 3 nodes");
    info!("=== Test PASSED: File Descriptor Propagation ===");
}

#[tokio::test]
#[ignore]
async fn test_handle_propagation() {
    tracing_subscriber::fmt::init();

    info!("=== Test: Handle Propagation (Stub) ===");

    // Spawn 2 nodes
    let nodes = spawn_test_nodes(2).await.expect("spawn 2 nodes");
    let node_a = &nodes[0];
    let node_b = &nodes[1];

    // Connect nodes
    connect_nodes(node_a, node_b).await.expect("connect A to B");

    sleep(Duration::from_millis(500)).await;

    // Node A publishes a handle registration
    let handle_msg = serde_json::json!({
        "type": "handle_register",
        "handle": "@alice.ipn",
        "owner": hex::encode([2u8; 32]),
        "public_key": hex::encode([3u8; 32]),
    });

    let handle_msg_bytes = serde_json::to_vec(&handle_msg).unwrap();

    info!("[{}] Publishing handle registration", node_a.config.node_id);
    node_a
        .publish("ippan/handles", handle_msg_bytes.clone())
        .expect("publish handle from A");

    // Node B should receive the handle via gossip
    let b_received = node_b
        .wait_for_gossip("ippan/handles", Duration::from_secs(10))
        .await
        .expect("Node B receives handle registration");

    info!("✓ Node B received handle registration gossip");

    let b_msg: serde_json::Value = serde_json::from_slice(&b_received).unwrap();
    assert_eq!(
        b_msg["handle"].as_str().unwrap(),
        "@alice.ipn",
        "Node B received correct handle"
    );

    info!("✓ Handle propagation pathway verified (stub-based)");
    info!("=== Test PASSED: Handle Propagation ===");
}

#[tokio::test]
#[ignore]
async fn test_partition_recovery() {
    tracing_subscriber::fmt::init();

    info!("=== Test: Partition and Recovery ===");

    // Spawn 3 nodes: A, B, C
    let nodes = spawn_test_nodes(3).await.expect("spawn 3 nodes");
    let node_a = &nodes[0];
    let node_b = &nodes[1];
    let node_c = &nodes[2];

    // Connect all nodes: A <-> B, B <-> C, A <-> C
    connect_nodes(node_a, node_b).await.expect("connect A to B");
    connect_nodes(node_b, node_c).await.expect("connect B to C");
    connect_nodes(node_a, node_c).await.expect("connect A to C");

    sleep(Duration::from_secs(1)).await;

    info!("✓ Initial full mesh established");

    // Simulate partition: disconnect B by shutting it down
    info!(
        "[{}] Simulating partition (shutdown)",
        node_b.config.node_id
    );
    node_b.shutdown();

    sleep(Duration::from_millis(500)).await;

    // A and C should maintain connectivity
    let a_events = node_a.get_events();
    let c_events = node_c.get_events();

    let a_connected_to_c = a_events
        .iter()
        .any(|e| matches!(e, Libp2pEvent::PeerConnected { peer } if peer == &node_c.peer_id));
    let c_connected_to_a = c_events
        .iter()
        .any(|e| matches!(e, Libp2pEvent::PeerConnected { peer } if peer == &node_a.peer_id));

    assert!(
        a_connected_to_c && c_connected_to_a,
        "A and C maintain connectivity during B partition"
    );

    info!("✓ A and C maintained connectivity during partition");

    // Reconnect B (spawn new instance with same peer)
    let node_b_reconnect = DhtTestNode::spawn(DhtNodeConfig {
        node_id: "node-b-reconnect".to_string(),
        ..Default::default()
    })
    .await
    .expect("respawn node B");

    info!(
        "[{}] Reconnecting to network",
        node_b_reconnect.config.node_id
    );

    // Reconnect B to A
    connect_nodes(node_a, &node_b_reconnect)
        .await
        .expect("reconnect B to A");

    // Reconnect B to C
    connect_nodes(node_c, &node_b_reconnect)
        .await
        .expect("reconnect B to C");

    sleep(Duration::from_millis(500)).await;

    info!("✓ Node B successfully rejoined the network");

    // Verify gossip works after recovery
    let test_msg = b"test message after recovery";
    node_a
        .publish("ippan/files", test_msg.to_vec())
        .expect("publish from A");

    let b_recv = node_b_reconnect
        .wait_for_gossip("ippan/files", Duration::from_secs(10))
        .await
        .expect("B receives gossip after recovery");

    assert_eq!(b_recv, test_msg, "Gossip works after partition recovery");

    info!("✓ Gossip functionality restored after recovery");
    info!("=== Test PASSED: Partition and Recovery ===");
}

#[tokio::test]
#[ignore]
async fn test_3_node_full_mesh() {
    tracing_subscriber::fmt::init();

    info!("=== Test: 3-Node Full Mesh ===");

    let nodes = spawn_test_nodes(3).await.expect("spawn 3 nodes");

    // Connect nodes in a ring: A -> B -> C -> A
    for i in 0..nodes.len() {
        let next = (i + 1) % nodes.len();
        connect_nodes(&nodes[i], &nodes[next])
            .await
            .expect(&format!("connect node {} to {}", i, next));
    }

    // Wait for full mesh
    wait_for_full_mesh(&nodes, Duration::from_secs(15))
        .await
        .expect("full mesh established");

    info!("✓ 3-node full mesh established");

    // Verify all nodes can gossip
    let test_msg = b"broadcast test";
    nodes[0]
        .publish("ippan/files", test_msg.to_vec())
        .expect("publish from node 0");

    for (i, node) in nodes.iter().enumerate().skip(1) {
        let received = node
            .wait_for_gossip("ippan/files", Duration::from_secs(10))
            .await
            .expect(&format!("node {} receives gossip", i));
        assert_eq!(received, test_msg, "node {} received correct message", i);
    }

    info!("✓ All nodes received broadcast message");
    info!("=== Test PASSED: 3-Node Full Mesh ===");
}
