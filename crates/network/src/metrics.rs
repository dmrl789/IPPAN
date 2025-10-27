use parking_lot::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

/// Network metrics tracker
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
}

impl NetworkMetrics {
    /// Create a new network metrics tracker
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
        }
    }

    /// Record a sent message
    pub fn record_message_sent(&self, bytes: usize) {
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
        self.bytes_sent.fetch_add(bytes as u64, Ordering::Relaxed);
    }

    /// Record a received message
    pub fn record_message_received(&self, bytes: usize) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
        self.bytes_received.fetch_add(bytes as u64, Ordering::Relaxed);
    }

    /// Record a failed message
    pub fn record_message_failed(&self) {
        self.messages_failed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a dropped message
    pub fn record_message_dropped(&self) {
        self.messages_dropped.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a connection opened
    pub fn record_connection_opened(&self) {
        self.connections_opened.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a connection closed
    pub fn record_connection_closed(&self) {
        self.connections_closed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a failed connection attempt
    pub fn record_connection_failed(&self) {
        self.connections_failed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record message latency
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
        
        // Update average latency using exponential moving average
        let mut avg = self.avg_latency_ms.write();
        let _samples = self.latency_samples.fetch_add(1, Ordering::Relaxed) + 1;
        let alpha = 0.1; // EMA weight
        *avg = *avg * (1.0 - alpha) + (latency_ms as f64) * alpha;
        
        drop(avg); // Explicit drop to release lock
    }

    /// Get current network metrics snapshot
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

    /// Get number of active connections
    pub fn active_connections(&self) -> u64 {
        let opened = self.connections_opened.load(Ordering::Relaxed);
        let closed = self.connections_closed.load(Ordering::Relaxed);
        opened.saturating_sub(closed)
    }

    /// Reset all metrics
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
}

impl Default for NetworkMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of network metrics at a point in time
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
    /// Calculate messages per second
    pub fn messages_per_second(&self) -> f64 {
        if self.uptime_seconds == 0 {
            return 0.0;
        }
        (self.messages_sent + self.messages_received) as f64 / self.uptime_seconds as f64
    }

    /// Calculate bytes per second
    pub fn bytes_per_second(&self) -> f64 {
        if self.uptime_seconds == 0 {
            return 0.0;
        }
        (self.bytes_sent + self.bytes_received) as f64 / self.uptime_seconds as f64
    }

    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        let total = self.messages_sent + self.messages_failed;
        if total == 0 {
            return 1.0;
        }
        self.messages_sent as f64 / total as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_recording() {
        let metrics = NetworkMetrics::new();
        
        metrics.record_message_sent(100);
        metrics.record_message_received(200);
        metrics.record_connection_opened();
        
        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.messages_sent, 1);
        assert_eq!(snapshot.messages_received, 1);
        assert_eq!(snapshot.bytes_sent, 100);
        assert_eq!(snapshot.bytes_received, 200);
        assert_eq!(snapshot.active_connections, 1);
    }

    #[test]
    fn test_latency_tracking() {
        let metrics = NetworkMetrics::new();
        
        metrics.record_latency(Duration::from_millis(10));
        metrics.record_latency(Duration::from_millis(20));
        metrics.record_latency(Duration::from_millis(30));
        
        let snapshot = metrics.snapshot();
        assert!(snapshot.avg_latency_ms > 0.0);
        assert_eq!(snapshot.max_latency_ms, 30);
        assert_eq!(snapshot.latency_samples, 3);
    }

    #[test]
    fn test_active_connections() {
        let metrics = NetworkMetrics::new();
        
        metrics.record_connection_opened();
        metrics.record_connection_opened();
        metrics.record_connection_closed();
        
        assert_eq!(metrics.active_connections(), 1);
    }

    #[test]
    fn test_snapshot_calculations() {
        let snapshot = NetworkMetricsSnapshot {
            messages_sent: 100,
            messages_received: 50,
            messages_failed: 10,
            messages_dropped: 5,
            bytes_sent: 10_000,
            bytes_received: 5_000,
            connections_opened: 10,
            connections_closed: 5,
            connections_failed: 2,
            active_connections: 5,
            uptime_seconds: 10,
            avg_latency_ms: 15.5,
            max_latency_ms: 50,
            latency_samples: 100,
        };
        
        assert_eq!(snapshot.messages_per_second(), 15.0);
        assert_eq!(snapshot.bytes_per_second(), 1500.0);
        assert!((snapshot.success_rate() - 0.909).abs() < 0.01);
    }
}
