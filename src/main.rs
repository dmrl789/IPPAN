use ippan::{
    config::Config,
    node::IppanNode,
    utils::logging,
};


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    logging::init_logging();
    
    log::info!("Starting IPPAN node...");
    
    // Load configuration
    let config = Config::load()?;
    log::info!("Loaded configuration");
    
    // Create and initialize node
    let mut node = IppanNode::new(config).await?;
    
    // TODO: Initialize API layer when ready
    // node.init_api();
    // log::info!("Initialized API layer");
    
    // Start the node
    node.start().await?;
    log::info!("IPPAN node started successfully");
    
    // Run the main node loop
    log::info!("Running IPPAN node...");
    node.run().await?;
    
    log::info!("IPPAN node stopped");
    Ok(())
}
