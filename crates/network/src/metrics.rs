//! Network metrics and monitoring for IPPAN
//!
//! Provides comprehensive metrics collection, monitoring, and reporting
//! for the IPPAN network layer.

use anyhow::Result;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::{interval, sleep};
use tracing::{debug, error, info, warn};

/// Network metrics collector
pub struct NetworkMetrics {
    metrics: Arc<RwLock<MetricsData>>,
    is_running: Arc<std::sync::atomic::AtomicBool>,
}

/// Metrics data structure
#[derive(Debug, Clone)]
pub struct MetricsData {
    // Connection metrics
    pub total_connections: u64,
    pub active_connections: u64,
    pub failed_connections: u64,
    pub connection_attempts: u64,
    
    // Message metrics
    pub messages_sent: u64,
    pub messages_received: u64,
    pub messages_dropped: u64,
    pub message_errors: u64,
    
    // Bandwidth metrics
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub peak_bandwidth_sent: u64,
    pub peak_bandwidth_received: u64,
    
    // Peer metrics
    pub peers_discovered: u64,
    pub peers_connected: u64,
    pub peers_disconnected: u64,
    pub peer_exchanges: u64,
    
    // Protocol metrics
    pub handshakes_successful: u64,
    pub handshakes_failed: u64,
    pub ping_requests: u64,
    pub pong_responses: u64,
    
    // Error metrics
    pub timeout_errors: u64,
    pub serialization_errors: u64,
    pub network_errors: u64,
    pub protocol_errors: u64,
    
    // Performance metrics
    pub average_latency_ms: f64,
    pub max_latency_ms: f64,
    pub min_latency_ms: f64,
    
    // Timestamps
    #[serde(skip)]
    pub start_time: Instant,
    #[serde(skip)]
    pub last_update: Instant,
}

impl Default for MetricsData {
    fn default() -> Self {
        Self {
            total_connections: 0,
            active_connections: 0,
            failed_connections: 0,
            connection_attempts: 0,
            messages_sent: 0,
            messages_received: 0,
            messages_dropped: 0,
            message_errors: 0,
            bytes_sent: 0,
            bytes_received: 0,
            peak_bandwidth_sent: 0,
            peak_bandwidth_received: 0,
            peers_discovered: 0,
            peers_connected: 0,
            peers_disconnected: 0,
            peer_exchanges: 0,
            handshakes_successful: 0,
            handshakes_failed: 0,
            ping_requests: 0,
            pong_responses: 0,
            timeout_errors: 0,
            serialization_errors: 0,
            network_errors: 0,
            protocol_errors: 0,
            average_latency_ms: 0.0,
            max_latency_ms: 0.0,
            min_latency_ms: 0.0,
            start_time: Instant::now(),
            last_update: Instant::now(),
        }
    }
}

/// Per-peer metrics
#[derive(Debug, Clone)]
pub struct PeerMetrics {
    pub peer_id: String,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    #[serde(skip)]
    pub connection_time: Instant,
    #[serde(skip)]
    pub last_activity: Instant,
    pub latency_ms: f64,
    pub reputation_score: f64,
    pub error_count: u64,
}

impl Default for PeerMetrics {
    fn default() -> Self {
        Self {
            peer_id: String::new(),
            messages_sent: 0,
            messages_received: 0,
            bytes_sent: 0,
            bytes_received: 0,
            connection_time: Instant::now(),
            last_activity: Instant::now(),
            latency_ms: 0.0,
            reputation_score: 0.5,
            error_count: 0,
        }
    }
}

