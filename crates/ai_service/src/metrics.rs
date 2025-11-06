//! Production-ready metrics collection and reporting

use crate::AIServiceError;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Metrics collector for production monitoring
#[derive(Debug, Clone)]
pub struct MetricsCollector {
    // Request metrics
    pub total_requests: Arc<AtomicU64>,
    pub successful_requests: Arc<AtomicU64>,
    pub failed_requests: Arc<AtomicU64>,
    pub request_duration_ms: Arc<AtomicU64>,

    // Service-specific metrics
    pub llm_requests: Arc<AtomicU64>,
    pub analytics_requests: Arc<AtomicU64>,
    pub smart_contract_requests: Arc<AtomicU64>,
    pub optimization_requests: Arc<AtomicU64>,

    // System metrics
    pub memory_usage_bytes: Arc<AtomicU64>,
    pub cpu_usage_percent: Arc<AtomicU64>,
    pub active_connections: Arc<AtomicUsize>,

    // Error metrics
    pub error_count: Arc<AtomicU64>,
    pub error_rate: Arc<AtomicU64>,

    // Start time for uptime calculation
    start_time: SystemTime,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            total_requests: Arc::new(AtomicU64::new(0)),
            successful_requests: Arc::new(AtomicU64::new(0)),
            failed_requests: Arc::new(AtomicU64::new(0)),
            request_duration_ms: Arc::new(AtomicU64::new(0)),
            llm_requests: Arc::new(AtomicU64::new(0)),
            analytics_requests: Arc::new(AtomicU64::new(0)),
            smart_contract_requests: Arc::new(AtomicU64::new(0)),
            optimization_requests: Arc::new(AtomicU64::new(0)),
            memory_usage_bytes: Arc::new(AtomicU64::new(0)),
            cpu_usage_percent: Arc::new(AtomicU64::new(0)),
            active_connections: Arc::new(AtomicUsize::new(0)),
            error_count: Arc::new(AtomicU64::new(0)),
            error_rate: Arc::new(AtomicU64::new(0)),
            start_time: SystemTime::now(),
        }
    }

    /// Record a request
    pub fn record_request(&self, success: bool, duration_ms: u64) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        if success {
            self.successful_requests.fetch_add(1, Ordering::Relaxed);
        } else {
            self.failed_requests.fetch_add(1, Ordering::Relaxed);
        }
        self.request_duration_ms
            .fetch_add(duration_ms, Ordering::Relaxed);
    }

    /// Record a service-specific request
    pub fn record_service_request(&self, service: &str) {
        match service {
            "llm" => {
                self.llm_requests.fetch_add(1, Ordering::Relaxed);
            }
            "analytics" => {
                self.analytics_requests.fetch_add(1, Ordering::Relaxed);
            }
            "smart_contracts" => {
                self.smart_contract_requests.fetch_add(1, Ordering::Relaxed);
            }
            "optimization" => {
                self.optimization_requests.fetch_add(1, Ordering::Relaxed);
            }
            _ => {}
        }
    }

    /// Record an error
    pub fn record_error(&self) {
        self.error_count.fetch_add(1, Ordering::Relaxed);
        self.update_error_rate();
    }

    /// Update system metrics
    pub fn update_system_metrics(&self, memory_bytes: u64, cpu_percent: u64) {
        self.memory_usage_bytes
            .store(memory_bytes, Ordering::Relaxed);
        self.cpu_usage_percent.store(cpu_percent, Ordering::Relaxed);
    }

    /// Update active connections
    pub fn update_active_connections(&self, count: usize) {
        self.active_connections.store(count, Ordering::Relaxed);
    }

    /// Get current metrics snapshot
    pub fn get_snapshot(&self) -> MetricsSnapshot {
        let total_requests = self.total_requests.load(Ordering::Relaxed);
        let successful_requests = self.successful_requests.load(Ordering::Relaxed);
        let failed_requests = self.failed_requests.load(Ordering::Relaxed);
        let request_duration_ms = self.request_duration_ms.load(Ordering::Relaxed);

        let success_rate = if total_requests > 0 {
            successful_requests as f64 / total_requests as f64
        } else {
            0.0
        };

        let avg_duration_ms = if total_requests > 0 {
            request_duration_ms as f64 / total_requests as f64
        } else {
            0.0
        };

        let uptime = self.start_time.elapsed().unwrap_or_default();

        MetricsSnapshot {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            uptime_seconds: uptime.as_secs(),
            total_requests,
            successful_requests,
            failed_requests,
            success_rate,
            avg_duration_ms,
            llm_requests: self.llm_requests.load(Ordering::Relaxed),
            analytics_requests: self.analytics_requests.load(Ordering::Relaxed),
            smart_contract_requests: self.smart_contract_requests.load(Ordering::Relaxed),
            optimization_requests: self.optimization_requests.load(Ordering::Relaxed),
            memory_usage_bytes: self.memory_usage_bytes.load(Ordering::Relaxed),
            cpu_usage_percent: self.cpu_usage_percent.load(Ordering::Relaxed),
            active_connections: self.active_connections.load(Ordering::Relaxed),
            error_count: self.error_count.load(Ordering::Relaxed),
            error_rate: self.error_rate.load(Ordering::Relaxed) as f64 / 100.0,
        }
    }

    /// Update error rate
    fn update_error_rate(&self) {
        let total_requests = self.total_requests.load(Ordering::Relaxed);
        let error_count = self.error_count.load(Ordering::Relaxed);

        if total_requests > 0 {
            let rate = (error_count as f64 / total_requests as f64 * 100.0) as u64;
            self.error_rate.store(rate, Ordering::Relaxed);
        }
    }
}

/// Metrics snapshot for reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub timestamp: u64,
    pub uptime_seconds: u64,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub success_rate: f64,
    pub avg_duration_ms: f64,
    pub llm_requests: u64,
    pub analytics_requests: u64,
    pub smart_contract_requests: u64,
    pub optimization_requests: u64,
    pub memory_usage_bytes: u64,
    pub cpu_usage_percent: u64,
    pub active_connections: usize,
    pub error_count: u64,
    pub error_rate: f64,
}

/// Metrics exporter for external monitoring systems
pub trait MetricsExporter {
    /// Export metrics to external system
    fn export_metrics(&self, snapshot: &MetricsSnapshot) -> Result<(), AIServiceError>;
}

/// Prometheus metrics exporter
pub struct PrometheusExporter {
    _endpoint: String,
}

impl PrometheusExporter {
    pub fn new(endpoint: String) -> Self {
        Self { _endpoint: endpoint }
    }
}

impl MetricsExporter for PrometheusExporter {
    fn export_metrics(&self, snapshot: &MetricsSnapshot) -> Result<(), AIServiceError> {
        // In a real implementation, this would format metrics in Prometheus format
        // and send them to the specified endpoint
        tracing::info!("Exporting metrics to Prometheus: {:?}", snapshot);
        Ok(())
    }
}

/// JSON metrics exporter
pub struct JsonExporter {
    _endpoint: String,
}

impl JsonExporter {
    pub fn new(endpoint: String) -> Self {
        Self { _endpoint: endpoint }
    }
}

impl MetricsExporter for JsonExporter {
    fn export_metrics(&self, snapshot: &MetricsSnapshot) -> Result<(), AIServiceError> {
        // In a real implementation, this would serialize metrics to JSON
        // and send them to the specified endpoint
        let json = serde_json::to_string(snapshot)?;
        tracing::info!("Exporting metrics to JSON endpoint: {}", json);
        Ok(())
    }
}
