//! HashTimer? implementation for deterministic time-based ordering
//!
//! This module provides deterministic ordering of consensus events using
//! cryptographic hashes combined with timestamps.

use blake3::Hasher;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// HashTimer represents a moment in consensus time with deterministic ordering
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HashTimer {
    /// Timestamp when the HashTimer was created
    pub timestamp: DateTime<Utc>,
    /// Cryptographic hash for deterministic ordering
    pub hash: String,
    /// Round number for consensus
    pub round: u64,
}

impl HashTimer {
    /// Create a new HashTimer for the current moment
    pub fn now() -> Self {
        Self::for_round(0)
    }

    /// Create a HashTimer for a specific consensus round
    pub fn for_round(round: u64) -> Self {
        let now = Utc::now();
        let hash = Self::compute_hash(&now, round);

        Self {
            timestamp: now,
            hash,
            round,
        }
    }

    /// Create a HashTimer with specific timestamp and round
    pub fn new(timestamp: DateTime<Utc>, round: u64) -> Self {
        let hash = Self::compute_hash(&timestamp, round);

        Self {
            timestamp,
            hash,
            round,
        }
    }

    /// Compute the deterministic hash for a timestamp and round
    fn compute_hash(timestamp: &DateTime<Utc>, round: u64) -> String {
        let mut hasher = Hasher::new();
        hasher.update(&timestamp.timestamp_nanos_opt().unwrap_or(0).to_le_bytes());
        hasher.update(&round.to_le_bytes());
        hasher.finalize().to_hex().to_string()
    }

    /// Deterministic ordering of HashTimers by hash
    pub fn order(a: &Self, b: &Self) -> Ordering {
        // First order by round
        match a.round.cmp(&b.round) {
            Ordering::Equal => {
                // Then by hash for determinism
                match a.hash.cmp(&b.hash) {
                    Ordering::Equal => {
                        // Finally by timestamp as tiebreaker
                        a.timestamp.cmp(&b.timestamp)
                    }
                    ord => ord,
                }
            }
            ord => ord,
        }
    }

    /// Check if this HashTimer is from the same round as another
    pub fn same_round(&self, other: &Self) -> bool {
        self.round == other.round
    }

    /// Get the next round's HashTimer
    pub fn next_round(&self) -> Self {
        Self::for_round(self.round + 1)
    }

    /// Calculate elapsed time from another HashTimer
    pub fn elapsed_since(&self, other: &Self) -> chrono::Duration {
        self.timestamp - other.timestamp
    }

    /// Verify the hash matches the timestamp and round
    pub fn verify(&self) -> bool {
        let expected_hash = Self::compute_hash(&self.timestamp, self.round);
        self.hash == expected_hash
    }
}

impl PartialOrd for HashTimer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HashTimer {
    fn cmp(&self, other: &Self) -> Ordering {
        Self::order(self, other)
    }
}

impl std::fmt::Display for HashTimer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "HashTimer(round={}, hash={}, time={})",
            self.round,
            &self.hash[..8],
            self.timestamp.format("%Y-%m-%d %H:%M:%S")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hashtimer_creation() {
        let ht = HashTimer::now();
        assert!(!ht.hash.is_empty());
        assert_eq!(ht.round, 0);
    }

    #[test]
    fn test_hashtimer_ordering() {
        let ht1 = HashTimer::for_round(1);
        let ht2 = HashTimer::for_round(2);

        assert!(ht1 < ht2);
        assert_eq!(HashTimer::order(&ht1, &ht2), Ordering::Less);
    }

    #[test]
    fn test_hashtimer_verification() {
        let ht = HashTimer::now();
        assert!(ht.verify());
    }

    #[test]
    fn test_hashtimer_next_round() {
        let ht1 = HashTimer::for_round(5);
        let ht2 = ht1.next_round();

        assert_eq!(ht2.round, 6);
        assert!(ht1 < ht2);
    }

    #[test]
    fn test_hashtimer_same_round() {
        let ht1 = HashTimer::for_round(3);
        let ht2 = HashTimer::for_round(3);
        let ht3 = HashTimer::for_round(4);

        assert!(ht1.same_round(&ht2));
        assert!(!ht1.same_round(&ht3));
    }

    #[test]
    fn test_hashtimer_display() {
        let ht = HashTimer::for_round(42);
        let display = format!("{ht}");
        assert!(display.contains("round=42"));
    }
}