/// Metrics collector implementation
impl NetworkMetrics {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(MetricsData::default())),
            is_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Start the metrics collector
    pub async fn start(&self) -> Result<()> {
        self.is_running.store(true, std::sync::atomic::Ordering::SeqCst);
        
        // Start metrics collection tasks
        self.start_metrics_aggregation().await;
        self.start_metrics_reporting().await;

        info!("Network metrics collector started");
        Ok(())
    }

    /// Stop the metrics collector
    pub async fn stop(&self) -> Result<()> {
        self.is_running.store(false, std::sync::atomic::Ordering::SeqCst);
        info!("Network metrics collector stopped");
        Ok(())
    }

    /// Record a connection attempt
    pub fn record_connection_attempt(&self) {
        let mut metrics = self.metrics.write();
        metrics.connection_attempts += 1;
        metrics.last_update = Instant::now();
    }

    /// Record a successful connection
    pub fn record_connection_success(&self) {
        let mut metrics = self.metrics.write();
        metrics.total_connections += 1;
        metrics.active_connections += 1;
        metrics.last_update = Instant::now();
    }

    /// Record a failed connection
    pub fn record_connection_failure(&self) {
        let mut metrics = self.metrics.write();
        metrics.failed_connections += 1;
        metrics.active_connections = metrics.active_connections.saturating_sub(1);
        metrics.last_update = Instant::now();
    }

    /// Record a message sent
    pub fn record_message_sent(&self, bytes: usize) {
        let mut metrics = self.metrics.write();
        metrics.messages_sent += 1;
        metrics.bytes_sent += bytes as u64;
        metrics.last_update = Instant::now();
    }

    /// Record a message received
    pub fn record_message_received(&self, bytes: usize) {
        let mut metrics = self.metrics.write();
        metrics.messages_received += 1;
        metrics.bytes_received += bytes as u64;
        metrics.last_update = Instant::now();
    }

    /// Record a message dropped
    pub fn record_message_dropped(&self) {
        let mut metrics = self.metrics.write();
        metrics.messages_dropped += 1;
        metrics.last_update = Instant::now();
    }

    /// Record a message error
    pub fn record_message_error(&self) {
        let mut metrics = self.metrics.write();
        metrics.message_errors += 1;
        metrics.last_update = Instant::now();
    }

    /// Record peer discovery
    pub fn record_peer_discovered(&self) {
        let mut metrics = self.metrics.write();
        metrics.peers_discovered += 1;
        metrics.last_update = Instant::now();
    }

    /// Record peer connection
    pub fn record_peer_connected(&self) {
        let mut metrics = self.metrics.write();
        metrics.peers_connected += 1;
        metrics.last_update = Instant::now();
    }

    /// Record peer disconnection
    pub fn record_peer_disconnected(&self) {
        let mut metrics = self.metrics.write();
        metrics.peers_disconnected += 1;
        metrics.active_connections = metrics.active_connections.saturating_sub(1);
        metrics.last_update = Instant::now();
    }

    /// Record peer exchange
    pub fn record_peer_exchange(&self) {
        let mut metrics = self.metrics.write();
        metrics.peer_exchanges += 1;
        metrics.last_update = Instant::now();
    }

    /// Record handshake success
    pub fn record_handshake_success(&self) {
        let mut metrics = self.metrics.write();
        metrics.handshakes_successful += 1;
        metrics.last_update = Instant::now();
    }

    /// Record handshake failure
    pub fn record_handshake_failure(&self) {
        let mut metrics = self.metrics.write();
        metrics.handshakes_failed += 1;
        metrics.last_update = Instant::now();
    }

    /// Record ping request
    pub fn record_ping_request(&self) {
        let mut metrics = self.metrics.write();
        metrics.ping_requests += 1;
        metrics.last_update = Instant::now();
    }

    /// Record pong response
    pub fn record_pong_response(&self) {
        let mut metrics = self.metrics.write();
        metrics.pong_responses += 1;
        metrics.last_update = Instant::now();
    }

    /// Record timeout error
    pub fn record_timeout_error(&self) {
        let mut metrics = self.metrics.write();
        metrics.timeout_errors += 1;
        metrics.last_update = Instant::now();
    }

    /// Record serialization error
    pub fn record_serialization_error(&self) {
        let mut metrics = self.metrics.write();
        metrics.serialization_errors += 1;
        metrics.last_update = Instant::now();
    }

    /// Record network error
    pub fn record_network_error(&self) {
        let mut metrics = self.metrics.write();
        metrics.network_errors += 1;
        metrics.last_update = Instant::now();
    }

    /// Record protocol error
    pub fn record_protocol_error(&self) {
        let mut metrics = self.metrics.write();
        metrics.protocol_errors += 1;
        metrics.last_update = Instant::now();
    }

    /// Record latency measurement
    pub fn record_latency(&self, latency_ms: f64) {
        let mut metrics = self.metrics.write();
        
        // Update min/max latency
        if metrics.min_latency_ms == 0.0 || latency_ms < metrics.min_latency_ms {
            metrics.min_latency_ms = latency_ms;
        }
        if latency_ms > metrics.max_latency_ms {
            metrics.max_latency_ms = latency_ms;
        }
        
        // Update average latency (simple moving average)
        if metrics.average_latency_ms == 0.0 {
            metrics.average_latency_ms = latency_ms;
        } else {
            metrics.average_latency_ms = (metrics.average_latency_ms + latency_ms) / 2.0;
        }
        
        metrics.last_update = Instant::now();
    }

    /// Get current metrics
    pub fn get_metrics(&self) -> MetricsData {
        self.metrics.read().clone()
    }

    /// Get metrics summary
    pub fn get_summary(&self) -> MetricsSummary {
        let metrics = self.metrics.read();
        let uptime = metrics.start_time.elapsed();
        
        MetricsSummary {
            uptime_seconds: uptime.as_secs(),
            total_connections: metrics.total_connections,
            active_connections: metrics.active_connections,
            messages_per_second: if uptime.as_secs() > 0 {
                metrics.messages_sent as f64 / uptime.as_secs() as f64
            } else {
                0.0
            },
            bytes_per_second: if uptime.as_secs() > 0 {
                metrics.bytes_sent as f64 / uptime.as_secs() as f64
            } else {
                0.0
            },
            average_latency_ms: metrics.average_latency_ms,
            error_rate: if metrics.messages_sent > 0 {
                (metrics.message_errors as f64 / metrics.messages_sent as f64) * 100.0
            } else {
                0.0
            },
        }
    }

    /// Start metrics aggregation
    async fn start_metrics_aggregation(&self) {
        let metrics = self.metrics.clone();
        let is_running = self.is_running.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(60)); // 1 minute
            
            while is_running.load(std::sync::atomic::Ordering::SeqCst) {
                interval.tick().await;
                
                // Update peak bandwidth metrics
                {
                    let mut metrics_guard = metrics.write();
                    let current_bandwidth_sent = metrics_guard.bytes_sent;
                    let current_bandwidth_received = metrics_guard.bytes_received;
                    
                    if current_bandwidth_sent > metrics_guard.peak_bandwidth_sent {
                        metrics_guard.peak_bandwidth_sent = current_bandwidth_sent;
                    }
                    if current_bandwidth_received > metrics_guard.peak_bandwidth_received {
                        metrics_guard.peak_bandwidth_received = current_bandwidth_received;
                    }
                }
            }
        });
    }

    /// Start metrics reporting
    async fn start_metrics_reporting(&self) {
        let metrics = self.metrics.clone();
        let is_running = self.is_running.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(300)); // 5 minutes
            
            while is_running.load(std::sync::atomic::Ordering::SeqCst) {
                interval.tick().await;
                
                let summary = {
                    let metrics_guard = metrics.read();
                    let uptime = metrics_guard.start_time.elapsed();
                    
                    MetricsSummary {
                        uptime_seconds: uptime.as_secs(),
                        total_connections: metrics_guard.total_connections,
                        active_connections: metrics_guard.active_connections,
                        messages_per_second: if uptime.as_secs() > 0 {
                            metrics_guard.messages_sent as f64 / uptime.as_secs() as f64
                        } else {
                            0.0
                        },
                        bytes_per_second: if uptime.as_secs() > 0 {
                            metrics_guard.bytes_sent as f64 / uptime.as_secs() as f64
                        } else {
                            0.0
                        },
                        average_latency_ms: metrics_guard.average_latency_ms,
                        error_rate: if metrics_guard.messages_sent > 0 {
                            (metrics_guard.message_errors as f64 / metrics_guard.messages_sent as f64) * 100.0
                        } else {
                            0.0
                        },
                    }
                };
                
                info!("Network metrics: {:?}", summary);
            }
        });
    }
}

