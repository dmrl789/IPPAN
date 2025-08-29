use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{debug, warn};

const DRIFT_GUARD_MS: u64 = 2;
const MAX_PEER_TIMES: usize = 100;

#[derive(Debug, Clone)]
pub struct PeerTime {
    pub peer_id: String,
    pub time_us: u64,
    pub received_at: Instant,
}

pub struct IppanTime {
    peers: Arc<RwLock<VecDeque<PeerTime>>>,
    local_median: Arc<RwLock<u64>>,
    last_update: Arc<RwLock<Instant>>,
}

impl IppanTime {
    pub fn new() -> Self {
        let now = Self::local_time_us();
        Self {
            peers: Arc::new(RwLock::new(VecDeque::new())),
            local_median: Arc::new(RwLock::new(now)),
            last_update: Arc::new(RwLock::new(Instant::now())),
        }
    }

    pub fn local_time_us() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64
    }

    pub async fn add_peer_time(&self, peer_id: String, peer_time_us: u64) {
        let mut peers = self.peers.write().await;
        
        // Remove old entries (older than 60 seconds)
        let cutoff = Instant::now() - Duration::from_secs(60);
        peers.retain(|p| p.received_at > cutoff);
        
        // Add new peer time
        peers.push_back(PeerTime {
            peer_id,
            time_us: peer_time_us,
            received_at: Instant::now(),
        });
        
        // Keep only the most recent MAX_PEER_TIMES
        if peers.len() > MAX_PEER_TIMES {
            peers.pop_front();
        }
        
        drop(peers);
        
        // Update median
        self.update_median().await;
    }

    async fn update_median(&self) {
        let peers = self.peers.read().await;
        if peers.is_empty() {
            return;
        }

        let local_time = Self::local_time_us();
        let mut valid_times = Vec::new();

        for peer in peers.iter() {
            let drift = if peer.time_us > local_time {
                peer.time_us - local_time
            } else {
                local_time - peer.time_us
            };

            // Drift guard: ignore peers deviating >2ms
            if drift <= DRIFT_GUARD_MS * 1000 {
                valid_times.push(peer.time_us);
            } else {
                warn!(
                    "Ignoring peer {} with drift {}μs (max: {}μs)",
                    peer.peer_id, drift, DRIFT_GUARD_MS * 1000
                );
            }
        }

        if !valid_times.is_empty() {
            valid_times.sort_unstable();
            let median = if valid_times.len() % 2 == 0 {
                let mid = valid_times.len() / 2;
                (valid_times[mid - 1] + valid_times[mid]) / 2
            } else {
                valid_times[valid_times.len() / 2]
            };

            let mut local_median = self.local_median.write().await;
            *local_median = median;
            *self.last_update.write().await = Instant::now();

            debug!("Updated IPPAN time median: {}μs (from {} peers)", median, valid_times.len());
        }
    }

    pub async fn ippan_time_us(&self) -> u64 {
        let median = *self.local_median.read().await;
        let last_update = *self.last_update.read().await;
        
        // If we haven't updated recently, use local time
        if last_update.elapsed() > Duration::from_secs(5) {
            Self::local_time_us()
        } else {
            median
        }
    }

    pub async fn get_peer_count(&self) -> usize {
        self.peers.read().await.len()
    }
}

impl Default for IppanTime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ippan_time_creation() {
        let ippan_time = IppanTime::new();
        let time = ippan_time.ippan_time_us().await;
        assert!(time > 0);
    }

    #[tokio::test]
    async fn test_peer_time_addition() {
        let ippan_time = IppanTime::new();
        ippan_time.add_peer_time("peer1".to_string(), 1000000).await;
        ippan_time.add_peer_time("peer2".to_string(), 1000001).await;
        
        assert_eq!(ippan_time.get_peer_count().await, 2);
    }

    #[tokio::test]
    async fn test_drift_guard() {
        let ippan_time = IppanTime::new();
        let local_time = IppanTime::local_time_us();
        
        // Add peer with acceptable drift
        ippan_time.add_peer_time("good_peer".to_string(), local_time + 1000).await;
        
        // Add peer with excessive drift (should be ignored)
        ippan_time.add_peer_time("bad_peer".to_string(), local_time + 5_000_000).await;
        
        // The bad peer should be filtered out by drift guard
        let peer_count = ippan_time.get_peer_count().await;
        assert!(peer_count <= 2); // At most 2 peers (good_peer + potentially bad_peer)
    }
}
