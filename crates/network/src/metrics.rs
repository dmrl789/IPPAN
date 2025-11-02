//! IPPAN Network Metrics
//!
//! Provides deterministic, thread-safe tracking of network activity
//! (messages, connections, bytes, and latency) with optional async
//! aggregation and periodic reporting for production environments.

use anyhow::Result;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use std::time::{Duration, Instant};
use tokio::time::interval;
use tracing::{debug, info};

/// Core deterministic network metrics
#[derive(Debug)]
pub struct NetworkMetrics {
    // Message counters
    messages_sent: AtomicU64,
    messages_received: AtomicU64,
    messages_failed: AtomicU64,
    messages_dropped: AtomicU64,

    // Byte counters
    bytes_sent: AtomicU64,
    bytes_received: AtomicU64,

    // Connection counters
    connections_opened: AtomicU64,
    connections_closed: AtomicU64,
    connections_failed: AtomicU64,

    // Timing
    start_time: Instant,

    // Latency tracking
    avg_latency_ms: RwLock<f64>,
    max_latency_ms: AtomicU64,
    latency_samples: AtomicU64,

    // Async state (optional)
    is_running: Arc<AtomicBool>,
}

impl NetworkMetrics {
    /// Create a new deterministic metrics tracker
    pub fn new() -> Self {
        Self {
            messages_sent: AtomicU64::new(0),
            messages_received: AtomicU64::new(0),
            messages_failed: AtomicU64::new(0),
            messages_dropped: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            connections_opened: AtomicU64::new(0),
            connections_closed: AtomicU64::new(0),
            connections_failed: AtomicU64::new(0),
            start_time: Instant::now(),
            avg_latency_ms: RwLock::new(0.0),
            max_latency_ms: AtomicU64::new(0),
            latency_samples: AtomicU64::new(0),
            is_running: Arc::new(AtomicBool::new(false)),
        }
    }

    // ----------------------------
    // Recording primitives
    // ----------------------------

    pub fn record_message_sent(&self, bytes: usize) {
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
        self.bytes_sent.fetch_add(bytes as u64, Ordering::Relaxed);
    }

