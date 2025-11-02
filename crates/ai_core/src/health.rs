//! Health checks and monitoring for AI Core

use crate::errors::{AiCoreError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Health status of the AI Core system
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Starting,
    ShuttingDown,
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub name: String,
    pub status: HealthStatus,
    pub message: String,
    pub duration_us: u64,
    pub timestamp: u64,
    pub metadata: HashMap<String, String>,
}

/// System health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    pub status: HealthStatus,
    pub checks: Vec<HealthCheck>,
    pub uptime_seconds: u64,
    pub version: String,
    pub metadata: HashMap<String, String>,
}

/// Performance metrics
#[derive(Debug, Default)]
pub struct PerformanceMetrics {
    pub total_executions: AtomicU64,
    pub successful_executions: AtomicU64,
    pub failed_executions: AtomicU64,
    pub avg_execution_time_us: AtomicU64,
    pub total_memory_usage: AtomicU64,
    pub peak_memory_usage: AtomicU64,
    pub cache_hit_rate: AtomicU64,
}

/// Health monitor for AI Core
pub struct HealthMonitor {
    start_time: Instant,
    metrics: Arc<PerformanceMetrics>,
    health_checks: HashMap<String, Box<dyn HealthChecker + Send + Sync>>,
    config: HealthConfig,
}

/// Health checker trait
pub trait HealthChecker {
    fn check(&self) -> Result<HealthCheck>;
    fn name(&self) -> &str;
}

/// Health configuration
#[derive(Debug, Clone)]
pub struct HealthConfig {
    pub max_check_time_ms: u64,
    pub check_interval_seconds: u64,
    pub memory_threshold_bytes: u64,
    pub cpu_threshold_percent: u64,
    pub enable_detailed_logging: bool,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            max_check_time_ms: 5000,
            check_interval_seconds: 30,
            memory_threshold_bytes: 1_000_000_000, // 1 GB
            cpu_threshold_percent: 80,
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

