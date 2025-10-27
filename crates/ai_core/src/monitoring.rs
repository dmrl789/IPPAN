//! Monitoring system for AI Core

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Enable monitoring
    pub enabled: bool,
    /// Metrics collection interval (seconds)
    pub collection_interval: u64,
    /// Metrics retention period (seconds)
    pub retention_period: u64,
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
    /// Execution time threshold (microseconds)
    pub execution_time: u64,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            collection_interval: 60,
            retention_period: 3600,
            alert_thresholds: AlertThresholds {
                cpu_usage: 80.0,
                memory_usage: 80.0,
                execution_time: 1_000_000, // 1 second
            },
        }
    }
}

/// Monitoring system
#[derive(Debug, Clone)]
pub struct MonitoringSystem {
    config: MonitoringConfig,
    metrics: HashMap<String, f64>,
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
    pub fn record_metric(&mut self, name: String, value: f64) {
        self.metrics.insert(name, value);
    }

    /// Get a metric value
    pub fn get_metric(&self, name: &str) -> Option<f64> {
        self.metrics.get(name).copied()
    }

    /// Check if alerts should be triggered
    pub fn check_alerts(&self) -> Vec<String> {
        let mut alerts = Vec::new();
        
        if let Some(cpu_usage) = self.metrics.get("cpu_usage") {
            if *cpu_usage > self.config.alert_thresholds.cpu_usage {
                alerts.push(format!("High CPU usage: {:.2}%", cpu_usage));
            }
        }
        
        if let Some(memory_usage) = self.metrics.get("memory_usage") {
            if *memory_usage > self.config.alert_thresholds.memory_usage {
                alerts.push(format!("High memory usage: {:.2}%", memory_usage));
            }
        }
        
        if let Some(execution_time) = self.metrics.get("execution_time") {
            if *execution_time > self.config.alert_thresholds.execution_time as f64 {
                alerts.push(format!("Slow execution: {:.2}μs", execution_time));
            }
        }
        
        alerts
    }
}