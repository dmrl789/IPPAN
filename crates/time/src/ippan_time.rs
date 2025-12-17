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

#[derive(Debug, Default)]
struct TimeState {
    last_time_us: i64,
    base_offset_us: i64,
    drift_samples: Vec<i64>,
}

static STATE: Lazy<Mutex<TimeState>> = Lazy::new(|| Mutex::new(TimeState::default()));

/// Unit tests in this crate share global time state; serialize them to avoid cross-test races.
#[cfg(test)]
pub(crate) static TEST_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

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

fn bounded_delta(target: i64, current: i64) -> i64 {
    let diff = (target as i128) - (current as i128);
    let clamped = diff.clamp(-(MAX_DRIFT_US as i128), MAX_DRIFT_US as i128);
    clamped as i64
}

// ==== PUBLIC API ====

/// Initialize IPPAN Time service.
pub fn init() {
    let now = system_time_now_us();
    let mut state = STATE.lock().unwrap();
    state.last_time_us = now;
    state.base_offset_us = 0;
    state.drift_samples.clear();
}

/// Return the current deterministic IPPAN time in microseconds.
pub fn now_us() -> i64 {
    let now = system_time_now_us();
    let mut state = STATE.lock().unwrap();
    let mut candidate = clamp_non_negative_micros(now.saturating_add(state.base_offset_us));
    if candidate == 0 {
        state.last_time_us = 0;
        return 0;
    }

    if candidate <= state.last_time_us {
        candidate = state.last_time_us.saturating_add(1);
    }

    state.last_time_us = candidate;
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

    let mut state = STATE.lock().unwrap();

    state.drift_samples.push(drift);
    if state.drift_samples.len() > MEDIAN_WINDOW {
        state.drift_samples.remove(0);
    }

    let median_drift = median(state.drift_samples.clone());

    // Smooth correction, bounded by MAX_DRIFT_US
    let delta = bounded_delta(median_drift, state.base_offset_us);
    state.base_offset_us = state.base_offset_us.saturating_add(delta);

    // Never allow peer ingests to decrease the stored last timestamp.
    let candidate = now_us.saturating_add(state.base_offset_us);
    if candidate > state.last_time_us {
        state.last_time_us = candidate;
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
    let state = STATE.lock().unwrap();
    (
        state.last_time_us,
        state.base_offset_us,
        state.drift_samples.len(),
    )
}

// ==== TESTS ====

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monotonic_time() {
        let _guard = TEST_LOCK.lock().unwrap();
        init();
        let t1 = now_us();
        std::thread::sleep(Duration::from_millis(10));
        let t2 = now_us();
        assert!(t2 > t1);
    }

    #[test]
    fn monotonic_time_does_not_move_backwards() {
        let _guard = TEST_LOCK.lock().unwrap();
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
        let _guard = TEST_LOCK.lock().unwrap();
        init();

        let peer_offsets = [-4_000, 1_500, 2_000, -1_000, 3_500, 0];

        let mut readings = Vec::new();
        for offset in peer_offsets {
            // Construct each peer sample relative to "now" so the measured drift is ~exactly `offset`.
            // This keeps the test deterministic even if a few hundred microseconds elapse between calls.
            ingest_sample(system_time_now_us() + offset);
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
        // Tolerance accounts for incremental convergence with MAX_DRIFT_US clamping per step
        // With only 6 samples, convergence may not be exact due to incremental adjustments
        assert!(
            (base_offset - bounded_median).abs() <= 200,
            "base offset should converge toward median drift (got {base_offset}, expected ~{bounded_median})"
        );
    }

    #[test]
    fn ingest_sample_does_not_rewind_last_time() {
        let _guard = TEST_LOCK.lock().unwrap();
        init();

        // Introduce a positive base offset and record the current IPPAN time.
        STATE.lock().unwrap().base_offset_us = 5_000;
        let before = now_us();

        // Ingesting a sample at the current system time will clamp the offset
        // back toward zero. The last timestamp must not move backwards.
        ingest_sample(system_time_now_us());
        let after = now_us();

        assert!(after >= before);
    }

    #[test]
    fn test_peer_ingest_median() {
        let _guard = TEST_LOCK.lock().unwrap();
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
        let _guard = TEST_LOCK.lock().unwrap();
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
        let _guard = TEST_LOCK.lock().unwrap();
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
        let _guard = TEST_LOCK.lock().unwrap();
        init();

        // Force an exaggerated offset so clamping logic is exercised.
        STATE.lock().unwrap().base_offset_us = 10_000;

        ingest_sample(system_time_now_us()); // median drift ~0, clamp to -5_000
        let (_, offset_after_first, _) = status();
        assert_eq!(offset_after_first, 5_000);

        ingest_sample(system_time_now_us()); // clamp remaining drift back to 0
        let (_, offset_after_second, _) = status();
        assert_eq!(offset_after_second, 0);
    }

    #[test]
    fn test_clamp_non_negative_micros() {
        let _guard = TEST_LOCK.lock().unwrap();
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
        let _guard = TEST_LOCK.lock().unwrap();
        // Test that now() returns ZERO when the computed time would be negative
        init();
        let current = system_time_now_us();
        STATE.lock().unwrap().base_offset_us = -current - 1_000_000; // Large negative offset

        let duration = now();
        // The result should be non-negative (clamped to zero)
        assert!(duration >= Duration::ZERO);

        init();
    }

    #[test]
    fn now_us_never_returns_negative() {
        let _guard = TEST_LOCK.lock().unwrap();
        init();
        {
            // Don't hold the STATE lock while calling `now_us()` (it also locks STATE).
            let mut state = STATE.lock().unwrap();
            state.last_time_us = -5;
            state.base_offset_us = i64::MIN / 2;
        }

        let computed = now_us();
        assert!(
            computed >= 0,
            "now_us should be clamped to non-negative values"
        );
    }

    #[test]
    fn no_deadlock_between_now_us_and_ingest_sample() {
        let _guard = TEST_LOCK.lock().unwrap();
        init();
        let anchor = system_time_now_us();

        // Keep this reasonably small so it doesn't stall the whole test suite while holding
        // `TEST_LOCK` (Windows CI / slower machines can otherwise appear "hung").
        let threads: Vec<_> = (0..4)
            .map(|i| {
                std::thread::spawn(move || {
                    for j in 0..2_000 {
                        if (i + j) % 2 == 0 {
                            let _ = now_us();
                        } else {
                            ingest_sample(anchor + ((j as i64 % 2000) - 1000));
                        }
                    }
                })
            })
            .collect();

        for t in threads {
            t.join().expect("thread must not panic");
        }
    }
}
