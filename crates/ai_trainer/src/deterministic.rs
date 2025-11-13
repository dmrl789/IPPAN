//! Deterministic utilities for reproducible training
//!
//! Provides LCG-based RNG, deterministic hashing, and tie-breaking logic
//! to ensure identical models across platforms and runs.

use std::num::Wrapping;

/// Linear Congruential Generator for deterministic pseudo-randomness
/// Uses constants from Numerical Recipes (glibc)
#[derive(Clone, Debug)]
pub struct LcgRng {
    state: Wrapping<i64>,
}

impl LcgRng {
    // LCG constants (compatible with glibc)
    const MULTIPLIER: i64 = 1103515245;
    const INCREMENT: i64 = 12345;
    const MODULUS: i64 = 1 << 31;

    pub fn new(seed: i64) -> Self {
        Self {
            state: Wrapping(seed.abs() % Self::MODULUS),
        }
    }

    /// Generate next random i64 in range [0, MODULUS)
    pub fn next_i64(&mut self) -> i64 {
        self.state = self.state * Wrapping(Self::MULTIPLIER) + Wrapping(Self::INCREMENT);
        (self.state.0 & (Self::MODULUS - 1)).abs()
    }

    /// Generate random value in range [0, max)
    pub fn next_range(&mut self, max: i64) -> i64 {
        if max <= 0 {
            return 0;
        }
        self.next_i64() % max
    }

    /// Generate random f64-like value in [0.0, 1.0) as fixed-point i64
    /// Returns value in range [0, 1_000_000) representing 0.0 to 1.0
    pub fn next_unit_micro(&mut self) -> i64 {
        let r = self.next_i64();
        (r * 1_000_000) / Self::MODULUS
    }
}

/// Deterministic xxhash64-like hash in pure i64 arithmetic
/// Simplified version for row ordering
pub fn xxhash64_i64(data: &[i64], seed: i64) -> i64 {
    const PRIME1: i64 = 0x9E3779B185EBCA87_u64 as i64;
    const PRIME2: i64 = 0xC2B2AE3D27D4EB4F_u64 as i64;
    const PRIME3: i64 = 0x165667B19E3779F9_u64 as i64;
    const PRIME5: i64 = 0x85EBCA77C2B2AE63_u64 as i64;

    let mut h = seed.wrapping_add(PRIME5);

    for &val in data {
        h = h.wrapping_add(val.wrapping_mul(PRIME3));
        h = h.rotate_left(17).wrapping_mul(PRIME2);
    }

    h ^= h >> 33;
    h = h.wrapping_mul(PRIME1);
    h ^= h >> 29;
    h = h.wrapping_mul(PRIME2);
    h ^= h >> 32;

    h
}

/// Deterministic tie-breaker for split selection
/// Returns consistent ordering based on (feature_idx, threshold, node_id)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SplitTieBreaker {
    pub feature_idx: usize,
    pub threshold: i64,
    pub node_id: usize,
}

impl SplitTieBreaker {
    pub fn new(feature_idx: usize, threshold: i64, node_id: usize) -> Self {
        Self {
            feature_idx,
            threshold,
            node_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lcg_determinism() {
        let mut rng1 = LcgRng::new(42);
        let mut rng2 = LcgRng::new(42);

        for _ in 0..100 {
            assert_eq!(rng1.next_i64(), rng2.next_i64());
        }
    }

    #[test]
    fn test_lcg_range() {
        let mut rng = LcgRng::new(42);
        for _ in 0..100 {
            let val = rng.next_range(10);
            assert!(val >= 0 && val < 10);
        }
    }

    #[test]
    fn test_xxhash64_determinism() {
        let data = vec![1, 2, 3, 4, 5];
        let h1 = xxhash64_i64(&data, 42);
        let h2 = xxhash64_i64(&data, 42);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_xxhash64_different_seeds() {
        let data = vec![1, 2, 3, 4, 5];
        let h1 = xxhash64_i64(&data, 42);
        let h2 = xxhash64_i64(&data, 43);
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_tie_breaker_ordering() {
        let t1 = SplitTieBreaker::new(0, 100, 0);
        let t2 = SplitTieBreaker::new(0, 100, 1);
        let t3 = SplitTieBreaker::new(1, 50, 0);

        assert!(t1 < t2);
        assert!(t1 < t3);
    }
}
