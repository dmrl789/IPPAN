use clap::Parser;
use ippan_node::Node;
use tracing::{info, Level};

#[cfg(not(target_os = "windows"))]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[derive(Parser)]
#[command(name = "ippan-node")]
#[command(about = "IPPAN - High Performance Blockchain Node")]
struct Cli {
    /// HTTP server port
    #[arg(long, default_value = "8080")]
    http_port: u16,

    /// P2P port
    #[arg(long, default_value = "8081")]
    p2p_port: u16,

    /// Bootstrap peers
    #[arg(long)]
    bootstrap_peers: Vec<String>,

    /// Number of shards
    #[arg(long, default_value = "1")]
    shards: usize,

    /// Log level
    #[arg(long, default_value = "info")]
    log_level: Level,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(cli.log_level)
        .init();

    info!("Starting IPPAN node...");
    info!("HTTP port: {}", cli.http_port);
    info!("P2P port: {}", cli.p2p_port);
    info!("Shards: {}", cli.shards);

    // Create and start the node
    let mut node = Node::new(cli.http_port, cli.p2p_port, cli.shards).await?;

    // Add bootstrap peers
    for peer in cli.bootstrap_peers {
        node.add_bootstrap_peer(&peer).await?;
    }

    // Start the node
    node.start().await?;

    // Keep the node running
    tracing::info!("Node is running. Press Ctrl+C to stop.");
    tokio::signal::ctrl_c().await?;
    tracing::info!("Shutting down node...");

    Ok(())
}
