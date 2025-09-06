//! Telemetry and monitoring for IPPAN consensus system
//!
//! Provides metrics collection and monitoring for block sizes, proof sizes,
//! and other consensus-related metrics.

use crate::consensus::limits::MAX_BLOCK_SIZE_BYTES;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Block size telemetry data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockSizeMetrics {
    /// Current block size in bytes
    pub current_size: usize,
    /// Maximum block size in bytes
    pub max_size: usize,
    /// Average block size in bytes
    pub avg_size: f64,
    /// Number of blocks created
    pub block_count: u64,
    /// Number of blocks that hit size warnings
    pub warning_count: u64,
    /// Number of blocks that exceeded size limit
    pub violation_count: u64,
}

/// Round proof telemetry data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofSizeMetrics {
    /// Current proof size in bytes
    pub current_size: usize,
    /// Maximum proof size in bytes
    pub max_size: usize,
    /// Average proof size in bytes
    pub avg_size: f64,
    /// Number of proofs generated
    pub proof_count: u64,
    /// Number of proofs that exceeded target size
    pub oversized_count: u64,
}

/// Consensus telemetry collector
#[derive(Debug)]
pub struct ConsensusTelemetry {
    /// Block size metrics
    block_size_metrics: Arc<BlockSizeMetricsCollector>,
    /// Proof size metrics
    proof_size_metrics: Arc<ProofSizeMetricsCollector>,
    /// Round metrics
    round_metrics: Arc<RoundMetricsCollector>,
}

/// Block size metrics collector
#[derive(Debug)]
struct BlockSizeMetricsCollector {
    current_size: AtomicUsize,
    max_size: AtomicUsize,
    total_size: AtomicU64,
    block_count: AtomicU64,
    warning_count: AtomicU64,
    violation_count: AtomicU64,
}

/// Proof size metrics collector
#[derive(Debug)]
struct ProofSizeMetricsCollector {
    current_size: AtomicUsize,
    max_size: AtomicUsize,
    total_size: AtomicU64,
    proof_count: AtomicU64,
    oversized_count: AtomicU64,
}

/// Round metrics collector
#[derive(Debug)]
struct RoundMetricsCollector {
    current_round: AtomicU64,
    total_rounds: AtomicU64,
    avg_round_duration: AtomicU64,
    max_round_duration: AtomicU64,
}

impl ConsensusTelemetry {
    /// Create a new telemetry collector
    pub fn new() -> Self {
        Self {
            block_size_metrics: Arc::new(BlockSizeMetricsCollector::new()),
            proof_size_metrics: Arc::new(ProofSizeMetricsCollector::new()),
            round_metrics: Arc::new(RoundMetricsCollector::new()),
        }
    }

    /// Record block size metrics
    pub fn record_block_size(&self, size: usize) {
        self.block_size_metrics.record_size(size);
        
        // Log warnings for large blocks
        if size > (MAX_BLOCK_SIZE_BYTES * 3 / 4) {
            warn!(
                "Block size warning: {} bytes is within 25% of maximum {} bytes",
                size,
                MAX_BLOCK_SIZE_BYTES
            );
            self.block_size_metrics.increment_warnings();
        }
        
        if size > MAX_BLOCK_SIZE_BYTES {
            warn!(
                "Block size violation: {} bytes exceeds maximum {} bytes",
                size,
                MAX_BLOCK_SIZE_BYTES
            );
            self.block_size_metrics.increment_violations();
        }
    }

    /// Record proof size metrics
    pub fn record_proof_size(&self, size: usize, target_size: usize) {
        self.proof_size_metrics.record_size(size);
        
        if size > target_size {
            warn!(
                "Proof size warning: {} bytes exceeds target {} bytes",
                size,
                target_size
            );
            self.proof_size_metrics.increment_oversized();
        }
    }

    /// Record round metrics
    pub fn record_round(&self, round_number: u64, duration_ms: u64) {
        self.round_metrics.record_round(round_number, duration_ms);
    }

