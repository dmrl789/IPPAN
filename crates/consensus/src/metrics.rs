//! Prometheus metrics for consensus and AI operations

use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

/// Scale factor for fixed-point confidence scores (10000 = 100.00%)
const CONFIDENCE_SCALE: i64 = 10000;

/// Consensus metrics collector (fully deterministic, integer-only)
pub struct ConsensusMetrics {
    // AI Selection metrics
    ai_selection_total: Arc<Mutex<u64>>,
    ai_selection_success: Arc<Mutex<u64>>,
    ai_selection_fallback: Arc<Mutex<u64>>,
    ai_selection_latency_us: Arc<Mutex<Vec<u64>>>,
    ai_confidence_scores: Arc<Mutex<Vec<i64>>>, // Scaled 0-10000

    // Validator selection distribution
    validator_selection_count: Arc<Mutex<HashMap<String, u64>>>,

    // Telemetry metrics
    telemetry_updates: Arc<Mutex<u64>>,
    telemetry_load_errors: Arc<Mutex<u64>>,

    // Model metrics
    model_reload_total: Arc<Mutex<u64>>,
    model_reload_success: Arc<Mutex<u64>>,
    model_reload_failures: Arc<Mutex<u64>>,
    model_validation_errors: Arc<Mutex<u64>>,

    // Round metrics
    rounds_finalized: Arc<Mutex<u64>>,
    blocks_proposed: Arc<Mutex<u64>>,
    blocks_validated: Arc<Mutex<u64>>,

    // Reputation metrics (scaled integers)
    avg_reputation_score: Arc<Mutex<i64>>, // Scaled 0-10000
    min_reputation_score: Arc<Mutex<i32>>,
    max_reputation_score: Arc<Mutex<i32>>,
}

impl ConsensusMetrics {
    pub fn new() -> Self {
        Self {
            ai_selection_total: Arc::new(Mutex::new(0)),
            ai_selection_success: Arc::new(Mutex::new(0)),
            ai_selection_fallback: Arc::new(Mutex::new(0)),
            ai_selection_latency_us: Arc::new(Mutex::new(Vec::new())),
            ai_confidence_scores: Arc::new(Mutex::new(Vec::new())),
            validator_selection_count: Arc::new(Mutex::new(HashMap::new())),
            telemetry_updates: Arc::new(Mutex::new(0)),
            telemetry_load_errors: Arc::new(Mutex::new(0)),
            model_reload_total: Arc::new(Mutex::new(0)),
            model_reload_success: Arc::new(Mutex::new(0)),
            model_reload_failures: Arc::new(Mutex::new(0)),
            model_validation_errors: Arc::new(Mutex::new(0)),
            rounds_finalized: Arc::new(Mutex::new(0)),
            blocks_proposed: Arc::new(Mutex::new(0)),
            blocks_validated: Arc::new(Mutex::new(0)),
            avg_reputation_score: Arc::new(Mutex::new(0)),
            min_reputation_score: Arc::new(Mutex::new(10000)),
            max_reputation_score: Arc::new(Mutex::new(0)),
        }
    }

    // AI Selection metrics

    pub fn record_ai_selection_attempt(&self) {
        *self.ai_selection_total.lock() += 1;
    }

    pub fn record_ai_selection_success(&self, confidence_score: i64, latency_us: u64) {
        *self.ai_selection_success.lock() += 1;
        self.ai_confidence_scores.lock().push(confidence_score);
        self.ai_selection_latency_us.lock().push(latency_us);

        // Keep only last 1000 samples
        let mut scores = self.ai_confidence_scores.lock();
        let score_len = scores.len();
        if score_len > 1000 {
            scores.drain(0..score_len - 1000);
        }
        let mut latencies = self.ai_selection_latency_us.lock();
        let latency_len = latencies.len();
        if latency_len > 1000 {
            latencies.drain(0..latency_len - 1000);
        }
    }

    pub fn record_ai_selection_fallback(&self) {
        *self.ai_selection_fallback.lock() += 1;
    }

    pub fn record_validator_selected(&self, validator_id: &[u8; 32]) {
        let key = hex::encode(validator_id);
        let mut counts = self.validator_selection_count.lock();
        *counts.entry(key).or_insert(0) += 1;
    }

    // Telemetry metrics

    pub fn record_telemetry_update(&self) {
        *self.telemetry_updates.lock() += 1;
    }

    pub fn record_telemetry_load_error(&self) {
        *self.telemetry_load_errors.lock() += 1;
    }

