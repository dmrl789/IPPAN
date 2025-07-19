//! Metrics and monitoring system for IPPAN
//! 
//! Provides real-time metrics collection, performance monitoring, and health tracking

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

/// Metric types supported by the monitoring system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
    Summary,
}

/// Individual metric with value and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub name: String,
    pub value: f64,
    pub metric_type: MetricType,
    pub labels: HashMap<String, String>,
    pub timestamp: u64,
    pub description: String,
}

/// Histogram bucket for distribution metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramBucket {
    pub upper_bound: f64,
    pub count: u64,
}

/// Summary quantile for percentile metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryQuantile {
    pub quantile: f64,
    pub value: f64,
}

/// Performance metrics for different system components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub storage_operations: StorageMetrics,
    pub network_operations: NetworkMetrics,
    pub consensus_operations: ConsensusMetrics,
    pub api_operations: ApiMetrics,
    pub system_metrics: SystemMetrics,
}

/// Storage-specific metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageMetrics {
    pub files_stored: u64,
    pub files_retrieved: u64,
    pub files_deleted: u64,
    pub total_storage_bytes: u64,
    pub used_storage_bytes: u64,
    pub shard_count: u32,
    pub replication_factor: u32,
    pub storage_operations_per_second: f64,
    pub average_storage_latency_ms: f64,
    pub encryption_operations: u64,
    pub decryption_operations: u64,
}

/// Network-specific metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMetrics {
    pub connected_peers: u32,
    pub total_peers: u32,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub network_latency_ms: f64,
    pub connection_attempts: u64,
    pub connection_failures: u64,
    pub discovery_operations: u64,
}

/// Consensus-specific metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusMetrics {
    pub current_round: u64,
    pub blocks_created: u64,
    pub transactions_processed: u64,
    pub consensus_participation_rate: f64,
    pub round_duration_ms: f64,
    pub validator_count: u32,
    pub total_stake: u64,
    pub zk_proof_generations: u64,
    pub zk_proof_verifications: u64,
}

/// API-specific metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub requests_per_second: f64,
    pub average_response_time_ms: f64,
    pub endpoint_usage: HashMap<String, u64>,
    pub error_codes: HashMap<u16, u64>,
    pub active_connections: u32,
}

/// System-level metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub uptime_seconds: u64,
    pub memory_usage_bytes: u64,
    pub cpu_usage_percent: f64,
    pub disk_usage_bytes: u64,
    pub thread_count: u32,
    pub open_file_descriptors: u32,
    pub system_load_average: f64,
}

/// Health status of different system components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub overall_status: HealthState,
    pub components: HashMap<String, ComponentHealth>,
    pub last_check: u64,
    pub uptime_seconds: u64,
}

/// Health state enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthState {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// Individual component health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub status: HealthState,
    pub message: String,
    pub last_check: u64,
    pub response_time_ms: Option<f64>,
}

/// Main metrics collector and manager
pub struct MetricsCollector {
    metrics: Arc<RwLock<Vec<Metric>>>,
    performance_metrics: Arc<RwLock<PerformanceMetrics>>,
    health_status: Arc<RwLock<HealthStatus>>,
    start_time: Instant,
    system_start_time: SystemTime,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        let system_start_time = SystemTime::now();
        let start_time = Instant::now();
        
