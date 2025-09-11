//! Enhanced structured logger for IPPAN
//! 
//! Provides comprehensive logging with structured data, error tracking, and performance monitoring

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{error, info, warn, debug, Level};
use tracing_subscriber::{
    fmt::{format::FmtSpan},
    EnvFilter, FmtSubscriber,
};

/// Log entry with structured data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub target: String,
    pub message: String,
    pub fields: HashMap<String, serde_json::Value>,
    pub error_details: Option<ErrorDetails>,
    pub performance_metrics: Option<LogPerformanceMetrics>,
    pub correlation_id: Option<String>,
    pub session_id: Option<String>,
    pub user_id: Option<String>,
    pub request_id: Option<String>,
}

/// Log level enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

/// Error details for structured error logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetails {
    pub error_type: String,
    pub error_code: Option<String>,
    pub stack_trace: Option<String>,
    pub context: HashMap<String, serde_json::Value>,
    pub severity: ErrorSeverity,
    pub recovery_action: Option<String>,
    pub affected_components: Vec<String>,
}

/// Error severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
    Fatal,
}

/// Performance metrics for logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogPerformanceMetrics {
    pub duration_ms: f64,
    pub memory_usage_bytes: Option<u64>,
    pub cpu_usage_percent: Option<f64>,
    pub operation_count: Option<u64>,
    pub throughput_ops_per_sec: Option<f64>,
    pub cache_hit_rate: Option<f64>,
    pub network_latency_ms: Option<f64>,
}

/// Logger configuration
#[derive(Debug, Clone)]
pub struct LoggerConfig {
    pub log_level: Level,
    pub enable_structured_logging: bool,
    pub enable_error_tracking: bool,
    pub enable_performance_tracking: bool,
    pub enable_correlation_tracking: bool,
    pub max_log_entries: usize,
    pub log_file_path: Option<String>,
    pub enable_console_output: bool,
    pub enable_json_output: bool,
    pub enable_file_rotation: bool,
    pub max_file_size_mb: u64,
    pub max_files: u32,
    pub compression_enabled: bool,
    pub encryption_enabled: bool,
    pub retention_days: u32,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            log_level: Level::INFO,
            enable_structured_logging: true,
            enable_error_tracking: true,
            enable_performance_tracking: true,
            enable_correlation_tracking: true,
            max_log_entries: 100_000,
            log_file_path: Some("logs/ippan.log".to_string()),
            enable_console_output: true,
            enable_json_output: true,
            enable_file_rotation: true,
            max_file_size_mb: 100,
            max_files: 10,
            compression_enabled: true,
            encryption_enabled: false,
            retention_days: 30,
        }
    }
}

/// Error tracking system
#[derive(Debug, Clone)]
pub struct ErrorTracker {
    pub errors: Vec<ErrorDetails>,
    pub error_counts: HashMap<String, u64>,
    pub critical_errors: Vec<ErrorDetails>,
    pub error_trends: HashMap<String, Vec<u64>>,
    pub recovery_actions: HashMap<String, u64>,
}

/// Performance tracking system
#[derive(Debug, Clone)]
pub struct PerformanceTracker {
    pub slow_operations: Vec<LogPerformanceMetrics>,
    pub operation_stats: HashMap<String, OperationStats>,
    pub performance_trends: HashMap<String, Vec<f64>>,
    pub bottleneck_analysis: Vec<BottleneckAnalysis>,
}

/// Operation statistics
#[derive(Debug, Clone)]
pub struct OperationStats {
    pub count: u64,
    pub total_duration_ms: f64,
    pub avg_duration_ms: f64,
    pub min_duration_ms: f64,
    pub max_duration_ms: f64,
    pub p50_duration_ms: f64,
    pub p95_duration_ms: f64,
    pub p99_duration_ms: f64,
    pub error_rate: f64,
    pub throughput_ops_per_sec: f64,
}

/// Bottleneck analysis
#[derive(Debug, Clone)]
pub struct BottleneckAnalysis {
    pub operation_name: String,
    pub bottleneck_type: BottleneckType,
    pub severity: f64,
    pub recommendations: Vec<String>,
    pub detected_at: DateTime<Utc>,
}

