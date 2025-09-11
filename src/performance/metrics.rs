//! Performance metrics and monitoring for IPPAN
//! 
//! This module provides comprehensive performance monitoring and metrics
//! collection for high-throughput operations.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Performance metrics collector
pub struct PerformanceMetrics {
    // Transaction metrics
    pub transactions_processed: AtomicU64,
    pub transactions_per_second: AtomicU64,
    pub peak_tps: AtomicU64,
    pub average_tps: AtomicU64,
    
    // Block metrics
    pub blocks_processed: AtomicU64,
    pub blocks_per_second: AtomicU64,
    pub peak_bps: AtomicU64,
    pub average_bps: AtomicU64,
    
    // Network metrics
    pub messages_sent: AtomicU64,
    pub messages_received: AtomicU64,
    pub bytes_sent: AtomicU64,
    pub bytes_received: AtomicU64,
    pub network_latency: AtomicU64,
    
    // Memory metrics
    pub memory_usage: AtomicUsize,
    pub peak_memory_usage: AtomicUsize,
    pub cache_hit_rate: AtomicU64,
    pub cache_miss_rate: AtomicU64,
    
    // Consensus metrics
    pub consensus_rounds: AtomicU64,
    pub consensus_latency: AtomicU64,
    pub validator_count: AtomicUsize,
    pub active_validators: AtomicUsize,
    
    // Error metrics
    pub error_count: AtomicU64,
    pub validation_errors: AtomicU64,
    pub network_errors: AtomicU64,
    pub consensus_errors: AtomicU64,
    
    // Timing metrics
    pub total_processing_time: AtomicU64,
    pub average_processing_time: AtomicU64,
    pub peak_processing_time: AtomicU64,
    
    // Custom metrics
    custom_metrics: Arc<RwLock<HashMap<String, AtomicU64>>>,
    
    // Start time for calculating averages
    start_time: Instant,
}

impl PerformanceMetrics {
    /// Create a new performance metrics collector
    pub fn new() -> Self {
        Self {
            transactions_processed: AtomicU64::new(0),
            transactions_per_second: AtomicU64::new(0),
            peak_tps: AtomicU64::new(0),
            average_tps: AtomicU64::new(0),
            
            blocks_processed: AtomicU64::new(0),
            blocks_per_second: AtomicU64::new(0),
            peak_bps: AtomicU64::new(0),
            average_bps: AtomicU64::new(0),
            
            messages_sent: AtomicU64::new(0),
            messages_received: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            network_latency: AtomicU64::new(0),
            
            memory_usage: AtomicUsize::new(0),
            peak_memory_usage: AtomicUsize::new(0),
            cache_hit_rate: AtomicU64::new(0),
            cache_miss_rate: AtomicU64::new(0),
            
            consensus_rounds: AtomicU64::new(0),
            consensus_latency: AtomicU64::new(0),
            validator_count: AtomicUsize::new(0),
            active_validators: AtomicUsize::new(0),
            
            error_count: AtomicU64::new(0),
            validation_errors: AtomicU64::new(0),
            network_errors: AtomicU64::new(0),
            consensus_errors: AtomicU64::new(0),
            
            total_processing_time: AtomicU64::new(0),
            average_processing_time: AtomicU64::new(0),
            peak_processing_time: AtomicU64::new(0),
            
            custom_metrics: Arc::new(RwLock::new(HashMap::new())),
            start_time: Instant::now(),
        }
    }

