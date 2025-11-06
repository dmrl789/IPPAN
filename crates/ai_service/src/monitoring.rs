//! Production monitoring and observability for AI Service

use crate::errors::AIServiceError;
use crate::types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info, instrument, warn};

/// Service health status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Starting,
    Stopping,
}

/// Service metrics
#[derive(Debug, Default)]
pub struct ServiceMetrics {
    pub total_requests: AtomicU64,
    pub successful_requests: AtomicU64,
    pub failed_requests: AtomicU64,
    pub active_connections: AtomicUsize,
    pub avg_response_time_us: AtomicU64,
    pub peak_memory_usage: AtomicU64,
    pub current_memory_usage: AtomicU64,
    pub cache_hit_rate: AtomicU64,
    pub error_rate: AtomicU64,
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub enabled: bool,
    pub metrics_interval: Duration,
    pub health_check_interval: Duration,
    pub memory_threshold: u64,
    pub cpu_threshold: f64,
    pub error_rate_threshold: f64,
    pub response_time_threshold: u64,
    pub enable_alerting: bool,
    pub alert_cooldown: Duration,
    pub enable_metrics_export: bool,
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
            error_rate_threshold: 5.0,
            response_time_threshold: 5_000_000, // 5s
            enable_alerting: true,
            alert_cooldown: Duration::from_secs(300),
            enable_metrics_export: false,
            metrics_endpoint: None,
        }
    }
}

/// Alert handler trait
pub trait AlertHandler {
    fn handle_alert(&self, alert: &ServiceAlert) -> Result<(), AIServiceError>;
    fn name(&self) -> &str;
}

