use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::info;

#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub peer_id: String,
    pub addresses: Vec<String>,
    pub last_seen: Instant,
    pub is_connected: bool,
    pub capabilities: Vec<String>, // e.g., ["gossip", "kad", "mdns"]
}

#[derive(Debug, Clone)]
pub struct DiscoveryStats {
    pub total_peers: usize,
    pub connected_peers: usize,
    pub mdns_peers: usize,
    pub kad_peers: usize,
    pub bootstrap_peers: usize,
    pub node_id: String,
}

pub struct PeerDiscoveryDemo {
    peers: Arc<RwLock<HashMap<String, PeerInfo>>>,
    node_id: String,
    discovery_interval: Duration,
}

impl PeerDiscoveryDemo {
    pub fn new(node_id: String) -> Self {
        Self {
            peers: Arc::new(RwLock::new(HashMap::new())),
            node_id,
            discovery_interval: Duration::from_secs(5), // Discover peers every 5 seconds for demo
        }
    }

    pub async fn start_discovery(&self) {
        let peers = self.peers.clone();
        let node_id = self.node_id.clone();
        let discovery_interval = self.discovery_interval;

        tokio::spawn(async move {
            let mut interval = interval(discovery_interval);
            
            loop {
                interval.tick().await;
                
                // Simulate mDNS discovery (local network)
                Self::simulate_mdns_discovery(&peers, &node_id).await;
                
                // Simulate Kademlia DHT discovery (global network)
                Self::simulate_kad_discovery(&peers, &node_id).await;
                
                // Clean up stale peers
                Self::cleanup_stale_peers(&peers).await;
                
                // Print current stats
                let stats = Self::get_discovery_stats(&peers, &node_id).await;
                info!("Discovery Stats: {:?}", stats);
            }
        });
    }

    async fn simulate_mdns_discovery(
        peers: &Arc<RwLock<HashMap<String, PeerInfo>>>,
        node_id: &str,
    ) {
        // Simulate discovering peers on local network
        let local_peers = vec![
            ("node-local-1", "192.168.1.100:8081"),
            ("node-local-2", "192.168.1.101:8081"),
            ("node-local-3", "192.168.1.102:8081"),
        ];

        for (peer_id, addr) in local_peers {
            if peer_id != node_id {
                let mut peers_guard = peers.write().await;
                peers_guard.insert(peer_id.to_string(), PeerInfo {
                    peer_id: peer_id.to_string(),
                    addresses: vec![addr.to_string()],
                    last_seen: Instant::now(),
                    is_connected: true,
                    capabilities: vec!["mdns".to_string(), "gossip".to_string()],
                });
                info!("🌐 mDNS discovered local peer: {} at {}", peer_id, addr);
            }
        }
    }

    async fn simulate_kad_discovery(
        peers: &Arc<RwLock<HashMap<String, PeerInfo>>>,
        node_id: &str,
    ) {
        // Simulate discovering peers via Kademlia DHT
        let global_peers = vec![
            ("node-global-1", "203.0.113.1:8081"),
            ("node-global-2", "203.0.113.2:8081"),
            ("node-global-3", "203.0.113.3:8081"),
        ];

        for (peer_id, addr) in global_peers {
            if peer_id != node_id {
                let mut peers_guard = peers.write().await;
                if !peers_guard.contains_key(peer_id) {
                    peers_guard.insert(peer_id.to_string(), PeerInfo {
                        peer_id: peer_id.to_string(),
                        addresses: vec![addr.to_string()],
                        last_seen: Instant::now(),
                        is_connected: true,
                        capabilities: vec!["kad".to_string(), "gossip".to_string()],
                    });
                    info!("🌍 Kademlia discovered global peer: {} at {}", peer_id, addr);
                }
            }
        }
    }

