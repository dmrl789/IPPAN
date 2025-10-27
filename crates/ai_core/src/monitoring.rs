//! Monitoring system for AI Core

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

/// Alert thresholds
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

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            enable_performance_monitoring: true,
            interval_seconds: 60,
            retention_days: 7,
            alert_thresholds: AlertThresholds {
                cpu_usage: 80.0,
                memory_usage: 85.0,
                execution_time_ms: 5000,
                error_rate: 5.0,
            },
        }
    }
}

/// Monitoring system
pub struct MonitoringSystem {
    config: MonitoringConfig,
    metrics: HashMap<String, MetricValue>,
}

/// Metric value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricValue {
    pub value: f64,
    pub timestamp: u64,
    pub tags: HashMap<String, String>,
}

impl MonitoringSystem {
    /// Create a new monitoring system
    pub fn new(config: MonitoringConfig) -> Self {
        Self {
            config,
            metrics: HashMap::new(),
        }
    }

    /// Record a metric
    pub fn record_metric(&mut self, name: String, value: f64, tags: HashMap<String, String>) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        self.metrics.insert(name, MetricValue {
            value,
            timestamp,
            tags,
        });
    }

    /// Get metric value
    pub fn get_metric(&self, name: &str) -> Option<&MetricValue> {
        self.metrics.get(name)
    }

    /// Get all metrics
    pub fn get_all_metrics(&self) -> &HashMap<String, MetricValue> {
        &self.metrics
    }

    /// Check if alerts should be triggered
    pub fn check_alerts(&self) -> Vec<Alert> {
        let mut alerts = Vec::new();
        
        for (name, metric) in &self.metrics {
            match name.as_str() {
                "cpu_usage" => {
                    if metric.value > self.config.alert_thresholds.cpu_usage {
                        alerts.push(Alert {
                            metric_name: name.clone(),
                            value: metric.value,
                            threshold: self.config.alert_thresholds.cpu_usage,
                            severity: AlertSeverity::Warning,
                        });
                    }
                }
                "memory_usage" => {
                    if metric.value > self.config.alert_thresholds.memory_usage {
                        alerts.push(Alert {
                            metric_name: name.clone(),
                            value: metric.value,
                            threshold: self.config.alert_thresholds.memory_usage,
                            severity: AlertSeverity::Warning,
                        });
                    }
                }
                "execution_time_ms" => {
                    if metric.value > self.config.alert_thresholds.execution_time_ms as f64 {
                        alerts.push(Alert {
                            metric_name: name.clone(),
                            value: metric.value,
                            threshold: self.config.alert_thresholds.execution_time_ms as f64,
                            severity: AlertSeverity::Critical,
                        });
                    }
                }
                "error_rate" => {
                    if metric.value > self.config.alert_thresholds.error_rate {
                        alerts.push(Alert {
                            metric_name: name.clone(),
                            value: metric.value,
                            threshold: self.config.alert_thresholds.error_rate,
                            severity: AlertSeverity::Critical,
                        });
                    }
                }
                _ => {}
            }
        }
        
        alerts
    }
}

/// Alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub metric_name: String,
    pub value: f64,
    pub threshold: f64,
    pub severity: AlertSeverity,
}

/// Alert severity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}