/// Service alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceAlert {
    pub id: String,
    pub alert_type: AlertType,
    pub severity: AlertSeverity,
    pub message: String,
    pub timestamp: u64,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertType {
    HighMemoryUsage,
    HighCpuUsage,
    HighErrorRate,
    SlowResponseTime,
    ServiceUnavailable,
    HealthCheckFailed,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Core monitoring engine
pub struct ServiceMonitor {
    metrics: Arc<ServiceMetrics>,
    config: MonitoringConfig,
    start_time: Instant,
    last_health_check: Instant,
    last_alert_time: Instant,
    status: ServiceStatus,
    alert_handlers: Vec<Box<dyn AlertHandler + Send + Sync>>,
}

impl ServiceMonitor {
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

    pub fn register_alert_handler(&mut self, handler: Box<dyn AlertHandler + Send + Sync>) {
        self.alert_handlers.push(handler);
    }

    #[instrument(skip(self))]
    pub fn record_request(&self, success: bool, response_time_us: u64) {
        self.metrics.total_requests.fetch_add(1, Ordering::Relaxed);

        if success {
            self.metrics
                .successful_requests
                .fetch_add(1, Ordering::Relaxed);
        } else {
            self.metrics.failed_requests.fetch_add(1, Ordering::Relaxed);
        }

        let total = self.metrics.total_requests.load(Ordering::Relaxed);
        let current_avg = self.metrics.avg_response_time_us.load(Ordering::Relaxed);
        let new_avg = ((current_avg * (total - 1)) + response_time_us) / total;
        self.metrics
            .avg_response_time_us
            .store(new_avg, Ordering::Relaxed);

        let failed = self.metrics.failed_requests.load(Ordering::Relaxed);
        let error_rate = (failed * 100) / total;
        self.metrics.error_rate.store(error_rate, Ordering::Relaxed);

        debug!(
            "Recorded request: success={}, response_time_us={}, error_rate={}%",
            success, response_time_us, error_rate
        );
    }

    pub fn record_memory_usage(&self, memory_usage: u64) {
        self.metrics
            .current_memory_usage
            .store(memory_usage, Ordering::Relaxed);
        let current_peak = self.metrics.peak_memory_usage.load(Ordering::Relaxed);
        if memory_usage > current_peak {
            self.metrics
                .peak_memory_usage
                .store(memory_usage, Ordering::Relaxed);
        }
    }

    pub fn record_cache_hit(&self) {
        let total = self.metrics.total_requests.load(Ordering::Relaxed);
        let hits = self.metrics.cache_hit_rate.load(Ordering::Relaxed);
        if total > 0 {
            let hit_rate = ((hits + 1) * 100) / total;
            self.metrics
                .cache_hit_rate
                .store(hit_rate, Ordering::Relaxed);
        }
    }

    #[instrument(skip(self))]
    pub async fn check_health(&mut self) -> ServiceStatus {
        let now = Instant::now();
        self.last_health_check = now;

        let mut status = ServiceStatus::Healthy;
        let mut alerts = Vec::new();

        // Memory
        let mem = self.metrics.current_memory_usage.load(Ordering::Relaxed);
        if mem > self.config.memory_threshold {
            status = ServiceStatus::Unhealthy;
            alerts.push(ServiceAlert {
                id: format!("mem_{}", now.elapsed().as_secs()),
                alert_type: AlertType::HighMemoryUsage,
                severity: AlertSeverity::High,
                message: format!("Memory {} > {}", mem, self.config.memory_threshold),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                metadata: [("memory_usage".into(), mem.to_string())].into(),
            });
        }

        // Error rate
        let error_rate = self.metrics.error_rate.load(Ordering::Relaxed);
        if error_rate as f64 > self.config.error_rate_threshold {
            status = ServiceStatus::Degraded;
            alerts.push(ServiceAlert {
                id: format!("err_{}", now.elapsed().as_secs()),
                alert_type: AlertType::HighErrorRate,
                severity: AlertSeverity::Medium,
                message: format!(
                    "Error rate {}% > {}%",
                    error_rate, self.config.error_rate_threshold
                ),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                metadata: [("error_rate".into(), error_rate.to_string())].into(),
            });
        }

        // Response time
        let rt = self.metrics.avg_response_time_us.load(Ordering::Relaxed);
        if rt > self.config.response_time_threshold {
            status = ServiceStatus::Degraded;
            alerts.push(ServiceAlert {
                id: format!("rt_{}", now.elapsed().as_secs()),
                alert_type: AlertType::SlowResponseTime,
                severity: AlertSeverity::Medium,
                message: format!(
                    "Response time {}μs > {}μs",
                    rt, self.config.response_time_threshold
                ),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                metadata: [("response_time".into(), rt.to_string())].into(),
            });
        }

        self.status = status.clone();

        // Alert dispatch
        if self.config.enable_alerting
            && now.duration_since(self.last_alert_time) > self.config.alert_cooldown
        {
            for alert in alerts {
                self.handle_alert(alert).await;
            }
            self.last_alert_time = now;
        }

        status
    }

    async fn handle_alert(&self, alert: ServiceAlert) {
        for handler in &self.alert_handlers {
            if let Err(e) = handler.handle_alert(&alert) {
                error!("Alert handler {} failed: {}", handler.name(), e);
            }
        }
    }

    pub fn get_status(&self) -> ServiceStatus {
        self.status.clone()
    }

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

    pub async fn get_health_report(&mut self) -> ServiceHealthReport {
        let status = self.check_health().await;
        let metrics = self.get_metrics();
        ServiceHealthReport {
            status,
            metrics,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    pub async fn start_monitoring(&mut self) -> Result<(), AIServiceError> {
        if !self.config.enabled {
            return Ok(());
        }

        info!("Starting service monitoring");
        let metrics = self.metrics.clone();
        let config = self.config.clone();
        let mut monitor = self.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.metrics_interval);
            loop {
                interval.tick().await;
                let mem = get_memory_usage().unwrap_or(0);
                monitor.record_memory_usage(mem);
                monitor.check_health().await;

                if config.enable_metrics_export {
                    if let Some(endpoint) = &config.metrics_endpoint {
                        if let Err(e) = export_metrics(&metrics, endpoint).await {
                            error!("Metrics export failed: {}", e);
                        }
                    }
                }
            }
        });

        Ok(())
    }

    pub fn stop_monitoring(&mut self) {
        info!("Stopping service monitoring");
        self.status = ServiceStatus::Stopping;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceMetricsSnapshot {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub active_connections: usize,
    pub avg_response_time_us: u64,
    pub peak_memory_usage: u64,
    pub current_memory_usage: u64,
    pub cache_hit_rate: u64,
    pub error_rate: u64,
    pub uptime_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealthReport {
    pub status: ServiceStatus,
    pub metrics: ServiceMetricsSnapshot,
    pub timestamp: u64,
    pub version: String,
}

fn get_memory_usage() -> Result<u64, AIServiceError> {
    #[cfg(target_os = "linux")]
    {
        // Read from /proc/self/status for most accurate RSS measurement
        use std::fs;
        if let Ok(status) = fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    // Format: "VmRSS:      123456 kB"
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Ok(kb) = parts[1].parse::<u64>() {
                            return Ok(kb * 1024); // Convert KB to bytes
                        }
                    }
                }
            }
        }
    }

    // Fallback: use sysinfo crate
    use sysinfo::{ProcessExt, ProcessRefreshKind, System, SystemExt};
    let mut sys = System::new();
    if let Ok(pid) = sysinfo::get_current_pid() {
        sys.refresh_process_specifics(pid, ProcessRefreshKind::new());
        if let Some(process) = sys.process(pid) {
            return Ok(process.memory());
        }
    }

    // Ultimate fallback
    Ok(100_000_000)
}

