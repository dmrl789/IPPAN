//! HashTimer implementation
//! 
//! Provides precise timestamping with 0.1 microsecond precision

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

/// HashTimer structure for precise timestamping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashTimer {
    pub timestamp_ns: u64,        // Nanosecond precision timestamp
    pub hash: String,             // Hash of the timestamp
    pub node_id: String,          // Node that created this HashTimer
    pub round: u64,               // Consensus round
    pub sequence: u64,            // Sequence number within round
    pub drift_ns: i64,            // Clock drift in nanoseconds
    pub precision_ns: u64,        // Precision of this HashTimer
}

/// IPPAN Time Manager for global time synchronization
pub struct IppanTimeManager {
    node_id: String,
    local_clock: Arc<RwLock<LocalClock>>,
    network_times: Arc<RwLock<HashMap<String, NetworkTime>>>,
    drift_history: Arc<RwLock<Vec<DriftMeasurement>>>,
    precision_target: u64, // Target precision in nanoseconds
}

/// Local clock with drift tracking
#[derive(Debug, Clone)]
pub struct LocalClock {
    pub offset_ns: i64,           // Offset from network time
    pub drift_rate_ns_per_sec: f64, // Drift rate
    pub last_sync: Instant,        // Last synchronization time
    pub precision_ns: u64,         // Current precision
}

/// Network time from other nodes
#[derive(Debug, Clone)]
pub struct NetworkTime {
    pub node_id: String,
    pub timestamp_ns: u64,
    pub received_at: Instant,
    pub precision_ns: u64,
    pub trust_score: f64,
}

/// Drift measurement
#[derive(Debug, Clone)]
pub struct DriftMeasurement {
    pub timestamp: Instant,
    pub drift_ns: i64,
    pub source_node: String,
    pub precision_ns: u64,
}

impl HashTimer {
    /// Create a new HashTimer with current time
    pub fn new(node_id: &str, round: u64, sequence: u64) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        
        let hash = Self::compute_hash(now, node_id, round, sequence);
        
