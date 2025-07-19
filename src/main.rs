use ippan::{
    node::IppanNode,
    config::Config,
    utils::logging::{init_logging, LoggingConfig},
    log_node_start,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging system
    let logging_config = LoggingConfig::from_env();
    logging_config.validate()?;
    init_logging(&logging_config)?;

    log::info!("Starting IPPAN node");

    // Load configuration
    let config = Config::load()?;
    log::info!("Configuration loaded successfully");

    // Create and start the node
    let mut node = IppanNode::new(config).await?;
    
    // Log node start
    log_node_start!(format!("{:?}", node.node_id()));
    
    // Start the node
    node.start().await?;

    Ok(())
}
