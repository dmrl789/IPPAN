use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::time::{Instant, Duration};

/// Prometheus metrics for IPPAN node
pub struct Metrics {
    // Counters
    transactions_received: AtomicU64,
    transactions_verified: AtomicU64,
    transactions_finalized: AtomicU64,
    blocks_built: AtomicU64,
    rounds_completed: AtomicU64,
    
    // Gauges
    mempool_size: AtomicU64,
    active_peers: AtomicU64,
    
    // Histograms for latency tracking
    transaction_latency: Arc<RwLock<Vec<Duration>>>,
    block_build_time: Arc<RwLock<Vec<Duration>>>,
    round_duration: Arc<RwLock<Vec<Duration>>>,
    
    // Start time for uptime calculation
    start_time: Instant,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            transactions_received: AtomicU64::new(0),
            transactions_verified: AtomicU64::new(0),
            transactions_finalized: AtomicU64::new(0),
            blocks_built: AtomicU64::new(0),
            rounds_completed: AtomicU64::new(0),
            mempool_size: AtomicU64::new(0),
            active_peers: AtomicU64::new(0),
            transaction_latency: Arc::new(RwLock::new(Vec::new())),
            block_build_time: Arc::new(RwLock::new(Vec::new())),
            round_duration: Arc::new(RwLock::new(Vec::new())),
            start_time: Instant::now(),
        }
    }

    /// Record a transaction received
    pub fn record_transaction_received(&self) {
        self.transactions_received.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a transaction verified
    pub fn record_transaction_verified(&self) {
        self.transactions_verified.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a transaction finalized
    pub fn record_transaction_finalized(&self) {
        self.transactions_finalized.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a block built
    pub fn record_block_built(&self) {
        self.blocks_built.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a round completed
    pub fn record_round_completed(&self) {
        self.rounds_completed.fetch_add(1, Ordering::Relaxed);
    }

    /// Update mempool size
    pub fn update_mempool_size(&self, size: u64) {
        self.mempool_size.store(size, Ordering::Relaxed);
    }

    /// Update active peers count
    pub fn update_active_peers(&self, count: u64) {
        self.active_peers.store(count, Ordering::Relaxed);
    }

    /// Record transaction latency
    pub fn record_transaction_latency(&self, latency: Duration) {
        let mut latencies = self.transaction_latency.write();
        latencies.push(latency);
        
        // Keep only last 1000 measurements
        if latencies.len() > 1000 {
            latencies.remove(0);
        }
    }

    /// Record block build time
    pub fn record_block_build_time(&self, duration: Duration) {
        let mut times = self.block_build_time.write();
        times.push(duration);
        
        if times.len() > 1000 {
            times.remove(0);
        }
    }

    /// Record round duration
    pub fn record_round_duration(&self, duration: Duration) {
        let mut durations = self.round_duration.write();
        durations.push(duration);
        
        if durations.len() > 1000 {
            durations.remove(0);
        }
    }

    /// Get Prometheus metrics in text format
    pub fn get_prometheus_metrics(&self) -> String {
        let mut metrics = String::new();
        
        // Counters
        metrics.push_str(&format!("# HELP ippan_transactions_received_total Total transactions received\n"));
        metrics.push_str(&format!("# TYPE ippan_transactions_received_total counter\n"));
        metrics.push_str(&format!("ippan_transactions_received_total {}\n", 
            self.transactions_received.load(Ordering::Relaxed)));
        
        metrics.push_str(&format!("# HELP ippan_transactions_verified_total Total transactions verified\n"));
        metrics.push_str(&format!("# TYPE ippan_transactions_verified_total counter\n"));
        metrics.push_str(&format!("ippan_transactions_verified_total {}\n", 
            self.transactions_verified.load(Ordering::Relaxed)));
        
        metrics.push_str(&format!("# HELP ippan_transactions_finalized_total Total transactions finalized\n"));
        metrics.push_str(&format!("# TYPE ippan_transactions_finalized_total counter\n"));
        metrics.push_str(&format!("ippan_transactions_finalized_total {}\n", 
            self.transactions_finalized.load(Ordering::Relaxed)));
        
        metrics.push_str(&format!("# HELP ippan_blocks_built_total Total blocks built\n"));
        metrics.push_str(&format!("# TYPE ippan_blocks_built_total counter\n"));
        metrics.push_str(&format!("ippan_blocks_built_total {}\n", 
            self.blocks_built.load(Ordering::Relaxed)));
        
        metrics.push_str(&format!("# HELP ippan_rounds_completed_total Total rounds completed\n"));
        metrics.push_str(&format!("# TYPE ippan_rounds_completed_total counter\n"));
        metrics.push_str(&format!("ippan_rounds_completed_total {}\n", 
            self.rounds_completed.load(Ordering::Relaxed)));
        
        // Gauges
        metrics.push_str(&format!("# HELP ippan_mempool_size Current mempool size\n"));
        metrics.push_str(&format!("# TYPE ippan_mempool_size gauge\n"));
        metrics.push_str(&format!("ippan_mempool_size {}\n", 
            self.mempool_size.load(Ordering::Relaxed)));
        
        metrics.push_str(&format!("# HELP ippan_active_peers Current active peers\n"));
        metrics.push_str(&format!("# TYPE ippan_active_peers gauge\n"));
        metrics.push_str(&format!("ippan_active_peers {}\n", 
            self.active_peers.load(Ordering::Relaxed)));
        
        // Uptime
        let uptime = self.start_time.elapsed().as_secs();
        metrics.push_str(&format!("# HELP ippan_uptime_seconds Node uptime in seconds\n"));
        metrics.push_str(&format!("# TYPE ippan_uptime_seconds gauge\n"));
        metrics.push_str(&format!("ippan_uptime_seconds {}\n", uptime));
        
        // Histograms (simplified - in production you'd use proper histogram buckets)
        if let Some(latency_stats) = self.calculate_latency_stats(&self.transaction_latency.read()) {
            metrics.push_str(&format!("# HELP ippan_transaction_latency_p50_ms Transaction latency 50th percentile\n"));
            metrics.push_str(&format!("# TYPE ippan_transaction_latency_p50_ms gauge\n"));
            metrics.push_str(&format!("ippan_transaction_latency_p50_ms {:.2}\n", latency_stats.p50));
            
            metrics.push_str(&format!("# HELP ippan_transaction_latency_p95_ms Transaction latency 95th percentile\n"));
            metrics.push_str(&format!("# TYPE ippan_transaction_latency_p95_ms gauge\n"));
            metrics.push_str(&format!("ippan_transaction_latency_p95_ms {:.2}\n", latency_stats.p95));
            
            metrics.push_str(&format!("# HELP ippan_transaction_latency_p99_ms Transaction latency 99th percentile\n"));
            metrics.push_str(&format!("# TYPE ippan_transaction_latency_p99_ms gauge\n"));
            metrics.push_str(&format!("ippan_transaction_latency_p99_ms {:.2}\n", latency_stats.p99));
        }
        
        metrics
    }

    /// Calculate latency statistics
    fn calculate_latency_stats(&self, latencies: &[Duration]) -> Option<LatencyStats> {
        if latencies.is_empty() {
            return None;
        }

        let mut sorted: Vec<f64> = latencies.iter()
            .map(|d| d.as_millis() as f64)
            .collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let len = sorted.len();
        let p50_idx = (len as f64 * 0.5) as usize;
        let p95_idx = (len as f64 * 0.95) as usize;
        let p99_idx = (len as f64 * 0.99) as usize;

        Some(LatencyStats {
            p50: sorted[p50_idx.min(len - 1)],
            p95: sorted[p95_idx.min(len - 1)],
            p99: sorted[p99_idx.min(len - 1)],
        })
    }

    /// Get current TPS (transactions per second)
    pub fn get_current_tps(&self) -> f64 {
        let finalized = self.transactions_finalized.load(Ordering::Relaxed);
        let uptime = self.start_time.elapsed().as_secs_f64();
        if uptime > 0.0 {
            finalized as f64 / uptime
        } else {
            0.0
        }
    }
}

#[derive(Debug)]
struct LatencyStats {
    p50: f64,
    p95: f64,
    p99: f64,
}