        let performance_metrics = PerformanceMetrics {
            storage_operations: StorageMetrics {
                files_stored: 0,
                files_retrieved: 0,
                files_deleted: 0,
                total_storage_bytes: 0,
                used_storage_bytes: 0,
                shard_count: 0,
                replication_factor: 3,
                storage_operations_per_second: 0.0,
                average_storage_latency_ms: 0.0,
                encryption_operations: 0,
                decryption_operations: 0,
            },
            network_operations: NetworkMetrics {
                connected_peers: 0,
                total_peers: 0,
                messages_sent: 0,
                messages_received: 0,
                bytes_sent: 0,
                bytes_received: 0,
                network_latency_ms: 0.0,
                connection_attempts: 0,
                connection_failures: 0,
                discovery_operations: 0,
            },
            consensus_operations: ConsensusMetrics {
                current_round: 0,
                blocks_created: 0,
                transactions_processed: 0,
                consensus_participation_rate: 0.0,
                round_duration_ms: 0.0,
                validator_count: 0,
                total_stake: 0,
                zk_proof_generations: 0,
                zk_proof_verifications: 0,
            },
            api_operations: ApiMetrics {
                total_requests: 0,
                successful_requests: 0,
                failed_requests: 0,
                requests_per_second: 0.0,
                average_response_time_ms: 0.0,
                endpoint_usage: HashMap::new(),
                error_codes: HashMap::new(),
                active_connections: 0,
            },
            system_metrics: SystemMetrics {
                uptime_seconds: 0,
                memory_usage_bytes: 0,
                cpu_usage_percent: 0.0,
                disk_usage_bytes: 0,
                thread_count: 0,
                open_file_descriptors: 0,
                system_load_average: 0.0,
            },
        };

        let health_status = HealthStatus {
            overall_status: HealthState::Healthy,
            components: HashMap::new(),
            last_check: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            uptime_seconds: 0,
        };

