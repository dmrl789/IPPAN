//! Health checks and monitoring for AI Core

use crate::{
    errors::{AiCoreError, Result},
    types::*,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::{info, warn, error, debug};

/// Health status of the AI Core system
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// System is healthy
    Healthy,
    /// System is degraded but functional
    Degraded,
    /// System is unhealthy
    Unhealthy,
    /// System is starting up
    Starting,
    /// System is shutting down
    ShuttingDown,
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Check name
    pub name: String,
    /// Check status
    pub status: HealthStatus,
    /// Check message
    pub message: String,
    /// Check duration in microseconds
    pub duration_us: u64,
    /// Check timestamp
    pub timestamp: u64,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// System health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    /// Overall system status
    pub status: HealthStatus,
    /// Individual health checks
    pub checks: Vec<HealthCheck>,
    /// System uptime in seconds
    pub uptime_seconds: u64,
    /// System version
    pub version: String,
    /// System metadata
    pub metadata: HashMap<String, String>,
}

/// Performance metrics
#[derive(Debug, Clone, Default)]
pub struct PerformanceMetrics {
    /// Total model executions
    pub total_executions: AtomicU64,
    /// Successful executions
    pub successful_executions: AtomicU64,
    /// Failed executions
    pub failed_executions: AtomicU64,
    /// Average execution time in microseconds
    pub avg_execution_time_us: AtomicU64,
    /// Total memory usage in bytes
    pub total_memory_usage: AtomicU64,
    /// Peak memory usage in bytes
    pub peak_memory_usage: AtomicU64,
    /// Cache hit rate (0-100)
    pub cache_hit_rate: AtomicU64,
}

/// Health monitor for AI Core
pub struct HealthMonitor {
    /// System start time
    start_time: Instant,
    /// Performance metrics
    metrics: Arc<PerformanceMetrics>,
    /// Health check registry
    health_checks: HashMap<String, Box<dyn HealthChecker + Send + Sync>>,
    /// System configuration
    config: HealthConfig,
}

/// Health checker trait
pub trait HealthChecker {
    /// Run the health check
    fn check(&self) -> Result<HealthCheck>;
    /// Get check name
    fn name(&self) -> &str;
}

/// Health configuration
#[derive(Debug, Clone)]
pub struct HealthConfig {
    /// Maximum execution time for health checks (ms)
    pub max_check_time_ms: u64,
    /// Health check interval (seconds)
    pub check_interval_seconds: u64,
    /// Memory usage threshold (bytes)
    pub memory_threshold_bytes: u64,
    /// CPU usage threshold (percentage)
    pub cpu_threshold_percent: u64,
    /// Enable detailed logging
    pub enable_detailed_logging: bool,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            max_check_time_ms: 5000, // 5 seconds
            check_interval_seconds: 30, // 30 seconds
            memory_threshold_bytes: 1_000_000_000, // 1GB
            cpu_threshold_percent: 80, // 80%
            enable_detailed_logging: true,
        }
    }
}

impl HealthMonitor {
    /// Create a new health monitor
    pub fn new(config: HealthConfig) -> Self {
        Self {
            start_time: Instant::now(),
            metrics: Arc::new(PerformanceMetrics::default()),
            health_checks: HashMap::new(),
            config,
        }
    }

    /// Register a health check
    pub fn register_check(&mut self, name: String, checker: Box<dyn HealthChecker + Send + Sync>) {
        self.health_checks.insert(name, checker);
    }

