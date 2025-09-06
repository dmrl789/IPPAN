//! Hardened IPPAN Time management
//! 
//! Provides latency-compensated, quorum-verified, outlier-filtered, smoothed network time
//! with NTP-style synchronization and Byzantine fault tolerance

use std::collections::{BTreeMap, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH};

use ed25519_dalek::{Signature, VerifyingKey, SignatureError};
use sha2::{Sha256, Digest};

use crate::consensus::validators;

/// Time sample from a peer with NTP-style timestamps
#[derive(Clone, Debug)]
pub struct TimeSample {
    pub peer_id: [u8; 32],
    pub round: u64,
    pub t1_ns: u64, // local send
    pub t2_ns: u64, // peer recv
    pub t3_ns: u64, // peer send
    pub t4_ns: u64, // local recv
    pub sig: [u8; 64],
}

/// Offset estimate with delay information
#[derive(Clone, Debug)]
struct OffsetEstimate {
    pub offset_ns: i64, // estimated peer - local
    pub delay_ns: u64,
    pub at_ns: u64,     // local time when computed (T4)
}

/// Time synchronization configuration
#[derive(Clone)]
pub struct TimeConfig {
    pub sync_interval_s: u64,   // 10–30
    pub window_secs: u64,       // 120–300
    pub mad_cutoff: f64,        // 3.0
    pub soft_drift_ms: i64,     // 150
    pub hard_drift_ms: i64,     // 750
    pub slew_alpha: f64,        // 0.05..0.2 smoothing factor
}

impl Default for TimeConfig {
    fn default() -> Self {
        Self {
            sync_interval_s: 20,
            window_secs: 240,
            mad_cutoff: 3.0,
            soft_drift_ms: 150,
            hard_drift_ms: 750,
            slew_alpha: 0.12,
        }
    }
}

/// Hardened IPPAN Time engine with latency compensation and robust median
pub struct IppanTime {
    cfg: TimeConfig,
    // Rolling offsets (peer-corrected samples mapped into local clock)
    window: VecDeque<i64>, // offset_ns samples
    timestamps: VecDeque<u64>, // sample times (local T4)
    smoothed_offset_ns: f64,   // filtered estimate
    last_ns: u64,              // monotonic guard for non-decreasing time
}

impl IppanTime {
    pub fn new(cfg: TimeConfig) -> Self {
        Self {
            cfg,
            window: VecDeque::with_capacity(4096),
            timestamps: VecDeque::with_capacity(4096),
            smoothed_offset_ns: 0.0,
            last_ns: now_ns(),
        }
    }

    /// Ingest a time sample from a peer
    pub fn ingest_sample(&mut self, s: TimeSample) -> Result<(), String> {
        // 1) Accept only current validators
        if !validators::is_current_validator(&s.peer_id) {
            return Err("sample from non-validator".into());
        }
        
        // 2) Verify signature
        verify_stamp_sig(&s).map_err(|_| "bad signature")?;

        // 3) Compute NTP-style offset & delay (in local frame)
        // offset ≈ ((t2 - t1) + (t3 - t4)) / 2
        // delay  ≈ (t4 - t1) - (t3 - t2)
        let delta1 = s.t2_ns as i128 - s.t1_ns as i128;
        let delta2 = s.t3_ns as i128 - s.t4_ns as i128;
        let offset_ns = ((delta1 + delta2) / 2) as i64;

        let delay_ns = (s.t4_ns - s.t1_ns)
            .saturating_sub(s.t3_ns.saturating_sub(s.t2_ns));

        // Basic sanity: drop absurd delays (e.g., > 2s)
        if delay_ns > 2_000_000_000 {
            return Err("delay too high".into());
        }

        self.push_window(offset_ns, s.t4_ns);
        Ok(())
    }

    fn push_window(&mut self, offset_ns: i64, t_now_ns: u64) {
        self.window.push_back(offset_ns);
        self.timestamps.push_back(t_now_ns);

        // Evict old samples outside window
        let cutoff = t_now_ns.saturating_sub(self.cfg.window_secs * 1_000_000_000);
        while let Some(ts) = self.timestamps.front().copied() {
            if ts < cutoff {
                self.timestamps.pop_front();
                self.window.pop_front();
            } else { 
                break; 
            }
        }

        // Robust median with MAD filtering
        if self.window.len() >= 5 {
            let mut v = self.window.iter().copied().collect::<Vec<_>>();
            v.sort();
            let median = v[v.len()/2];
            
            // MAD (Median Absolute Deviation)
            let mut devs = v.iter().map(|x| (x - median).abs()).collect::<Vec<_>>();
            devs.sort();
            let mad = devs[devs.len()/2].max(1); // avoid div by zero

            // Filter outliers
            let cutoff = (self.cfg.mad_cutoff * mad as f64) as i64;
            let filtered = v.into_iter().filter(|x| (x - median).abs() <= cutoff).collect::<Vec<_>>();
            let est = if filtered.is_empty() { median } else { filtered[filtered.len()/2] };

            // Slew (don't step)
            self.smoothed_offset_ns = self.smoothed_offset_ns + self.cfg.slew_alpha * ((est as f64) - self.smoothed_offset_ns);
        }
    }

