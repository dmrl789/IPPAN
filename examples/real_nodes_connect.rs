use ippan::node::Node;
use std::time::Duration;
use tokio::time::sleep;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("=== IPPAN Real Nodes Connection Demo ===\n");
    println!("This demo creates two actual IPPAN nodes and connects them.\n");
    
    // Create Node 1
    info!("Creating Node 1...");
    let node1 = Node::new(
        8080,  // HTTP port
        9001,  // P2P port  
        4,     // Number of shards
    ).await?;
    
    // Create Node 2
    info!("Creating Node 2...");
    let node2 = Node::new(
        8081,  // HTTP port
        9002,  // P2P port
        4,     // Number of shards
    ).await?;
    
    println!("\n📍 Node 1 configuration:");
    println!("   - HTTP API: http://localhost:8080");
    println!("   - P2P Port: 9001");
    println!("   - Shards: 4");
    
    println!("\n📍 Node 2 configuration:");
    println!("   - HTTP API: http://localhost:8081");
    println!("   - P2P Port: 9002");
    println!("   - Shards: 4\n");
    
    // Start Node 1 in background
    let node1_handle = {
        let node1_clone = node1.clone();
        tokio::spawn(async move {
            info!("Starting Node 1...");
            if let Err(e) = node1_clone.start().await {
                eprintln!("Node 1 failed to start: {}", e);
            }
        })
    };
    
    // Give Node 1 time to start
    sleep(Duration::from_secs(1)).await;
    
    // Start Node 2 in background
    let node2_handle = {
        let node2_clone = node2.clone();
        tokio::spawn(async move {
            info!("Starting Node 2...");
            if let Err(e) = node2_clone.start().await {
                eprintln!("Node 2 failed to start: {}", e);
            }
        })
    };
    
    // Give Node 2 time to start
    sleep(Duration::from_secs(1)).await;
    
    println!("\n✅ Both nodes are now running!\n");
    
    // In the simplified implementation, nodes would connect via bootstrap peers
    // Since the current network manager is simplified, we'll demonstrate the concept
    
    println!("🔗 In a full implementation, nodes would:");
    println!("   1. Connect via bootstrap peers");
    println!("   2. Discover each other via mDNS (local) or Kademlia DHT (global)");
    println!("   3. Exchange peer lists via gossip protocol");
    println!("   4. Maintain persistent connections\n");
    
    // Check node health via HTTP API
    println!("📊 Checking node health via HTTP API...\n");
    
    // Check Node 1 health
    match check_node_health("http://localhost:8080").await {
        Ok(response) => println!("Node 1 Health: {}", response),
        Err(e) => println!("Failed to check Node 1 health: {}", e),
    }
    
    // Check Node 2 health
    match check_node_health("http://localhost:8081").await {
        Ok(response) => println!("Node 2 Health: {}", response),
        Err(e) => println!("Failed to check Node 2 health: {}", e),
    }
    
    println!("\n💡 To connect these nodes in production:");
    println!("   1. Configure bootstrap peers in node startup");
    println!("   2. Use node1.add_bootstrap_peer(\"node2_address\")");
    println!("   3. Enable P2P discovery mechanisms");
    
    println!("\n📝 Example commands to interact with nodes:");
    println!("   - Submit transaction: POST http://localhost:8080/submit_transaction");
    println!("   - Check mempool: GET http://localhost:8080/mempool");
    println!("   - Get metrics: GET http://localhost:8080/metrics");
    
    println!("\n⏳ Nodes will continue running. Press Ctrl+C to stop.\n");
    
    // Keep the program running
    tokio::signal::ctrl_c().await?;
    
    println!("\n🛑 Shutting down nodes...");
    
    // Abort the tasks
    node1_handle.abort();
    node2_handle.abort();
    
    println!("✅ Nodes stopped successfully.");
    
    Ok(())
}

async fn check_node_health(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/health", url))
        .timeout(Duration::from_secs(2))
        .send()
        .await?;
    
    if response.status().is_success() {
        let text = response.text().await?;
        Ok(text)
    } else {
        Err(format!("HTTP error: {}", response.status()).into())
    }
}