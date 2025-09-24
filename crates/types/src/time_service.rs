use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::cmp;
use std::collections::VecDeque;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const DEFAULT_MAX_SAMPLES: usize = 100;
const DEFAULT_SLEW_LIMIT_US: u64 = 1_000_000; // allow up to 1s of slewing per call
const DEFAULT_MAX_PEER_DRIFT_US: i64 = 5 * 60 * 1_000_000; // clamp peer drift to +-5 minutes

/// IPPAN Time service providing monotonic microsecond precision
pub struct IppanTime {
    /// Base time offset (microseconds) derived from peer consensus.
    base_offset_us: i64,
    /// Last emitted monotonic time in microseconds.
    last_time_us: u64,
    /// Peer time drift samples (microseconds).
    peer_samples: VecDeque<i64>,
    /// Maximum number of peer samples to keep.
    max_samples: usize,
    /// Maximum slew applied per `now_us` call.
    max_slew_us: u64,
    /// Maximum absolute peer drift we accept when ingesting samples.
    max_peer_drift_us: i64,
}

impl IppanTime {
    /// Initialize the IPPAN Time service
    pub fn init() {
        let now_us = system_time_us();

        let mut time_service = IPPAN_TIME.write();
        time_service.base_offset_us = 0;
        time_service.last_time_us = now_us;
        time_service.peer_samples.clear();
    }

    /// Get current IPPAN time in microseconds
    pub fn now_us() -> u64 {
        let mut time_service = IPPAN_TIME.write();
        let system_now_us = system_time_us();
        let mut target = apply_offset(system_now_us, time_service.base_offset_us);

        if time_service.last_time_us == 0 {
            time_service.last_time_us = target;
            return target;
        }

        if target <= time_service.last_time_us {
            target = time_service.last_time_us.saturating_add(1);
        } else {
            let diff = target - time_service.last_time_us;
            if diff > time_service.max_slew_us {
                target = time_service
                    .last_time_us
                    .saturating_add(time_service.max_slew_us);
            }
        }

        time_service.last_time_us = target;
        target
    }

    /// Ingest a peer time sample for median calculation
    pub fn ingest_sample(peer_time_us: u64) {
        let mut time_service = IPPAN_TIME.write();
        let local_time_us = system_time_us();
        let drift = peer_time_us as i128 - local_time_us as i128;
        let drift = clamp_drift(drift, time_service.max_peer_drift_us);

        time_service.peer_samples.push_back(drift);

        if time_service.peer_samples.len() > time_service.max_samples {
            time_service.peer_samples.pop_front();
        }

        if time_service.peer_samples.len() >= 3 {
            let median_drift = Self::calculate_median_drift(&time_service.peer_samples);
            time_service.base_offset_us = median_drift;
        }
    }

    /// Calculate median drift from peer samples
    fn calculate_median_drift(samples: &VecDeque<i64>) -> i64 {
        let mut sorted_samples: Vec<i64> = samples.iter().copied().collect();
        sorted_samples.sort_unstable();

        let median_index = sorted_samples.len() / 2;
        sorted_samples[median_index]
    }

    /// Get current time with peer-adjusted offset
    pub fn now_adjusted() -> Duration {
        Duration::from_micros(Self::now_us())
    }
}

fn system_time_us() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as u64
}

fn apply_offset(now_us: u64, offset_us: i64) -> u64 {
    if offset_us >= 0 {
        now_us.saturating_add(offset_us as u64)
    } else {
        let abs = offset_us.checked_abs().unwrap_or(i64::MAX) as u64;
        now_us.saturating_sub(abs)
    }
}

fn clamp_drift(drift: i128, limit: i64) -> i64 {
    let limit = limit as i128;
    cmp::min(cmp::max(drift, -limit), limit) as i64
}

/// Global IPPAN Time instance
static IPPAN_TIME: Lazy<RwLock<IppanTime>> = Lazy::new(|| {
    let now_us = system_time_us();
    RwLock::new(IppanTime {
        base_offset_us: 0,
        last_time_us: now_us,
        peer_samples: VecDeque::new(),
        max_samples: DEFAULT_MAX_SAMPLES,
        max_slew_us: DEFAULT_SLEW_LIMIT_US,
        max_peer_drift_us: DEFAULT_MAX_PEER_DRIFT_US,
    })
});

/// Convenience function to get current IPPAN time
pub fn ippan_time_now() -> u64 {
    IppanTime::now_us()
}

/// Convenience function to initialize IPPAN time
pub fn ippan_time_init() {
    IppanTime::init();
}

/// Convenience function to ingest peer sample
pub fn ippan_time_ingest_sample(peer_time_us: u64) {
    IppanTime::ingest_sample(peer_time_us);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_monotonic_time() {
        ippan_time_init();

        let time1 = ippan_time_now();
        thread::sleep(Duration::from_millis(1));
        let time2 = ippan_time_now();

        // Time should be monotonically increasing
        assert!(time2 > time1);
    }

    #[test]
    fn test_peer_sample_ingestion() {
        ippan_time_init();

        // Use samples close to local time to simulate honest peers
        let base_time = ippan_time_now();
        let peer_times = vec![base_time + 500, base_time + 1_000, base_time + 750];

        for peer_time in peer_times {
            ippan_time_ingest_sample(peer_time);
        }

        // Base offset should track the peer median (positive drift)
        let service = super::IPPAN_TIME.read();
        assert!(service.base_offset_us > 0);
        drop(service);

        let current_time = ippan_time_now();
        assert!(current_time > 0);
    }

    #[test]
    fn test_time_precision() {
        ippan_time_init();

        let time1 = ippan_time_now();
        let time2 = ippan_time_now();

        // Should have microsecond precision with enforced monotonicity
        let diff = time2.saturating_sub(time1);
        assert!(diff <= 1_000);
    }

    #[test]
    fn test_negative_peer_drift() {
        ippan_time_init();

        let base_time = ippan_time_now();
        let earlier_peer = base_time.saturating_sub(1_500);

        ippan_time_ingest_sample(earlier_peer);
        ippan_time_ingest_sample(earlier_peer.saturating_sub(250));
        ippan_time_ingest_sample(earlier_peer.saturating_sub(500));

        let service = super::IPPAN_TIME.read();
        assert!(service.base_offset_us <= -1_000);
    }
}
