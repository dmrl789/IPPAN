//! Logging utilities for IPPAN
//! 
//! This module provides logging functionality using tracing.

use tracing::{Level};
use tracing_subscriber::EnvFilter;

pub fn init_logging() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
}

/// Initialize logging with custom level
pub fn init_logging_with_level(level: Level) {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env()
            .add_directive(format!("ippan={}", level).parse().unwrap()))
        .init();
}

/// Initialize logging for development with debug level
pub fn init_dev_logging() {
    init_logging_with_level(Level::DEBUG);
}

/// Initialize logging for production with warning level
pub fn init_prod_logging() {
    init_logging_with_level(Level::WARN);
}

/// Log a structured event with metadata
pub fn log_event(event_type: &str, message: &str, metadata: Option<serde_json::Value>) {
    let mut log_data = serde_json::Map::new();
    log_data.insert("event_type".to_string(), serde_json::Value::String(event_type.to_string()));
    log_data.insert("message".to_string(), serde_json::Value::String(message.to_string()));
    
    if let Some(meta) = metadata {
        log_data.insert("metadata".to_string(), meta);
    }
    
    tracing::info!(event = %serde_json::to_string(&log_data).unwrap_or_default());
}

/// Log a performance metric
pub fn log_performance(operation: &str, duration_ms: u64, metadata: Option<serde_json::Value>) {
    let mut log_data = serde_json::Map::new();
    log_data.insert("operation".to_string(), serde_json::Value::String(operation.to_string()));
    log_data.insert("duration_ms".to_string(), serde_json::Value::Number(duration_ms.into()));
    
    if let Some(meta) = metadata {
        log_data.insert("metadata".to_string(), meta);
    }
    
    tracing::info!(performance = %serde_json::to_string(&log_data).unwrap_or_default());
}

/// Log an error with context
pub fn log_error(error: &dyn std::error::Error, context: &str) {
    tracing::error!(error = %error, context = %context);
}

/// Log a warning with context
pub fn log_warning(message: &str, context: &str) {
    tracing::warn!(message = %message, context = %context);
}

/// Log an info message with context
pub fn log_info(message: &str, context: &str) {
    tracing::info!(message = %message, context = %context);
}

/// Log a debug message with context
pub fn log_debug(message: &str, context: &str) {
    tracing::debug!(message = %message, context = %context);
}

/// Log a trace message with context
pub fn log_trace(message: &str, context: &str) {
    tracing::trace!(message = %message, context = %context);
}

/// Log network event
pub fn log_network_event(event: &str, peer_id: &str, details: Option<serde_json::Value>) {
    let mut log_data = serde_json::Map::new();
    log_data.insert("event".to_string(), serde_json::Value::String(event.to_string()));
    log_data.insert("peer_id".to_string(), serde_json::Value::String(peer_id.to_string()));
    
    if let Some(details) = details {
        log_data.insert("details".to_string(), details);
    }
    
    tracing::info!(network = %serde_json::to_string(&log_data).unwrap_or_default());
}

/// Log consensus event
pub fn log_consensus_event(event: &str, block_hash: &str, details: Option<serde_json::Value>) {
    let mut log_data = serde_json::Map::new();
    log_data.insert("event".to_string(), serde_json::Value::String(event.to_string()));
    log_data.insert("block_hash".to_string(), serde_json::Value::String(block_hash.to_string()));
    
    if let Some(details) = details {
        log_data.insert("details".to_string(), details);
    }
    
    tracing::info!(consensus = %serde_json::to_string(&log_data).unwrap_or_default());
}

/// Log storage event
pub fn log_storage_event(event: &str, file_hash: &str, details: Option<serde_json::Value>) {
    let mut log_data = serde_json::Map::new();
    log_data.insert("event".to_string(), serde_json::Value::String(event.to_string()));
    log_data.insert("file_hash".to_string(), serde_json::Value::String(file_hash.to_string()));
    
    if let Some(details) = details {
        log_data.insert("details".to_string(), details);
    }
    
    tracing::info!(storage = %serde_json::to_string(&log_data).unwrap_or_default());
}

/// Log transaction event
pub fn log_transaction_event(event: &str, tx_hash: &str, details: Option<serde_json::Value>) {
    let mut log_data = serde_json::Map::new();
    log_data.insert("event".to_string(), serde_json::Value::String(event.to_string()));
    log_data.insert("tx_hash".to_string(), serde_json::Value::String(tx_hash.to_string()));
    
    if let Some(details) = details {
        log_data.insert("details".to_string(), details);
    }
    
    tracing::info!(transaction = %serde_json::to_string(&log_data).unwrap_or_default());
}