    /// Run all health checks asynchronously
    pub async fn run_health_checks(&self) -> SystemHealth {
        let mut checks = Vec::new();
        let mut overall_status = HealthStatus::Healthy;

        for (name, checker) in &self.health_checks {
            let check_start = Instant::now();

            let check_result = match self.run_single_check(checker.as_ref()).await {
                Ok(check) => check,
                Err(e) => HealthCheck {
                    name: name.clone(),
                    status: HealthStatus::Unhealthy,
                    message: format!("Health check failed: {}", e),
                    duration_us: check_start.elapsed().as_micros() as u64,
                    timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    metadata: HashMap::new(),
                },
            };

            match check_result.status {
                HealthStatus::Unhealthy => overall_status = HealthStatus::Unhealthy,
                HealthStatus::Degraded if overall_status == HealthStatus::Healthy => {
                    overall_status = HealthStatus::Degraded
                }
                _ => {}
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
        let timeout_duration = Duration::from_millis(self.config.max_check_time_ms);

        let result = tokio::time::timeout(timeout_duration, async { checker.check() })
            .await
            .map_err(|_| AiCoreError::Internal("Health check timeout".into()))?;

        let mut check = result?;
        check.duration_us = check_start.elapsed().as_micros() as u64;
        check.timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Ok(check)
    }

    /// Collect system metadata for dashboard / API
    fn get_system_metadata(&self) -> HashMap<String, String> {
        let mut metadata = HashMap::new();

        // Memory and CPU info
        if let Ok(mem) = get_memory_usage() {
            metadata.insert("memory_usage_bytes".to_string(), mem.to_string());
        }
        metadata.insert("cpu_cores".to_string(), num_cpus::get().to_string());

        // System load
        if let Ok(load) = get_load_average() {
            metadata.insert("load_average".to_string(), load.to_string());
        }

        // Performance metrics
        metadata.insert(
            "total_executions".to_string(),
            self.metrics
                .total_executions
                .load(Ordering::Relaxed)
                .to_string(),
        );
        metadata.insert(
            "successful_executions".to_string(),
            self.metrics
                .successful_executions
                .load(Ordering::Relaxed)
                .to_string(),
        );
        metadata.insert(
            "failed_executions".to_string(),
            self.metrics
                .failed_executions
                .load(Ordering::Relaxed)
                .to_string(),
        );

        metadata
    }

    /// Get performance metrics reference
    pub fn get_metrics(&self) -> &PerformanceMetrics {
        &self.metrics
    }

    /// Record one execution into metrics
    pub fn record_execution(&self, success: bool, duration_us: u64, memory_usage: u64) {
        self.metrics
            .total_executions
            .fetch_add(1, Ordering::Relaxed);
        if success {
            self.metrics
                .successful_executions
                .fetch_add(1, Ordering::Relaxed);
        } else {
            self.metrics
                .failed_executions
                .fetch_add(1, Ordering::Relaxed);
        }

        // Rolling average update
        let total = self.metrics.total_executions.load(Ordering::Relaxed);
        let current_avg = self.metrics.avg_execution_time_us.load(Ordering::Relaxed);
        let new_avg = ((current_avg * (total - 1)) + duration_us) / total;
        self.metrics
            .avg_execution_time_us
            .store(new_avg, Ordering::Relaxed);

        // Memory metrics
        self.metrics
            .total_memory_usage
            .store(memory_usage, Ordering::Relaxed);
        let current_peak = self.metrics.peak_memory_usage.load(Ordering::Relaxed);
        if memory_usage > current_peak {
            self.metrics
                .peak_memory_usage
                .store(memory_usage, Ordering::Relaxed);
        }
    }

    pub async fn is_healthy(&self) -> bool {
        self.run_health_checks().await.status == HealthStatus::Healthy
    }

    pub async fn get_status(&self) -> HealthStatus {
        self.run_health_checks().await.status
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
            format!(
                "Memory usage {} exceeds threshold {}",
                memory_usage, self.threshold
            )
        } else {
            format!("Memory usage {} is within limits", memory_usage)
        };

        Ok(HealthCheck {
            name: self.name().to_string(),
            status,
            message,
            duration_us: 0,
            timestamp: 0,
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("memory_usage_bytes".into(), memory_usage.to_string());
                meta.insert("threshold_bytes".into(), self.threshold.to_string());
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
        // In production, this would pull live metrics
        let total_executions = 100;
        let failed_executions = 5;

        if total_executions < self.min_executions {
            return Ok(HealthCheck {
                name: self.name().to_string(),
                status: HealthStatus::Degraded,
                message: format!(
                    "Insufficient executions: {} < {}",
                    total_executions, self.min_executions
                ),
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

        Ok(HealthCheck {
            name: self.name().to_string(),
            status,
            message: format!(
                "Failure rate: {:.2}% (threshold: {:.2}%)",
                failure_rate * 100.0,
                self.max_failure_rate * 100.0
            ),
            duration_us: 0,
            timestamp: 0,
            metadata: {
                let mut m = HashMap::new();
                m.insert("total_executions".into(), total_executions.to_string());
                m.insert("failed_executions".into(), failed_executions.to_string());
                m.insert("failure_rate".into(), failure_rate.to_string());
                m
            },
        })
    }
}

/// Get current memory usage (placeholder)
fn get_memory_usage() -> Result<u64> {
    Ok(100_000_000) // 100 MB placeholder
}

/// Get system load average (placeholder)
fn get_load_average() -> Result<f64> {
    Ok(0.5)
}

#[cfg(all(test, feature = "enable-tests"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_monitor_creation() {
        let monitor = HealthMonitor::new(HealthConfig::default());
        assert_eq!(monitor.health_checks.len(), 0);
    }

    #[tokio::test]
    async fn test_memory_checker() {
        let checker = MemoryUsageChecker::new(1_000_000_000);
        let result = checker.check().unwrap();
        assert_eq!(result.name, "memory_usage");
    }

    #[tokio::test]
    async fn test_model_execution_checker() {
        let checker = ModelExecutionChecker::new(0.1, 10);
        let result = checker.check().unwrap();
        assert_eq!(result.name, "model_execution");
    }

    #[tokio::test]
    async fn test_health_checks_integration() {
        let mut monitor = HealthMonitor::new(HealthConfig::default());
        monitor.register_check(
            "memory".into(),
            Box::new(MemoryUsageChecker::new(1_000_000_000)),
        );
        monitor.register_check(
            "execution".into(),
            Box::new(ModelExecutionChecker::new(0.1, 10)),
        );

        let health = monitor.run_health_checks().await;
        assert_eq!(health.checks.len(), 2);
        assert!(health.uptime_seconds >= 0);
    }
}