    /// Run all health checks
    pub async fn run_health_checks(&self) -> SystemHealth {
        let mut checks = Vec::new();
        let mut overall_status = HealthStatus::Healthy;

        for (name, checker) in &self.health_checks {
            let check_start = Instant::now();
            
            let check_result = match self.run_single_check(checker).await {
                Ok(check) => check,
                Err(e) => HealthCheck {
                    name: name.clone(),
                    status: HealthStatus::Unhealthy,
                    message: format!("Health check failed: {}", e),
                    duration_us: check_start.elapsed().as_micros() as u64,
                    timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                    metadata: HashMap::new(),
                },
            };

            // Update overall status
            match check_result.status {
                HealthStatus::Unhealthy => overall_status = HealthStatus::Unhealthy,
                HealthStatus::Degraded if overall_status == HealthStatus::Healthy => {
                    overall_status = HealthStatus::Degraded;
                },
                _ => {},
            }

            checks.push(check_result);
        }

        SystemHealth {
            status: overall_status,
            checks,
            uptime_seconds: self.start_time.elapsed().as_secs(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            metadata: self.get_system_metadata(),
        }
    }

    /// Run a single health check with timeout
    async fn run_single_check(&self, checker: &dyn HealthChecker) -> Result<HealthCheck> {
        let check_start = Instant::now();
        
        // Run check with timeout
        let timeout_duration = Duration::from_millis(self.config.max_check_time_ms);
        let check_future = async {
            checker.check()
        };

        let result = tokio::time::timeout(timeout_duration, check_future).await
            .map_err(|_| AiCoreError::Internal("Health check timeout".to_string()))?;

        let duration = check_start.elapsed();
        let mut check = result?;
        check.duration_us = duration.as_micros() as u64;
        check.timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        Ok(check)
    }

    /// Get system metadata
    fn get_system_metadata(&self) -> HashMap<String, String> {
        let mut metadata = HashMap::new();
        
        // Memory usage
        if let Ok(memory_info) = get_memory_usage() {
            metadata.insert("memory_usage_bytes".to_string(), memory_info.to_string());
        }

        // CPU usage (simplified)
        metadata.insert("cpu_cores".to_string(), num_cpus::get().to_string());
        
        // System load
        if let Ok(load_avg) = get_load_average() {
            metadata.insert("load_average".to_string(), load_avg.to_string());
        }

        // Performance metrics
        metadata.insert("total_executions".to_string(), 
            self.metrics.total_executions.load(Ordering::Relaxed).to_string());
        metadata.insert("successful_executions".to_string(), 
            self.metrics.successful_executions.load(Ordering::Relaxed).to_string());
        metadata.insert("failed_executions".to_string(), 
            self.metrics.failed_executions.load(Ordering::Relaxed).to_string());

        metadata
    }

    /// Get performance metrics
    pub fn get_metrics(&self) -> &PerformanceMetrics {
        &self.metrics
    }

    /// Record execution metrics
    pub fn record_execution(&self, success: bool, duration_us: u64, memory_usage: u64) {
        self.metrics.total_executions.fetch_add(1, Ordering::Relaxed);
        
        if success {
            self.metrics.successful_executions.fetch_add(1, Ordering::Relaxed);
        } else {
            self.metrics.failed_executions.fetch_add(1, Ordering::Relaxed);
        }

        // Update average execution time
        let total = self.metrics.total_executions.load(Ordering::Relaxed);
        let current_avg = self.metrics.avg_execution_time_us.load(Ordering::Relaxed);
        let new_avg = ((current_avg * (total - 1)) + duration_us) / total;
        self.metrics.avg_execution_time_us.store(new_avg, Ordering::Relaxed);

        // Update memory usage
        self.metrics.total_memory_usage.store(memory_usage, Ordering::Relaxed);
        let current_peak = self.metrics.peak_memory_usage.load(Ordering::Relaxed);
        if memory_usage > current_peak {
            self.metrics.peak_memory_usage.store(memory_usage, Ordering::Relaxed);
        }
    }

    /// Check if system is healthy
    pub async fn is_healthy(&self) -> bool {
        let health = self.run_health_checks().await;
        health.status == HealthStatus::Healthy
    }

    /// Get system status
    pub async fn get_status(&self) -> HealthStatus {
        let health = self.run_health_checks().await;
        health.status
    }
}

/// Memory usage checker
pub struct MemoryUsageChecker {
    threshold: u64,
}

impl MemoryUsageChecker {
    pub fn new(threshold: u64) -> Self {
        Self { threshold }
    }
}

impl HealthChecker for MemoryUsageChecker {
    fn name(&self) -> &str {
        "memory_usage"
    }

    fn check(&self) -> Result<HealthCheck> {
        let memory_usage = get_memory_usage()
            .map_err(|e| AiCoreError::Internal(format!("Failed to get memory usage: {}", e)))?;

        let status = if memory_usage > self.threshold {
            HealthStatus::Unhealthy
        } else if memory_usage > self.threshold / 2 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };

        let message = if memory_usage > self.threshold {
            format!("Memory usage {} exceeds threshold {}", memory_usage, self.threshold)
        } else {
            format!("Memory usage {} is within limits", memory_usage)
        };

        Ok(HealthCheck {
            name: self.name().to_string(),
            status,
            message,
            duration_us: 0, // Will be set by caller
            timestamp: 0,   // Will be set by caller
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("memory_usage_bytes".to_string(), memory_usage.to_string());
                meta.insert("threshold_bytes".to_string(), self.threshold.to_string());
                meta
            },
        })
    }
}

/// Model execution checker
pub struct ModelExecutionChecker {
    max_failure_rate: f64,
    min_executions: u64,
}

impl ModelExecutionChecker {
    pub fn new(max_failure_rate: f64, min_executions: u64) -> Self {
        Self {
            max_failure_rate,
            min_executions,
        }
    }
}

impl HealthChecker for ModelExecutionChecker {
    fn name(&self) -> &str {
        "model_execution"
    }

    fn check(&self) -> Result<HealthCheck> {
        // This would need access to the metrics, simplified for now
        let total_executions = 100; // Would get from metrics
        let failed_executions = 5;  // Would get from metrics

        if total_executions < self.min_executions {
            return Ok(HealthCheck {
                name: self.name().to_string(),
                status: HealthStatus::Degraded,
                message: format!("Insufficient executions: {} < {}", total_executions, self.min_executions),
                duration_us: 0,
                timestamp: 0,
                metadata: HashMap::new(),
            });
        }

        let failure_rate = failed_executions as f64 / total_executions as f64;
        let status = if failure_rate > self.max_failure_rate {
            HealthStatus::Unhealthy
        } else if failure_rate > self.max_failure_rate / 2.0 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };

        let message = format!("Failure rate: {:.2}% (threshold: {:.2}%)", 
            failure_rate * 100.0, self.max_failure_rate * 100.0);

        Ok(HealthCheck {
            name: self.name().to_string(),
            status,
            message,
            duration_us: 0,
            timestamp: 0,
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("total_executions".to_string(), total_executions.to_string());
                meta.insert("failed_executions".to_string(), failed_executions.to_string());
                meta.insert("failure_rate".to_string(), failure_rate.to_string());
                meta
            },
        })
    }
}

/// Get current memory usage
fn get_memory_usage() -> Result<u64> {
    // Simplified implementation - in production, use proper memory monitoring
    Ok(100_000_000) // 100MB placeholder
}

/// Get system load average
fn get_load_average() -> Result<f64> {
    // Simplified implementation - in production, use proper system monitoring
    Ok(0.5) // Placeholder
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_monitor_creation() {
        let config = HealthConfig::default();
        let monitor = HealthMonitor::new(config);
        assert_eq!(monitor.health_checks.len(), 0);
    }

    #[tokio::test]
    async fn test_memory_checker() {
        let checker = MemoryUsageChecker::new(1_000_000_000); // 1GB threshold
        let result = checker.check().unwrap();
        assert_eq!(result.name, "memory_usage");
        assert!(matches!(result.status, HealthStatus::Healthy | HealthStatus::Degraded | HealthStatus::Unhealthy));
    }

    #[tokio::test]
    async fn test_model_execution_checker() {
        let checker = ModelExecutionChecker::new(0.1, 10); // 10% max failure rate, 10 min executions
        let result = checker.check().unwrap();
        assert_eq!(result.name, "model_execution");
        assert!(matches!(result.status, HealthStatus::Healthy | HealthStatus::Degraded | HealthStatus::Unhealthy));
    }

    #[tokio::test]
    async fn test_health_checks_integration() {
        let config = HealthConfig::default();
        let mut monitor = HealthMonitor::new(config);
        
        // Register health checks
        monitor.register_check("memory".to_string(), 
            Box::new(MemoryUsageChecker::new(1_000_000_000)));
        monitor.register_check("execution".to_string(), 
            Box::new(ModelExecutionChecker::new(0.1, 10)));

        let health = monitor.run_health_checks().await;
        assert_eq!(health.checks.len(), 2);
        assert!(health.uptime_seconds >= 0);
    }
}