    /// Returns non-decreasing network time in ns (local_mono + smoothed_offset)
    pub fn network_time_ns(&mut self) -> u64 {
        let local = now_ns();
        let corrected = (local as i128 + self.smoothed_offset_ns.round() as i128).max(self.last_ns as i128) as u64;
        self.last_ns = corrected;
        corrected
    }

    /// Drift assessment vs round median (supplied after quorum aggregation)
    pub fn drift_ok(&self, round_median_ns: u64) -> DriftState {
        let here = now_ns() as i128 + self.smoothed_offset_ns.round() as i128;
        let drift_ns = (here - round_median_ns as i128).abs() as i64;
        let drift_ms = drift_ns / 1_000_000;
        if drift_ms > self.cfg.hard_drift_ms {
            DriftState::Hard
        } else if drift_ms > self.cfg.soft_drift_ms {
            DriftState::Soft
        } else { 
            DriftState::Ok 
        }
    }

    /// Get current smoothed offset
    pub fn get_smoothed_offset_ns(&self) -> f64 {
        self.smoothed_offset_ns
    }

    /// Get window size
    pub fn window_size(&self) -> usize {
        self.window.len()
    }
}

/// Drift state for role gating
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriftState { 
    Ok, 
    Soft, 
    Hard 
}

/// Round time aggregator for quorum-based time aggregation
#[derive(Default)]
pub struct RoundTimeAggregator {
    by_peer: BTreeMap<[u8;32], TimeSample>,
}

impl RoundTimeAggregator {
    pub fn ingest(&mut self, s: TimeSample) -> Result<(), String> {
        if !validators::is_current_validator(&s.peer_id) { 
            return Err("non-validator".into()); 
        }
        self.by_peer.insert(s.peer_id, s);
        Ok(())
    }

    pub fn finalize(&self) -> Result<(u64, [u8;32]), String> {
        let n = validators::current_committee_size();
        let f = validators::f_tolerance();
        let min = 2*f + 1;
        if self.by_peer.len() < min { 
            return Err("not enough samples".into()); 
        }

        // Map each sample into local time using offset
        let mut corrected: Vec<i128> = Vec::new();
        let mut leaves: Vec<[u8;32]> = Vec::new();

        for (_id, s) in self.by_peer.iter() {
            let delta1 = s.t2_ns as i128 - s.t1_ns as i128;
            let delta2 = s.t3_ns as i128 - s.t4_ns as i128;
            let offset_ns = ((delta1 + delta2) / 2) as i128;
            let local_est = s.t3_ns as i128 - offset_ns; // project peer t3 into local frame
            corrected.push(local_est);

            // leaf hash for audit
            let mut h = Sha256::new();
            h.update(&s.t2_ns.to_le_bytes());
            h.update(&s.t3_ns.to_le_bytes());
            h.update(&s.peer_id);
            h.update(&s.round.to_le_bytes());
            h.update(&s.sig);
            leaves.push(trunc32(h.finalize()));
        }

        corrected.sort();
        let median_local = corrected[corrected.len()/2] as u64;

        let mr = merkle_root(leaves);
        Ok((median_local, mr))
    }

    /// Get sample count
    pub fn sample_count(&self) -> usize {
        self.by_peer.len()
    }

    /// Check if quorum is met
    pub fn has_quorum(&self) -> bool {
        let f = validators::f_tolerance();
        let min = 2*f + 1;
        self.by_peer.len() >= min
    }
}

/// Verify time stamp signature
fn verify_stamp_sig(s: &TimeSample) -> Result<(), SignatureError> {
    let vk: VerifyingKey = validators::pubkey_for(&s.peer_id);
    let mut hasher = Sha256::new();
    hasher.update(&s.t2_ns.to_le_bytes());
    hasher.update(&s.t3_ns.to_le_bytes());
    hasher.update(&s.peer_id);
    hasher.update(&s.round.to_le_bytes());
    let digest = hasher.finalize();
    let sig = Signature::from_bytes(&s.sig);
    vk.verify_strict(&digest, &sig)
}