    /// Record transaction processing
    pub fn record_transaction(&self, processing_time: Duration) {
        self.transactions_processed.fetch_add(1, Ordering::Relaxed);
        self.total_processing_time.fetch_add(processing_time.as_micros() as u64, Ordering::Relaxed);
        
        // Update TPS
        let elapsed = self.start_time.elapsed();
        if elapsed.as_secs() > 0 {
            let tps = self.transactions_processed.load(Ordering::Relaxed) / elapsed.as_secs();
            self.transactions_per_second.store(tps, Ordering::Relaxed);
            
            // Update peak TPS
            let current_peak = self.peak_tps.load(Ordering::Relaxed);
            if tps > current_peak {
                self.peak_tps.store(tps, Ordering::Relaxed);
            }
        }
        
        // Update average processing time
        let total_time = self.total_processing_time.load(Ordering::Relaxed);
        let total_transactions = self.transactions_processed.load(Ordering::Relaxed);
        if total_transactions > 0 {
            let avg_time = total_time / total_transactions;
            self.average_processing_time.store(avg_time, Ordering::Relaxed);
        }
        
        // Update peak processing time
        let current_peak = self.peak_processing_time.load(Ordering::Relaxed);
        let processing_time_micros = processing_time.as_micros() as u64;
        if processing_time_micros > current_peak {
            self.peak_processing_time.store(processing_time_micros, Ordering::Relaxed);
        }
    }

    /// Record block processing
    pub fn record_block(&self, processing_time: Duration) {
        self.blocks_processed.fetch_add(1, Ordering::Relaxed);
        
        // Update BPS
        let elapsed = self.start_time.elapsed();
        if elapsed.as_secs() > 0 {
            let bps = self.blocks_processed.load(Ordering::Relaxed) / elapsed.as_secs();
            self.blocks_per_second.store(bps, Ordering::Relaxed);
            
            // Update peak BPS
            let current_peak = self.peak_bps.load(Ordering::Relaxed);
            if bps > current_peak {
                self.peak_bps.store(bps, Ordering::Relaxed);
            }
        }
    }

    /// Record network activity
    pub fn record_network_send(&self, bytes: usize) {
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
        self.bytes_sent.fetch_add(bytes as u64, Ordering::Relaxed);
    }

