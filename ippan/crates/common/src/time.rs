use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};


/// Peer time information
#[derive(Debug, Clone)]
struct PeerTime {
    peer_id: String,
    time_us: u64,
    timestamp: Instant,
}

/// IPPAN Time implementation with median-of-peers and drift guard
pub struct IppanTime {
    peers: Arc<RwLock<VecDeque<PeerTime>>>,
    local_median: Arc<RwLock<u64>>,
    last_update: Arc<RwLock<Instant>>,
    drift_guard_ms: u64,
    max_peers: usize,
}

impl IppanTime {
    /// Create a new IPPAN time instance
    pub fn new() -> Self {
        Self {
            peers: Arc::new(RwLock::new(VecDeque::new())),
            local_median: Arc::new(RwLock::new(Self::local_time_us())),
            last_update: Arc::new(RwLock::new(Instant::now())),
            drift_guard_ms: 2, // 2ms drift guard as per PRD
            max_peers: 100,
        }
    }

    /// Get local time in microseconds
    pub fn local_time_us() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_micros() as u64
    }

    /// Add peer time information
    pub async fn add_peer_time(&self, peer_id: String, peer_time_us: u64) {
        let mut peers = self.peers.write().unwrap();
        
        // Remove old entries (older than 60 seconds)
        let cutoff = Instant::now() - Duration::from_secs(60);
        peers.retain(|pt| pt.timestamp > cutoff);
        
        // Add new peer time
        let peer_time = PeerTime {
            peer_id,
            time_us: peer_time_us,
            timestamp: Instant::now(),
        };
        
        peers.push_back(peer_time);
        
        // Limit number of peers
        if peers.len() > self.max_peers {
            peers.pop_front();
        }
        
        // Update median
        self.update_median().await;
    }

    /// Update the median time
    async fn update_median(&self) {
        let peers = self.peers.read().unwrap();
        if peers.is_empty() {
            return;
        }

        let local_time = Self::local_time_us();
        let mut valid_times = Vec::new();
        
        // Collect valid peer times (within drift guard)
        for peer_time in peers.iter() {
            let time_diff = if peer_time.time_us > local_time {
                peer_time.time_us - local_time
            } else {
                local_time - peer_time.time_us
            };
            
            // Convert to milliseconds for drift guard check
            let time_diff_ms = time_diff / 1000;
            
            if time_diff_ms <= self.drift_guard_ms {
                valid_times.push(peer_time.time_us);
            }
        }
        
        if valid_times.is_empty() {
            // No valid peers, use local time
            *self.local_median.write().unwrap() = local_time;
        } else {
            // Calculate median
            valid_times.sort_unstable();
            let median = if valid_times.len() % 2 == 0 {
                (valid_times[valid_times.len() / 2 - 1] + valid_times[valid_times.len() / 2]) / 2
            } else {
                valid_times[valid_times.len() / 2]
            };
            
            *self.local_median.write().unwrap() = median;
        }
        
        *self.last_update.write().unwrap() = Instant::now();
    }

    /// Get IPPAN time in microseconds
    pub async fn ippan_time_us(&self) -> u64 {
        // Update median if needed (every 100ms)
        let last_update = *self.last_update.read().unwrap();
        if last_update.elapsed() > Duration::from_millis(100) {
            self.update_median().await;
        }
        
        *self.local_median.read().unwrap()
    }

    /// Get the number of active peers
    pub fn peer_count(&self) -> usize {
        self.peers.read().unwrap().len()
    }

    /// Get drift guard setting
    pub fn drift_guard_ms(&self) -> u64 {
        self.drift_guard_ms
    }

    /// Set drift guard (for testing)
    pub fn set_drift_guard_ms(&mut self, drift_ms: u64) {
        self.drift_guard_ms = drift_ms;
    }
}

impl Default for IppanTime {
    fn default() -> Self {
        Self::new()
    }
}

/// Get current IPPAN time in microseconds (simplified version)
pub fn ippan_time_us() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_micros() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_ippan_time_creation() {
        let ippan_time = IppanTime::new();
        assert_eq!(ippan_time.peer_count(), 0);
        assert_eq!(ippan_time.drift_guard_ms(), 2);
    }

    #[tokio::test]
    async fn test_local_time() {
        let time1 = IppanTime::local_time_us();
        sleep(Duration::from_millis(1)).await;
        let time2 = IppanTime::local_time_us();
        
        assert!(time2 > time1);
    }

    #[tokio::test]
    async fn test_peer_time_addition() {
        let ippan_time = IppanTime::new();
        let local_time = IppanTime::local_time_us();
        
        // Add peer time within drift guard
        ippan_time.add_peer_time("peer1".to_string(), local_time + 1000).await;
        assert_eq!(ippan_time.peer_count(), 1);
        
        // Add peer time outside drift guard
        ippan_time.add_peer_time("peer2".to_string(), local_time + 5000).await;
        assert_eq!(ippan_time.peer_count(), 2);
        
        let ippan_time_us = ippan_time.ippan_time_us().await;
        assert!(ippan_time_us > 0);
    }

    #[tokio::test]
    async fn test_median_calculation() {
        let mut ippan_time = IppanTime::new();
        ippan_time.set_drift_guard_ms(100); // Larger drift guard for testing
        
        let base_time = IppanTime::local_time_us();
        
        // Add multiple peer times
        ippan_time.add_peer_time("peer1".to_string(), base_time + 1000).await;
        ippan_time.add_peer_time("peer2".to_string(), base_time + 2000).await;
        ippan_time.add_peer_time("peer3".to_string(), base_time + 3000).await;
        
        let median = ippan_time.ippan_time_us().await;
        assert!(median >= base_time + 1000);
        assert!(median <= base_time + 3000);
    }
}
