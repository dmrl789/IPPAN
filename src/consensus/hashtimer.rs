//! HashTimer implementation
//! 
//! Provides precise timestamping with 0.1 microsecond precision

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// HashTimer provides precise timing for blocks and transactions
/// with IPPAN Time integration (0.1 microsecond precision)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashTimer {
    /// Unix timestamp in nanoseconds for maximum precision
    pub timestamp_ns: u64,
    /// IPPAN Time median value from network nodes
    pub ippan_time_ns: u64,
    /// Hash of the content being timed
    pub content_hash: [u8; 32],
    /// Node ID that created this timer
    pub node_id: [u8; 32],
}

impl HashTimer {
    /// Create a new HashTimer with current system time
    pub fn new(content_hash: [u8; 32], node_id: [u8; 32]) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        
        Self {
            timestamp_ns: now,
            ippan_time_ns: now, // Will be updated with actual median time
            content_hash,
            node_id,
        }
    }

    /// Create a HashTimer with specific IPPAN Time
    pub fn with_ippan_time(
        content_hash: [u8; 32], 
        node_id: [u8; 32], 
        ippan_time_ns: u64
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        
        Self {
            timestamp_ns: now,
            ippan_time_ns,
            content_hash,
            node_id,
        }
    }

    /// Get timestamp in seconds
    pub fn timestamp_seconds(&self) -> u64 {
        self.timestamp_ns / 1_000_000_000
    }

    /// Get IPPAN Time in seconds
    pub fn ippan_time_seconds(&self) -> u64 {
        self.ippan_time_ns / 1_000_000_000
    }

    /// Get microsecond precision timestamp
    pub fn timestamp_micros(&self) -> u64 {
        self.timestamp_ns / 1_000
    }

    /// Get IPPAN Time in microseconds
    pub fn ippan_time_micros(&self) -> u64 {
        self.ippan_time_ns / 1_000
    }

    /// Validate that the timer is within acceptable time bounds
    pub fn is_valid(&self, max_drift_seconds: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        
        let drift_ns = if self.timestamp_ns > now {
            self.timestamp_ns - now
        } else {
            now - self.timestamp_ns
        };
        
        let max_drift_ns = max_drift_seconds * 1_000_000_000;
        drift_ns <= max_drift_ns
    }

    /// Validate that IPPAN Time is within acceptable bounds of local time
    pub fn is_ippan_time_valid(&self, max_drift_seconds: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        
        let drift_ns = if self.ippan_time_ns > now {
            self.ippan_time_ns - now
        } else {
            now - self.ippan_time_ns
        };
        
        let max_drift_ns = max_drift_seconds * 1_000_000_000;
        drift_ns <= max_drift_ns
    }

    /// Check if this timer is for the given content
    pub fn matches_content(&self, content_hash: &[u8; 32]) -> bool {
        &self.content_hash == content_hash
    }

    /// Get the time difference between this timer and another in nanoseconds
    pub fn time_diff_ns(&self, other: &HashTimer) -> i64 {
        self.timestamp_ns as i64 - other.timestamp_ns as i64
    }

    /// Get the IPPAN Time difference between this timer and another in nanoseconds
    pub fn ippan_time_diff_ns(&self, other: &HashTimer) -> i64 {
        self.ippan_time_ns as i64 - other.ippan_time_ns as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hashtimer_creation() {
        let hash = [1u8; 32];
        let time = HashTimer::current_time_nanos();
        let node_id = [2u8; 32];
        
        let hashtimer = HashTimer::for_block(hash, time, node_id);
        
        assert_eq!(hashtimer.hash, hash);
        assert_eq!(hashtimer.ippan_time, time);
        assert_eq!(hashtimer.node_id, node_id);
    }

    #[test]
    fn test_precision() {
        let hashtimer = HashTimer {
            hash: [0u8; 32],
            ippan_time: 1234567890123456789, // Some time in nanoseconds
            node_id: [0u8; 32],
            signature: [0u8; 64],
        };
        
        let precision = hashtimer.precision_microseconds();
        assert!(precision >= 0.0 && precision < 1000.0);
    }
}