    /// Record network receive
    pub fn record_network_receive(&self, bytes: usize) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
        self.bytes_received.fetch_add(bytes as u64, Ordering::Relaxed);
    }

    /// Record network latency
    pub fn record_network_latency(&self, latency: Duration) {
        self.network_latency.store(latency.as_micros() as u64, Ordering::Relaxed);
    }

    /// Record memory usage
    pub fn record_memory_usage(&self, usage: usize) {
        self.memory_usage.store(usage, Ordering::Relaxed);
        
        // Update peak memory usage
        let current_peak = self.peak_memory_usage.load(Ordering::Relaxed);
        if usage > current_peak {
            self.peak_memory_usage.store(usage, Ordering::Relaxed);
        }
    }

    /// Record cache hit
    pub fn record_cache_hit(&self) {
        self.cache_hit_rate.fetch_add(1, Ordering::Relaxed);
    }

    /// Record cache miss
    pub fn record_cache_miss(&self) {
        self.cache_miss_rate.fetch_add(1, Ordering::Relaxed);
    }

    /// Record consensus activity
    pub fn record_consensus_round(&self, latency: Duration) {
        self.consensus_rounds.fetch_add(1, Ordering::Relaxed);
        self.consensus_latency.store(latency.as_micros() as u64, Ordering::Relaxed);
    }

    /// Record validator count
    pub fn record_validator_count(&self, count: usize) {
        self.validator_count.store(count, Ordering::Relaxed);
    }

    /// Record active validators
    pub fn record_active_validators(&self, count: usize) {
        self.active_validators.store(count, Ordering::Relaxed);
    }

    /// Record error
    pub fn record_error(&self, error_type: ErrorType) {
        self.error_count.fetch_add(1, Ordering::Relaxed);
        
        match error_type {
            ErrorType::Validation => { self.validation_errors.fetch_add(1, Ordering::Relaxed); },
            ErrorType::Network => { self.network_errors.fetch_add(1, Ordering::Relaxed); },
            ErrorType::Consensus => { self.consensus_errors.fetch_add(1, Ordering::Relaxed); },
        }
    }

    /// Record custom metric
    pub async fn record_custom_metric(&self, name: String, value: u64) {
        let mut custom_metrics = self.custom_metrics.write().await;
        if let Some(metric) = custom_metrics.get(&name) {
            metric.fetch_add(value, Ordering::Relaxed);
        } else {
            let new_metric = AtomicU64::new(value);
            custom_metrics.insert(name, new_metric);
        }
    }

    /// Get all metrics as a summary
    pub fn get_summary(&self) -> PerformanceSummary {
        let elapsed = self.start_time.elapsed();
        let total_transactions = self.transactions_processed.load(Ordering::Relaxed);
        let total_blocks = self.blocks_processed.load(Ordering::Relaxed);
        
        PerformanceSummary {
            uptime: elapsed,
            transactions_processed: total_transactions,
            blocks_processed: total_blocks,
            current_tps: self.transactions_per_second.load(Ordering::Relaxed),
            peak_tps: self.peak_tps.load(Ordering::Relaxed),
            current_bps: self.blocks_per_second.load(Ordering::Relaxed),
            peak_bps: self.peak_bps.load(Ordering::Relaxed),
            memory_usage: self.memory_usage.load(Ordering::Relaxed),
            peak_memory_usage: self.peak_memory_usage.load(Ordering::Relaxed),
            cache_hit_rate: self.cache_hit_rate.load(Ordering::Relaxed),
            cache_miss_rate: self.cache_miss_rate.load(Ordering::Relaxed),
            error_count: self.error_count.load(Ordering::Relaxed),
            network_latency: self.network_latency.load(Ordering::Relaxed),
            consensus_latency: self.consensus_latency.load(Ordering::Relaxed),
            average_processing_time: self.average_processing_time.load(Ordering::Relaxed),
            peak_processing_time: self.peak_processing_time.load(Ordering::Relaxed),
        }
    }

    /// Get detailed metrics
    pub async fn get_detailed_metrics(&self) -> DetailedMetrics {
        let summary = self.get_summary();
        let custom_metrics = self.custom_metrics.read().await;
        
        DetailedMetrics {
            summary,
            custom_metrics: custom_metrics.iter()
                .map(|(k, v)| (k.clone(), v.load(Ordering::Relaxed)))
                .collect(),
        }
    }

    /// Reset all metrics
    pub fn reset(&mut self) {
        self.transactions_processed.store(0, Ordering::Relaxed);
        self.transactions_per_second.store(0, Ordering::Relaxed);
        self.peak_tps.store(0, Ordering::Relaxed);
        self.average_tps.store(0, Ordering::Relaxed);
        
        self.blocks_processed.store(0, Ordering::Relaxed);
        self.blocks_per_second.store(0, Ordering::Relaxed);
        self.peak_bps.store(0, Ordering::Relaxed);
        self.average_bps.store(0, Ordering::Relaxed);
        
        self.messages_sent.store(0, Ordering::Relaxed);
        self.messages_received.store(0, Ordering::Relaxed);
        self.bytes_sent.store(0, Ordering::Relaxed);
        self.bytes_received.store(0, Ordering::Relaxed);
        self.network_latency.store(0, Ordering::Relaxed);
        
        self.memory_usage.store(0, Ordering::Relaxed);
        self.peak_memory_usage.store(0, Ordering::Relaxed);
        self.cache_hit_rate.store(0, Ordering::Relaxed);
        self.cache_miss_rate.store(0, Ordering::Relaxed);
        
        self.consensus_rounds.store(0, Ordering::Relaxed);
        self.consensus_latency.store(0, Ordering::Relaxed);
        self.validator_count.store(0, Ordering::Relaxed);
        self.active_validators.store(0, Ordering::Relaxed);
        
        self.error_count.store(0, Ordering::Relaxed);
        self.validation_errors.store(0, Ordering::Relaxed);
        self.network_errors.store(0, Ordering::Relaxed);
        self.consensus_errors.store(0, Ordering::Relaxed);
        
        self.total_processing_time.store(0, Ordering::Relaxed);
        self.average_processing_time.store(0, Ordering::Relaxed);
        self.peak_processing_time.store(0, Ordering::Relaxed);
        
        self.start_time = Instant::now();
    }
}

/// Error types for metrics
#[derive(Debug, Clone)]
pub enum ErrorType {
    Validation,
    Network,
    Consensus,
}

