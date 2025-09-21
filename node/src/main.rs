use anyhow::Result;
use tracing::info;
use tracing_subscriber::EnvFilter;
use ippan_types::{ippan_time_init, ippan_time_now, HashTimer, IppanTimeMicros};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .init();

    // Initialize IPPAN Time service
    ippan_time_init();
    info!("IPPAN Time service initialized");

    info!("IPPAN node booting...");

    // Create a boot HashTimer to demonstrate the system
    let current_time = IppanTimeMicros(ippan_time_now());
    let domain = "boot";
    let payload = b"node_startup";
    let nonce = ippan_types::random_nonce();
    let node_id = b"ippan_node_001";
    
    let boot_hashtimer = HashTimer::derive(
        domain,
        current_time,
        domain.as_bytes(),
        payload,
        &nonce,
        node_id,
    );
    
    info!("Boot HashTimer: {}", boot_hashtimer.to_hex());
    info!("Current IPPAN Time: {} microseconds", current_time.0);
    
    // Demonstrate HashTimer creation for different contexts
    let tx_hashtimer = HashTimer::now_tx("demo_tx", b"transfer_100_tokens", &nonce, node_id);
    let block_hashtimer = HashTimer::now_block("demo_block", b"block_creation", &nonce, node_id);
    let round_hashtimer = HashTimer::now_round("demo_round", b"consensus_round", &nonce, node_id);
    
    info!("Transaction HashTimer: {}", tx_hashtimer.to_hex());
    info!("Block HashTimer: {}", block_hashtimer.to_hex());
    info!("Round HashTimer: {}", round_hashtimer.to_hex());
    
    // Demonstrate time monotonicity
    info!("Testing time monotonicity...");
    for i in 0..5 {
        let time1 = ippan_time_now();
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        let time2 = ippan_time_now();
        info!("Time check {}: {} -> {} (diff: {})", i, time1, time2, time2 - time1);
    }
    
    // TODO: Initialize and start core components
    // - Storage layer
    // - P2P network
    // - Mempool
    // - Consensus engine
    // - RPC API
    
    info!("IPPAN node is ready and running");
    
    // Keep the node running
    tokio::signal::ctrl_c().await?;
    info!("Shutting down IPPAN node");
    
    Ok(())
}
