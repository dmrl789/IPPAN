//! Monitoring and metrics system for IPPAN
//! 
//! Provides real-time metrics collection, performance monitoring, and health tracking

pub mod metrics;
pub mod dashboard;

use crate::Result;
use axum::Router;
use std::sync::Arc;

/// Main monitoring system that integrates metrics collection and dashboard
pub struct MonitoringSystem {
    metrics_collector: Arc<metrics::MetricsCollector>,
    dashboard_server: dashboard::DashboardServer,
}

impl MonitoringSystem {
    /// Create a new monitoring system
    pub fn new() -> Self {
        let metrics_collector = Arc::new(metrics::MetricsCollector::new());
        let dashboard_server = dashboard::DashboardServer::new(Arc::clone(&metrics_collector));
        
        Self {
            metrics_collector,
            dashboard_server,
        }
    }

    /// Get the metrics collector
    pub fn get_metrics_collector(&self) -> Arc<metrics::MetricsCollector> {
        Arc::clone(&self.metrics_collector)
    }

    /// Create the monitoring router with dashboard endpoints
    pub fn create_router(&self) -> Router {
        self.dashboard_server.create_router()
    }

    /// Start the monitoring system
    pub async fn start(&self) -> Result<()> {
        log::info!("Starting IPPAN monitoring system...");
        
        // Initialize system health components
        self.update_system_health().await;
        
        log::info!("Monitoring system started successfully");
        Ok(())
    }

    /// Stop the monitoring system
    pub async fn stop(&self) -> Result<()> {
        log::info!("Stopping IPPAN monitoring system...");
        log::info!("Monitoring system stopped");
        Ok(())
    }

    /// Update system health status
    async fn update_system_health(&self) {
        // Update storage health
        self.metrics_collector.update_component_health(
            "storage".to_string(),
            metrics::HealthState::Healthy,
            "Storage system is operational".to_string(),
            Some(5.0),
        ).await;

        // Update network health
        self.metrics_collector.update_component_health(
            "network".to_string(),
            metrics::HealthState::Healthy,
            "Network system is operational".to_string(),
            Some(10.0),
        ).await;

        // Update consensus health
        self.metrics_collector.update_component_health(
            "consensus".to_string(),
            metrics::HealthState::Healthy,
            "Consensus system is operational".to_string(),
            Some(15.0),
        ).await;

        // Update API health
        self.metrics_collector.update_component_health(
            "api".to_string(),
            metrics::HealthState::Healthy,
            "API system is operational".to_string(),
            Some(2.0),
        ).await;
    }

    /// Record a system metric
    pub async fn record_metric(&self, name: String, value: f64, metric_type: metrics::MetricType, description: String) {
        self.metrics_collector.record_metric(name, value, metric_type, description).await;
    }

    /// Record an API request
    pub async fn record_api_request(&self, endpoint: String, status_code: u16, response_time_ms: f64) {
        self.metrics_collector.record_api_request(endpoint, status_code, response_time_ms).await;
    }

    /// Record a storage operation
    pub async fn record_storage_operation(&self, operation_type: &str, latency_ms: f64) {
        self.metrics_collector.record_storage_operation(operation_type, latency_ms).await;
    }

    /// Record a network operation
    pub async fn record_network_operation(&self, operation_type: &str, bytes: u64) {
        self.metrics_collector.record_network_operation(operation_type, bytes).await;
    }

    /// Get performance metrics
    pub async fn get_performance_metrics(&self) -> metrics::PerformanceMetrics {
        self.metrics_collector.get_performance_metrics().await
    }

    /// Get health status
    pub async fn get_health_status(&self) -> metrics::HealthStatus {
        self.metrics_collector.get_health_status().await
    }

    /// Get Prometheus metrics
    pub async fn get_prometheus_metrics(&self) -> String {
        self.metrics_collector.get_prometheus_metrics().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_monitoring_system_creation() {
        let monitoring = MonitoringSystem::new();
        let health_status = monitoring.get_health_status().await;
        assert_eq!(health_status.overall_status, metrics::HealthState::Healthy);
    }

    #[tokio::test]
    async fn test_record_metric() {
        let monitoring = MonitoringSystem::new();
        
        monitoring.record_metric(
            "test_metric".to_string(),
            42.0,
            metrics::MetricType::Counter,
            "Test metric".to_string(),
        ).await;
        
        let metrics = monitoring.metrics_collector.get_metrics().await;
        assert_eq!(metrics.len(), 1);
    }

    #[tokio::test]
    async fn test_record_api_request() {
        let monitoring = MonitoringSystem::new();
        
        monitoring.record_api_request("/health".to_string(), 200, 50.0).await;
        
        let performance_metrics = monitoring.get_performance_metrics().await;
        assert_eq!(performance_metrics.api_operations.total_requests, 1);
    }

    #[tokio::test]
    async fn test_record_storage_operation() {
        let monitoring = MonitoringSystem::new();
        
        monitoring.record_storage_operation("store", 25.0).await;
        
        let performance_metrics = monitoring.get_performance_metrics().await;
        assert_eq!(performance_metrics.storage_operations.files_stored, 1);
    }
} 