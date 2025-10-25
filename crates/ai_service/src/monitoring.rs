//! Production monitoring and observability for AI Service

use crate::errors::AIServiceError;
use crate::types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::{info, warn, error, debug, instrument};

/// Service health status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceStatus {
    /// Service is healthy
    Healthy,
    /// Service is degraded
    Degraded,
    /// Service is unhealthy
    Unhealthy,
    /// Service is starting
    Starting,
    /// Service is stopping
    Stopping,
}

/// Service metrics
#[derive(Debug, Clone, Default)]
pub struct ServiceMetrics {
    /// Total requests processed
    pub total_requests: AtomicU64,
    /// Successful requests
    pub successful_requests: AtomicU64,
    /// Failed requests
    pub failed_requests: AtomicU64,
    /// Active connections
    pub active_connections: AtomicUsize,
    /// Average response time (microseconds)
    pub avg_response_time_us: AtomicU64,
    /// Peak memory usage (bytes)
    pub peak_memory_usage: AtomicU64,
    /// Current memory usage (bytes)
    pub current_memory_usage: AtomicU64,
    /// Cache hit rate (0-100)
    pub cache_hit_rate: AtomicU64,
    /// Error rate (0-100)
    pub error_rate: AtomicU64,
}

/// Service monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Enable monitoring
    pub enabled: bool,
    /// Metrics collection interval
    pub metrics_interval: Duration,
    /// Health check interval
    pub health_check_interval: Duration,
    /// Memory threshold (bytes)
    pub memory_threshold: u64,
    /// CPU threshold (percentage)
    pub cpu_threshold: f64,
    /// Error rate threshold (percentage)
    pub error_rate_threshold: f64,
    /// Response time threshold (microseconds)
    pub response_time_threshold: u64,
    /// Enable alerting
    pub enable_alerting: bool,
    /// Alert cooldown period
    pub alert_cooldown: Duration,
    /// Enable metrics export
    pub enable_metrics_export: bool,
    /// Metrics export endpoint
    pub metrics_endpoint: Option<String>,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            metrics_interval: Duration::from_secs(60),
            health_check_interval: Duration::from_secs(30),
            memory_threshold: 1_000_000_000, // 1GB
            cpu_threshold: 80.0,
            error_rate_threshold: 5.0, // 5%
            response_time_threshold: 5_000_000, // 5 seconds
            enable_alerting: true,
            alert_cooldown: Duration::from_secs(300), // 5 minutes
            enable_metrics_export: false,
            metrics_endpoint: None,
        }
    }
}

/// Service monitor
pub struct ServiceMonitor {
    /// Service metrics
    metrics: Arc<ServiceMetrics>,
    /// Monitoring configuration
    config: MonitoringConfig,
    /// Service start time
    start_time: Instant,
    /// Last health check time
    last_health_check: Instant,
    /// Last alert time
    last_alert_time: Instant,
    /// Service status
    status: ServiceStatus,
    /// Alert handlers
    alert_handlers: Vec<Box<dyn AlertHandler + Send + Sync>>,
}

/// Alert handler trait
pub trait AlertHandler {
    /// Handle an alert
    fn handle_alert(&self, alert: &ServiceAlert) -> Result<(), AIServiceError>;
    /// Get handler name
    fn name(&self) -> &str;
}

/// Service alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceAlert {
    /// Alert ID
    pub id: String,
    /// Alert type
    pub alert_type: AlertType,
    /// Alert severity
    pub severity: AlertSeverity,
    /// Alert message
    pub message: String,
    /// Alert timestamp
    pub timestamp: u64,
    /// Alert metadata
    pub metadata: HashMap<String, String>,
}

/// Alert type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertType {
    /// High memory usage
    HighMemoryUsage,
    /// High CPU usage
    HighCpuUsage,
    /// High error rate
    HighErrorRate,
    /// Slow response time
    SlowResponseTime,
    /// Service unavailable
    ServiceUnavailable,
    /// Health check failed
    HealthCheckFailed,
    /// Custom alert
    Custom(String),
}

/// Alert severity
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
    /// Low severity
    Low,
    /// Medium severity
    Medium,
    /// High severity
    High,
    /// Critical severity
    Critical,
}