    pub fn record_message_received(&self, bytes: usize) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
        self.bytes_received
            .fetch_add(bytes as u64, Ordering::Relaxed);
    }

    pub fn record_message_failed(&self) {
        self.messages_failed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_message_dropped(&self) {
        self.messages_dropped.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_connection_opened(&self) {
        self.connections_opened.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_connection_closed(&self) {
        self.connections_closed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_connection_failed(&self) {
        self.connections_failed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_latency(&self, latency: Duration) {
        let latency_ms = latency.as_millis() as u64;

        // Update max latency
        let mut current_max = self.max_latency_ms.load(Ordering::Relaxed);
        while latency_ms > current_max {
            match self.max_latency_ms.compare_exchange_weak(
                current_max,
                latency_ms,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => current_max = x,
            }
        }

        // Update average latency (EMA)
        let mut avg = self.avg_latency_ms.write();
        self.latency_samples.fetch_add(1, Ordering::Relaxed);
        let alpha = 0.1;
        *avg = *avg * (1.0 - alpha) + (latency_ms as f64) * alpha;
    }

    // ----------------------------
    // Aggregation and snapshots
    // ----------------------------

    pub fn snapshot(&self) -> NetworkMetricsSnapshot {
        NetworkMetricsSnapshot {
            messages_sent: self.messages_sent.load(Ordering::Relaxed),
            messages_received: self.messages_received.load(Ordering::Relaxed),
            messages_failed: self.messages_failed.load(Ordering::Relaxed),
            messages_dropped: self.messages_dropped.load(Ordering::Relaxed),
            bytes_sent: self.bytes_sent.load(Ordering::Relaxed),
            bytes_received: self.bytes_received.load(Ordering::Relaxed),
            connections_opened: self.connections_opened.load(Ordering::Relaxed),
            connections_closed: self.connections_closed.load(Ordering::Relaxed),
            connections_failed: self.connections_failed.load(Ordering::Relaxed),
            active_connections: self.active_connections(),
            uptime_seconds: self.start_time.elapsed().as_secs(),
            avg_latency_ms: *self.avg_latency_ms.read(),
            max_latency_ms: self.max_latency_ms.load(Ordering::Relaxed),
            latency_samples: self.latency_samples.load(Ordering::Relaxed),
        }
    }

    pub fn active_connections(&self) -> u64 {
        let opened = self.connections_opened.load(Ordering::Relaxed);
        let closed = self.connections_closed.load(Ordering::Relaxed);
        opened.saturating_sub(closed)
    }

    pub fn reset(&self) {
        self.messages_sent.store(0, Ordering::Relaxed);
        self.messages_received.store(0, Ordering::Relaxed);
        self.messages_failed.store(0, Ordering::Relaxed);
        self.messages_dropped.store(0, Ordering::Relaxed);
        self.bytes_sent.store(0, Ordering::Relaxed);
        self.bytes_received.store(0, Ordering::Relaxed);
        self.connections_opened.store(0, Ordering::Relaxed);
        self.connections_closed.store(0, Ordering::Relaxed);
        self.connections_failed.store(0, Ordering::Relaxed);
        *self.avg_latency_ms.write() = 0.0;
        self.max_latency_ms.store(0, Ordering::Relaxed);
        self.latency_samples.store(0, Ordering::Relaxed);
    }

    // ----------------------------
    // Async reporting
    // ----------------------------

    pub async fn start_reporting(self: Arc<Self>, interval_secs: u64) -> Result<()> {
        if self.is_running.load(Ordering::SeqCst) {
            return Ok(()); // already running
        }
        self.is_running.store(true, Ordering::SeqCst);
        let is_running = self.is_running.clone();
        let metrics = self;

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(interval_secs));
            while is_running.load(Ordering::SeqCst) {
                ticker.tick().await;
                let snapshot = metrics.snapshot();
                info!(
                    target: "network::metrics",
                    "Metrics snapshot: messages={} recv={} conn={} uptime={}s avg_lat={:.2}ms",
                    snapshot.messages_sent,
                    snapshot.messages_received,
                    snapshot.active_connections,
                    snapshot.uptime_seconds,
                    snapshot.avg_latency_ms
                );
            }
        });

        Ok(())
    }

    pub async fn stop_reporting(&self) {
        self.is_running.store(false, Ordering::SeqCst);
    }
}

impl Default for NetworkMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Read-only metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMetricsSnapshot {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub messages_failed: u64,
    pub messages_dropped: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub connections_opened: u64,
    pub connections_closed: u64,
    pub connections_failed: u64,
    pub active_connections: u64,
    pub uptime_seconds: u64,
    pub avg_latency_ms: f64,
    pub max_latency_ms: u64,
    pub latency_samples: u64,
}

impl NetworkMetricsSnapshot {
    pub fn messages_per_second(&self) -> f64 {
        if self.uptime_seconds == 0 {
            0.0
        } else {
            (self.messages_sent + self.messages_received) as f64 / self.uptime_seconds as f64
        }
    }

    pub fn bytes_per_second(&self) -> f64 {
        if self.uptime_seconds == 0 {
            0.0
        } else {
            (self.bytes_sent + self.bytes_received) as f64 / self.uptime_seconds as f64
        }
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.messages_sent + self.messages_failed;
        if total == 0 {
            1.0
        } else {
            self.messages_sent as f64 / total as f64
        }
    }
}

/// Generic trait for metric collection extensibility
pub trait MetricsCollector: Send + Sync {
    fn record_metric(&self, name: &str, value: f64);
    fn get_metric(&self, name: &str) -> Option<f64>;
    fn get_all_metrics(&self) -> HashMap<String, f64>;
}

impl MetricsCollector for NetworkMetrics {
    fn record_metric(&self, name: &str, value: f64) {
        debug!("Custom metric {}: {}", name, value);
        let _ = value;
    }

    fn get_metric(&self, _name: &str) -> Option<f64> {
        None
    }

    fn get_all_metrics(&self) -> HashMap<String, f64> {
        HashMap::new()
    }
}

// ------------------------------------------------------------
// ✅ Tests
// ------------------------------------------------------------
#[cfg(all(test, feature = "enable-tests"))]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_basic_counters() {
        let metrics = NetworkMetrics::new();
        metrics.record_message_sent(100);
        metrics.record_message_received(200);
        metrics.record_connection_opened();

        let snap = metrics.snapshot();
        assert_eq!(snap.messages_sent, 1);
        assert_eq!(snap.messages_received, 1);
        assert_eq!(snap.bytes_sent, 100);
        assert_eq!(snap.bytes_received, 200);
        assert_eq!(snap.active_connections, 1);
    }

    #[test]
    fn test_latency_recording() {
        let metrics = NetworkMetrics::new();
        metrics.record_latency(Duration::from_millis(10));
        metrics.record_latency(Duration::from_millis(20));
        let snap = metrics.snapshot();
        assert!(snap.avg_latency_ms > 0.0);
        assert_eq!(snap.max_latency_ms, 20);
    }

    #[test]
    fn test_snapshot_rates() {
        let snap = NetworkMetricsSnapshot {
            messages_sent: 100,
            messages_received: 50,
            messages_failed: 10,
            messages_dropped: 0,
            bytes_sent: 1000,
            bytes_received: 500,
            connections_opened: 5,
            connections_closed: 3,
            connections_failed: 0,
            active_connections: 2,
            uptime_seconds: 10,
            avg_latency_ms: 12.3,
            max_latency_ms: 40,
            latency_samples: 2,
        };
        assert_eq!(snap.messages_per_second(), 15.0);
        assert_eq!(snap.bytes_per_second(), 150.0);
        assert!((snap.success_rate() - 0.909).abs() < 0.01);
    }
}
