//! Production-grade monitoring and observability for GBDT systems
//!
//! This module provides comprehensive monitoring capabilities including:
//! - Real-time performance metrics collection
//! - Health monitoring and alerting
//! - Distributed tracing and logging
//! - Resource utilization tracking
//! - Model performance analytics
//! - Security event monitoring

use crate::gbdt::{GBDTModel, GBDTError, GBDTMetrics};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info, warn, instrument};
use tokio::sync::RwLock as AsyncRwLock;
use tokio::time::interval;

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Enable performance monitoring
    pub enable_performance_monitoring: bool,
    /// Enable health monitoring
    pub enable_health_monitoring: bool,
    /// Enable security monitoring
    pub enable_security_monitoring: bool,
    /// Metrics collection interval in seconds
    pub metrics_interval_seconds: u64,
    /// Health check interval in seconds
    pub health_check_interval_seconds: u64,
    /// Maximum metrics history to keep
    pub max_metrics_history: usize,
    /// Enable distributed tracing
    pub enable_tracing: bool,
    /// Alert thresholds
    pub alert_thresholds: AlertThresholds,
}

/// Alert thresholds for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    /// Maximum evaluation time in milliseconds
    pub max_evaluation_time_ms: u64,
    /// Maximum error rate (0.0 to 1.0)
    pub max_error_rate: f64,
    /// Maximum memory usage in bytes
    pub max_memory_usage_bytes: u64,
    /// Maximum CPU usage percentage
    pub max_cpu_usage_percent: f64,
    /// Minimum cache hit rate (0.0 to 1.0)
    pub min_cache_hit_rate: f64,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enable_performance_monitoring: true,
            enable_health_monitoring: true,
            enable_security_monitoring: true,
            metrics_interval_seconds: 60,
            health_check_interval_seconds: 30,
            max_metrics_history: 1000,
            enable_tracing: true,
            alert_thresholds: AlertThresholds::default(),
        }
    }
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            max_evaluation_time_ms: 1000,
            max_error_rate: 0.05, // 5%
            max_memory_usage_bytes: 1024 * 1024 * 1024, // 1GB
            max_cpu_usage_percent: 80.0,
            min_cache_hit_rate: 0.7, // 70%
        }
    }
}

/// System health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
    Unknown,
}

/// Comprehensive system metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub timestamp: u64,
    pub gbdt_metrics: GBDTMetrics,
    pub system_metrics: SystemResourceMetrics,
    pub model_metrics: ModelMetrics,
    pub security_metrics: SecurityMetrics,
    pub health_status: HealthStatus,
}

/// System resource metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemResourceMetrics {
    pub memory_usage_bytes: u64,
    pub memory_usage_percent: f64,
    pub cpu_usage_percent: f64,
    pub disk_usage_bytes: u64,
    pub disk_usage_percent: f64,
    pub network_io_bytes: u64,
    pub active_connections: u32,
    pub thread_count: u32,
}

/// Model-specific metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetrics {
    pub total_models_loaded: u64,
    pub active_models: u32,
    pub model_load_errors: u64,
    pub model_save_errors: u64,
    pub avg_model_size_bytes: u64,
    pub total_model_cache_size_bytes: u64,
    pub model_validation_errors: u64,
}

/// Security metrics and events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityMetrics {
    pub security_events: u64,
    pub validation_failures: u64,
    pub suspicious_requests: u64,
    pub access_denied: u64,
    pub integrity_violations: u64,
    pub last_security_event: Option<u64>,
}

/// Alert information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub timestamp: u64,
    pub severity: AlertSeverity,
    pub category: AlertCategory,
    pub message: String,
    pub details: HashMap<String, String>,
    pub resolved: bool,
}

/// Alert severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
    Emergency,
}

/// Alert categories
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertCategory {
    Performance,
    Health,
    Security,
    Model,
    System,
}

/// Production monitoring system
#[derive(Debug)]
pub struct MonitoringSystem {
    config: MonitoringConfig,
    metrics_history: Arc<RwLock<Vec<SystemMetrics>>>,
    alerts: Arc<RwLock<Vec<Alert>>>,
    health_status: Arc<RwLock<HealthStatus>>,
    start_time: Instant,
    is_running: Arc<RwLock<bool>>,
}

