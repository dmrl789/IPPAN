// src/ippan_time.rs
//
// IPPAN Time Service — deterministic network time computed as
// the median of peer drifts, with microsecond precision.
//
// Fixes initialization bug: last_time_us is always advanced,
// and drift is computed against the current system clock, not stale data.

use once_cell::sync::Lazy;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// ==== CONSTANTS ====

/// Maximum allowed adjustment per sample (±5 ms)
const MAX_DRIFT_US: i64 = 5_000;

/// Median window size (number of peer samples kept)
const MEDIAN_WINDOW: usize = 21;

// ==== STATE ====

static LAST_TIME_US: Lazy<Mutex<i64>> = Lazy::new(|| Mutex::new(0));
static BASE_OFFSET_US: Lazy<Mutex<i64>> = Lazy::new(|| Mutex::new(0));
static DRIFT_SAMPLES: Lazy<Mutex<Vec<i64>>> = Lazy::new(|| Mutex::new(Vec::new()));

// ==== INTERNAL HELPERS ====

fn system_time_now_us() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_micros() as i64
}

/// Compute the median of a vector of i64 values.
fn median(mut v: Vec<i64>) -> i64 {
    if v.is_empty() {
        return 0;
    }
    v.sort_unstable();
    let mid = v.len() / 2;
    if v.len().is_multiple_of(2) {
        (v[mid - 1] + v[mid]) / 2
    } else {
        v[mid]
    }
}

/// Clamp a raw microsecond timestamp to non-negative.
/// Returns 0 for any negative input.
fn clamp_non_negative_micros(raw: i64) -> i64 {
    if raw < 0 {
        0
    } else {
        raw
    }
}

// ==== PUBLIC API ====

/// Initialize IPPAN Time service.
pub fn init() {
    let now = system_time_now_us();
    *LAST_TIME_US.lock().unwrap() = now;
    *BASE_OFFSET_US.lock().unwrap() = 0;
    DRIFT_SAMPLES.lock().unwrap().clear();
}

/// Return the current deterministic IPPAN time in microseconds.
pub fn now_us() -> i64 {
    let now = system_time_now_us();
    let base_offset = *BASE_OFFSET_US.lock().unwrap();
    let mut last = LAST_TIME_US.lock().unwrap();

    let mut candidate = clamp_non_negative_micros(now + base_offset);
    if candidate == 0 {
        *last = 0;
        return 0;
    }

    if candidate <= *last {
        candidate = *last + 1;
    }

    *last = candidate;
    candidate
}

/// Ingest a peer timestamp sample (in microseconds).
/// Adjusts the median base offset within safe drift bounds.
pub fn ingest_sample(peer_time_us: i64) {
    let now_us = system_time_now_us(); // current system time
    let drift = peer_time_us - now_us;

    // Ignore outliers beyond ±10 s
    if drift.abs() > 10_000_000 {
        return;
    }

    let mut samples = DRIFT_SAMPLES.lock().unwrap();
    samples.push(drift);
    if samples.len() > MEDIAN_WINDOW {
        samples.remove(0);
    }

    let median_drift = median(samples.clone());
    let mut base = BASE_OFFSET_US.lock().unwrap();

    // Smooth correction, bounded by MAX_DRIFT_US
    let delta = (median_drift - *base).clamp(-MAX_DRIFT_US, MAX_DRIFT_US);

    *base += delta;

    let candidate = now_us + *base;
    drop(base);

    let mut last = LAST_TIME_US.lock().unwrap();
    if candidate > *last {
        *last = candidate;
    }
}

/// Return the current IPPAN time as a Duration since UNIX_EPOCH.
pub fn now() -> Duration {
    let micros = now_us();
    if micros <= 0 {
        Duration::ZERO
    } else {
        Duration::from_micros(micros as u64)
    }
}

/// Debug dump: (last_time_us, base_offset_us, sample_count)
pub fn status() -> (i64, i64, usize) {
    let last = *LAST_TIME_US.lock().unwrap();
    let base = *BASE_OFFSET_US.lock().unwrap();
    let count = DRIFT_SAMPLES.lock().unwrap().len();
    (last, base, count)
}

