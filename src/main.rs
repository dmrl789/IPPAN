use ippan::{
    node::IppanNode,
    config::Config,
    utils::logging::{LoggerConfig, StructuredLogger},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging system
    let logging_config = LoggerConfig::default();
    let logger = StructuredLogger::new(logging_config);
    logger.init()?;

    log::info!("Starting IPPAN node");

    // Load configuration
    let config = Config::load()?;
    log::info!("Configuration loaded successfully");

    // Create and start the node
    let mut node = IppanNode::new(config).await?;
    
    // Start the node
    node.start().await?;

    Ok(())
}