        HashTimer {
            timestamp_ns: now,
            hash,
            node_id: node_id.to_string(),
            round,
            sequence,
            drift_ns: 0,
            precision_ns: 100, // 100ns precision target
        }
    }

    /// Create a new HashTimer with optimized performance (for high-frequency operations)
    pub fn new_optimized(node_id: &str, round: u64, sequence: u64) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        
        // Use optimized hash computation
        let hash = Self::compute_hash_optimized(now, node_id, round, sequence);
        
        HashTimer {
            timestamp_ns: now,
            hash,
            node_id: node_id.to_string(),
            round,
            sequence,
            drift_ns: 0,
            precision_ns: 100,
        }
    }

    /// Create HashTimer with specific timestamp
    pub fn with_timestamp(
        timestamp_ns: u64,
        node_id: &str,
        round: u64,
        sequence: u64,
        drift_ns: i64,
    ) -> Self {
        let hash = Self::compute_hash(timestamp_ns, node_id, round, sequence);
        
        HashTimer {
            timestamp_ns,
            hash,
            node_id: node_id.to_string(),
            round,
            sequence,
            drift_ns,
            precision_ns: 100,
        }
    }

    /// Compute hash for HashTimer
    fn compute_hash(timestamp_ns: u64, node_id: &str, round: u64, sequence: u64) -> String {
        // Use constant-length input to the hash function to reduce timing side channels
        let input = Self::build_constant_length_input(timestamp_ns, node_id, round, sequence);
        let mut hasher = Sha256::new();
        hasher.update(&input);
        format!("{:x}", hasher.finalize())
    }

    /// Compute hash for HashTimer with optimized performance
    fn compute_hash_optimized(timestamp_ns: u64, node_id: &str, round: u64, sequence: u64) -> String {
        // Optimized path still uses constant-length input bytes
        let input = Self::build_constant_length_input(timestamp_ns, node_id, round, sequence);
        let mut hasher = Sha256::new();
        hasher.update(&input);
        format!("{:x}", hasher.finalize())
    }

    /// Build a constant-length byte array for hashing to reduce data-dependent timing
    fn build_constant_length_input(timestamp_ns: u64, node_id: &str, round: u64, sequence: u64) -> [u8; 56] {
        // Layout: [ts(8)] [node_hash(32)] [round(8)] [seq(8)] = 56 bytes
        let mut bytes = [0u8; 56];

        // timestamp_ns (u64, little-endian)
        bytes[0..8].copy_from_slice(&timestamp_ns.to_le_bytes());

        // node_id hashed to 32 bytes to achieve fixed length
        let mut node_hasher = Sha256::new();
        node_hasher.update(node_id.as_bytes());
        let node_hash = node_hasher.finalize();
        bytes[8..40].copy_from_slice(&node_hash);

        // round (u64, little-endian)
        bytes[40..48].copy_from_slice(&round.to_le_bytes());

        // sequence (u64, little-endian)
        bytes[48..56].copy_from_slice(&sequence.to_le_bytes());

        bytes
    }

    /// Check if HashTimer matches content hash
    pub fn matches_content(&self, content_hash: &[u8; 32]) -> bool {
        let content_hash_str = content_hash.iter().map(|b| format!("{:02x}", b)).collect::<String>();
        self.hash == content_hash_str
    }

    /// Check if HashTimer is valid within time drift
    pub fn is_valid(&self, max_drift_secs: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        
        let time_diff = if self.timestamp_ns > now {
            self.timestamp_ns - now
        } else {
            now - self.timestamp_ns
        };

        let max_drift_ns = max_drift_secs * 1_000_000_000;
        time_diff <= max_drift_ns
    }

    /// Get IPPAN time in microseconds
    pub fn ippan_time_micros(&self) -> u64 {
        self.timestamp_ns / 1000
    }

    /// Get IPPAN time in nanoseconds (alias for timestamp_ns)
    pub fn ippan_time_ns(&self) -> u64 {
        self.timestamp_ns
    }

    /// Check if IPPAN time is valid within drift
    pub fn is_ippan_time_valid(&self, max_drift_secs: u64) -> bool {
        self.is_valid(max_drift_secs)
    }

    /// Validate HashTimer
    pub fn validate(&self) -> bool {
        // Check hash
        let expected_hash = Self::compute_hash(self.timestamp_ns, &self.node_id, self.round, self.sequence);
        if self.hash != expected_hash {
            return false;
        }

        // Check timestamp is reasonable (not too far in future/past)
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        
        let time_diff = if self.timestamp_ns > now {
            self.timestamp_ns - now
        } else {
            now - self.timestamp_ns
        };

        // Allow 1 second tolerance
        if time_diff > 1_000_000_000 {
            return false;
        }

        // Check precision is reasonable
        if self.precision_ns > 1_000_000 { // 1ms max precision
            return false;
        }

        true
    }

    /// Get timestamp as Duration since epoch
    pub fn as_duration(&self) -> Duration {
        Duration::from_nanos(self.timestamp_ns)
    }

    /// Get timestamp as SystemTime
    pub fn as_system_time(&self) -> SystemTime {
        UNIX_EPOCH + Duration::from_nanos(self.timestamp_ns)
    }

    /// Get precision in microseconds
    pub fn precision_us(&self) -> u64 {
        self.precision_ns / 1000
    }

    /// Get precision in milliseconds
    pub fn precision_ms(&self) -> u64 {
        self.precision_ns / 1_000_000
    }
}

impl IppanTimeManager {
    /// Create new IPPAN Time Manager
    pub fn new(node_id: &str, precision_target_ns: u64) -> Self {
        IppanTimeManager {
            node_id: node_id.to_string(),
            local_clock: Arc::new(RwLock::new(LocalClock {
                offset_ns: 0,
                drift_rate_ns_per_sec: 0.0,
                last_sync: Instant::now(),
                precision_ns: precision_target_ns,
            })),
            network_times: Arc::new(RwLock::new(HashMap::new())),
            drift_history: Arc::new(RwLock::new(Vec::new())),
            precision_target: precision_target_ns,
        }
    }

    /// Create HashTimer with synchronized time
    pub async fn create_hashtimer(&self, round: u64, sequence: u64) -> HashTimer {
        let synchronized_time = self.get_synchronized_time().await;
        let drift = self.get_current_drift().await;
        
        HashTimer::with_timestamp(
            synchronized_time,
            &self.node_id,
            round,
            sequence,
            drift,
        )
    }