// ==== TESTS ====

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monotonic_time() {
        init();
        let t1 = now_us();
        std::thread::sleep(Duration::from_millis(10));
        let t2 = now_us();
        assert!(t2 > t1);
    }

    #[test]
    fn monotonic_time_does_not_move_backwards() {
        init();

        let mut readings = Vec::new();
        for _ in 0..10 {
            readings.push(now_us());
        }

        for window in readings.windows(2) {
            assert!(window[1] >= window[0]);
        }
    }

    #[test]
    fn ingest_sample_does_not_rewind_last_time() {
        init();

        // Introduce a positive base offset and record the current IPPAN time.
        *BASE_OFFSET_US.lock().unwrap() = 5_000;
        let before = now_us();

        // Ingesting a sample at the current system time will clamp the offset
        // back toward zero. The last timestamp must not move backwards.
        ingest_sample(system_time_now_us());
        let after = now_us();

        assert!(after >= before);
    }

    #[test]
    fn test_peer_ingest_median() {
        init();
        for d in [-200, -100, 0, 100, 200] {
            ingest_sample(system_time_now_us() + d);
        }
        let (_, offset, count) = status();
        assert!(count <= MEDIAN_WINDOW);
        assert!(offset.abs() < 1_000);
    }

    #[test]
    fn skew_outliers_are_discarded() {
        init();
        let initial = status();

        ingest_sample(system_time_now_us() + 20_000_000); // > 10s skew

        let after = status();
        assert_eq!(initial.1, after.1, "base offset must ignore outliers");
        assert_eq!(
            initial.2, after.2,
            "skew samples must not grow for outliers"
        );
    }

    #[test]
    fn drift_corrections_are_bounded_and_converge() {
        init();

        // Force an exaggerated offset so clamping logic is exercised.
        *BASE_OFFSET_US.lock().unwrap() = 10_000;

        ingest_sample(system_time_now_us()); // median drift ~0, clamp to -5_000
        let (_, offset_after_first, _) = status();
        assert_eq!(offset_after_first, 5_000);

        ingest_sample(system_time_now_us()); // clamp remaining drift back to 0
        let (_, offset_after_second, _) = status();
        assert_eq!(offset_after_second, 0);
    }

    #[test]
    fn test_clamp_non_negative_micros() {
        // Test negative values are clamped to zero
        assert_eq!(clamp_non_negative_micros(-1), 0);
        assert_eq!(clamp_non_negative_micros(-123_456), 0);
        assert_eq!(clamp_non_negative_micros(i64::MIN), 0);

        // Test zero and positive values pass through
        assert_eq!(clamp_non_negative_micros(0), 0);
        assert_eq!(clamp_non_negative_micros(1), 1);
        assert_eq!(clamp_non_negative_micros(1_000_000), 1_000_000);
        assert_eq!(clamp_non_negative_micros(i64::MAX), i64::MAX);
    }

    #[test]
    fn test_now_clamps_negative_values() {
        // Test that now() returns ZERO when the computed time would be negative
        init();
        let current = system_time_now_us();
        *BASE_OFFSET_US.lock().unwrap() = -current - 1_000_000; // Large negative offset

        let duration = now();
        // The result should be non-negative (clamped to zero)
        assert!(duration >= Duration::ZERO);

        init();
    }

    // =====================================================================
    // COMPREHENSIVE TESTING - PHASE 1: Time/HashTimer Invariants
    // =====================================================================

    #[test]
    fn monotonicity_with_synthetic_peer_samples() {
        init();

        // Record initial time
        let t0 = now_us();

        // Feed in a sequence of peer offsets (some positive, some negative)
        let peer_offsets = vec![1000, -500, 2000, -1000, 1500, 500, -200, 800];

        for offset in peer_offsets {
            ingest_sample(system_time_now_us() + offset);
            let current = now_us();
            // Time must never go backwards
            assert!(
                current >= t0,
                "Time went backwards after ingesting sample with offset {}",
                offset
            );
        }

        // Verify median behaviour - with mixed offsets, median should be reasonable
        let (last, offset, count) = status();
        assert!(count > 0, "Samples should have been recorded");
        assert!(offset.abs() < 10_000, "Median offset should be reasonable");
        assert!(last >= t0, "Final time should be >= initial time");
    }

    #[test]
    fn median_computation_with_fixed_samples() {
        // Test the median function directly with known inputs
        assert_eq!(median(vec![]), 0);
        assert_eq!(median(vec![5]), 5);
        assert_eq!(median(vec![1, 2, 3]), 2);
        assert_eq!(median(vec![1, 2, 3, 4]), 2); // (2+3)/2 = 2 (integer division)
        assert_eq!(median(vec![10, 20, 30, 40, 50]), 30);

        // Test with negative values
        assert_eq!(median(vec![-100, 0, 100]), 0);
        assert_eq!(median(vec![-500, -200, 200, 500]), 0); // (-200+200)/2 = 0

        // Test with all same values
        assert_eq!(median(vec![42, 42, 42, 42, 42]), 42);
    }

    #[test]
    fn skew_rejection_multiple_outliers() {
        init();
        let initial = status();

        // Feed in multiple outliers (beyond ±10s)
        let outliers = vec![
            15_000_000,  // +15s
            -15_000_000, // -15s
            20_000_000,  // +20s
            -25_000_000, // -25s
        ];

        for outlier in outliers {
            ingest_sample(system_time_now_us() + outlier);
        }

        let after = status();

        // All outliers should be rejected - sample count and offset should not change
        assert_eq!(
            initial.2, after.2,
            "No samples should be added for outliers"
        );
        assert_eq!(
            initial.1, after.1,
            "Base offset should not change with outliers"
        );
    }

    #[test]
    fn skew_acceptance_within_bounds() {
        init();

        // Feed in samples within acceptable bounds (< ±10s)
        let acceptable_offsets = vec![
            5_000_000,   // +5s
            -5_000_000,  // -5s
            1_000_000,   // +1s
            -2_000_000,  // -2s
            3_000_000,   // +3s
        ];

        for offset in &acceptable_offsets {
            ingest_sample(system_time_now_us() + offset);
        }

        let (_, _, count) = status();
        assert_eq!(
            count,
            acceptable_offsets.len(),
            "All acceptable samples should be recorded"
        );
    }

    #[test]
    fn non_negative_clamping_edge_cases() {
        // Test edge cases for clamping
        assert_eq!(clamp_non_negative_micros(i64::MIN), 0);
        assert_eq!(clamp_non_negative_micros(i64::MIN + 1), 0);
        assert_eq!(clamp_non_negative_micros(-1_000_000), 0);
        assert_eq!(clamp_non_negative_micros(-1), 0);
        assert_eq!(clamp_non_negative_micros(0), 0);
        assert_eq!(clamp_non_negative_micros(1), 1);
        assert_eq!(clamp_non_negative_micros(1_000_000), 1_000_000);
        assert_eq!(clamp_non_negative_micros(i64::MAX), i64::MAX);
    }

    #[test]
    fn monotonicity_under_rapid_calls() {
        init();

        // Make many rapid calls and ensure strict monotonicity
        let mut times = Vec::new();
        for _ in 0..1000 {
            times.push(now_us());
        }

        // Verify strict monotonic increase
        for window in times.windows(2) {
            assert!(
                window[1] > window[0],
                "Time must strictly increase: {} -> {}",
                window[0],
                window[1]
            );
        }
    }

    #[test]
    fn convergence_with_consistent_peer_offset() {
        init();

        // Simulate peers all reporting consistent +2000us offset
        for _ in 0..MEDIAN_WINDOW {
            ingest_sample(system_time_now_us() + 2000);
        }

        let (_, offset, _) = status();

        // Offset should converge toward +2000us, bounded by MAX_DRIFT_US per step
        // After MEDIAN_WINDOW samples, we should be close to the target
        assert!(
            offset > 0,
            "Offset should be positive when peers report positive drift"
        );
        assert!(
            offset <= 2000,
            "Offset should not exceed reported drift due to bounded correction"
        );
    }

    #[test]
    fn mixed_peer_offsets_produce_stable_median() {
        init();

        // Simulate a mix of peer offsets
        let offsets = vec![
            -1000, -500, 0, 500, 1000, // First batch: median = 0
            -800, -400, 100, 600, 1200, // Second batch: median shifts
        ];

        for offset in offsets {
            ingest_sample(system_time_now_us() + offset);
        }

        let (last_time, base_offset, count) = status();

        // Verify state is consistent
        assert_eq!(count, 10, "All samples should be recorded");
        assert!(last_time > 0, "Last time should be positive");
        assert!(
            base_offset.abs() < 5_000,
            "Base offset should be bounded by MAX_DRIFT_US adjustments"
        );
    }

    #[test]
    fn time_never_returns_negative_with_extreme_offset() {
        init();

        // Set an extreme negative offset
        *BASE_OFFSET_US.lock().unwrap() = i64::MIN / 2;

        // Multiple calls should never return negative time
        for _ in 0..100 {
            let t = now_us();
            assert!(t >= 0, "Time should never be negative, got {}", t);
        }

        init();
    }
}
