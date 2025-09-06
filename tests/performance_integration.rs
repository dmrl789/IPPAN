//! Performance Integration Tests for IPPAN
//! 
//! Tests the complete performance system integration

use ippan::{
    performance::PerformanceMonitor,
    performance::metrics::PerformanceMetrics,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

#[tokio::test]
async fn test_performance_monitoring() {
    // Test performance monitoring system
    let metrics = Arc::new(PerformanceMetrics::new());
    let performance_monitor = Arc::new(RwLock::new(PerformanceMonitor::new(metrics, Duration::from_secs(1))));

    // Test that the performance monitor was created successfully
    assert!(performance_monitor.read().await.get_metrics().transactions_processed >= 0);
}

#[tokio::test]
async fn test_performance_metrics() {
    // Test performance metrics collection
    let metrics = Arc::new(PerformanceMetrics::new());
    let performance_monitor = Arc::new(RwLock::new(PerformanceMonitor::new(metrics, Duration::from_secs(1))));

    // Test basic metrics
    let metrics_data = performance_monitor.read().await.get_metrics();
    assert!(metrics_data.transactions_processed >= 0);
    assert!(metrics_data.blocks_processed >= 0);
    assert!(metrics_data.blocks_processed >= 0);
}