    // Model metrics

    pub fn record_model_reload_attempt(&self) {
        *self.model_reload_total.lock() += 1;
    }

    pub fn record_model_reload_success(&self) {
        *self.model_reload_success.lock() += 1;
    }

    pub fn record_model_reload_failure(&self) {
        *self.model_reload_failures.lock() += 1;
    }

    pub fn record_model_validation_error(&self) {
        *self.model_validation_errors.lock() += 1;
    }

    // Round metrics

    pub fn record_round_finalized(&self) {
        *self.rounds_finalized.lock() += 1;
    }

    pub fn record_block_proposed(&self) {
        *self.blocks_proposed.lock() += 1;
    }

    pub fn record_block_validated(&self) {
        *self.blocks_validated.lock() += 1;
    }

    // Reputation metrics

    pub fn record_reputation_scores(&self, scores: &HashMap<[u8; 32], i32>) {
        if scores.is_empty() {
            return;
        }

        let sum: i64 = scores.values().map(|&s| s as i64).sum();
        let avg = sum / scores.len() as i64; // Integer average
        *self.avg_reputation_score.lock() = avg;

        if let Some(&min) = scores.values().min() {
            *self.min_reputation_score.lock() = min;
        }
        if let Some(&max) = scores.values().max() {
            *self.max_reputation_score.lock() = max;
        }
    }

    // Getters for Prometheus export

    pub fn get_ai_selection_total(&self) -> u64 {
        *self.ai_selection_total.lock()
    }

    pub fn get_ai_selection_success(&self) -> u64 {
        *self.ai_selection_success.lock()
    }

    pub fn get_ai_selection_fallback(&self) -> u64 {
        *self.ai_selection_fallback.lock()
    }

    pub fn get_ai_selection_success_rate(&self) -> f64 {
        let total = *self.ai_selection_total.lock();
        if total == 0 {
            return 0.0;
        }
        let success = *self.ai_selection_success.lock();
        // Return as f64 for Prometheus compatibility
        success as f64 / total as f64
    }

    pub fn get_avg_ai_confidence(&self) -> f64 {
        let scores = self.ai_confidence_scores.lock();
        if scores.is_empty() {
            return 0.0;
        }
        // Integer average, then convert to f64 for Prometheus
        let sum: i64 = scores.iter().sum();
        let avg_scaled = sum / scores.len() as i64;
        avg_scaled as f64 / CONFIDENCE_SCALE as f64
    }

    pub fn get_avg_ai_latency_us(&self) -> f64 {
        let latencies = self.ai_selection_latency_us.lock();
        if latencies.is_empty() {
            return 0.0;
        }
        // Integer average, then convert to f64 for Prometheus
        let sum: u64 = latencies.iter().sum();
        (sum / latencies.len() as u64) as f64
    }

    pub fn get_validator_selection_distribution(&self) -> HashMap<String, u64> {
        self.validator_selection_count.lock().clone()
    }

    pub fn get_telemetry_updates(&self) -> u64 {
        *self.telemetry_updates.lock()
    }

    pub fn get_telemetry_load_errors(&self) -> u64 {
        *self.telemetry_load_errors.lock()
    }

    pub fn get_model_reload_total(&self) -> u64 {
        *self.model_reload_total.lock()
    }

    pub fn get_model_reload_success_rate(&self) -> f64 {
        let total = *self.model_reload_total.lock();
        if total == 0 {
            return 0.0;
        }
        let success = *self.model_reload_success.lock();
        // Return as f64 for Prometheus compatibility
        success as f64 / total as f64
    }

    pub fn get_model_validation_errors(&self) -> u64 {
        *self.model_validation_errors.lock()
    }

    pub fn get_rounds_finalized(&self) -> u64 {
        *self.rounds_finalized.lock()
    }

    pub fn get_blocks_proposed(&self) -> u64 {
        *self.blocks_proposed.lock()
    }

    pub fn get_blocks_validated(&self) -> u64 {
        *self.blocks_validated.lock()
    }

    pub fn get_avg_reputation_score(&self) -> f64 {
        // Convert from scaled integer to f64 for Prometheus
        *self.avg_reputation_score.lock() as f64 / CONFIDENCE_SCALE as f64
    }

    pub fn get_min_reputation_score(&self) -> i32 {
        *self.min_reputation_score.lock()
    }

    pub fn get_max_reputation_score(&self) -> i32 {
        *self.max_reputation_score.lock()
    }