    async fn cleanup_stale_peers(peers: &Arc<RwLock<HashMap<String, PeerInfo>>>) {
        let mut peers_guard = peers.write().await;
        let now = Instant::now();
        let stale_threshold = Duration::from_secs(30); // 30 seconds for demo

        peers_guard.retain(|peer_id, peer_info| {
            if now.duration_since(peer_info.last_seen) > stale_threshold {
                info!("🗑️ Removing stale peer: {}", peer_id);
                false
            } else {
                true
            }
        });
    }

    async fn get_discovery_stats(
        peers: &Arc<RwLock<HashMap<String, PeerInfo>>>,
        node_id: &str,
    ) -> DiscoveryStats {
        let peers_guard = peers.read().await;
        let total_peers = peers_guard.len();
        let connected_peers = peers_guard.values().filter(|p| p.is_connected).count();
        let mdns_peers = peers_guard.values().filter(|p| p.capabilities.contains(&"mdns".to_string())).count();
        let kad_peers = peers_guard.values().filter(|p| p.capabilities.contains(&"kad".to_string())).count();
        let bootstrap_peers = peers_guard.values().filter(|p| p.capabilities.contains(&"bootstrap".to_string())).count();

        DiscoveryStats {
            total_peers,
            connected_peers,
            mdns_peers,
            kad_peers,
            bootstrap_peers,
            node_id: node_id.to_string(),
        }
    }

    pub async fn add_bootstrap_peer(&self, peer_addr: &str) {
        let peer_id = format!("bootstrap-{}", hex::encode(&peer_addr.as_bytes()[..8]));
        
        let mut peers = self.peers.write().await;
        peers.insert(peer_id.clone(), PeerInfo {
            peer_id: peer_id.clone(),
            addresses: vec![peer_addr.to_string()],
            last_seen: Instant::now(),
            is_connected: true,
            capabilities: vec!["bootstrap".to_string(), "gossip".to_string()],
        });
        
        info!("🔗 Added bootstrap peer: {} at {}", peer_id, peer_addr);
    }

    pub async fn get_peer_list(&self) -> Vec<PeerInfo> {
        self.peers.read().await.values().cloned().collect()
    }
}

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("=== IPPAN Automatic Peer Discovery Demo ===\n");

    // Create a demo node
    let demo = PeerDiscoveryDemo::new("demo-node-1".to_string());
    
    // Add some bootstrap peers
    demo.add_bootstrap_peer("bootstrap.ippan.net:8081").await;
    demo.add_bootstrap_peer("seed1.ippan.net:8081").await;
    
    println!("Starting automatic peer discovery...");
    println!("Discovery will run every 5 seconds for this demo.\n");
    
    // Start the discovery process
    demo.start_discovery().await;
    
    // Let it run for a while
    tokio::time::sleep(Duration::from_secs(30)).await;
    
    // Show final peer list
    println!("\n=== Final Peer List ===");
    let peers = demo.get_peer_list().await;
    for peer in peers {
        println!("Peer: {} at {:?}", peer.peer_id, peer.addresses);
        println!("  Capabilities: {:?}", peer.capabilities);
        println!("  Last seen: {:?} ago", Instant::now().duration_since(peer.last_seen));
        println!("  Connected: {}", peer.is_connected);
        println!();
    }
    
    println!("=== Discovery Summary ===");
    println!("✅ mDNS: Automatically discovers peers on local network");
    println!("✅ Kademlia DHT: Discovers peers across the internet");
    println!("✅ Bootstrap: Manual peer configuration for initial connection");
    println!("✅ Gossip: Message propagation between connected peers");
    println!("✅ Stale cleanup: Removes inactive peers automatically");
    println!();
    println!("In a real IPPAN network, nodes would:");
    println!("1. Start with bootstrap peers for initial connectivity");
    println!("2. Use mDNS to find local peers automatically");
    println!("3. Use Kademlia DHT to find global peers");
    println!("4. Exchange peer lists with connected nodes");
    println!("5. Maintain connections and propagate messages via gossipsub");
}
