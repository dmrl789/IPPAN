use ippan::{
    node::IppanNode,
    config::Config,
    utils::logging::{LoggerConfig, StructuredLogger},
    performance_test::PerformanceTestSuite,
    testing_framework::TestingFramework,
    security_audit::SecurityAuditFramework,
};
use clap::{Command, Arg};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let matches = Command::new("IPPAN")
        .version("1.0")
        .about("Immutable Proof & Availability Network")
        .arg(
            Arg::new("performance-test")
                .long("performance-test")
                .short('p')
                .action(clap::ArgAction::SetTrue)
                .help("Run performance tests")
        )
        .arg(
            Arg::new("test-suite")
                .long("test-suite")
                .short('t')
                .action(clap::ArgAction::SetTrue)
                .help("Run comprehensive test suite")
        )
        .arg(
            Arg::new("security-audit")
                .long("security-audit")
                .short('s')
                .action(clap::ArgAction::SetTrue)
                .help("Run security audit")
        )
        .get_matches();

    // Check if performance test is requested
    if matches.get_flag("performance-test") {
        println!("Running IPPAN Performance Tests...");
        let mut test_suite = PerformanceTestSuite::new();
        test_suite.run_all_tests().await?;
        return Ok(());
    }

    // Check if comprehensive test suite is requested
    if matches.get_flag("test-suite") {
        println!("Running IPPAN Comprehensive Test Suite...");
        let mut test_framework = TestingFramework::new();
        test_framework.run_all_tests().await?;
        return Ok(());
    }

    // Check if security audit is requested
    if matches.get_flag("security-audit") {
        println!("Running IPPAN Security Audit...");
        let mut security_framework = SecurityAuditFramework::new();
        security_framework.run_security_audit().await?;
        return Ok(());
    }

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
