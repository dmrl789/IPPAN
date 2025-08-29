use prometheus_client::{
    encoding::text::encode,
    metrics::{counter::Counter, gauge::Gauge, histogram::Histogram},
    registry::Registry,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Clone)]
pub struct Metrics {
    // Transaction metrics
    pub ingress_tx_total: Counter,
    pub verified_tx_total: Counter,
    pub rejected_tx_total: Counter,
    pub finalized_tx_total: Counter,
    
    // Mempool metrics
    pub mempool_size: Gauge,
    pub mempool_size_per_shard: Vec<Gauge>,
    
    // Block metrics
    pub blocks_created_total: Counter,
    pub blocks_finalized_total: Counter,
    pub block_size_bytes: Histogram,
    pub block_creation_duration_ms: Histogram,
    
    // Round metrics
    pub rounds_started_total: Counter,
    pub rounds_finalized_total: Counter,
    pub round_duration_ms: Histogram,
    pub round_transaction_count: Histogram,
    
    // Network metrics
    pub peers_connected: Gauge,
    pub messages_received_total: Counter,
    pub messages_sent_total: Counter,
    
    // State metrics
    pub accounts_total: Gauge,
    pub total_balance: Gauge,
    pub state_root_updates_total: Counter,
    
    // Performance metrics
    pub transaction_processing_duration_ms: Histogram,
    pub block_processing_duration_ms: Histogram,
    pub round_processing_duration_ms: Histogram,
    
    // Error metrics
    pub errors_total: Counter,
    pub validation_errors_total: Counter,
    pub network_errors_total: Counter,
    
    // Registry for Prometheus
    pub registry: Registry,
}

