//! Enhanced logging system for IPPAN
//! 
//! Provides structured logging, error tracking, and performance monitoring

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn, debug, Level};
use tracing_subscriber::{
    fmt::{format::FmtSpan},
    EnvFilter, FmtSubscriber,
};

/// Log entry with structured data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub target: String,
    pub message: String,
    pub fields: HashMap<String, serde_json::Value>,
    pub error_details: Option<ErrorDetails>,
    pub performance_metrics: Option<LogPerformanceMetrics>,
}

/// Log level enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// Error details for structured error logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetails {
    pub error_type: String,
    pub error_code: Option<String>,
    pub stack_trace: Option<String>,
    pub context: HashMap<String, serde_json::Value>,
    pub severity: ErrorSeverity,
}

/// Error severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Performance metrics for logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogPerformanceMetrics {
    pub duration_ms: f64,
    pub memory_usage_bytes: Option<u64>,
    pub cpu_usage_percent: Option<f64>,
    pub operation_count: Option<u64>,
}

/// Structured logger with enhanced capabilities
pub struct StructuredLogger {
    logs: Arc<RwLock<Vec<LogEntry>>>,
    error_tracker: Arc<RwLock<ErrorTracker>>,
    performance_tracker: Arc<RwLock<PerformanceTracker>>,
    config: LoggerConfig,
}

/// Error tracking system
#[derive(Debug, Clone)]
pub struct ErrorTracker {
    pub errors: Vec<ErrorDetails>,
    pub error_counts: HashMap<String, u64>,
    pub critical_errors: Vec<ErrorDetails>,
}

/// Performance tracking system
#[derive(Debug, Clone)]
pub struct PerformanceTracker {
    pub slow_operations: Vec<LogPerformanceMetrics>,
    pub operation_stats: HashMap<String, OperationStats>,
}

/// Operation statistics
#[derive(Debug, Clone)]
pub struct OperationStats {
    pub count: u64,
    pub total_duration_ms: f64,
    pub avg_duration_ms: f64,
    pub min_duration_ms: f64,
    pub max_duration_ms: f64,
}

/// Logger configuration
#[derive(Debug, Clone)]
pub struct LoggerConfig {
    pub log_level: Level,
    pub enable_structured_logging: bool,
    pub enable_error_tracking: bool,
    pub enable_performance_tracking: bool,
    pub max_log_entries: usize,
    pub log_file_path: Option<String>,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            log_level: Level::INFO,
            enable_structured_logging: true,
            enable_error_tracking: true,
            enable_performance_tracking: true,
            max_log_entries: 10000,
            log_file_path: None,
        }
    }
}

impl StructuredLogger {
    /// Create a new structured logger
    pub fn new(config: LoggerConfig) -> Self {
        Self {
            logs: Arc::new(RwLock::new(Vec::new())),
            error_tracker: Arc::new(RwLock::new(ErrorTracker {
                errors: Vec::new(),
                error_counts: HashMap::new(),
                critical_errors: Vec::new(),
            })),
            performance_tracker: Arc::new(RwLock::new(PerformanceTracker {
                slow_operations: Vec::new(),
                operation_stats: HashMap::new(),
            })),
            config,
        }
    }

