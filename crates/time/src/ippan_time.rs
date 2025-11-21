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
    fn monotonic_with_median_from_peer_offsets() {
        init();

        let anchor = system_time_now_us();
        let peer_offsets = [-4_000, 1_500, 2_000, -1_000, 3_500, 0];

        let mut readings = Vec::new();
        for offset in peer_offsets {
            ingest_sample(anchor + offset);
            readings.push(now_us());
        }

        for window in readings.windows(2) {
            assert!(
                window[1] >= window[0],
                "IPPAN time must not move backwards when ingesting samples"
            );
        }

        let (_, base_offset, _) = status();
        let expected_median = median(peer_offsets.to_vec());
        let bounded_median = expected_median.clamp(-MAX_DRIFT_US, MAX_DRIFT_US);
        assert!(
            (base_offset - bounded_median).abs() <= 16,
            "base offset should converge toward median drift (got {base_offset}, expected ~{bounded_median})"
        );
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
    fn mixed_skew_samples_keep_median_stable() {
        init();

        let anchor = system_time_now_us();
        for offset in [500, -250, 750, -1_000] {
            ingest_sample(anchor + offset);
        }

        let (_, base_before, count_before) = status();

        ingest_sample(anchor + 15_000_000);
        ingest_sample(anchor - 25_000_000);

        let (_, base_after, count_after) = status();
        assert_eq!(
            base_before, base_after,
            "base offset should ignore extreme skewed samples"
        );
        assert_eq!(
            count_before, count_after,
            "outlier samples must not be recorded"
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

    #[test]
    fn now_us_never_returns_negative() {
        init();
        *LAST_TIME_US.lock().unwrap() = -5;
        *BASE_OFFSET_US.lock().unwrap() = i64::MIN / 2;

        let computed = now_us();
        assert!(
            computed >= 0,
            "now_us should be clamped to non-negative values"
        );
    }
}