impl ServiceMonitor {
    /// Create a new service monitor
    pub fn new(config: MonitoringConfig) -> Self {
        Self {
            metrics: Arc::new(ServiceMetrics::default()),
            config,
            start_time: Instant::now(),
            last_health_check: Instant::now(),
            last_alert_time: Instant::now(),
            status: ServiceStatus::Starting,
            alert_handlers: Vec::new(),
        }
    }

    /// Register an alert handler
    pub fn register_alert_handler(&mut self, handler: Box<dyn AlertHandler + Send + Sync>) {
        self.alert_handlers.push(handler);
    }

    /// Record a request
    #[instrument(skip(self))]
    pub fn record_request(&self, success: bool, response_time_us: u64) {
        self.metrics.total_requests.fetch_add(1, Ordering::Relaxed);
        
        if success {
            self.metrics.successful_requests.fetch_add(1, Ordering::Relaxed);
        } else {
            self.metrics.failed_requests.fetch_add(1, Ordering::Relaxed);
        }

        // Update average response time
        let total = self.metrics.total_requests.load(Ordering::Relaxed);
        let current_avg = self.metrics.avg_response_time_us.load(Ordering::Relaxed);
        let new_avg = ((current_avg * (total - 1)) + response_time_us) / total;
        self.metrics.avg_response_time_us.store(new_avg, Ordering::Relaxed);

        // Update error rate
        let failed = self.metrics.failed_requests.load(Ordering::Relaxed);
        let error_rate = (failed * 100) / total;
        self.metrics.error_rate.store(error_rate, Ordering::Relaxed);

        debug!(
            "Recorded request: success={}, response_time_us={}, error_rate={}%",
            success, response_time_us, error_rate
        );
    }

    /// Record memory usage
    pub fn record_memory_usage(&self, memory_usage: u64) {
        self.metrics.current_memory_usage.store(memory_usage, Ordering::Relaxed);
        
        let current_peak = self.metrics.peak_memory_usage.load(Ordering::Relaxed);
        if memory_usage > current_peak {
            self.metrics.peak_memory_usage.store(memory_usage, Ordering::Relaxed);
        }
    }

    /// Record cache hit
    pub fn record_cache_hit(&self) {
        // Simplified cache hit rate calculation
        let total = self.metrics.total_requests.load(Ordering::Relaxed);
        let hits = self.metrics.cache_hit_rate.load(Ordering::Relaxed);
        let new_hits = hits + 1;
        let hit_rate = (new_hits * 100) / total;
        self.metrics.cache_hit_rate.store(hit_rate, Ordering::Relaxed);
    }

    /// Check service health
    #[instrument(skip(self))]
    pub async fn check_health(&mut self) -> ServiceStatus {
        let now = Instant::now();
        self.last_health_check = now;

        let mut status = ServiceStatus::Healthy;
        let mut alerts = Vec::new();

        // Check memory usage
        let memory_usage = self.metrics.current_memory_usage.load(Ordering::Relaxed);
        if memory_usage > self.config.memory_threshold {
            status = ServiceStatus::Unhealthy;
            alerts.push(ServiceAlert {
                id: format!("memory_{}", now.elapsed().as_secs()),
                alert_type: AlertType::HighMemoryUsage,
                severity: AlertSeverity::High,
                message: format!("Memory usage {} exceeds threshold {}", memory_usage, self.config.memory_threshold),
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert("memory_usage".to_string(), memory_usage.to_string());
                    meta.insert("threshold".to_string(), self.config.memory_threshold.to_string());
                    meta
                },
            });
        }

