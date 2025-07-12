use ippan::{
    config::Config,
    node::IppanNode,
    utils::logging,
};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    logging::init()?;
    
    log::info!("Starting IPPAN node...");
    
    // Load configuration
    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "config/default.toml".to_string());
    
    let config = Config::load(&config_path)?;
    log::info!("Loaded configuration from: {}", config_path);
    
    // Create and initialize node
    let mut node = IppanNode::new(config).await?;
    
    // Initialize API layer after node creation
    node.init_api();
    log::info!("Initialized API layer");
    
    // Start the node
    node.start().await?;
    log::info!("IPPAN node started successfully");
    
    // Run the main node loop
    log::info!("Running IPPAN node...");
    node.run().await?;
    
    log::info!("IPPAN node stopped");
    Ok(())
}