    /// Initialize the logging system
    pub fn init(&self) -> Result<(), Box<dyn std::error::Error>> {
        let subscriber = FmtSubscriber::builder()
            .with_env_filter(EnvFilter::from_default_env())
            .with_span_events(FmtSpan::CLOSE)
            .with_file(true)
            .with_line_number(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_target(false)
            .with_level(true)
            .with_ansi(false)
            .pretty()
            .finish();

        tracing::subscriber::set_global_default(subscriber)?;
        
        info!("Structured logging system initialized");
        Ok(())
    }

    /// Log a structured message
    pub async fn log(
        &self,
        level: LogLevel,
        target: &str,
        message: &str,
        fields: HashMap<String, serde_json::Value>,
    ) {
        let entry = LogEntry {
            timestamp: Utc::now(),
            level: level.clone(),
            target: target.to_string(),
            message: message.to_string(),
            fields,
            error_details: None,
            performance_metrics: None,
        };

        // Log to tracing
        match level {
            LogLevel::Trace => debug!("{}", message),
            LogLevel::Debug => debug!("{}", message),
            LogLevel::Info => info!("{}", message),
            LogLevel::Warn => warn!("{}", message),
            LogLevel::Error => error!("{}", message),
        }

        // Store in structured logs
        if self.config.enable_structured_logging {
            let mut logs = self.logs.write().await;
            logs.push(entry);
            
            // Maintain log size limit
            if logs.len() > self.config.max_log_entries {
                logs.remove(0);
            }
        }
    }

    /// Log an error with details
    pub async fn log_error(
        &self,
        target: &str,
        message: &str,
        error_details: ErrorDetails,
        fields: HashMap<String, serde_json::Value>,
    ) {
        let entry = LogEntry {
            timestamp: Utc::now(),
            level: LogLevel::Error,
            target: target.to_string(),
            message: message.to_string(),
            fields,
            error_details: Some(error_details.clone()),
            performance_metrics: None,
        };

        // Log to tracing
        error!("{}", message);

        // Store in structured logs
        if self.config.enable_structured_logging {
            let mut logs = self.logs.write().await;
            logs.push(entry);
            
            if logs.len() > self.config.max_log_entries {
                logs.remove(0);
            }
        }

        // Track errors
        if self.config.enable_error_tracking {
            let mut error_tracker = self.error_tracker.write().await;
            error_tracker.errors.push(error_details.clone());
            
            let error_type = error_details.error_type.clone();
            *error_tracker.error_counts.entry(error_type).or_insert(0) += 1;
            
            if matches!(error_details.severity, ErrorSeverity::Critical) {
                error_tracker.critical_errors.push(error_details);
            }
        }
    }

    /// Log performance metrics
    pub async fn log_performance(
        &self,
        target: &str,
        message: &str,
        operation_name: &str,
        duration_ms: f64,
        fields: HashMap<String, serde_json::Value>,
    ) {
        let performance_metrics = LogPerformanceMetrics {
            duration_ms,
            memory_usage_bytes: None,
            cpu_usage_percent: None,
            operation_count: Some(1),
        };

        let entry = LogEntry {
            timestamp: Utc::now(),
            level: LogLevel::Info,
            target: target.to_string(),
            message: message.to_string(),
            fields,
            error_details: None,
            performance_metrics: Some(performance_metrics.clone()),
        };

        // Log to tracing
        info!("{} ({}ms)", message, duration_ms);

        // Store in structured logs
        if self.config.enable_structured_logging {
            let mut logs = self.logs.write().await;
            logs.push(entry);
            
            if logs.len() > self.config.max_log_entries {
                logs.remove(0);
            }
        }

        // Track performance
        if self.config.enable_performance_tracking {
            let mut performance_tracker = self.performance_tracker.write().await;
            
            // Update operation stats
            let stats = performance_tracker.operation_stats
                .entry(operation_name.to_string())
                .or_insert(OperationStats {
                    count: 0,
                    total_duration_ms: 0.0,
                    avg_duration_ms: 0.0,
                    min_duration_ms: f64::MAX,
                    max_duration_ms: 0.0,
                });
            
            stats.count += 1;
            stats.total_duration_ms += duration_ms;
            stats.avg_duration_ms = stats.total_duration_ms / stats.count as f64;
            stats.min_duration_ms = stats.min_duration_ms.min(duration_ms);
            stats.max_duration_ms = stats.max_duration_ms.max(duration_ms);

            // Track slow operations (over 100ms)
            if duration_ms > 100.0 {
                performance_tracker.slow_operations.push(performance_metrics);
                
                // Keep only last 1000 slow operations
                if performance_tracker.slow_operations.len() > 1000 {
                    performance_tracker.slow_operations.remove(0);
                }
            }
        }
    }

    /// Get all logs
    pub async fn get_logs(&self) -> Vec<LogEntry> {
        let logs = self.logs.read().await;
        logs.clone()
    }

    /// Get logs by level
    pub async fn get_logs_by_level(&self, level: LogLevel) -> Vec<LogEntry> {
        let logs = self.logs.read().await;
        logs.iter()
            .filter(|log| log.level == level)
            .cloned()
            .collect()
    }

    /// Get error statistics
    pub async fn get_error_stats(&self) -> HashMap<String, u64> {
        let error_tracker = self.error_tracker.read().await;
        error_tracker.error_counts.clone()
    }

    /// Get critical errors
    pub async fn get_critical_errors(&self) -> Vec<ErrorDetails> {
        let error_tracker = self.error_tracker.read().await;
        error_tracker.critical_errors.clone()
    }

    /// Get performance statistics
    pub async fn get_performance_stats(&self) -> HashMap<String, OperationStats> {
        let performance_tracker = self.performance_tracker.read().await;
        performance_tracker.operation_stats.clone()
    }

    /// Get slow operations
    pub async fn get_slow_operations(&self) -> Vec<LogPerformanceMetrics> {
        let performance_tracker = self.performance_tracker.read().await;
        performance_tracker.slow_operations.clone()
    }

    /// Clear old logs
    pub async fn clear_old_logs(&self, max_age_hours: u64) {
        let mut logs = self.logs.write().await;
        let cutoff = Utc::now() - chrono::Duration::hours(max_age_hours as i64);
        
        logs.retain(|log| log.timestamp > cutoff);
    }

    /// Export logs to JSON
    pub async fn export_logs(&self) -> String {
        let logs = self.logs.read().await;
        serde_json::to_string_pretty(&*logs).unwrap_or_default()
    }

    /// Create error details
    pub fn create_error_details(
        error_type: &str,
        error_code: Option<String>,
        stack_trace: Option<String>,
        context: HashMap<String, serde_json::Value>,
        severity: ErrorSeverity,
    ) -> ErrorDetails {
        ErrorDetails {
            error_type: error_type.to_string(),
            error_code,
            stack_trace,
            context,
            severity,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_logger_creation() {
        let config = LoggerConfig::default();
        let logger = StructuredLogger::new(config);
        assert_eq!(logger.config.log_level, Level::INFO);
    }

    #[tokio::test]
    async fn test_logging() {
        let config = LoggerConfig::default();
        let logger = StructuredLogger::new(config);
        
        logger.log(
            LogLevel::Info,
            "test",
            "Test message",
            HashMap::new(),
        ).await;
        
        let logs = logger.get_logs().await;
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].message, "Test message");
    }

    #[tokio::test]
    async fn test_error_logging() {
        let config = LoggerConfig::default();
        let logger = StructuredLogger::new(config);
        
        let error_details = StructuredLogger::create_error_details(
            "TestError",
            Some("TEST001".to_string()),
            Some("stack trace".to_string()),
            HashMap::new(),
            ErrorSeverity::High,
        );
        
        logger.log_error(
            "test",
            "Test error",
            error_details,
            HashMap::new(),
        ).await;
        
        let error_stats = logger.get_error_stats().await;
        assert_eq!(error_stats.get("TestError"), Some(&1));
    }

    #[tokio::test]
    async fn test_performance_logging() {
        let config = LoggerConfig::default();
        let logger = StructuredLogger::new(config);
        
        logger.log_performance(
            "test",
            "Test operation",
            "test_op",
            150.0,
            HashMap::new(),
        ).await;
        
        let performance_stats = logger.get_performance_stats().await;
        assert!(performance_stats.contains_key("test_op"));
        
        let slow_ops = logger.get_slow_operations().await;
        assert_eq!(slow_ops.len(), 1);
        assert_eq!(slow_ops[0].duration_ms, 150.0);
    }

    #[tokio::test]
    async fn test_log_level_filtering() {
        let config = LoggerConfig::default();
        let logger = StructuredLogger::new(config);
        
        logger.log(LogLevel::Info, "test", "Info message", HashMap::new()).await;
        logger.log(LogLevel::Error, "test", "Error message", HashMap::new()).await;
        
        let error_logs = logger.get_logs_by_level(LogLevel::Error).await;
        assert_eq!(error_logs.len(), 1);
        assert_eq!(error_logs[0].message, "Error message");
    }

    #[tokio::test]
    async fn test_log_export() {
        let config = LoggerConfig::default();
        let logger = StructuredLogger::new(config);
        
        logger.log(LogLevel::Info, "test", "Export test", HashMap::new()).await;
        
        let exported = logger.export_logs().await;
        assert!(exported.contains("Export test"));
    }
}