    /// Get block size metrics
    pub fn get_block_size_metrics(&self) -> BlockSizeMetrics {
        self.block_size_metrics.get_metrics()
    }

    /// Get proof size metrics
    pub fn get_proof_size_metrics(&self) -> ProofSizeMetrics {
        self.proof_size_metrics.get_metrics()
    }

    /// Get all metrics as JSON
    pub fn get_all_metrics_json(&self) -> String {
        let metrics = serde_json::json!({
            "block_size": self.get_block_size_metrics(),
            "proof_size": self.get_proof_size_metrics(),
            "round": self.round_metrics.get_metrics(),
            "limits": {
                "max_block_size_bytes": MAX_BLOCK_SIZE_BYTES,
                "typical_block_size_min_bytes": 4 * 1024,
                "typical_block_size_max_bytes": 32 * 1024,
            }
        });
        
        serde_json::to_string_pretty(&metrics).unwrap_or_default()
    }

    /// Log current metrics
    pub fn log_metrics(&self) {
        let block_metrics = self.get_block_size_metrics();
        let proof_metrics = self.get_proof_size_metrics();
        
        info!(
            "Consensus telemetry - Blocks: {} created, {} warnings, {} violations, avg size: {:.1} bytes",
            block_metrics.block_count,
            block_metrics.warning_count,
            block_metrics.violation_count,
            block_metrics.avg_size
        );
        
        info!(
            "Consensus telemetry - Proofs: {} generated, {} oversized, avg size: {:.1} bytes",
            proof_metrics.proof_count,
            proof_metrics.oversized_count,
            proof_metrics.avg_size
        );
    }
}

impl BlockSizeMetricsCollector {
    fn new() -> Self {
        Self {
            current_size: AtomicUsize::new(0),
            max_size: AtomicUsize::new(0),
            total_size: AtomicU64::new(0),
            block_count: AtomicU64::new(0),
            warning_count: AtomicU64::new(0),
            violation_count: AtomicU64::new(0),
        }
    }

    fn record_size(&self, size: usize) {
        self.current_size.store(size, Ordering::Relaxed);
        self.total_size.fetch_add(size as u64, Ordering::Relaxed);
        self.block_count.fetch_add(1, Ordering::Relaxed);
        
        // Update max size
        let mut current_max = self.max_size.load(Ordering::Relaxed);
        while size > current_max {
            match self.max_size.compare_exchange_weak(
                current_max,
                size,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current_max = actual,
            }
        }
    }

    fn increment_warnings(&self) {
        self.warning_count.fetch_add(1, Ordering::Relaxed);
    }

    fn increment_violations(&self) {
        self.violation_count.fetch_add(1, Ordering::Relaxed);
    }

    fn get_metrics(&self) -> BlockSizeMetrics {
        let block_count = self.block_count.load(Ordering::Relaxed);
        let total_size = self.total_size.load(Ordering::Relaxed);
        
        BlockSizeMetrics {
            current_size: self.current_size.load(Ordering::Relaxed),
            max_size: self.max_size.load(Ordering::Relaxed),
            avg_size: if block_count > 0 {
                total_size as f64 / block_count as f64
            } else {
                0.0
            },
            block_count,
            warning_count: self.warning_count.load(Ordering::Relaxed),
            violation_count: self.violation_count.load(Ordering::Relaxed),
        }
    }
}

impl ProofSizeMetricsCollector {
    fn new() -> Self {
        Self {
            current_size: AtomicUsize::new(0),
            max_size: AtomicUsize::new(0),
            total_size: AtomicU64::new(0),
            proof_count: AtomicU64::new(0),
            oversized_count: AtomicU64::new(0),
        }
    }

    fn record_size(&self, size: usize) {
        self.current_size.store(size, Ordering::Relaxed);
        self.total_size.fetch_add(size as u64, Ordering::Relaxed);
        self.proof_count.fetch_add(1, Ordering::Relaxed);
        
        // Update max size
        let mut current_max = self.max_size.load(Ordering::Relaxed);
        while size > current_max {
            match self.max_size.compare_exchange_weak(
                current_max,
                size,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current_max = actual,
            }
        }
    }