    /// Get synchronized time from network
    pub async fn get_synchronized_time(&self) -> u64 {
        let network_times = self.network_times.read().await;
        
        if network_times.is_empty() {
            // No network times available, use local time
            return SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64;
        }

        // Calculate median time from network
        let mut times: Vec<u64> = network_times.values()
            .map(|nt| nt.timestamp_ns)
            .collect();
        times.sort();

        let median_time = if times.len() % 2 == 0 {
            (times[times.len() / 2 - 1] + times[times.len() / 2]) / 2
        } else {
            times[times.len() / 2]
        };

        // Apply local offset
        let local_clock = self.local_clock.read().await;
        (median_time as i64 + local_clock.offset_ns) as u64
    }

    /// Get current drift measurement
    pub async fn get_current_drift(&self) -> i64 {
        let local_clock = self.local_clock.read().await;
        local_clock.offset_ns
    }

    /// Add network time from another node
    pub async fn add_network_time(&self, node_id: &str, timestamp_ns: u64, precision_ns: u64) {
        let mut network_times = self.network_times.write().await;
        
        network_times.insert(node_id.to_string(), NetworkTime {
            node_id: node_id.to_string(),
            timestamp_ns,
            received_at: Instant::now(),
            precision_ns,
            trust_score: 1.0, // Default trust score
        });

        // Update drift measurement
        self.update_drift_measurement(node_id, timestamp_ns, precision_ns).await;
    }

    /// Update drift measurement
    async fn update_drift_measurement(&self, node_id: &str, network_time_ns: u64, precision_ns: u64) {
        let local_time_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        let drift_ns = local_time_ns as i64 - network_time_ns as i64;

        let measurement = DriftMeasurement {
            timestamp: Instant::now(),
            drift_ns,
            source_node: node_id.to_string(),
            precision_ns,
        };

        let mut drift_history = self.drift_history.write().await;
        drift_history.push(measurement);

        // Keep only last 1000 measurements
        if drift_history.len() > 1000 {
            drift_history.remove(0);
        }

        // Update local clock offset
        self.update_local_clock_offset(drift_ns).await;
    }

    /// Update local clock offset
    async fn update_local_clock_offset(&self, new_drift_ns: i64) {
        let mut local_clock = self.local_clock.write().await;
        
        // Simple moving average for offset
        let alpha = 0.1; // Smoothing factor
        local_clock.offset_ns = (local_clock.offset_ns as f64 * (1.0 - alpha) + new_drift_ns as f64 * alpha) as i64;
        
        // Update precision based on drift stability
        let _drift_history = self.drift_history.read().await;
        if _drift_history.len() > 10 {
            let recent_drifts: Vec<i64> = _drift_history.iter()
                .rev()
                .take(10)
                .map(|d| d.drift_ns)
                .collect();
            
            let variance = Self::calculate_variance(&recent_drifts);
            local_clock.precision_ns = (variance * 1_000_000.0) as u64; // Convert to nanoseconds
        }
        
        local_clock.last_sync = Instant::now();
    }

    /// Calculate variance of drift measurements
    fn calculate_variance(drifts: &[i64]) -> f64 {
        if drifts.is_empty() {
            return 0.0;
        }

        let mean = drifts.iter().sum::<i64>() as f64 / drifts.len() as f64;
        let variance = drifts.iter()
            .map(|&d| {
                let diff = d as f64 - mean;
                diff * diff
            })
            .sum::<f64>() / drifts.len() as f64;

        variance
    }

    /// Detect clock drift
    pub async fn detect_drift(&self) -> DriftAnalysis {
        let drift_history = self.drift_history.read().await;
        
        if drift_history.len() < 10 {
            return DriftAnalysis {
                has_drift: false,
                drift_rate_ns_per_sec: 0.0,
                precision_ns: self.precision_target,
                confidence: 0.0,
            };
        }

        // Calculate drift rate over time
        let recent_measurements: Vec<&DriftMeasurement> = drift_history.iter()
            .rev()
            .take(100)
            .collect();

        if recent_measurements.len() < 10 {
            return DriftAnalysis {
                has_drift: false,
                drift_rate_ns_per_sec: 0.0,
                precision_ns: self.precision_target,
                confidence: 0.0,
            };
        }

        // Calculate linear regression for drift rate
        let (slope, confidence) = Self::linear_regression(recent_measurements.clone());
        
        let has_drift = slope.abs() > 1000.0; // 1μs per second threshold
        
        let avg_precision = recent_measurements.iter()
            .map(|m| m.precision_ns)
            .sum::<u64>() / recent_measurements.len() as u64;

        DriftAnalysis {
            has_drift,
            drift_rate_ns_per_sec: slope,
            precision_ns: avg_precision,
            confidence,
        }
    }