/// Metrics summary for reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSummary {
    pub uptime_seconds: u64,
    pub total_connections: u64,
    pub active_connections: u64,
    pub messages_per_second: f64,
    pub bytes_per_second: f64,
    pub average_latency_ms: f64,
    pub error_rate: f64,
}

/// Metrics collector trait for extensibility
pub trait MetricsCollector: Send + Sync {
    fn record_metric(&self, name: &str, value: f64);
    fn get_metric(&self, name: &str) -> Option<f64>;
    fn get_all_metrics(&self) -> HashMap<String, f64>;
}

impl MetricsCollector for NetworkMetrics {
    fn record_metric(&self, name: &str, value: f64) {
        // This would be implemented to record custom metrics
        debug!("Recording metric {}: {}", name, value);
    }

    fn get_metric(&self, name: &str) -> Option<f64> {
        // This would be implemented to retrieve custom metrics
        debug!("Getting metric: {}", name);
        None
    }

    fn get_all_metrics(&self) -> HashMap<String, f64> {
        // This would be implemented to retrieve all custom metrics
        HashMap::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let metrics = NetworkMetrics::new();
        let data = metrics.get_metrics();
        assert_eq!(data.total_connections, 0);
    }

    #[test]
    fn test_metrics_recording() {
        let metrics = NetworkMetrics::new();
        metrics.record_connection_attempt();
        metrics.record_connection_success();
        metrics.record_message_sent(1024);
        
        let data = metrics.get_metrics();
        assert_eq!(data.connection_attempts, 1);
        assert_eq!(data.total_connections, 1);
        assert_eq!(data.messages_sent, 1);
        assert_eq!(data.bytes_sent, 1024);
    }

    #[test]
    fn test_metrics_summary() {
        let metrics = NetworkMetrics::new();
        metrics.record_connection_success();
        metrics.record_message_sent(1024);
        
        let summary = metrics.get_summary();
        assert_eq!(summary.total_connections, 1);
        assert_eq!(summary.active_connections, 1);
    }
}