/// Performance summary
#[derive(Debug, Clone)]
pub struct PerformanceSummary {
    pub uptime: Duration,
    pub transactions_processed: u64,
    pub blocks_processed: u64,
    pub current_tps: u64,
    pub peak_tps: u64,
    pub current_bps: u64,
    pub peak_bps: u64,
    pub memory_usage: usize,
    pub peak_memory_usage: usize,
    pub cache_hit_rate: u64,
    pub cache_miss_rate: u64,
    pub error_count: u64,
    pub network_latency: u64,
    pub consensus_latency: u64,
    pub average_processing_time: u64,
    pub peak_processing_time: u64,
}

/// Detailed metrics including custom metrics
#[derive(Debug, Clone)]
pub struct DetailedMetrics {
    pub summary: PerformanceSummary,
    pub custom_metrics: HashMap<String, u64>,
}

/// Performance monitor for real-time monitoring
pub struct PerformanceMonitor {
    metrics: Arc<PerformanceMetrics>,
    monitoring_interval: Duration,
    is_monitoring: Arc<std::sync::atomic::AtomicBool>,
}

impl PerformanceMonitor {
    /// Create a new performance monitor
    pub fn new(metrics: Arc<PerformanceMetrics>, monitoring_interval: Duration) -> Self {
        Self {
            metrics,
            monitoring_interval,
            is_monitoring: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Start monitoring
    pub fn start_monitoring(&self) {
        self.is_monitoring.store(true, Ordering::Relaxed);
        
        let metrics = self.metrics.clone();
        let interval = self.monitoring_interval;
        let is_monitoring = self.is_monitoring.clone();
        
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            
            while is_monitoring.load(Ordering::Relaxed) {
                interval_timer.tick().await;
                
                let summary = metrics.get_summary();
                log::info!("Performance Summary: TPS: {}, BPS: {}, Memory: {}MB, Errors: {}", 
                    summary.current_tps,
                    summary.current_bps,
                    summary.memory_usage / 1024 / 1024,
                    summary.error_count
                );
            }
        });
    }

    /// Stop monitoring
    pub fn stop_monitoring(&self) {
        self.is_monitoring.store(false, Ordering::Relaxed);
    }

    /// Get current metrics
    pub fn get_metrics(&self) -> PerformanceSummary {
        self.metrics.get_summary()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_metrics() {
        let metrics = PerformanceMetrics::new();
        
        // Record some transactions
        metrics.record_transaction(Duration::from_millis(10));
        metrics.record_transaction(Duration::from_millis(20));
        metrics.record_transaction(Duration::from_millis(15));
        
        // Record some blocks
        metrics.record_block(Duration::from_millis(100));
        metrics.record_block(Duration::from_millis(150));
        
        // Record network activity
        metrics.record_network_send(1024);
        metrics.record_network_receive(2048);
        
        // Record memory usage
        metrics.record_memory_usage(1024 * 1024);
        
        // Record cache activity
        metrics.record_cache_hit();
        metrics.record_cache_miss();
        
        // Record consensus activity
        metrics.record_consensus_round(Duration::from_millis(50));
        
        // Record errors
        metrics.record_error(ErrorType::Validation);
        metrics.record_error(ErrorType::Network);
        
        // Get summary
        let summary = metrics.get_summary();
        assert_eq!(summary.transactions_processed, 3);
        assert_eq!(summary.blocks_processed, 2);
        assert_eq!(summary.memory_usage, 1024 * 1024);
        assert_eq!(summary.cache_hit_rate, 1);
        assert_eq!(summary.cache_miss_rate, 1);
        assert_eq!(summary.error_count, 2);
    }

    #[tokio::test]
    async fn test_performance_monitor() {
        let metrics = Arc::new(PerformanceMetrics::new());
        let monitor = PerformanceMonitor::new(metrics.clone(), Duration::from_millis(100));
        
        // Start monitoring
        monitor.start_monitoring();
        
        // Record some activity
        metrics.record_transaction(Duration::from_millis(10));
        metrics.record_block(Duration::from_millis(100));
        
        // Get metrics
        let current_metrics = monitor.get_metrics();
        assert_eq!(current_metrics.transactions_processed, 1);
        assert_eq!(current_metrics.blocks_processed, 1);
        
        // Stop monitoring
        monitor.stop_monitoring();
    }
}