/// Bottleneck types
#[derive(Debug, Clone)]
pub enum BottleneckType {
    CPU,
    Memory,
    Network,
    Disk,
    Database,
    Cache,
    Lock,
    Custom(String),
}

/// Logger statistics
#[derive(Debug, Clone)]
pub struct LoggerStatistics {
    pub total_logs: u64,
    pub logs_by_level: HashMap<LogLevel, u64>,
    pub total_errors: u64,
    pub critical_errors: u64,
    pub total_performance_measurements: u64,
    pub slow_operations_count: u64,
    pub average_log_size_bytes: f64,
    pub logs_per_second: f64,
    pub uptime_seconds: u64,
    pub memory_usage_bytes: u64,
    pub disk_usage_bytes: u64,
}

/// Enhanced structured logger
pub struct StructuredLogger {
    logs: Arc<RwLock<Vec<LogEntry>>>,
    error_tracker: Arc<RwLock<ErrorTracker>>,
    performance_tracker: Arc<RwLock<PerformanceTracker>>,
    config: LoggerConfig,
    start_time: Instant,
    system_start_time: SystemTime,
    correlation_context: Arc<RwLock<HashMap<String, String>>>,
}

impl StructuredLogger {
    /// Create a new structured logger
    pub fn new() -> Self {
        Self {
            logs: Arc::new(RwLock::new(Vec::new())),
            error_tracker: Arc::new(RwLock::new(ErrorTracker {
                errors: Vec::new(),
                error_counts: HashMap::new(),
                critical_errors: Vec::new(),
                error_trends: HashMap::new(),
                recovery_actions: HashMap::new(),
            })),
            performance_tracker: Arc::new(RwLock::new(PerformanceTracker {
                slow_operations: Vec::new(),
                operation_stats: HashMap::new(),
                performance_trends: HashMap::new(),
                bottleneck_analysis: Vec::new(),
            })),
            config: LoggerConfig::default(),
            start_time: Instant::now(),
            system_start_time: SystemTime::now(),
            correlation_context: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new structured logger with custom configuration
    pub fn with_config(config: LoggerConfig) -> Self {
        Self {
            logs: Arc::new(RwLock::new(Vec::new())),
            error_tracker: Arc::new(RwLock::new(ErrorTracker {
                errors: Vec::new(),
                error_counts: HashMap::new(),
                critical_errors: Vec::new(),
                error_trends: HashMap::new(),
                recovery_actions: HashMap::new(),
            })),
            performance_tracker: Arc::new(RwLock::new(PerformanceTracker {
                slow_operations: Vec::new(),
                operation_stats: HashMap::new(),
                performance_trends: HashMap::new(),
                bottleneck_analysis: Vec::new(),
            })),
            config,
            start_time: Instant::now(),
            system_start_time: SystemTime::now(),
            correlation_context: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start the structured logger
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.config.enable_console_output {
            let subscriber = FmtSubscriber::builder()
                .with_env_filter(EnvFilter::from_default_env())
                .with_span_events(FmtSpan::CLOSE)
                .with_file(true)
                .with_line_number(true)
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_target(false)
                .with_level(true)
                .with_ansi(true)
                .pretty()
                .finish();

            // Only set global default if none is already set
            static GLOBAL_DEFAULT_SET: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
            if !GLOBAL_DEFAULT_SET.swap(true, std::sync::atomic::Ordering::SeqCst) {
                tracing::subscriber::set_global_default(subscriber)?;
            }
        }
        
        self.log(LogLevel::Info, "structured_logger", "Structured logging system initialized", HashMap::new()).await;
        Ok(())
    }

    /// Stop the structured logger
    pub async fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.log(LogLevel::Info, "structured_logger", "Structured logging system shutting down", HashMap::new()).await;
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
            id: format!("log_{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()),
            timestamp: Utc::now(),
            level: level.clone(),
            target: target.to_string(),
            message: message.to_string(),
            fields,
            error_details: None,
            performance_metrics: None,
            correlation_id: self.get_correlation_id().await,
            session_id: self.get_session_id().await,
            user_id: self.get_user_id().await,
            request_id: self.get_request_id().await,
        };

        // Log to tracing
        match level {
            LogLevel::Trace => debug!("{}", message),
            LogLevel::Debug => debug!("{}", message),
            LogLevel::Info => info!("{}", message),
            LogLevel::Warn => warn!("{}", message),
            LogLevel::Error => error!("{}", message),
            LogLevel::Fatal => error!("FATAL: {}", message),
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
        let mut entry_fields = fields;
        entry_fields.insert("error_type".to_string(), serde_json::Value::String(error_details.error_type.clone()));
        entry_fields.insert("error_severity".to_string(), serde_json::Value::String(format!("{:?}", error_details.severity)));

        let entry = LogEntry {
            id: format!("log_{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()),
            timestamp: Utc::now(),
            level: LogLevel::Error,
            target: target.to_string(),
            message: message.to_string(),
            fields: entry_fields,
            error_details: Some(error_details.clone()),
            performance_metrics: None,
            correlation_id: self.get_correlation_id().await,
            session_id: self.get_session_id().await,
            user_id: self.get_user_id().await,
            request_id: self.get_request_id().await,
        };

        // Log to tracing
        error!("{}: {}", message, error_details.error_type);

        // Store in structured logs
        if self.config.enable_structured_logging {
            let mut logs = self.logs.write().await;
            logs.push(entry);
            
            // Maintain log size limit
            if logs.len() > self.config.max_log_entries {
                logs.remove(0);
            }
        }

        // Track error
        if self.config.enable_error_tracking {
            self.track_error(error_details).await;
        }
    }

    /// Log performance metrics
    pub async fn log_performance(
        &self,
        target: &str,
        message: &str,
        performance_metrics: LogPerformanceMetrics,
        fields: HashMap<String, serde_json::Value>,
    ) {
        let mut entry_fields = fields;
        entry_fields.insert("duration_ms".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(performance_metrics.duration_ms).unwrap()));
        entry_fields.insert("operation_count".to_string(), serde_json::Value::Number(serde_json::Number::from(performance_metrics.operation_count.unwrap_or(1))));

        let entry = LogEntry {
            id: format!("log_{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()),
            timestamp: Utc::now(),
            level: LogLevel::Info,
            target: target.to_string(),
            message: message.to_string(),
            fields: entry_fields,
            error_details: None,
            performance_metrics: Some(performance_metrics.clone()),
            correlation_id: self.get_correlation_id().await,
            session_id: self.get_session_id().await,
            user_id: self.get_user_id().await,
            request_id: self.get_request_id().await,
        };

        // Log to tracing
        info!("{}: {}ms", message, performance_metrics.duration_ms);

        // Store in structured logs
        if self.config.enable_structured_logging {
            let mut logs = self.logs.write().await;
            logs.push(entry);
            
            // Maintain log size limit
            if logs.len() > self.config.max_log_entries {
                logs.remove(0);
            }
        }

        // Track performance
        if self.config.enable_performance_tracking {
            self.track_performance(target, performance_metrics).await;
        }
    }

    /// Set correlation context
    pub async fn set_correlation_context(&self, context: HashMap<String, String>) {
        let mut correlation_context = self.correlation_context.write().await;
        *correlation_context = context;
    }

    /// Get correlation ID
    async fn get_correlation_id(&self) -> Option<String> {
        let correlation_context = self.correlation_context.read().await;
        correlation_context.get("correlation_id").cloned()
    }

    /// Get session ID
    async fn get_session_id(&self) -> Option<String> {
        let correlation_context = self.correlation_context.read().await;
        correlation_context.get("session_id").cloned()
    }

    /// Get user ID
    async fn get_user_id(&self) -> Option<String> {
        let correlation_context = self.correlation_context.read().await;
        correlation_context.get("user_id").cloned()
    }

    /// Get request ID
    async fn get_request_id(&self) -> Option<String> {
        let correlation_context = self.correlation_context.read().await;
        correlation_context.get("request_id").cloned()
    }

    /// Track error
    async fn track_error(&self, error_details: ErrorDetails) {
        let mut error_tracker = self.error_tracker.write().await;
        
        // Add to errors list
        error_tracker.errors.push(error_details.clone());
        
        // Update error counts
        *error_tracker.error_counts.entry(error_details.error_type.clone()).or_insert(0) += 1;
        
        // Track critical errors
        if matches!(error_details.severity, ErrorSeverity::Critical | ErrorSeverity::Fatal) {
            error_tracker.critical_errors.push(error_details.clone());
        }
        
        // Update error trends
        let now = Utc::now().timestamp() as u64;
        let minute_key = now / 60; // Group by minute
        *error_tracker.error_trends.entry(error_details.error_type.clone()).or_insert_with(Vec::new).last_mut().unwrap_or(&mut 0) += 1;
        
        // Track recovery actions
        if let Some(recovery_action) = &error_details.recovery_action {
            *error_tracker.recovery_actions.entry(recovery_action.clone()).or_insert(0) += 1;
        }
    }

    /// Track performance
    async fn track_performance(&self, operation_name: &str, metrics: LogPerformanceMetrics) {
        let mut performance_tracker = self.performance_tracker.write().await;
        
        // Update operation stats
        let stats = performance_tracker.operation_stats.entry(operation_name.to_string()).or_insert(OperationStats {
            count: 0,
            total_duration_ms: 0.0,
            avg_duration_ms: 0.0,
            min_duration_ms: f64::MAX,
            max_duration_ms: 0.0,
            p50_duration_ms: 0.0,
            p95_duration_ms: 0.0,
            p99_duration_ms: 0.0,
            error_rate: 0.0,
            throughput_ops_per_sec: 0.0,
        });
        
        stats.count += 1;
        stats.total_duration_ms += metrics.duration_ms;
        stats.avg_duration_ms = stats.total_duration_ms / stats.count as f64;
        stats.min_duration_ms = stats.min_duration_ms.min(metrics.duration_ms);
        stats.max_duration_ms = stats.max_duration_ms.max(metrics.duration_ms);
        
        if let Some(throughput) = metrics.throughput_ops_per_sec {
            stats.throughput_ops_per_sec = throughput;
        }
        
        // Track slow operations
        if metrics.duration_ms > 1000.0 { // Operations taking more than 1 second
            performance_tracker.slow_operations.push(metrics.clone());
            
            // Maintain slow operations list size
            if performance_tracker.slow_operations.len() > 1000 {
                performance_tracker.slow_operations.remove(0);
            }
        }
        
        // Update performance trends
        let now = Utc::now().timestamp() as u64;
        let minute_key = now / 60; // Group by minute
        performance_tracker.performance_trends.entry(operation_name.to_string()).or_insert_with(Vec::new).push(metrics.duration_ms);
        
        // Maintain trend history (keep last 1440 minutes = 24 hours)
        if let Some(trend) = performance_tracker.performance_trends.get_mut(operation_name) {
            if trend.len() > 1440 {
                trend.remove(0);
            }
        }
        
        // Analyze bottlenecks
        self.analyze_bottlenecks(operation_name, &metrics).await;
    }

    /// Analyze bottlenecks
    async fn analyze_bottlenecks(&self, operation_name: &str, metrics: &LogPerformanceMetrics) {
        let mut performance_tracker = self.performance_tracker.write().await;
        
        // Simple bottleneck detection logic
        let mut bottlenecks = Vec::new();
        
        if let Some(cpu_usage) = metrics.cpu_usage_percent {
            if cpu_usage > 80.0 {
                bottlenecks.push(BottleneckAnalysis {
                    operation_name: operation_name.to_string(),
                    bottleneck_type: BottleneckType::CPU,
                    severity: cpu_usage / 100.0,
                    recommendations: vec![
                        "Consider CPU optimization".to_string(),
                        "Check for CPU-intensive operations".to_string(),
                        "Consider parallel processing".to_string(),
                    ],
                    detected_at: Utc::now(),
                });
            }
        }
        
        if let Some(memory_usage) = metrics.memory_usage_bytes {
            if memory_usage > 1024 * 1024 * 1024 { // 1GB
                bottlenecks.push(BottleneckAnalysis {
                    operation_name: operation_name.to_string(),
                    bottleneck_type: BottleneckType::Memory,
                    severity: memory_usage as f64 / (1024.0 * 1024.0 * 1024.0),
                    recommendations: vec![
                        "Consider memory optimization".to_string(),
                        "Check for memory leaks".to_string(),
                        "Consider garbage collection tuning".to_string(),
                    ],
                    detected_at: Utc::now(),
                });
            }
        }
        
        if let Some(network_latency) = metrics.network_latency_ms {
            if network_latency > 1000.0 { // 1 second
                bottlenecks.push(BottleneckAnalysis {
                    operation_name: operation_name.to_string(),
                    bottleneck_type: BottleneckType::Network,
                    severity: network_latency / 1000.0,
                    recommendations: vec![
                        "Check network connectivity".to_string(),
                        "Consider network optimization".to_string(),
                        "Check for network congestion".to_string(),
                    ],
                    detected_at: Utc::now(),
                });
            }
        }
        
        // Add bottlenecks to analysis
        performance_tracker.bottleneck_analysis.extend(bottlenecks);
        
        // Maintain bottleneck analysis list size
        if performance_tracker.bottleneck_analysis.len() > 100 {
            performance_tracker.bottleneck_analysis.remove(0);
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
        logs.iter().filter(|log| log.level == level).cloned().collect()
    }

    /// Get logs by target
    pub async fn get_logs_by_target(&self, target: &str) -> Vec<LogEntry> {
        let logs = self.logs.read().await;
        logs.iter().filter(|log| log.target == target).cloned().collect()
    }

    /// Get logs by time range
    pub async fn get_logs_by_time_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Vec<LogEntry> {
        let logs = self.logs.read().await;
        logs.iter().filter(|log| log.timestamp >= start && log.timestamp <= end).cloned().collect()
    }

    /// Get error tracker
    pub async fn get_error_tracker(&self) -> ErrorTracker {
        let error_tracker = self.error_tracker.read().await;
        error_tracker.clone()
    }

    /// Get performance tracker
    pub async fn get_performance_tracker(&self) -> PerformanceTracker {
        let performance_tracker = self.performance_tracker.read().await;
        performance_tracker.clone()
    }

    /// Get logger statistics
    pub async fn get_statistics(&self) -> LoggerStatistics {
        let logs = self.logs.read().await;
        let error_tracker = self.error_tracker.read().await;
        let performance_tracker = self.performance_tracker.read().await;
        
        let mut logs_by_level = HashMap::new();
        let mut total_log_size = 0u64;
        
        for log in logs.iter() {
            *logs_by_level.entry(log.level.clone()).or_insert(0) += 1;
            total_log_size += serde_json::to_string(log).unwrap_or_default().len() as u64;
        }
        
        let uptime = self.start_time.elapsed().as_secs();
        let logs_per_second = if uptime > 0 { logs.len() as f64 / uptime as f64 } else { 0.0 };
        
        LoggerStatistics {
            total_logs: logs.len() as u64,
            logs_by_level,
            total_errors: error_tracker.errors.len() as u64,
            critical_errors: error_tracker.critical_errors.len() as u64,
            total_performance_measurements: performance_tracker.slow_operations.len() as u64,
            slow_operations_count: performance_tracker.slow_operations.len() as u64,
            average_log_size_bytes: if logs.is_empty() { 0.0 } else { total_log_size as f64 / logs.len() as f64 },
            logs_per_second,
            uptime_seconds: uptime,
            memory_usage_bytes: 0, // TODO: Implement actual memory usage tracking
            disk_usage_bytes: 0, // TODO: Implement actual disk usage tracking
        }
    }

    /// Clear old logs
    pub async fn clear_old_logs(&self, max_age: Duration) {
        let mut logs = self.logs.write().await;
        let cutoff_time = Utc::now() - chrono::Duration::from_std(max_age).unwrap();
        logs.retain(|log| log.timestamp > cutoff_time);
    }

    /// Export logs to JSON
    pub async fn export_logs_json(&self) -> String {
        let logs = self.logs.read().await;
        serde_json::to_string_pretty(&*logs).unwrap_or_default()
    }

    /// Export logs to CSV
    pub async fn export_logs_csv(&self) -> String {
        let logs = self.logs.read().await;
        let mut csv = String::from("timestamp,level,target,message,error_type,error_severity,duration_ms\n");
        
        for log in logs.iter() {
            let error_type = log.error_details.as_ref().map(|e| e.error_type.as_str()).unwrap_or("");
            let error_severity = log.error_details.as_ref().map(|e| format!("{:?}", e.severity)).unwrap_or_default();
            let duration_ms = log.performance_metrics.as_ref().map(|p| p.duration_ms.to_string()).unwrap_or_default();
            
            csv.push_str(&format!(
                "{},{},{},{},{},{},{}\n",
                log.timestamp.to_rfc3339(),
                format!("{:?}", log.level),
                log.target,
                log.message.replace(',', ";"),
                error_type,
                error_severity,
                duration_ms
            ));
        }
        
        csv
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_structured_logger_creation() {
        let logger = StructuredLogger::new();
        let stats = logger.get_statistics().await;
        assert_eq!(stats.total_logs, 0);
    }

    #[tokio::test]
    async fn test_log_message() {
        let logger = StructuredLogger::new();
        // Don't call start() to avoid initialization log
        
        let mut fields = HashMap::new();
        fields.insert("test_field".to_string(), serde_json::Value::String("test_value".to_string()));
        
        logger.log(LogLevel::Info, "test", "Test message", fields).await;
        
        let stats = logger.get_statistics().await;
        assert_eq!(stats.total_logs, 1);
    }

    #[tokio::test]
    async fn test_log_error() {
        let logger = StructuredLogger::new();
        // Don't call start() to avoid initialization log
        
        let error_details = ErrorDetails {
            error_type: "TestError".to_string(),
            error_code: Some("TEST001".to_string()),
            stack_trace: Some("test stack trace".to_string()),
            context: HashMap::new(),
            severity: ErrorSeverity::High,
            recovery_action: Some("Restart service".to_string()),
            affected_components: vec!["test_component".to_string()],
        };
        
        logger.log_error("test", "Test error", error_details, HashMap::new()).await;
        
        let stats = logger.get_statistics().await;
        assert_eq!(stats.total_errors, 1);
    }

    #[tokio::test]
    async fn test_log_performance() {
        let logger = StructuredLogger::new();
        // Don't call start() to avoid initialization log
        
        let performance_metrics = LogPerformanceMetrics {
            duration_ms: 150.0,
            memory_usage_bytes: Some(1024 * 1024),
            cpu_usage_percent: Some(25.0),
            operation_count: Some(10),
            throughput_ops_per_sec: Some(66.67),
            cache_hit_rate: Some(0.95),
            network_latency_ms: Some(50.0),
        };
        
        logger.log_performance("test", "Test performance", performance_metrics, HashMap::new()).await;
        
        let stats = logger.get_statistics().await;
        assert_eq!(stats.total_performance_measurements, 1);
    }

    #[tokio::test]
    async fn test_correlation_context() {
        let logger = StructuredLogger::new();
        // Don't call start() to avoid initialization log
        
        let mut context = HashMap::new();
        context.insert("correlation_id".to_string(), "test_correlation_123".to_string());
        context.insert("session_id".to_string(), "test_session_456".to_string());
        context.insert("user_id".to_string(), "test_user_789".to_string());
        
        logger.set_correlation_context(context).await;
        
        logger.log(LogLevel::Info, "test", "Test message with context", HashMap::new()).await;
        
        let logs = logger.get_logs().await;
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].correlation_id, Some("test_correlation_123".to_string()));
        assert_eq!(logs[0].session_id, Some("test_session_456".to_string()));
        assert_eq!(logs[0].user_id, Some("test_user_789".to_string()));
    }
}