        // Check error rate
        let error_rate = self.metrics.error_rate.load(Ordering::Relaxed);
        if error_rate as f64 > self.config.error_rate_threshold {
            status = ServiceStatus::Degraded;
            alerts.push(ServiceAlert {
                id: format!("error_rate_{}", now.elapsed().as_secs()),
                alert_type: AlertType::HighErrorRate,
                severity: AlertSeverity::Medium,
                message: format!("Error rate {}% exceeds threshold {}%", error_rate, self.config.error_rate_threshold),
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert("error_rate".to_string(), error_rate.to_string());
                    meta.insert("threshold".to_string(), self.config.error_rate_threshold.to_string());
                    meta
                },
            });
        }

        // Check response time
        let avg_response_time = self.metrics.avg_response_time_us.load(Ordering::Relaxed);
        if avg_response_time > self.config.response_time_threshold {
            status = ServiceStatus::Degraded;
            alerts.push(ServiceAlert {
                id: format!("response_time_{}", now.elapsed().as_secs()),
                alert_type: AlertType::SlowResponseTime,
                severity: AlertSeverity::Medium,
                message: format!("Average response time {}μs exceeds threshold {}μs", avg_response_time, self.config.response_time_threshold),
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert("response_time".to_string(), avg_response_time.to_string());
                    meta.insert("threshold".to_string(), self.config.response_time_threshold.to_string());
                    meta
                },
            });
        }

        // Update status
        self.status = status.clone();

        // Handle alerts
        if self.config.enable_alerting && now.duration_since(self.last_alert_time) > self.config.alert_cooldown {
            for alert in alerts {
                self.handle_alert(alert).await;
            }
            self.last_alert_time = now;
        }

        debug!("Health check completed: status={:?}", status);
        status
    }

    /// Handle an alert
    async fn handle_alert(&self, alert: ServiceAlert) {
        for handler in &self.alert_handlers {
            if let Err(e) = handler.handle_alert(&alert) {
                error!("Alert handler {} failed: {}", handler.name(), e);
            }
        }
    }

    /// Get service status
    pub fn get_status(&self) -> ServiceStatus {
        self.status.clone()
    }

    /// Get service metrics
    pub fn get_metrics(&self) -> ServiceMetricsSnapshot {
        ServiceMetricsSnapshot {
            total_requests: self.metrics.total_requests.load(Ordering::Relaxed),
            successful_requests: self.metrics.successful_requests.load(Ordering::Relaxed),
            failed_requests: self.metrics.failed_requests.load(Ordering::Relaxed),
            active_connections: self.metrics.active_connections.load(Ordering::Relaxed),
            avg_response_time_us: self.metrics.avg_response_time_us.load(Ordering::Relaxed),
            peak_memory_usage: self.metrics.peak_memory_usage.load(Ordering::Relaxed),
            current_memory_usage: self.metrics.current_memory_usage.load(Ordering::Relaxed),
            cache_hit_rate: self.metrics.cache_hit_rate.load(Ordering::Relaxed),
            error_rate: self.metrics.error_rate.load(Ordering::Relaxed),
            uptime_seconds: self.start_time.elapsed().as_secs(),
        }
    }

    /// Get service health report
    pub async fn get_health_report(&mut self) -> ServiceHealthReport {
        let status = self.check_health().await;
        let metrics = self.get_metrics();

        ServiceHealthReport {
            status,
            metrics,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Start monitoring
    pub async fn start_monitoring(&mut self) -> Result<(), AIServiceError> {
        if !self.config.enabled {
            return Ok(());
        }

        info!("Starting service monitoring");
        
        // Start background monitoring task
        let metrics = self.metrics.clone();
        let config = self.config.clone();
        let mut monitor = self.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.metrics_interval);
            loop {
                interval.tick().await;
                
                // Collect metrics
                let memory_usage = get_memory_usage().unwrap_or(0);
                monitor.record_memory_usage(memory_usage);
                
                // Check health
                monitor.check_health().await;
                
                // Export metrics if enabled
                if config.enable_metrics_export {
                    if let Some(endpoint) = &config.metrics_endpoint {
                        if let Err(e) = export_metrics(&metrics, endpoint).await {
                            error!("Failed to export metrics: {}", e);
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop monitoring
    pub fn stop_monitoring(&mut self) {
        info!("Stopping service monitoring");
        self.status = ServiceStatus::Stopping;
    }
}

/// Service metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceMetricsSnapshot {
    /// Total requests processed
    pub total_requests: u64,
    /// Successful requests
    pub successful_requests: u64,
    /// Failed requests
    pub failed_requests: u64,
    /// Active connections
    pub active_connections: usize,
    /// Average response time (microseconds)
    pub avg_response_time_us: u64,
    /// Peak memory usage (bytes)
    pub peak_memory_usage: u64,
    /// Current memory usage (bytes)
    pub current_memory_usage: u64,
    /// Cache hit rate (0-100)
    pub cache_hit_rate: u64,
    /// Error rate (0-100)
    pub error_rate: u64,
    /// Service uptime (seconds)
    pub uptime_seconds: u64,
}

/// Service health report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealthReport {
    /// Service status
    pub status: ServiceStatus,
    /// Service metrics
    pub metrics: ServiceMetricsSnapshot,
    /// Report timestamp
    pub timestamp: u64,
    /// Service version
    pub version: String,
}

/// Get current memory usage
fn get_memory_usage() -> Result<u64, AIServiceError> {
    // Simplified implementation - in production, use proper memory monitoring
    Ok(100_000_000) // 100MB placeholder
}

/// Export metrics to external endpoint
async fn export_metrics(metrics: &ServiceMetrics, endpoint: &str) -> Result<(), AIServiceError> {
    // Simplified implementation - in production, use proper metrics export
    debug!("Exporting metrics to: {}", endpoint);
    Ok(())
}

impl Clone for ServiceMonitor {
    fn clone(&self) -> Self {
        Self {
            metrics: self.metrics.clone(),
            config: self.config.clone(),
            start_time: self.start_time,
            last_health_check: self.last_health_check,
            last_alert_time: self.last_alert_time,
            status: self.status.clone(),
            alert_handlers: Vec::new(), // Don't clone handlers
        }
    }
}

/// Console alert handler
pub struct ConsoleAlertHandler;

impl AlertHandler for ConsoleAlertHandler {
    fn handle_alert(&self, alert: &ServiceAlert) -> Result<(), AIServiceError> {
        match alert.severity {
            AlertSeverity::Critical => error!("CRITICAL ALERT: {}", alert.message),
            AlertSeverity::High => error!("HIGH ALERT: {}", alert.message),
            AlertSeverity::Medium => warn!("MEDIUM ALERT: {}", alert.message),
            AlertSeverity::Low => info!("LOW ALERT: {}", alert.message),
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "console"
    }
}

/// File alert handler
pub struct FileAlertHandler {
    file_path: String,
}

impl FileAlertHandler {
    pub fn new(file_path: String) -> Self {
        Self { file_path }
    }
}

impl AlertHandler for FileAlertHandler {
    fn handle_alert(&self, alert: &ServiceAlert) -> Result<(), AIServiceError> {
        let log_entry = format!(
            "[{}] {} - {}: {}\n",
            alert.timestamp,
            alert.severity,
            alert.alert_type,
            alert.message
        );

        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)
            .map_err(|e| AIServiceError::Io(format!("Failed to open alert log file: {}", e)))?
            .write_all(log_entry.as_bytes())
            .map_err(|e| AIServiceError::Io(format!("Failed to write alert log: {}", e)))?;

        Ok(())
    }

    fn name(&self) -> &str {
        "file"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_service_monitor_creation() {
        let config = MonitoringConfig::default();
        let monitor = ServiceMonitor::new(config);
        assert_eq!(monitor.get_status(), ServiceStatus::Starting);
    }

    #[tokio::test]
    async fn test_request_recording() {
        let config = MonitoringConfig::default();
        let monitor = ServiceMonitor::new(config);
        
        monitor.record_request(true, 1000);
        monitor.record_request(false, 2000);
        
        let metrics = monitor.get_metrics();
        assert_eq!(metrics.total_requests, 2);
        assert_eq!(metrics.successful_requests, 1);
        assert_eq!(metrics.failed_requests, 1);
    }

    #[tokio::test]
    async fn test_health_check() {
        let config = MonitoringConfig {
            memory_threshold: 50_000_000, // 50MB
            error_rate_threshold: 10.0,
            response_time_threshold: 1000,
            ..Default::default()
        };
        let mut monitor = ServiceMonitor::new(config);
        
        // Record high memory usage
        monitor.record_memory_usage(100_000_000); // 100MB
        
        let status = monitor.check_health().await;
        assert_eq!(status, ServiceStatus::Unhealthy);
    }

    #[tokio::test]
    async fn test_alert_handlers() {
        let config = MonitoringConfig::default();
        let mut monitor = ServiceMonitor::new(config);
        
        // Register console alert handler
        monitor.register_alert_handler(Box::new(ConsoleAlertHandler));
        
        // Record high error rate
        for _ in 0..10 {
            monitor.record_request(false, 1000);
        }
        
        let status = monitor.check_health().await;
        assert_eq!(status, ServiceStatus::Degraded);
    }

    #[tokio::test]
    async fn test_health_report() {
        let config = MonitoringConfig::default();
        let mut monitor = ServiceMonitor::new(config);
        
        monitor.record_request(true, 1000);
        let report = monitor.get_health_report().await;
        
        assert!(report.uptime_seconds >= 0);
        assert!(!report.version.is_empty());
    }
}
