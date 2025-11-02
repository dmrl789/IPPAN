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
    if v.len() % 2 == 0 {
        (v[mid - 1] + v[mid]) / 2
    } else {
        v[mid]
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
    *LAST_TIME_US.lock().unwrap() = now;
    now + *BASE_OFFSET_US.lock().unwrap()
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
    *LAST_TIME_US.lock().unwrap() = now_us;
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

#[cfg(all(test, feature = "enable-tests"))]
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
    fn test_now_clamps_negative_values() {
        init();
        let current = system_time_now_us();
        *BASE_OFFSET_US.lock().unwrap() = -current - 1;

        let duration = now();
        assert_eq!(duration, Duration::ZERO);

        init();
    }
}