    /// Perform linear regression on drift measurements
    fn linear_regression(measurements: Vec<&DriftMeasurement>) -> (f64, f64) {
        let n = measurements.len() as f64;
        
        let x_values: Vec<f64> = measurements.iter()
            .enumerate()
            .map(|(i, _)| i as f64)
            .collect();
        
        let y_values: Vec<f64> = measurements.iter()
            .map(|m| m.drift_ns as f64)
            .collect();

        let sum_x: f64 = x_values.iter().sum();
        let sum_y: f64 = y_values.iter().sum();
        let sum_xy: f64 = x_values.iter().zip(y_values.iter()).map(|(x, y)| x * y).sum();
        let sum_x2: f64 = x_values.iter().map(|x| x * x).sum();

        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x * sum_x);
        let intercept = (sum_y - slope * sum_x) / n;

        // Calculate R-squared for confidence
        let y_mean = sum_y / n;
        let ss_tot: f64 = y_values.iter().map(|y| (y - y_mean).powi(2)).sum();
        let ss_res: f64 = y_values.iter().zip(x_values.iter()).map(|(y, x)| {
            let y_pred = slope * x + intercept;
            (y - y_pred).powi(2)
        }).sum();
        
        let r_squared = if ss_tot == 0.0 { 0.0 } else { 1.0 - (ss_res / ss_tot) };

        (slope, r_squared)
    }

    /// Validate timestamp precision
    pub fn validate_timestamp_precision(&self, timestamp_ns: u64, expected_precision_ns: u64) -> bool {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        let time_diff = if timestamp_ns > current_time {
            timestamp_ns - current_time
        } else {
            current_time - timestamp_ns
        };

        // Check if timestamp is within expected precision
        time_diff <= expected_precision_ns
    }

    /// Get time synchronization statistics
    pub async fn get_sync_stats(&self) -> SyncStats {
        let network_times = self.network_times.read().await;
        let _drift_history = self.drift_history.read().await;
        let local_clock = self.local_clock.read().await;

        let node_count = network_times.len();
        let avg_precision = if node_count > 0 {
            network_times.values()
                .map(|nt| nt.precision_ns)
                .sum::<u64>() / node_count as u64
        } else {
            0
        };

        let drift_analysis = self.detect_drift().await;

        SyncStats {
            node_count,
            avg_precision_ns: avg_precision,
            current_drift_ns: local_clock.offset_ns,
            drift_rate_ns_per_sec: drift_analysis.drift_rate_ns_per_sec,
            has_drift: drift_analysis.has_drift,
            confidence: drift_analysis.confidence,
            last_sync_age_secs: local_clock.last_sync.elapsed().as_secs(),
        }
    }

    /// Clean up old network times
    pub async fn cleanup_old_times(&self, max_age_secs: u64) {
        let mut network_times = self.network_times.write().await;
        let _now = Instant::now();
        
        network_times.retain(|_, nt| {
            nt.received_at.elapsed().as_secs() < max_age_secs
        });
    }
}

/// Drift analysis result
#[derive(Debug, Clone)]
pub struct DriftAnalysis {
    pub has_drift: bool,
    pub drift_rate_ns_per_sec: f64,
    pub precision_ns: u64,
    pub confidence: f64,
}

/// Synchronization statistics
#[derive(Debug, Clone)]
pub struct SyncStats {
    pub node_count: usize,
    pub avg_precision_ns: u64,
    pub current_drift_ns: i64,
    pub drift_rate_ns_per_sec: f64,
    pub has_drift: bool,
    pub confidence: f64,
    pub last_sync_age_secs: u64,
}

impl Default for HashTimer {
    fn default() -> Self {
        HashTimer::new("default_node", 0, 0)
    }
}