/// Log staking event
pub fn log_staking_event(event: &str, address: &str, amount: u64, details: Option<serde_json::Value>) {
    let mut log_data = serde_json::Map::new();
    log_data.insert("event".to_string(), serde_json::Value::String(event.to_string()));
    log_data.insert("address".to_string(), serde_json::Value::String(address.to_string()));
    log_data.insert("amount".to_string(), serde_json::Value::Number(amount.into()));
    
    if let Some(details) = details {
        log_data.insert("details".to_string(), details);
    }
    
    tracing::info!(staking = %serde_json::to_string(&log_data).unwrap_or_default());
}

/// Log domain event
pub fn log_domain_event(event: &str, domain: &str, details: Option<serde_json::Value>) {
    let mut log_data = serde_json::Map::new();
    log_data.insert("event".to_string(), serde_json::Value::String(event.to_string()));
    log_data.insert("domain".to_string(), serde_json::Value::String(domain.to_string()));
    
    if let Some(details) = details {
        log_data.insert("details".to_string(), details);
    }
    
    tracing::info!(domain = %serde_json::to_string(&log_data).unwrap_or_default());
}

/// Log DHT event
pub fn log_dht_event(event: &str, key: &str, details: Option<serde_json::Value>) {
    let mut log_data = serde_json::Map::new();
    log_data.insert("event".to_string(), serde_json::Value::String(event.to_string()));
    log_data.insert("key".to_string(), serde_json::Value::String(key.to_string()));
    
    if let Some(details) = details {
        log_data.insert("details".to_string(), details);
    }
    
    tracing::info!(dht = %serde_json::to_string(&log_data).unwrap_or_default());
}

/// Log API event
pub fn log_api_event(event: &str, endpoint: &str, details: Option<serde_json::Value>) {
    let mut log_data = serde_json::Map::new();
    log_data.insert("event".to_string(), serde_json::Value::String(event.to_string()));
    log_data.insert("endpoint".to_string(), serde_json::Value::String(endpoint.to_string()));
    
    if let Some(details) = details {
        log_data.insert("details".to_string(), details);
    }
    
    tracing::info!(api = %serde_json::to_string(&log_data).unwrap_or_default());
}

/// Log wallet event
pub fn log_wallet_event(event: &str, address: &str, details: Option<serde_json::Value>) {
    let mut log_data = serde_json::Map::new();
    log_data.insert("event".to_string(), serde_json::Value::String(event.to_string()));
    log_data.insert("address".to_string(), serde_json::Value::String(address.to_string()));
    
    if let Some(details) = details {
        log_data.insert("details".to_string(), details);
    }
    
    tracing::info!(wallet = %serde_json::to_string(&log_data).unwrap_or_default());
}

/// Log system metrics
pub fn log_system_metrics(cpu_usage: f64, memory_usage: u64, disk_usage: u64) {
    let mut log_data = serde_json::Map::new();
    log_data.insert("cpu_usage_percent".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(cpu_usage).unwrap_or(serde_json::Number::from(0))));
    log_data.insert("memory_usage_bytes".to_string(), serde_json::Value::Number(memory_usage.into()));
    log_data.insert("disk_usage_bytes".to_string(), serde_json::Value::Number(disk_usage.into()));
    
    tracing::info!(system_metrics = %serde_json::to_string(&log_data).unwrap_or_default());
}

/// Log startup information
pub fn log_startup(node_id: &str, version: &str, config: &serde_json::Value) {
    let mut log_data = serde_json::Map::new();
    log_data.insert("node_id".to_string(), serde_json::Value::String(node_id.to_string()));
    log_data.insert("version".to_string(), serde_json::Value::String(version.to_string()));
    log_data.insert("config".to_string(), config.clone());
    
    tracing::info!(startup = %serde_json::to_string(&log_data).unwrap_or_default());
}

/// Log shutdown information
pub fn log_shutdown(reason: &str, uptime_seconds: u64) {
    let mut log_data = serde_json::Map::new();
    log_data.insert("reason".to_string(), serde_json::Value::String(reason.to_string()));
    log_data.insert("uptime_seconds".to_string(), serde_json::Value::Number(uptime_seconds.into()));
    
    tracing::info!(shutdown = %serde_json::to_string(&log_data).unwrap_or_default());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logging_initialization() {
        init_logging();
        // Should not panic
    }

    #[test]
    fn test_logging_functions() {
        init_logging();
        
        log_info("Test info message", "test_context");
        log_warning("Test warning message", "test_context");
        log_debug("Test debug message", "test_context");
        
        // Should not panic
    }

    #[test]
    fn test_structured_logging() {
        init_logging();
        
        let metadata = serde_json::json!({
            "key": "value",
            "number": 42
        });
        
        log_event("test_event", "Test event message", Some(metadata));
        
        // Should not panic
    }
}