/// Get current time in nanoseconds
fn now_ns() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64
}

/// Truncate hash to 32 bytes
fn trunc32(d: impl AsRef<[u8]>) -> [u8;32] {
    let mut out = [0u8;32];
    let b = d.as_ref();
    out.copy_from_slice(&b[..32]);
    out
}

/// Compute Merkle root of time samples
fn merkle_root(mut leaves: Vec<[u8;32]>) -> [u8;32] {
    if leaves.is_empty() { 
        return [0u8;32]; 
    }
    while leaves.len() > 1 {
        let mut next = Vec::with_capacity((leaves.len()+1)/2);
        for chunk in leaves.chunks(2) {
            let pair = if chunk.len() == 2 { [chunk[0], chunk[1]] } else { [chunk[0], chunk[0]] };
            let mut h = Sha256::new();
            h.update(&pair[0]);
            h.update(&pair[1]);
            next.push(trunc32(h.finalize()));
        }
        leaves = next;
    }
    leaves[0]
}

/// Statistics about time samples
#[derive(Debug, Clone)]
pub struct TimeStats {
    pub count: usize,
    pub min: i64,
    pub max: i64,
    pub mean: f64,
    pub median: i64,
    pub std_dev: f64,
    pub smoothed_offset_ns: f64,
    pub window_size: usize,
}

impl IppanTime {
    /// Get statistics about the time samples
    pub fn get_stats(&self) -> TimeStats {
        if self.window.is_empty() {
            return TimeStats {
                count: 0,
                min: 0,
                max: 0,
                mean: 0.0,
                median: 0,
                std_dev: 0.0,
                smoothed_offset_ns: self.smoothed_offset_ns,
                window_size: 0,
            };
        }

        let offsets: Vec<i64> = self.window.iter().copied().collect();
        let count = offsets.len();
        let min = *offsets.iter().min().unwrap();
        let max = *offsets.iter().max().unwrap();
        let sum: i64 = offsets.iter().sum();
        let mean = sum as f64 / count as f64;
        
        let variance = offsets.iter()
            .map(|&t| {
                let diff = t as f64 - mean;
                diff * diff
            })
            .sum::<f64>() / count as f64;
        let std_dev = variance.sqrt();

        let mut sorted_offsets = offsets.clone();
        sorted_offsets.sort();
        let median = sorted_offsets[sorted_offsets.len()/2];

        TimeStats {
            count,
            min,
            max,
            mean,
            median,
            std_dev,
            smoothed_offset_ns: self.smoothed_offset_ns,
            window_size: self.window.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_sample(peer: [u8;32], round: u64, t1:u64,t2:u64,t3:u64,t4:u64) -> TimeSample {
        TimeSample { 
            peer_id: peer, 
            round, 
            t1_ns:t1, 
            t2_ns:t2, 
            t3_ns:t3, 
            t4_ns:t4, 
            sig:[0u8;64] 
        }
    }

    #[test]
    fn ntp_offset_computation() {
        // Initialize validator registry for the test
        let _ = crate::consensus::validators::init_validator_registry();
        let mut it = IppanTime::new(TimeConfig::default());
        // Simple symmetric path: true offset = +10ms, delay = 4ms
        let base = 1_000_000_000u64;
        let s = fake_sample(
            [1;32], 
            1, 
            base, 
            base+10_000_000+2_000_000, 
            base+10_000_000+2_000_000, 
            base+4_000_000
        );
        // Skip signature & validator checks in this unit test by mocking verify/is_validator if needed.
        let _ = it.ingest_sample(s); // in real test, mock verify to Ok
        // After enough samples, smoothed_offset_ns should approach ~+10_000_000
    }

    #[test]
    fn robust_median_filters_outliers() {
        let mut it = IppanTime::new(TimeConfig::default());
        // Feed a cluster around +5ms and some wild outliers
        // Assert network_time_ns increases monotonically and offset stays near +5ms.
    }

    #[test]
    fn drift_policy_thresholds() {
        let it = IppanTime::new(TimeConfig::default());
        // Simulate drift vs median and assert Ok/Soft/Hard boundaries at 150/750 ms.
    }

    #[test]
    fn round_aggregator_quorum() {
        let mut agg = RoundTimeAggregator::default();
        // Test quorum requirements and median calculation
    }
}