        Self {
            metrics: Arc::new(RwLock::new(Vec::new())),
            performance_metrics: Arc::new(RwLock::new(performance_metrics)),
            health_status: Arc::new(RwLock::new(health_status)),
            start_time,
            system_start_time,
        }
    }

    /// Record a metric
    pub async fn record_metric(&self, name: String, value: f64, metric_type: MetricType, description: String) {
        let mut metrics = self.metrics.write().await;
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        
        let metric = Metric {
            name,
            value,
            metric_type,
            labels: HashMap::new(),
            timestamp,
            description,
        };
        
        metrics.push(metric);
    }

    /// Record a metric with labels
    pub async fn record_metric_with_labels(
        &self,
        name: String,
        value: f64,
        metric_type: MetricType,
        labels: HashMap<String, String>,
        description: String,
    ) {
        let mut metrics = self.metrics.write().await;
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        
        let metric = Metric {
            name,
            value,
            metric_type,
            labels,
            timestamp,
            description,
        };
        
        metrics.push(metric);
    }

    /// Update storage metrics
    pub async fn update_storage_metrics(&self, metrics: StorageMetrics) {
        let mut performance_metrics = self.performance_metrics.write().await;
        performance_metrics.storage_operations = metrics;
    }

    /// Update network metrics
    pub async fn update_network_metrics(&self, metrics: NetworkMetrics) {
        let mut performance_metrics = self.performance_metrics.write().await;
        performance_metrics.network_operations = metrics;
    }

    /// Update consensus metrics
    pub async fn update_consensus_metrics(&self, metrics: ConsensusMetrics) {
        let mut performance_metrics = self.performance_metrics.write().await;
        performance_metrics.consensus_operations = metrics;
    }

    /// Update API metrics
    pub async fn update_api_metrics(&self, metrics: ApiMetrics) {
        let mut performance_metrics = self.performance_metrics.write().await;
        performance_metrics.api_operations = metrics;
    }

    /// Update system metrics
    pub async fn update_system_metrics(&self, metrics: SystemMetrics) {
        let mut performance_metrics = self.performance_metrics.write().await;
        performance_metrics.system_metrics = metrics;
    }

    /// Record API request
    pub async fn record_api_request(&self, endpoint: String, status_code: u16, response_time_ms: f64) {
        let mut performance_metrics = self.performance_metrics.write().await;
        let api_metrics = &mut performance_metrics.api_operations;
        
        api_metrics.total_requests += 1;
        
        if status_code < 400 {
            api_metrics.successful_requests += 1;
        } else {
            api_metrics.failed_requests += 1;
        }
        
        // Update endpoint usage
        *api_metrics.endpoint_usage.entry(endpoint).or_insert(0) += 1;
        
        // Update error codes
        if status_code >= 400 {
            *api_metrics.error_codes.entry(status_code).or_insert(0) += 1;
        }
        
        // Update average response time
        let total_time = api_metrics.average_response_time_ms * (api_metrics.total_requests - 1) as f64;
        api_metrics.average_response_time_ms = (total_time + response_time_ms) / api_metrics.total_requests as f64;
        
        // Update requests per second (simplified calculation)
        let uptime = self.start_time.elapsed().as_secs_f64();
        api_metrics.requests_per_second = api_metrics.total_requests as f64 / uptime;
    }

    /// Record storage operation
    pub async fn record_storage_operation(&self, operation_type: &str, latency_ms: f64) {
        let mut performance_metrics = self.performance_metrics.write().await;
        let storage_metrics = &mut performance_metrics.storage_operations;
        
        match operation_type {
            "store" => storage_metrics.files_stored += 1,
            "retrieve" => storage_metrics.files_retrieved += 1,
            "delete" => storage_metrics.files_deleted += 1,
            "encrypt" => storage_metrics.encryption_operations += 1,
            "decrypt" => storage_metrics.decryption_operations += 1,
            _ => {}
        }
        
        // Update average latency
        let total_operations = storage_metrics.files_stored + storage_metrics.files_retrieved + storage_metrics.files_deleted;
        if total_operations > 0 {
            let total_latency = storage_metrics.average_storage_latency_ms * (total_operations - 1) as f64;
            storage_metrics.average_storage_latency_ms = (total_latency + latency_ms) / total_operations as f64;
        }
        
        // Update operations per second
        let uptime = self.start_time.elapsed().as_secs_f64();
        storage_metrics.storage_operations_per_second = total_operations as f64 / uptime;
    }

    /// Record network operation
    pub async fn record_network_operation(&self, operation_type: &str, bytes: u64) {
        let mut performance_metrics = self.performance_metrics.write().await;
        let network_metrics = &mut performance_metrics.network_operations;
        
        match operation_type {
            "message_sent" => {
                network_metrics.messages_sent += 1;
                network_metrics.bytes_sent += bytes;
            }
            "message_received" => {
                network_metrics.messages_received += 1;
                network_metrics.bytes_received += bytes;
            }
            "connection_attempt" => network_metrics.connection_attempts += 1,
            "connection_failure" => network_metrics.connection_failures += 1,
            "discovery" => network_metrics.discovery_operations += 1,
            _ => {}
        }
    }

    /// Update component health
    pub async fn update_component_health(&self, component: String, status: HealthState, message: String, response_time_ms: Option<f64>) {
        let mut health_status = self.health_status.write().await;
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        
        let component_health = ComponentHealth {
            status: status.clone(),
            message,
            last_check: timestamp,
            response_time_ms,
        };
        
        health_status.components.insert(component, component_health);
        health_status.last_check = timestamp;
        
        // Update overall health status
        let mut overall_status = HealthState::Healthy;
        for (_, component_health) in &health_status.components {
            match component_health.status {
                HealthState::Unhealthy => {
                    overall_status = HealthState::Unhealthy;
                    break;
                }
                HealthState::Degraded => {
                    overall_status = HealthState::Degraded;
                }
                _ => {}
            }
        }
        
        health_status.overall_status = overall_status;
        health_status.uptime_seconds = self.system_start_time.elapsed().unwrap_or_default().as_secs();
    }

    /// Get all metrics
    pub async fn get_metrics(&self) -> Vec<Metric> {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }

    /// Get performance metrics
    pub async fn get_performance_metrics(&self) -> PerformanceMetrics {
        let performance_metrics = self.performance_metrics.read().await;
        performance_metrics.clone()
    }

    /// Get health status
    pub async fn get_health_status(&self) -> HealthStatus {
        let health_status = self.health_status.read().await;
        health_status.clone()
    }

    /// Get metrics in Prometheus format
    pub async fn get_prometheus_metrics(&self) -> String {
        let metrics = self.get_metrics().await;
        let mut prometheus_output = String::new();
        
        for metric in metrics {
            let metric_type = match metric.metric_type {
                MetricType::Counter => "counter",
                MetricType::Gauge => "gauge",
                MetricType::Histogram => "histogram",
                MetricType::Summary => "summary",
            };
            
            prometheus_output.push_str(&format!("# HELP {} {}\n", metric.name, metric.description));
            prometheus_output.push_str(&format!("# TYPE {} {}\n", metric.name, metric_type));
            
            let labels_str = if metric.labels.is_empty() {
                String::new()
            } else {
                let labels: Vec<String> = metric.labels.iter()
                    .map(|(k, v)| format!("{}=\"{}\"", k, v))
                    .collect();
                format!("{{{}}}", labels.join(","))
            };
            
            prometheus_output.push_str(&format!("{}{} {}\n", metric.name, labels_str, metric.value));
        }
        
        prometheus_output
    }

    /// Clear old metrics (older than specified duration)
    pub async fn clear_old_metrics(&self, max_age: Duration) {
        let mut metrics = self.metrics.write().await;
        let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let max_age_seconds = max_age.as_secs();
        
        metrics.retain(|metric| current_time - metric.timestamp < max_age_seconds);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_metrics_collector_creation() {
        let collector = MetricsCollector::new();
        let metrics = collector.get_metrics().await;
        assert_eq!(metrics.len(), 0);
    }

    #[tokio::test]
    async fn test_record_metric() {
        let collector = MetricsCollector::new();
        
        collector.record_metric(
            "test_metric".to_string(),
            42.0,
            MetricType::Counter,
            "Test metric".to_string(),
        ).await;
        
        let metrics = collector.get_metrics().await;
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].name, "test_metric");
        assert_eq!(metrics[0].value, 42.0);
    }

    #[tokio::test]
    async fn test_record_metric_with_labels() {
        let collector = MetricsCollector::new();
        let mut labels = HashMap::new();
        labels.insert("component".to_string(), "storage".to_string());
        labels.insert("operation".to_string(), "store".to_string());
        
        collector.record_metric_with_labels(
            "storage_operation".to_string(),
            100.0,
            MetricType::Counter,
            labels,
            "Storage operation metric".to_string(),
        ).await;
        
        let metrics = collector.get_metrics().await;
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].labels.len(), 2);
    }

    #[tokio::test]
    async fn test_record_api_request() {
        let collector = MetricsCollector::new();
        
        collector.record_api_request("/health".to_string(), 200, 50.0).await;
        collector.record_api_request("/status".to_string(), 404, 100.0).await;
        
        let performance_metrics = collector.get_performance_metrics().await;
        assert_eq!(performance_metrics.api_operations.total_requests, 2);
        assert_eq!(performance_metrics.api_operations.successful_requests, 1);
        assert_eq!(performance_metrics.api_operations.failed_requests, 1);
    }

    #[tokio::test]
    async fn test_record_storage_operation() {
        let collector = MetricsCollector::new();
        
        collector.record_storage_operation("store", 25.0).await;
        collector.record_storage_operation("retrieve", 30.0).await;
        
        let performance_metrics = collector.get_performance_metrics().await;
        assert_eq!(performance_metrics.storage_operations.files_stored, 1);
        assert_eq!(performance_metrics.storage_operations.files_retrieved, 1);
        assert_eq!(performance_metrics.storage_operations.average_storage_latency_ms, 27.5);
    }

    #[tokio::test]
    async fn test_update_component_health() {
        let collector = MetricsCollector::new();
        
        collector.update_component_health(
            "storage".to_string(),
            HealthState::Healthy,
            "Storage is healthy".to_string(),
            Some(10.0),
        ).await;
        
        let health_status = collector.get_health_status().await;
        assert_eq!(health_status.overall_status, HealthState::Healthy);
        assert!(health_status.components.contains_key("storage"));
    }

    #[tokio::test]
    async fn test_prometheus_metrics_format() {
        let collector = MetricsCollector::new();
        
        collector.record_metric(
            "test_counter".to_string(),
            42.0,
            MetricType::Counter,
            "Test counter".to_string(),
        ).await;
        
        let prometheus_output = collector.get_prometheus_metrics().await;
        assert!(prometheus_output.contains("test_counter"));
        assert!(prometheus_output.contains("42"));
    }
} 