async fn export_metrics(_metrics: &ServiceMetrics, endpoint: &str) -> Result<(), AIServiceError> {
    debug!("Exporting metrics to {}", endpoint);
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
            alert_handlers: Vec::new(),
        }
    }
}

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
        let entry = format!(
            "[{}] {:?} {:?}: {}\n",
            alert.timestamp, alert.severity, alert.alert_type, alert.message
        );

        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)
            .map_err(|e| AIServiceError::Io(format!("Failed to open alert file: {}", e)))?
            .write_all(entry.as_bytes())
            .map_err(|e| AIServiceError::Io(format!("Failed to write alert: {}", e)))?;
        Ok(())
    }
    fn name(&self) -> &str {
        "file"
    }
}

/// High-level monitoring service that wraps ServiceMonitor
#[derive(Clone)]
pub struct MonitoringService {
    monitor: ServiceMonitor,
    alerts: Vec<MonitoringAlert>,
    metrics_store: HashMap<String, Vec<f64>>,
}

impl MonitoringService {
    pub fn new(config: MonitoringConfig) -> Self {
        Self {
            monitor: ServiceMonitor::new(config),
            alerts: Vec::new(),
            metrics_store: HashMap::new(),
        }
    }

    pub fn add_metric(&mut self, metric_name: String, value: f64) {
        self.metrics_store
            .entry(metric_name.clone())
            .or_default()
            .push(value);

        // Also record in the underlying monitor
        if metric_name == "memory_usage" {
            self.monitor.record_memory_usage(value as u64);
        }
    }

    pub async fn check_alerts(&mut self) -> Result<Vec<MonitoringAlert>, AIServiceError> {
        let _status = self.monitor.check_health().await;
        let mut new_alerts = Vec::new();

        // Check for high memory usage
        if let Some(memory_values) = self.metrics_store.get("memory_usage") {
            if let Some(&latest) = memory_values.last() {
                if latest > 80.0 {
                    let alert = MonitoringAlert {
                        alert_id: format!("memory_{}", chrono::Utc::now().timestamp()),
                        alert_type: "high_memory_usage".to_string(),
                        severity: SeverityLevel::High,
                        title: "High Memory Usage".to_string(),
                        description: format!("Memory usage is at {:.1}%", latest),
                        metrics: [("memory_usage".to_string(), latest)].into(),
                        timestamp: chrono::Utc::now(),
                        status: AlertStatus::Active,
                        actions_taken: Vec::new(),
                    };
                    new_alerts.push(alert);
                }
            }
        }

        // Check for high CPU usage
        if let Some(cpu_values) = self.metrics_store.get("cpu_usage") {
            if let Some(&latest) = cpu_values.last() {
                if latest > 90.0 {
                    let alert = MonitoringAlert {
                        alert_id: format!("cpu_{}", chrono::Utc::now().timestamp()),
                        alert_type: "high_cpu_usage".to_string(),
                        severity: SeverityLevel::High,
                        title: "High CPU Usage".to_string(),
                        description: format!("CPU usage is at {:.1}%", latest),
                        metrics: [("cpu_usage".to_string(), latest)].into(),
                        timestamp: chrono::Utc::now(),
                        status: AlertStatus::Active,
                        actions_taken: Vec::new(),
                    };
                    new_alerts.push(alert);
                }
            }
        }

        self.alerts.extend(new_alerts.clone());
        Ok(new_alerts)
    }

    pub fn get_alerts(&self) -> &[MonitoringAlert] {
        &self.alerts
    }

    pub fn acknowledge_alert(&mut self, alert_id: &str) -> Result<(), AIServiceError> {
        if let Some(alert) = self.alerts.iter_mut().find(|a| a.alert_id == alert_id) {
            alert.status = AlertStatus::Acknowledged;
            Ok(())
        } else {
            Err(AIServiceError::ValidationError(format!(
                "Alert {} not found",
                alert_id
            )))
        }
    }

    pub fn resolve_alert(
        &mut self,
        alert_id: &str,
        resolution: String,
    ) -> Result<(), AIServiceError> {
        if let Some(alert) = self.alerts.iter_mut().find(|a| a.alert_id == alert_id) {
            alert.status = AlertStatus::Resolved;
            alert.actions_taken.push(resolution);
            Ok(())
        } else {
            Err(AIServiceError::ValidationError(format!(
                "Alert {} not found",
                alert_id
            )))
        }
    }

    pub fn get_statistics(&self) -> MonitoringStatistics {
        let total_metrics = self.metrics_store.values().map(|v| v.len()).sum();
        MonitoringStatistics {
            metrics_count: self.metrics_store.len(),
            total_data_points: total_metrics,
            active_alerts: self
                .alerts
                .iter()
                .filter(|a| a.status == AlertStatus::Active)
                .count(),
            resolved_alerts: self
                .alerts
                .iter()
                .filter(|a| a.status == AlertStatus::Resolved)
                .count(),
        }
    }
}

/// Monitoring statistics
#[derive(Debug, Clone)]
pub struct MonitoringStatistics {
    pub metrics_count: usize,
    pub total_data_points: usize,
    pub active_alerts: usize,
    pub resolved_alerts: usize,
}
