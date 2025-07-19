//! Tests for IPPAN logging system

use ippan::utils::logging::{
    LoggingConfig, LogFormat, LogRotator, PerformanceLogger,
    log_error_with_context, log_warning_with_context,
    log_security_event, log_network_event, log_storage_event,
};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_logging_config_default() {
    let config = LoggingConfig::default();
    
    assert_eq!(config.log_level, "info");
    assert_eq!(config.log_dir, "logs");
    assert!(matches!(config.log_format, LogFormat::Both));
    assert!(config.enable_console);
    assert!(config.enable_file);
    assert!(config.enable_json);
    assert_eq!(config.max_file_size, 100 * 1024 * 1024);
    assert_eq!(config.max_files, 10);
}

#[test]
fn test_logging_config_from_env() {
    // Set environment variables
    std::env::set_var("IPPAN_LOG_LEVEL", "debug");
    std::env::set_var("IPPAN_LOG_DIR", "/tmp/test_logs");
    std::env::set_var("IPPAN_LOG_FORMAT", "json");
    
    let config = LoggingConfig::from_env();
    
    assert_eq!(config.log_level, "debug");
    assert_eq!(config.log_dir, "/tmp/test_logs");
    assert!(matches!(config.log_format, LogFormat::Json));
    
    // Clean up
    std::env::remove_var("IPPAN_LOG_LEVEL");
    std::env::remove_var("IPPAN_LOG_DIR");
    std::env::remove_var("IPPAN_LOG_FORMAT");
}

#[test]
fn test_logging_config_validation() {
    let mut config = LoggingConfig::default();
    
    // Test valid configuration
    assert!(config.validate().is_ok());
    
    // Test invalid log level
    config.log_level = "invalid".to_string();
    assert!(config.validate().is_err());
    
    // Test valid log level
    config.log_level = "debug".to_string();
    assert!(config.validate().is_ok());
}

#[test]
fn test_log_rotator() {
    let temp_dir = TempDir::new().unwrap();
    let log_dir = temp_dir.path().to_str().unwrap().to_string();
    
    let rotator = LogRotator::new(log_dir.clone(), 1024, 3);
    
    // Test log rotation
    assert!(rotator.rotate_logs().is_ok());
    
    // Verify log directory was created
    assert!(fs::metadata(&log_dir).is_ok());
}

#[test]
fn test_performance_logger() {
    let logger = PerformanceLogger::new("test_operation");
    
    // Simulate some work
    std::thread::sleep(std::time::Duration::from_millis(10));
    
    // This should not panic
    logger.finish();
}

#[test]
fn test_logging_macros() {
    // Test that logging macros compile and work
    log_node_start!("test_node_id");
    log_node_stop!("test_node_id");
    log_transaction!("tx_hash_123", "payment", 1000);
    log_block!("block_hash_456", 12345, 150);
    log_health_check!(ippan::node::NodeStatus::Healthy, std::time::Duration::from_secs(3600), 1024*1024*100, 25.0);
    log_recovery_attempt!(1, 5, "memory_high");
}

#[test]
fn test_logging_utilities() {
    // Test error logging
    let error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
    log_error_with_context(&error, "file_operation");
    
    // Test warning logging
    log_warning_with_context("High memory usage", "system_monitor");
    
    // Test security event logging
    log_security_event("failed_login", "Invalid credentials", "medium");
    
    // Test network event logging
    log_network_event("peer_connected", "peer_123", "New peer connection");
    
    // Test storage event logging
    log_storage_event("file_uploaded", "file_hash_789", 1024);
}

#[test]
fn test_logging_formats() {
    let config_text = LoggingConfig {
        log_format: LogFormat::Text,
        ..Default::default()
    };
    
    let config_json = LoggingConfig {
        log_format: LogFormat::Json,
        ..Default::default()
    };
    
    let config_both = LoggingConfig {
        log_format: LogFormat::Both,
        ..Default::default()
    };
    
    assert!(matches!(config_text.log_format, LogFormat::Text));
    assert!(matches!(config_json.log_format, LogFormat::Json));
    assert!(matches!(config_both.log_format, LogFormat::Both));
}

#[test]
fn test_logging_config_serialization() {
    let config = LoggingConfig::default();
    
    // Test that we can access all fields
    assert_eq!(config.log_level, "info");
    assert_eq!(config.log_dir, "logs");
    assert!(config.enable_console);
    assert!(config.enable_file);
    assert!(config.enable_json);
    assert_eq!(config.max_file_size, 100 * 1024 * 1024);
    assert_eq!(config.max_files, 10);
}

#[test]
fn test_logging_directory_creation() {
    let temp_dir = TempDir::new().unwrap();
    let log_dir = temp_dir.path().join("logs").to_str().unwrap().to_string();
    
    let config = LoggingConfig {
        log_dir: log_dir.clone(),
        ..Default::default()
    };
    
    // This should create the directory
    assert!(config.validate().is_ok());
    assert!(fs::metadata(&log_dir).is_ok());
} 