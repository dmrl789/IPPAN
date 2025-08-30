use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::sleep;
use tracing::{info, warn};
use serde::{Deserialize, Serialize};

/// Represents a simplified blockchain node
#[derive(Clone)]
pub struct SimpleNode {
    pub id: String,
    pub http_port: u16,
    pub p2p_port: u16,
    pub peers: Arc<RwLock<Vec<PeerConnection>>>,
    pub message_sender: mpsc::UnboundedSender<NodeMessage>,
    pub message_receiver: Arc<RwLock<mpsc::UnboundedReceiver<NodeMessage>>>,
}

/// Represents a connection to another peer
#[derive(Clone, Debug)]
pub struct PeerConnection {
    pub peer_id: String,
    pub peer_address: String,
    pub is_connected: bool,
    pub message_sender: Option<mpsc::UnboundedSender<NodeMessage>>,
}

/// Messages that nodes can exchange
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeMessage {
    Hello { from: String, version: String },
    Transaction { from: String, data: String },
    Block { from: String, block_num: u64, data: String },
    Ping { from: String },
    Pong { from: String },
}

impl SimpleNode {
    /// Create a new node
    pub fn new(id: String, http_port: u16, p2p_port: u16) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        
        Self {
            id,
            http_port,
            p2p_port,
            peers: Arc::new(RwLock::new(Vec::new())),
            message_sender: tx,
            message_receiver: Arc::new(RwLock::new(rx)),
        }
    }

    /// Start the node
    pub async fn start(&self) {
        info!("🚀 Starting node {} on ports HTTP:{} P2P:{}", 
              self.id, self.http_port, self.p2p_port);
        
        // Start message handler
        self.start_message_handler().await;
        
        info!("✅ Node {} is running", self.id);
    }

    /// Connect to another node
    pub async fn connect_to_peer(&self, peer_node: &SimpleNode) {
        info!("🔗 Node {} connecting to node {}", self.id, peer_node.id);
        
        // Create bidirectional connection
        let peer_conn = PeerConnection {
            peer_id: peer_node.id.clone(),
            peer_address: format!("127.0.0.1:{}", peer_node.p2p_port),
            is_connected: true,
            message_sender: Some(peer_node.message_sender.clone()),
        };
        
        // Add peer to our list
        self.peers.write().await.push(peer_conn);
        
        // Add ourselves to peer's list
        let our_conn = PeerConnection {
            peer_id: self.id.clone(),
            peer_address: format!("127.0.0.1:{}", self.p2p_port),
            is_connected: true,
            message_sender: Some(self.message_sender.clone()),
        };
        
        peer_node.peers.write().await.push(our_conn);
        
        // Send hello message
        self.send_message_to_peer(
            &peer_node.id,
            NodeMessage::Hello {
                from: self.id.clone(),
                version: "1.0.0".to_string(),
            }
        ).await;
        
        info!("✅ Connection established between {} and {}", self.id, peer_node.id);
    }

    /// Send a message to a specific peer
    pub async fn send_message_to_peer(&self, peer_id: &str, message: NodeMessage) {
        let peers = self.peers.read().await;
        
        for peer in peers.iter() {
            if peer.peer_id == peer_id {
                if let Some(sender) = &peer.message_sender {
                    if let Err(e) = sender.send(message.clone()) {
                        warn!("Failed to send message to {}: {}", peer_id, e);
                    } else {
                        info!("📤 {} sent {:?} to {}", self.id, message, peer_id);
                    }
                    return;
                }
            }
        }
        
        warn!("Peer {} not found", peer_id);
    }

    /// Broadcast a message to all connected peers
    pub async fn broadcast_message(&self, message: NodeMessage) {
        let peers = self.peers.read().await;
        
        info!("📢 {} broadcasting {:?} to {} peers", 
              self.id, message, peers.len());
        
        for peer in peers.iter() {
            if let Some(sender) = &peer.message_sender {
                if let Err(e) = sender.send(message.clone()) {
                    warn!("Failed to broadcast to {}: {}", peer.peer_id, e);
                }
            }
        }
    }

    /// Start the message handler loop
    async fn start_message_handler(&self) {
        let id = self.id.clone();
        let peers = self.peers.clone();
        let receiver = self.message_receiver.clone();
        
        tokio::spawn(async move {
            let mut rx = receiver.write().await;
            
            while let Some(message) = rx.recv().await {
                match message {
                    NodeMessage::Hello { from, version } => {
                        info!("👋 {} received Hello from {} (version: {})", 
                              id, from, version);
                        
                        // Send Pong response
                        let peers_read = peers.read().await;
                        for peer in peers_read.iter() {
                            if peer.peer_id == from {
                                if let Some(sender) = &peer.message_sender {
                                    let _ = sender.send(NodeMessage::Pong { 
                                        from: id.clone() 
                                    });
                                }
                                break;
                            }
                        }
                    }
                    NodeMessage::Transaction { from, data } => {
                        info!("💰 {} received transaction from {}: {}", 
                              id, from, data);
                    }
                    NodeMessage::Block { from, block_num, data } => {
                        info!("📦 {} received block #{} from {}: {}", 
                              id, block_num, from, data);
                    }
                    NodeMessage::Ping { from } => {
                        info!("🏓 {} received Ping from {}", id, from);
                        
                        // Send Pong response
                        let peers_read = peers.read().await;
                        for peer in peers_read.iter() {
                            if peer.peer_id == from {
                                if let Some(sender) = &peer.message_sender {
                                    let _ = sender.send(NodeMessage::Pong { 
                                        from: id.clone() 
                                    });
                                }
                                break;
                            }
                        }
                    }
                    NodeMessage::Pong { from } => {
                        info!("🏓 {} received Pong from {}", id, from);
                    }
                }
            }
        });
    }

    /// Get the list of connected peers
    pub async fn get_peers(&self) -> Vec<String> {
        self.peers.read().await
            .iter()
            .filter(|p| p.is_connected)
            .map(|p| p.peer_id.clone())
            .collect()
    }

    /// Get peer count
    pub async fn peer_count(&self) -> usize {
        self.peers.read().await
            .iter()
            .filter(|p| p.is_connected)
            .count()
    }
}

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("=== IPPAN Two Nodes Connection Demo ===\n");
    
    // Create Node 1
    let node1 = SimpleNode::new(
        "node-1".to_string(),
        8080,  // HTTP port
        9001,  // P2P port
    );
    
    // Create Node 2
    let node2 = SimpleNode::new(
        "node-2".to_string(),
        8081,  // HTTP port
        9002,  // P2P port
    );
    
    // Start both nodes
    node1.start().await;
    node2.start().await;
    
    println!("\n📍 Node 1 running on HTTP:8080, P2P:9001");
    println!("📍 Node 2 running on HTTP:8081, P2P:9002\n");
    
    // Give nodes time to initialize
    sleep(Duration::from_millis(100)).await;
    
    // Connect the two nodes
    println!("🔗 Connecting nodes...\n");
    node1.connect_to_peer(&node2).await;
    
    // Give time for handshake
    sleep(Duration::from_millis(100)).await;
    
    // Check peer connections
    let node1_peers = node1.get_peers().await;
    let node2_peers = node2.get_peers().await;
    
    println!("\n📊 Connection Status:");
    println!("   Node 1 peers: {:?} (count: {})", node1_peers, node1.peer_count().await);
    println!("   Node 2 peers: {:?} (count: {})", node2_peers, node2.peer_count().await);
    
    println!("\n🔄 Exchanging messages...\n");
    
    // Node 1 sends a transaction to Node 2
    node1.send_message_to_peer(
        "node-2",
        NodeMessage::Transaction {
            from: "node-1".to_string(),
            data: "Alice sends 100 IPPAN to Bob".to_string(),
        }
    ).await;
    
    sleep(Duration::from_millis(100)).await;
    
    // Node 2 sends a block to Node 1
    node2.send_message_to_peer(
        "node-1",
        NodeMessage::Block {
            from: "node-2".to_string(),
            block_num: 42,
            data: "Block containing 5 transactions".to_string(),
        }
    ).await;
    
    sleep(Duration::from_millis(100)).await;
    
    // Node 1 broadcasts a message to all peers (which is just Node 2)
    println!("\n📢 Broadcasting message from Node 1...\n");
    node1.broadcast_message(
        NodeMessage::Transaction {
            from: "node-1".to_string(),
            data: "Broadcast: New validator joined".to_string(),
        }
    ).await;
    
    sleep(Duration::from_millis(100)).await;
    
    // Ping-Pong test
    println!("\n🏓 Testing Ping-Pong...\n");
    node1.send_message_to_peer(
        "node-2",
        NodeMessage::Ping {
            from: "node-1".to_string(),
        }
    ).await;
    
    sleep(Duration::from_millis(100)).await;
    
    // Keep the demo running for a bit to see all messages
    sleep(Duration::from_secs(1)).await;
    
    println!("\n=== Demo Summary ===");
    println!("✅ Successfully created two nodes");
    println!("✅ Established P2P connection between nodes");
    println!("✅ Exchanged various message types");
    println!("✅ Demonstrated direct messaging and broadcasting");
    println!("\n💡 In a real IPPAN network:");
    println!("   - Nodes would use libp2p for actual network communication");
    println!("   - Messages would be cryptographically signed");
    println!("   - Nodes would discover peers automatically via mDNS/Kademlia");
    println!("   - Consensus would be achieved through Byzantine Fault Tolerant algorithms");
}