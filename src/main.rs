use anyhow::{Context, Result};
use std::{panic, time::Duration};
use tracing::{debug, error, info};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

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
async fn main() -> Result<()> {
    // Panic + logging setup
    std::env::set_var("RUST_BACKTRACE", std::env::var("RUST_BACKTRACE").unwrap_or_else(|_| "full".into()));
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    panic::set_hook(Box::new(|pi| {
        eprintln!("[panic] {pi}");
        if let Ok(bt) = std::env::var("RUST_BACKTRACE") {
            eprintln!("[panic] RUST_BACKTRACE={bt}");
        }
    }));

    info!("IPPAN node starting…");
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

    // 1) Load config
    let cfg = Config::load().context("Config::load failed")?;
    info!("Configuration loaded successfully");

    // 2) Build node
    info!("creating IppanNode…");
    let mut node = IppanNode::new(cfg.clone())
        .await
        .map_err(|e| anyhow::anyhow!("IppanNode::new failed: {}", e))?;
    info!("IppanNode created.");

    // 3) Start services
    info!("starting node services…");
    
    // Variant C: start() spawns tasks and returns immediately -> keep process alive
    node.start().await.map_err(|e| anyhow::anyhow!("node.start failed: {}", e))?;
    info!("node started; entering keep-alive loop");
    tokio::signal::ctrl_c().await.expect("failed to wait for ctrl_c");
    info!("ctrl_c received; shutting down");

    info!("node exited normally.");
    Ok(())
}