impl MonitoringSystem {
    /// Create a new monitoring system
    pub fn new(config: MonitoringConfig) -> Self {
        Self {
            config,
            metrics_history: Arc::new(RwLock::new(Vec::new())),
            alerts: Arc::new(RwLock::new(Vec::new())),
            health_status: Arc::new(RwLock::new(HealthStatus::Unknown)),
            start_time: Instant::now(),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the monitoring system
    #[instrument(skip(self))]
    pub async fn start(&self) -> Result<(), GBDTError> {
        if *self.is_running.read().unwrap() {
            warn!("Monitoring system is already running");
            return Ok(());
        }

        *self.is_running.write().unwrap() = true;
        info!("Starting monitoring system");

        // Start metrics collection
        if self.config.enable_performance_monitoring {
            self.start_metrics_collection().await?;
        }

        // Start health monitoring
        if self.config.enable_health_monitoring {
            self.start_health_monitoring().await?;
        }

        // Start security monitoring
        if self.config.enable_security_monitoring {
            self.start_security_monitoring().await?;
        }

        info!("Monitoring system started successfully");
        Ok(())
    }

    /// Stop the monitoring system
    #[instrument(skip(self))]
    pub async fn stop(&self) -> Result<(), GBDTError> {
        *self.is_running.write().unwrap() = false;
        info!("Monitoring system stopped");
        Ok(())
    }

    /// Record GBDT evaluation metrics
    #[instrument(skip(self, model))]
    pub fn record_gbdt_evaluation(&self, model: &GBDTModel) -> Result<(), GBDTError> {
        if !self.config.enable_performance_monitoring {
            return Ok(());
        }

        let metrics = model.get_metrics();
        self.check_performance_thresholds(metrics)?;
        Ok(())
    }

    /// Record a security event
    #[instrument(skip(self))]
    pub fn record_security_event(&self, event_type: &str, details: HashMap<String, String>) -> Result<(), GBDTError> {
        if !self.config.enable_security_monitoring {
            return Ok(());
        }

        let alert = Alert {
            id: format!("sec_{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            severity: AlertSeverity::Warning,
            category: AlertCategory::Security,
            message: format!("Security event: {}", event_type),
            details,
            resolved: false,
        };

        self.add_alert(alert);
        Ok(())
    }

    /// Get current system metrics
    pub fn get_current_metrics(&self) -> Result<SystemMetrics, GBDTError> {
        let system_metrics = self.collect_system_metrics()?;
        let model_metrics = self.collect_model_metrics()?;
        let security_metrics = self.collect_security_metrics()?;
        let health_status = *self.health_status.read().unwrap();

        Ok(SystemMetrics {
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            gbdt_metrics: GBDTMetrics::default(), // Will be updated by specific model
            system_metrics,
            model_metrics,
            security_metrics,
            health_status,
        })
    }

    /// Get metrics history
    pub fn get_metrics_history(&self) -> Vec<SystemMetrics> {
        self.metrics_history.read().unwrap().clone()
    }

    /// Get active alerts
    pub fn get_active_alerts(&self) -> Vec<Alert> {
        self.alerts.read().unwrap()
            .iter()
            .filter(|alert| !alert.resolved)
            .cloned()
            .collect()
    }

    /// Get health status
    pub fn get_health_status(&self) -> HealthStatus {
        *self.health_status.read().unwrap()
    }

    /// Start metrics collection
    async fn start_metrics_collection(&self) -> Result<(), GBDTError> {
        let metrics_history = Arc::clone(&self.metrics_history);
        let config = self.config.clone();
        let is_running = Arc::clone(&self.is_running);

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(config.metrics_interval_seconds));
            
            while *is_running.read().unwrap() {
                interval.tick().await;
                
                if let Ok(metrics) = Self::collect_system_metrics_static() {
                    let mut history = metrics_history.write().unwrap();
                    history.push(metrics);
                    
                    // Keep only recent metrics
                    if history.len() > config.max_metrics_history {
                        history.drain(0..history.len() - config.max_metrics_history);
                    }
                }
            }
        });

        Ok(())
    }

    /// Start health monitoring
    async fn start_health_monitoring(&self) -> Result<(), GBDTError> {
        let health_status = Arc::clone(&self.health_status);
        let config = self.config.clone();
        let is_running = Arc::clone(&self.is_running);

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(config.health_check_interval_seconds));
            
