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
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();

        *IPPAN_TIME.write() = IppanTime {
            base_offset: Duration::ZERO,
            last_time: now,
            peer_samples: VecDeque::new(),
            max_samples: 100,
        };
    }

    /// Get current IPPAN time in microseconds
    pub fn now_us() -> u64 {
        Self::now_adjusted().as_micros() as u64
    }

    /// Ingest a peer time sample for median calculation
    pub fn ingest_sample(peer_time_us: u64) {
        let mut time_service = IPPAN_TIME.write();
        let peer_time = Duration::from_micros(peer_time_us);

        // Add sample
        time_service.peer_samples.push_back(peer_time);

        // Keep only recent samples
        if time_service.peer_samples.len() > time_service.max_samples {
            time_service.peer_samples.pop_front();
        }

        // Calculate median drift and adjust base offset
        if time_service.peer_samples.len() >= 3 {
            let median_drift = Self::calculate_median_drift(&time_service.peer_samples);
            time_service.base_offset_us = median_drift;
        }
    }

    /// Calculate median drift from peer samples
    fn calculate_median_drift(samples: &VecDeque<Duration>) -> Duration {
        let mut sorted_samples: Vec<Duration> = samples.iter().cloned().collect();
        sorted_samples.sort();

        let median_index = sorted_samples.len() / 2;
        let median = sorted_samples[median_index];

        // Calculate drift as difference from current system time
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();

        // Return the drift (positive means peers are ahead, negative means behind)
        median.saturating_sub(now)
    }

    /// Get current time with peer-adjusted offset
    pub fn now_adjusted() -> Duration {
        let mut time_service = IPPAN_TIME.write();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();

        // Apply drift corrections from peer samples.
        let target_time = now + time_service.base_offset;
        let current_time = time_service.last_time;

        // Slew towards the target time while keeping the clock monotonic.
        let slew_rate = Duration::from_micros(1);
        let adjusted_time = if target_time > current_time {
            let diff = target_time - current_time;
            if diff <= slew_rate {
                target_time
            } else {
                current_time + slew_rate
            }
        } else {
            current_time + slew_rate
        };

        time_service.last_time = adjusted_time;
        adjusted_time
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

        // Ingest some peer samples
        let peer_times = vec![
            1000000, // 1 second
            1000001, // 1 second + 1 microsecond
            1000002, // 1 second + 2 microseconds
        ];

        for peer_time in peer_times {
            ippan_time_ingest_sample(peer_time);
        }

        // Should not panic and should handle samples correctly
        let current_time = ippan_time_now();
        assert!(current_time > 0);
    }

    #[test]
    fn test_time_precision() {
        ippan_time_init();

        let time1 = ippan_time_now();
        let time2 = ippan_time_now();

        // Should have microsecond precision while remaining monotonic
        let diff = time2.saturating_sub(time1);
        assert!(diff <= 1_000);
    }
}