    fn increment_oversized(&self) {
        self.oversized_count.fetch_add(1, Ordering::Relaxed);
    }

    fn get_metrics(&self) -> ProofSizeMetrics {
        let proof_count = self.proof_count.load(Ordering::Relaxed);
        let total_size = self.total_size.load(Ordering::Relaxed);
        
        ProofSizeMetrics {
            current_size: self.current_size.load(Ordering::Relaxed),
            max_size: self.max_size.load(Ordering::Relaxed),
            avg_size: if proof_count > 0 {
                total_size as f64 / proof_count as f64
            } else {
                0.0
            },
            proof_count,
            oversized_count: self.oversized_count.load(Ordering::Relaxed),
        }
    }
}

impl RoundMetricsCollector {
    fn new() -> Self {
        Self {
            current_round: AtomicU64::new(0),
            total_rounds: AtomicU64::new(0),
            avg_round_duration: AtomicU64::new(0),
            max_round_duration: AtomicU64::new(0),
        }
    }

    fn record_round(&self, round_number: u64, duration_ms: u64) {
        self.current_round.store(round_number, Ordering::Relaxed);
        self.total_rounds.fetch_add(1, Ordering::Relaxed);
        
        // Update max duration
        let mut current_max = self.max_round_duration.load(Ordering::Relaxed);
        while duration_ms > current_max {
            match self.max_round_duration.compare_exchange_weak(
                current_max,
                duration_ms,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current_max = actual,
            }
        }
    }

    fn get_metrics(&self) -> HashMap<String, u64> {
        let mut metrics = HashMap::new();
        metrics.insert("current_round".to_string(), self.current_round.load(Ordering::Relaxed));
        metrics.insert("total_rounds".to_string(), self.total_rounds.load(Ordering::Relaxed));
        metrics.insert("max_round_duration_ms".to_string(), self.max_round_duration.load(Ordering::Relaxed));
        metrics
    }
}

impl Default for ConsensusTelemetry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_creation() {
        let telemetry = ConsensusTelemetry::new();
        let block_metrics = telemetry.get_block_size_metrics();
        assert_eq!(block_metrics.block_count, 0);
        assert_eq!(block_metrics.avg_size, 0.0);
    }

    #[test]
    fn test_block_size_recording() {
        let telemetry = ConsensusTelemetry::new();
        
        // Record some block sizes
        telemetry.record_block_size(1000);
        telemetry.record_block_size(2000);
        telemetry.record_block_size(3000);
        
        let metrics = telemetry.get_block_size_metrics();
        assert_eq!(metrics.block_count, 3);
        assert_eq!(metrics.current_size, 3000);
        assert_eq!(metrics.max_size, 3000);
        assert_eq!(metrics.avg_size, 2000.0);
    }

    #[test]
    fn test_proof_size_recording() {
        let telemetry = ConsensusTelemetry::new();
        
        // Record some proof sizes
        telemetry.record_proof_size(50000, 75000);
        telemetry.record_proof_size(80000, 75000); // Oversized
        telemetry.record_proof_size(60000, 75000);
        
        let metrics = telemetry.get_proof_size_metrics();
        assert_eq!(metrics.proof_count, 3);
        assert_eq!(metrics.current_size, 60000);
        assert_eq!(metrics.max_size, 80000);
        assert_eq!(metrics.oversized_count, 1);
    }

    #[test]
    fn test_metrics_json() {
        let telemetry = ConsensusTelemetry::new();
        telemetry.record_block_size(1000);
        telemetry.record_proof_size(50000, 75000);
        
        let json = telemetry.get_all_metrics_json();
        assert!(json.contains("block_size"));
        assert!(json.contains("proof_size"));
        assert!(json.contains("limits"));
    }
}