impl Metrics {
    pub fn new(shard_count: usize) -> Self {
        let mut registry = Registry::default();
        
        // Transaction metrics
        let ingress_tx_total = Counter::default();
        let verified_tx_total = Counter::default();
        let rejected_tx_total = Counter::default();
        let finalized_tx_total = Counter::default();
        
        registry.register("ippan_ingress_transactions_total", "Total transactions received", ingress_tx_total.clone());
        registry.register("ippan_verified_transactions_total", "Total transactions verified", verified_tx_total.clone());
        registry.register("ippan_rejected_transactions_total", "Total transactions rejected", rejected_tx_total.clone());
        registry.register("ippan_finalized_transactions_total", "Total transactions finalized", finalized_tx_total.clone());
        
        // Mempool metrics
        let mempool_size = Gauge::default();
        let mut mempool_size_per_shard = Vec::new();
        
        registry.register("ippan_mempool_size", "Current mempool size", mempool_size.clone());
        
        for i in 0..shard_count {
            let shard_gauge = Gauge::default();
            registry.register(
                &format!("ippan_mempool_size_shard_{}", i),
                &format!("Current mempool size for shard {}", i),
                shard_gauge.clone(),
            );
            mempool_size_per_shard.push(shard_gauge);
        }
        
        // Block metrics
        let blocks_created_total = Counter::default();
        let blocks_finalized_total = Counter::default();
        let block_size_bytes = Histogram::new(vec![
            1024.0, 4096.0, 8192.0, 16384.0, 32768.0, 65536.0
        ]);
        let block_creation_duration_ms = Histogram::new(vec![
            1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0
        ]);
        
        registry.register("ippan_blocks_created_total", "Total blocks created", blocks_created_total.clone());
        registry.register("ippan_blocks_finalized_total", "Total blocks finalized", blocks_finalized_total.clone());
        registry.register("ippan_block_size_bytes", "Block size distribution", block_size_bytes.clone());
        registry.register("ippan_block_creation_duration_ms", "Block creation duration", block_creation_duration_ms.clone());
        
        // Round metrics
        let rounds_started_total = Counter::default();
        let rounds_finalized_total = Counter::default();
        let round_duration_ms = Histogram::new(vec![
            50.0, 100.0, 200.0, 300.0, 400.0, 500.0, 750.0, 1000.0
        ]);
        let round_transaction_count = Histogram::new(vec![
            1.0, 10.0, 50.0, 100.0, 500.0, 1000.0, 5000.0, 10000.0
        ]);
        
        registry.register("ippan_rounds_started_total", "Total rounds started", rounds_started_total.clone());
        registry.register("ippan_rounds_finalized_total", "Total rounds finalized", rounds_finalized_total.clone());
        registry.register("ippan_round_duration_ms", "Round duration distribution", round_duration_ms.clone());
        registry.register("ippan_round_transaction_count", "Transactions per round", round_transaction_count.clone());
        
        // Network metrics
        let peers_connected = Gauge::default();
        let messages_received_total = Counter::default();
        let messages_sent_total = Counter::default();
        
        registry.register("ippan_peers_connected", "Number of connected peers", peers_connected.clone());
        registry.register("ippan_messages_received_total", "Total messages received", messages_received_total.clone());
        registry.register("ippan_messages_sent_total", "Total messages sent", messages_sent_total.clone());
        
        // State metrics
        let accounts_total = Gauge::default();
        let total_balance = Gauge::default();
        let state_root_updates_total = Counter::default();
        
        registry.register("ippan_accounts_total", "Total number of accounts", accounts_total.clone());
        registry.register("ippan_total_balance", "Total balance across all accounts", total_balance.clone());
        registry.register("ippan_state_root_updates_total", "Total state root updates", state_root_updates_total.clone());
        
        // Performance metrics
        let transaction_processing_duration_ms = Histogram::new(vec![
            0.1, 0.5, 1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0
        ]);
        let block_processing_duration_ms = Histogram::new(vec![
            1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0
        ]);
        let round_processing_duration_ms = Histogram::new(vec![
            10.0, 25.0, 50.0, 100.0, 200.0, 300.0, 400.0, 500.0, 750.0, 1000.0
        ]);
        
        registry.register("ippan_transaction_processing_duration_ms", "Transaction processing duration", transaction_processing_duration_ms.clone());
        registry.register("ippan_block_processing_duration_ms", "Block processing duration", block_processing_duration_ms.clone());
        registry.register("ippan_round_processing_duration_ms", "Round processing duration", round_processing_duration_ms.clone());
        
        // Error metrics
        let errors_total = Counter::default();
        let validation_errors_total = Counter::default();
        let network_errors_total = Counter::default();
        
        registry.register("ippan_errors_total", "Total errors", errors_total.clone());
        registry.register("ippan_validation_errors_total", "Total validation errors", validation_errors_total.clone());
        registry.register("ippan_network_errors_total", "Total network errors", network_errors_total.clone());
        
        Self {
            ingress_tx_total,
            verified_tx_total,
            rejected_tx_total,
            finalized_tx_total,
            mempool_size,
            mempool_size_per_shard,
            blocks_created_total,
            blocks_finalized_total,
            block_size_bytes,
            block_creation_duration_ms,
            rounds_started_total,
            rounds_finalized_total,
            round_duration_ms,
            round_transaction_count,
            peers_connected,
            messages_received_total,
            messages_sent_total,
            accounts_total,
            total_balance,
            state_root_updates_total,
            transaction_processing_duration_ms,
            block_processing_duration_ms,
            round_processing_duration_ms,
            errors_total,
            validation_errors_total,
            network_errors_total,
            registry,
        }
    }

    pub fn record_transaction_ingress(&self) {
        self.ingress_tx_total.inc();
    }

    pub fn record_transaction_verified(&self) {
        self.verified_tx_total.inc();
    }

    pub fn record_transaction_rejected(&self) {
        self.rejected_tx_total.inc();
    }

    pub fn record_transaction_finalized(&self) {
        self.finalized_tx_total.inc();
    }

    pub fn update_mempool_size(&self, size: usize) {
        self.mempool_size.set(size as f64);
    }

    pub fn update_mempool_shard_size(&self, shard_index: usize, size: usize) {
        if shard_index < self.mempool_size_per_shard.len() {
            self.mempool_size_per_shard[shard_index].set(size as f64);
        }
    }

    pub fn record_block_created(&self, size_bytes: usize, duration_ms: f64) {
        self.blocks_created_total.inc();
        self.block_size_bytes.observe(size_bytes as f64);
        self.block_creation_duration_ms.observe(duration_ms);
    }

    pub fn record_block_finalized(&self) {
        self.blocks_finalized_total.inc();
    }