    /// Export metrics in Prometheus text format
    pub fn export_prometheus(&self) -> String {
        let mut output = String::new();

        // AI Selection metrics
        output.push_str(
            "# HELP ippan_ai_selection_total Total number of AI validator selections attempted\n",
        );
        output.push_str("# TYPE ippan_ai_selection_total counter\n");
        output.push_str(&format!(
            "ippan_ai_selection_total {}\n",
            self.get_ai_selection_total()
        ));

        output.push_str(
            "# HELP ippan_ai_selection_success Number of successful AI validator selections\n",
        );
        output.push_str("# TYPE ippan_ai_selection_success counter\n");
        output.push_str(&format!(
            "ippan_ai_selection_success {}\n",
            self.get_ai_selection_success()
        ));

        output.push_str(
            "# HELP ippan_ai_selection_fallback Number of fallback selections (AI failed)\n",
        );
        output.push_str("# TYPE ippan_ai_selection_fallback counter\n");
        output.push_str(&format!(
            "ippan_ai_selection_fallback {}\n",
            self.get_ai_selection_fallback()
        ));

        output.push_str(
            "# HELP ippan_ai_selection_success_rate Success rate of AI validator selections\n",
        );
        output.push_str("# TYPE ippan_ai_selection_success_rate gauge\n");
        output.push_str(&format!(
            "ippan_ai_selection_success_rate {:.4}\n",
            self.get_ai_selection_success_rate()
        ));

        output.push_str("# HELP ippan_ai_confidence_avg Average AI confidence score (0-1)\n");
        output.push_str("# TYPE ippan_ai_confidence_avg gauge\n");
        output.push_str(&format!(
            "ippan_ai_confidence_avg {:.4}\n",
            self.get_avg_ai_confidence()
        ));

        output.push_str(
            "# HELP ippan_ai_latency_avg_us Average AI selection latency in microseconds\n",
        );
        output.push_str("# TYPE ippan_ai_latency_avg_us gauge\n");
        output.push_str(&format!(
            "ippan_ai_latency_avg_us {:.2}\n",
            self.get_avg_ai_latency_us()
        ));

        // Validator selection distribution
        output.push_str(
            "# HELP ippan_validator_selected_total Number of times each validator was selected\n",
        );
        output.push_str("# TYPE ippan_validator_selected_total counter\n");
        for (validator, count) in self.get_validator_selection_distribution() {
            output.push_str(&format!(
                "ippan_validator_selected_total{{validator=\"{}\"}} {}\n",
                validator, count
            ));
        }

        // Telemetry metrics
        output.push_str("# HELP ippan_telemetry_updates_total Total number of telemetry updates\n");
        output.push_str("# TYPE ippan_telemetry_updates_total counter\n");
        output.push_str(&format!(
            "ippan_telemetry_updates_total {}\n",
            self.get_telemetry_updates()
        ));

        output
            .push_str("# HELP ippan_telemetry_load_errors_total Number of telemetry load errors\n");
        output.push_str("# TYPE ippan_telemetry_load_errors_total counter\n");
        output.push_str(&format!(
            "ippan_telemetry_load_errors_total {}\n",
            self.get_telemetry_load_errors()
        ));

        // Model reload metrics
        output.push_str("# HELP ippan_model_reload_total Total number of model reload attempts\n");
        output.push_str("# TYPE ippan_model_reload_total counter\n");
        output.push_str(&format!(
            "ippan_model_reload_total {}\n",
            self.get_model_reload_total()
        ));

        output.push_str("# HELP ippan_model_reload_success_rate Success rate of model reloads\n");
        output.push_str("# TYPE ippan_model_reload_success_rate gauge\n");
        output.push_str(&format!(
            "ippan_model_reload_success_rate {:.4}\n",
            self.get_model_reload_success_rate()
        ));

        output.push_str(
            "# HELP ippan_model_validation_errors_total Number of model validation errors\n",
        );
        output.push_str("# TYPE ippan_model_validation_errors_total counter\n");
        output.push_str(&format!(
            "ippan_model_validation_errors_total {}\n",
            self.get_model_validation_errors()
        ));

        // Round metrics
        output.push_str("# HELP ippan_rounds_finalized_total Total number of rounds finalized\n");
        output.push_str("# TYPE ippan_rounds_finalized_total counter\n");
        output.push_str(&format!(
            "ippan_rounds_finalized_total {}\n",
            self.get_rounds_finalized()
        ));

        output.push_str("# HELP ippan_blocks_proposed_total Total number of blocks proposed\n");
        output.push_str("# TYPE ippan_blocks_proposed_total counter\n");
        output.push_str(&format!(
            "ippan_blocks_proposed_total {}\n",
            self.get_blocks_proposed()
        ));

        output.push_str("# HELP ippan_blocks_validated_total Total number of blocks validated\n");
        output.push_str("# TYPE ippan_blocks_validated_total counter\n");
        output.push_str(&format!(
            "ippan_blocks_validated_total {}\n",
            self.get_blocks_validated()
        ));

        // Reputation metrics
        output.push_str(
            "# HELP ippan_reputation_score_avg Average validator reputation score (0-10000)\n",
        );
        output.push_str("# TYPE ippan_reputation_score_avg gauge\n");
        output.push_str(&format!(
            "ippan_reputation_score_avg {:.2}\n",
            self.get_avg_reputation_score()
        ));

        output.push_str("# HELP ippan_reputation_score_min Minimum validator reputation score\n");
        output.push_str("# TYPE ippan_reputation_score_min gauge\n");
        output.push_str(&format!(
            "ippan_reputation_score_min {}\n",
            self.get_min_reputation_score()
        ));

        output.push_str("# HELP ippan_reputation_score_max Maximum validator reputation score\n");
        output.push_str("# TYPE ippan_reputation_score_max gauge\n");
        output.push_str(&format!(
            "ippan_reputation_score_max {}\n",
            self.get_max_reputation_score()
        ));

        output
    }
}