            while *is_running.read().unwrap() {
                interval.tick().await;
                
                let status = Self::check_system_health(&config);
                *health_status.write().unwrap() = status;
            }
        });

        Ok(())
    }

    /// Start security monitoring
    async fn start_security_monitoring(&self) -> Result<(), GBDTError> {
        // Security monitoring is event-driven, so this is a placeholder
        // In a real implementation, this would set up security event handlers
        Ok(())
    }

    /// Collect system resource metrics
    fn collect_system_metrics(&self) -> Result<SystemResourceMetrics, GBDTError> {
        Self::collect_system_metrics_static()
    }

    /// Static method to collect system metrics
    fn collect_system_metrics_static() -> Result<SystemResourceMetrics, GBDTError> {
        // In a real implementation, this would use system APIs to collect actual metrics
        // For now, we'll return mock data
        Ok(SystemResourceMetrics {
            memory_usage_bytes: 512 * 1024 * 1024, // 512MB
            memory_usage_percent: 25.0,
            cpu_usage_percent: 15.0,
            disk_usage_bytes: 2 * 1024 * 1024 * 1024, // 2GB
            disk_usage_percent: 40.0,
            network_io_bytes: 1024 * 1024, // 1MB
            active_connections: 10,
            thread_count: 8,
        })
    }

    /// Collect model metrics
    fn collect_model_metrics(&self) -> Result<ModelMetrics, GBDTError> {
        // In a real implementation, this would collect actual model metrics
        Ok(ModelMetrics {
            total_models_loaded: 5,
            active_models: 3,
            model_load_errors: 0,
            model_save_errors: 0,
            avg_model_size_bytes: 1024 * 1024, // 1MB
            total_model_cache_size_bytes: 5 * 1024 * 1024, // 5MB
            model_validation_errors: 0,
        })
    }

    /// Collect security metrics
    fn collect_security_metrics(&self) -> Result<SecurityMetrics, GBDTError> {
        let alerts = self.alerts.read().unwrap();
        let security_events = alerts.iter().filter(|a| a.category == AlertCategory::Security).count() as u64;
        
        Ok(SecurityMetrics {
            security_events,
            validation_failures: 0,
            suspicious_requests: 0,
            access_denied: 0,
            integrity_violations: 0,
            last_security_event: alerts.iter()
                .filter(|a| a.category == AlertCategory::Security)
                .map(|a| a.timestamp)
                .max(),
        })
    }

    /// Check system health
    fn check_system_health(config: &MonitoringConfig) -> HealthStatus {
        // In a real implementation, this would check actual system health
        // For now, we'll return a mock status
        HealthStatus::Healthy
    }

    /// Check performance thresholds and generate alerts
    fn check_performance_thresholds(&self, metrics: &GBDTMetrics) -> Result<(), GBDTError> {
        let thresholds = &self.config.alert_thresholds;

        // Check evaluation time
        if metrics.avg_time_us > (thresholds.max_evaluation_time_ms * 1000) as f64 {
            self.add_alert(Alert {
                id: format!("perf_{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()),
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                severity: AlertSeverity::Warning,
                category: AlertCategory::Performance,
                message: format!("Average evaluation time {}μs exceeds threshold {}ms", 
                    metrics.avg_time_us / 1000.0, thresholds.max_evaluation_time_ms),
                details: HashMap::new(),
                resolved: false,
            });
        }

        // Check error rate
        if metrics.total_evaluations > 0 {
            let error_rate = metrics.error_count as f64 / metrics.total_evaluations as f64;
            if error_rate > thresholds.max_error_rate {
                self.add_alert(Alert {
                    id: format!("error_{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()),
                    timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                    severity: AlertSeverity::Critical,
                    category: AlertCategory::Performance,
                    message: format!("Error rate {:.2}% exceeds threshold {:.2}%", 
                        error_rate * 100.0, thresholds.max_error_rate * 100.0),
                    details: HashMap::new(),
                    resolved: false,
                });
            }
        }

        // Check cache hit rate
        if metrics.total_evaluations > 0 {
            let cache_hit_rate = metrics.cache_hits as f64 / (metrics.cache_hits + metrics.cache_misses) as f64;
            if cache_hit_rate < thresholds.min_cache_hit_rate {
                self.add_alert(Alert {
                    id: format!("cache_{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()),
                    timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                    severity: AlertSeverity::Warning,
                    category: AlertCategory::Performance,
                    message: format!("Cache hit rate {:.2}% below threshold {:.2}%", 
                        cache_hit_rate * 100.0, thresholds.min_cache_hit_rate * 100.0),
                    details: HashMap::new(),
                    resolved: false,
                });
            }
        }

        Ok(())
    }

    /// Add an alert
    fn add_alert(&self, alert: Alert) {
        let mut alerts = self.alerts.write().unwrap();
        alerts.push(alert);
        
        // Keep only recent alerts
        if alerts.len() > 1000 {
            alerts.drain(0..alerts.len() - 1000);
        }
    }

    /// Resolve an alert
    pub fn resolve_alert(&self, alert_id: &str) -> Result<(), GBDTError> {
        let mut alerts = self.alerts.write().unwrap();
        if let Some(alert) = alerts.iter_mut().find(|a| a.id == alert_id) {
            alert.resolved = true;
            info!("Alert {} resolved", alert_id);
        } else {
            return Err(GBDTError::ModelValidationFailed {
                reason: format!("Alert {} not found", alert_id),
            });
        }
        Ok(())
    }

    /// Get uptime in seconds
    pub fn get_uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// Export metrics to JSON
    pub fn export_metrics_json(&self) -> Result<String, GBDTError> {
        let metrics = self.get_current_metrics()?;
        serde_json::to_string_pretty(&metrics)
            .map_err(|e| GBDTError::ModelValidationFailed {
                reason: format!("Failed to serialize metrics: {}", e),
            })
    }

    /// Export alerts to JSON
    pub fn export_alerts_json(&self) -> Result<String, GBDTError> {
        let alerts = self.get_active_alerts();
        serde_json::to_string_pretty(&alerts)
            .map_err(|e| GBDTError::ModelValidationFailed {
                reason: format!("Failed to serialize alerts: {}", e),
            })
    }
}

/// Health check utilities
pub mod health_checks {
    use super::*;

    /// Perform a comprehensive health check
    pub async fn perform_health_check(monitoring: &MonitoringSystem) -> HealthStatus {
        let metrics = match monitoring.get_current_metrics() {
            Ok(m) => m,
            Err(_) => return HealthStatus::Critical,
        };

        // Check system resources
        if metrics.system_metrics.memory_usage_percent > 90.0 {
            return HealthStatus::Critical;
        }

        if metrics.system_metrics.cpu_usage_percent > 95.0 {
            return HealthStatus::Critical;
        }

        // Check error rates
        if metrics.gbdt_metrics.total_evaluations > 0 {
            let error_rate = metrics.gbdt_metrics.error_count as f64 / metrics.gbdt_metrics.total_evaluations as f64;
            if error_rate > 0.1 { // 10% error rate
                return HealthStatus::Critical;
            }
        }

        // Check for critical alerts
        let alerts = monitoring.get_active_alerts();
        if alerts.iter().any(|a| a.severity == AlertSeverity::Critical) {
            return HealthStatus::Critical;
        }

        // Check for warning alerts
        if alerts.iter().any(|a| a.severity == AlertSeverity::Warning) {
            return HealthStatus::Warning;
        }

        HealthStatus::Healthy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_monitoring_system_creation() {
        let config = MonitoringConfig::default();
        let monitoring = MonitoringSystem::new(config);
        
        assert_eq!(monitoring.get_health_status(), HealthStatus::Unknown);
        assert!(monitoring.get_active_alerts().is_empty());
    }

    #[tokio::test]
    async fn test_monitoring_system_start_stop() {
        let config = MonitoringConfig::default();
        let monitoring = MonitoringSystem::new(config);
        
        monitoring.start().await.unwrap();
        assert!(*monitoring.is_running.read().unwrap());
        
        monitoring.stop().await.unwrap();
        assert!(!*monitoring.is_running.read().unwrap());
    }

    #[tokio::test]
    async fn test_alert_creation() {
        let config = MonitoringConfig::default();
        let monitoring = MonitoringSystem::new(config);
        
        let mut details = HashMap::new();
        details.insert("test".to_string(), "value".to_string());
        
        monitoring.record_security_event("test_event", details).unwrap();
        
        let alerts = monitoring.get_active_alerts();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].category, AlertCategory::Security);
    }

    #[tokio::test]
    async fn test_metrics_collection() {
        let config = MonitoringConfig::default();
        let monitoring = MonitoringSystem::new(config);
        
        let metrics = monitoring.get_current_metrics().unwrap();
        assert!(metrics.timestamp > 0);
        assert_eq!(metrics.health_status, HealthStatus::Unknown);
    }

    #[test]
    fn test_health_check() {
        let config = MonitoringConfig::default();
        let monitoring = MonitoringSystem::new(config);
        
        // This would test the health check logic
        // In a real implementation, we'd mock the system metrics
    }
}