    pub fn record_round_started(&self) {
        self.rounds_started_total.inc();
    }

    pub fn record_round_finalized(&self, duration_ms: f64, transaction_count: usize) {
        self.rounds_finalized_total.inc();
        self.round_duration_ms.observe(duration_ms);
        self.round_transaction_count.observe(transaction_count as f64);
    }

    pub fn update_peers_connected(&self, count: usize) {
        self.peers_connected.set(count as f64);
    }

    pub fn record_message_received(&self) {
        self.messages_received_total.inc();
    }

    pub fn record_message_sent(&self) {
        self.messages_sent_total.inc();
    }

    pub fn update_accounts_total(&self, count: usize) {
        self.accounts_total.set(count as f64);
    }

    pub fn update_total_balance(&self, balance: u64) {
        self.total_balance.set(balance as f64);
    }

    pub fn record_state_root_update(&self) {
        self.state_root_updates_total.inc();
    }

    pub fn record_transaction_processing_duration(&self, duration_ms: f64) {
        self.transaction_processing_duration_ms.observe(duration_ms);
    }

    pub fn record_block_processing_duration(&self, duration_ms: f64) {
        self.block_processing_duration_ms.observe(duration_ms);
    }

    pub fn record_round_processing_duration(&self, duration_ms: f64) {
        self.round_processing_duration_ms.observe(duration_ms);
    }

    pub fn record_error(&self) {
        self.errors_total.inc();
    }

    pub fn record_validation_error(&self) {
        self.validation_errors_total.inc();
    }

    pub fn record_network_error(&self) {
        self.network_errors_total.inc();
    }

    pub fn get_prometheus_metrics(&self) -> String {
        let mut buffer = Vec::new();
        encode(&mut buffer, &self.registry).unwrap();
        String::from_utf8(buffer).unwrap()
    }
}

pub struct MetricsCollector {
    metrics: Arc<Metrics>,
    last_report_time: Arc<RwLock<std::time::Instant>>,
    report_interval: std::time::Duration,
}

impl MetricsCollector {
    pub fn new(metrics: Arc<Metrics>, report_interval_secs: u64) -> Self {
        Self {
            metrics,
            last_report_time: Arc::new(RwLock::new(std::time::Instant::now())),
            report_interval: std::time::Duration::from_secs(report_interval_secs),
        }
    }

    pub async fn should_report(&self) -> bool {
        let last_report = *self.last_report_time.read().await;
        last_report.elapsed() >= self.report_interval
    }

    pub async fn mark_reported(&self) {
        *self.last_report_time.write().await = std::time::Instant::now();
    }

    pub async fn collect_and_report(&self) {
        if self.should_report().await {
            info!("Metrics report: {}", self.get_summary().await);
            self.mark_reported().await;
        }
    }

    pub async fn get_summary(&self) -> String {
        // This would collect current metrics and return a summary
        // For now, return a simple placeholder
        "Metrics summary placeholder".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let metrics = Metrics::new(4);
        assert_eq!(metrics.mempool_size_per_shard.len(), 4);
    }

    #[test]
    fn test_metrics_recording() {
        let metrics = Metrics::new(2);
        
        metrics.record_transaction_ingress();
        metrics.record_transaction_verified();
        metrics.record_transaction_finalized();
        
        metrics.update_mempool_size(100);
        metrics.update_mempool_shard_size(0, 50);
        metrics.update_mempool_shard_size(1, 50);
        
        metrics.record_block_created(16384, 25.0);
        metrics.record_block_finalized();
        
        metrics.record_round_started();
        metrics.record_round_finalized(200.0, 1000);
        
        // Verify metrics can be encoded
        let prometheus_output = metrics.get_prometheus_metrics();
        assert!(!prometheus_output.is_empty());
        assert!(prometheus_output.contains("ippan_ingress_transactions_total"));
    }

    #[tokio::test]
    async fn test_metrics_collector() {
        let metrics = Arc::new(Metrics::new(2));
        let collector = MetricsCollector::new(metrics, 1);
        
        assert!(!collector.should_report().await);
        
        // Wait a bit and check again
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        assert!(!collector.should_report().await);
    }
}
