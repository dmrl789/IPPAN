//! Monitoring system for AI Core
//!
//! Provides performance, reliability, and health tracking for deterministic AI modules,
//! with configurable alert thresholds and in-memory metrics storage.
//!
//! Features:
//! - CPU, memory, execution time, and error rate tracking
//! - Configurable alert thresholds
//! - Retention management for historical metrics
//! - Compatible with consensus-level deterministic AI evaluation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Enable monitoring
    pub enabled: bool,
    /// Enable performance monitoring
    pub enable_performance_monitoring: bool,
    /// Monitoring interval (seconds)
    pub interval_seconds: u64,
    /// Metrics retention period (days)
    pub retention_days: u32,
    /// Alert thresholds
    pub alert_thresholds: AlertThresholds,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            enable_performance_monitoring: true,
            interval_seconds: 60,
            retention_days: 7,
            alert_thresholds: AlertThresholds::default(),
        }
    }
}

/// Alert thresholds for monitored metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    /// CPU usage threshold (percentage)
    pub cpu_usage: f64,
    /// Memory usage threshold (percentage)
    pub memory_usage: f64,
    /// Execution time threshold (milliseconds)
    pub execution_time_ms: u64,
    /// Error rate threshold (percentage)
    pub error_rate: f64,
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            cpu_usage: 80.0,
            memory_usage: 85.0,
            execution_time_ms: 5000,
            error_rate: 5.0,
        }
    }
}

/// Runtime metric record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricValue {
    pub value: f64,
    pub timestamp: u64,
    pub tags: HashMap<String, String>,
}

/// Alert event structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub metric_name: String,
    pub value: f64,
    pub threshold: f64,
    pub severity: AlertSeverity,
}

/// Severity levels for alerts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

/// Monitoring system
pub struct MonitoringSystem {
    config: MonitoringConfig,
    metrics: HashMap<String, MetricValue>,
}

impl MonitoringSystem {
    /// Create a new monitoring system
    pub fn new(config: MonitoringConfig) -> Self {
        Self {
            config,
            metrics: HashMap::new(),
        }
    }

    /// Record a new metric
    pub fn record_metric(&mut self, name: String, value: f64, tags: HashMap<String, String>) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.metrics.insert(
            name,
            MetricValue {
                value,
                timestamp,
                tags,
            },
        );
    }

    /// Retrieve a specific metric by name
    pub fn get_metric(&self, name: &str) -> Option<&MetricValue> {
        self.metrics.get(name)
    }

    /// Retrieve all currently tracked metrics
    pub fn get_all_metrics(&self) -> &HashMap<String, MetricValue> {
        &self.metrics
    }

    /// Check for alert conditions and return triggered alerts
    pub fn check_alerts(&self) -> Vec<Alert> {
        let mut alerts = Vec::new();

        for (name, metric) in &self.metrics {
            match name.as_str() {
                "cpu_usage" if metric.value > self.config.alert_thresholds.cpu_usage => {
                    alerts.push(Alert {
                        metric_name: name.clone(),
                        value: metric.value,
                        threshold: self.config.alert_thresholds.cpu_usage,
                        severity: AlertSeverity::Warning,
                    });
                }
                "memory_usage" if metric.value > self.config.alert_thresholds.memory_usage => {
                    alerts.push(Alert {
                        metric_name: name.clone(),
                        value: metric.value,
                        threshold: self.config.alert_thresholds.memory_usage,
                        severity: AlertSeverity::Warning,
                    });
                }
                "execution_time_ms"
                    if metric.value > self.config.alert_thresholds.execution_time_ms as f64 =>
                {
                    alerts.push(Alert {
                        metric_name: name.clone(),
                        value: metric.value,
                        threshold: self.config.alert_thresholds.execution_time_ms as f64,
                        severity: AlertSeverity::Critical,
                    });
                }
                "error_rate" if metric.value > self.config.alert_thresholds.error_rate => {
                    alerts.push(Alert {
                        metric_name: name.clone(),
                        value: metric.value,
                        threshold: self.config.alert_thresholds.error_rate,
                        severity: AlertSeverity::Critical,
                    });
                }
                _ => {}
            }
        }

        alerts
    }

    /// Purge metrics older than the configured retention period
    pub fn cleanup_old_metrics(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let retention_secs = self.config.retention_days as u64 * 24 * 3600;

        self.metrics
            .retain(|_, m| now.saturating_sub(m.timestamp) <= retention_secs);
    }

    /// Count current stored metrics
    pub fn metrics_count(&self) -> usize {
        self.metrics.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_detection() {
        let mut monitor = MonitoringSystem::new(MonitoringConfig::default());
        monitor.record_metric("cpu_usage".to_string(), 90.0, HashMap::new());
        monitor.record_metric("error_rate".to_string(), 10.0, HashMap::new());

        let alerts = monitor.check_alerts();
        assert_eq!(alerts.len(), 2);
        assert!(alerts
            .iter()
            .any(|a| matches!(a.severity, AlertSeverity::Warning | AlertSeverity::Critical)));
    }

    #[test]
    fn test_cleanup_old_metrics() {
        let mut monitor = MonitoringSystem::new(MonitoringConfig::default());
        let mut tags = HashMap::new();
        tags.insert("node".to_string(), "validator1".to_string());

        monitor.record_metric("cpu_usage".to_string(), 50.0, tags.clone());
        monitor.cleanup_old_metrics();

        assert_eq!(monitor.metrics_count(), 1);
    }
}