impl Default for ConsensusMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_recording() {
        let metrics = ConsensusMetrics::new();

        // Record some AI selections
        metrics.record_ai_selection_attempt();
        metrics.record_ai_selection_success(8500, 1500); // 85% as scaled integer

        assert_eq!(metrics.get_ai_selection_total(), 1);
        assert_eq!(metrics.get_ai_selection_success(), 1);
        assert_eq!(metrics.get_ai_selection_success_rate(), 1.0);
        assert!(metrics.get_avg_ai_confidence() > 0.8);
        assert!(metrics.get_avg_ai_latency_us() > 1000.0);
    }

    #[test]
    fn test_fallback_metrics() {
        let metrics = ConsensusMetrics::new();

        metrics.record_ai_selection_attempt();
        metrics.record_ai_selection_fallback();

        assert_eq!(metrics.get_ai_selection_total(), 1);
        assert_eq!(metrics.get_ai_selection_fallback(), 1);
        assert_eq!(metrics.get_ai_selection_success_rate(), 0.0);
    }

    #[test]
    fn test_validator_selection_distribution() {
        let metrics = ConsensusMetrics::new();

        let validator1 = [1u8; 32];
        let validator2 = [2u8; 32];

        metrics.record_validator_selected(&validator1);
        metrics.record_validator_selected(&validator1);
        metrics.record_validator_selected(&validator2);

        let dist = metrics.get_validator_selection_distribution();
        assert_eq!(dist.get(&hex::encode(validator1)).copied().unwrap_or(0), 2);
        assert_eq!(dist.get(&hex::encode(validator2)).copied().unwrap_or(0), 1);
    }

    #[test]
    fn test_reputation_metrics() {
        let metrics = ConsensusMetrics::new();

        let mut scores = HashMap::new();
        scores.insert([1u8; 32], 8000);
        scores.insert([2u8; 32], 6000);
        scores.insert([3u8; 32], 9000);

        metrics.record_reputation_scores(&scores);

        assert!(metrics.get_avg_reputation_score() > 7000.0);
        assert!(metrics.get_avg_reputation_score() < 8000.0);
        assert_eq!(metrics.get_min_reputation_score(), 6000);
        assert_eq!(metrics.get_max_reputation_score(), 9000);
    }

    #[test]
    fn test_prometheus_export() {
        let metrics = ConsensusMetrics::new();

        metrics.record_ai_selection_attempt();
        metrics.record_ai_selection_success(9000, 1200); // 90% as scaled integer
        metrics.record_block_proposed();
        metrics.record_round_finalized();

        let output = metrics.export_prometheus();

        assert!(output.contains("ippan_ai_selection_total 1"));
        assert!(output.contains("ippan_ai_selection_success 1"));
        assert!(output.contains("ippan_blocks_proposed_total 1"));
        assert!(output.contains("ippan_rounds_finalized_total 1"));
    }
}
