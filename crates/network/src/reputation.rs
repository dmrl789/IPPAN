use parking_lot::RwLock;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, warn};

/// Peer reputation score
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReputationScore(i32);

impl ReputationScore {
    pub const MIN: i32 = -1000;
    pub const MAX: i32 = 1000;
    pub const INITIAL: i32 = 0;
    pub const THRESHOLD_BAN: i32 = -500;
    pub const THRESHOLD_WARN: i32 = -100;

    pub fn new(value: i32) -> Self {
        Self(value.clamp(Self::MIN, Self::MAX))
    }

    pub fn value(&self) -> i32 {
        self.0
    }

    pub fn is_banned(&self) -> bool {
        self.0 <= Self::THRESHOLD_BAN
    }

    pub fn is_warning(&self) -> bool {
        self.0 <= Self::THRESHOLD_WARN && self.0 > Self::THRESHOLD_BAN
    }
}

impl Default for ReputationScore {
    fn default() -> Self {
        Self::new(Self::INITIAL)
    }
}

/// Reputation tracking for a single peer
#[derive(Debug, Clone)]
struct PeerReputation {
    score: ReputationScore,
    successful_messages: u64,
    failed_messages: u64,
    invalid_messages: u64,
    last_seen: Instant,
    first_seen: Instant,
}

impl PeerReputation {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            score: ReputationScore::default(),
            successful_messages: 0,
            failed_messages: 0,
            invalid_messages: 0,
            last_seen: now,
            first_seen: now,
        }
    }

    fn update_score(&mut self, delta: i32) {
        let new_value = self.score.value() + delta;
        self.score = ReputationScore::new(new_value);
    }
}

/// Reputation manager for tracking peer behavior
#[derive(Debug)]
pub struct ReputationManager {
    peers: RwLock<HashMap<String, PeerReputation>>,
    decay_interval: Duration,
    decay_amount: i32,
    last_decay: RwLock<Instant>,
}

impl ReputationManager {
    /// Create a new reputation manager
    pub fn new(decay_interval: Duration, decay_amount: i32) -> Self {
        Self {
            peers: RwLock::new(HashMap::new()),
            decay_interval,
            decay_amount,
            last_decay: RwLock::new(Instant::now()),
        }
    }

    /// Record a successful message from a peer
    pub fn record_success(&self, peer_address: &str) {
        self.maybe_decay();
        let mut peers = self.peers.write();
        let reputation = peers.entry(peer_address.to_string()).or_insert_with(PeerReputation::new);
        reputation.successful_messages += 1;
        reputation.last_seen = Instant::now();
        reputation.update_score(1);
    }

    /// Record a failed message from a peer
    pub fn record_failure(&self, peer_address: &str) {
        self.maybe_decay();
        let mut peers = self.peers.write();
        let reputation = peers.entry(peer_address.to_string()).or_insert_with(PeerReputation::new);
        reputation.failed_messages += 1;
        reputation.last_seen = Instant::now();
        reputation.update_score(-5);
    }

    /// Record an invalid message from a peer
    pub fn record_invalid(&self, peer_address: &str) {
        self.maybe_decay();
        let mut peers = self.peers.write();
        let reputation = peers.entry(peer_address.to_string()).or_insert_with(PeerReputation::new);
        reputation.invalid_messages += 1;
        reputation.last_seen = Instant::now();
        reputation.update_score(-20);
        
        if reputation.score.is_banned() {
            warn!("Peer {} has been banned due to low reputation score: {}", peer_address, reputation.score.value());
        } else if reputation.score.is_warning() {
            warn!("Peer {} has low reputation score: {}", peer_address, reputation.score.value());
        }
    }

    /// Get the reputation score for a peer
    pub fn get_score(&self, peer_address: &str) -> ReputationScore {
        self.peers
            .read()
            .get(peer_address)
            .map(|r| r.score)
            .unwrap_or_default()
    }

    /// Check if a peer should be banned
    pub fn should_ban(&self, peer_address: &str) -> bool {
        self.get_score(peer_address).is_banned()
    }

    /// Get reputation statistics for a peer
    pub fn get_stats(&self, peer_address: &str) -> Option<PeerReputationStats> {
        self.peers.read().get(peer_address).map(|r| PeerReputationStats {
            score: r.score.value(),
            successful_messages: r.successful_messages,
            failed_messages: r.failed_messages,
            invalid_messages: r.invalid_messages,
            uptime_seconds: r.first_seen.elapsed().as_secs(),
            last_seen_seconds: r.last_seen.elapsed().as_secs(),
        })
    }

    /// Get all peer addresses
    pub fn list_peers(&self) -> Vec<String> {
        self.peers.read().keys().cloned().collect()
    }

    /// Remove a peer from reputation tracking
    pub fn remove_peer(&self, peer_address: &str) {
        self.peers.write().remove(peer_address);
        debug!("Removed peer {} from reputation tracking", peer_address);
    }

    /// Decay reputation scores periodically
    fn maybe_decay(&self) {
        let mut last_decay = self.last_decay.write();
        if last_decay.elapsed() >= self.decay_interval {
            let mut peers = self.peers.write();
            for (addr, reputation) in peers.iter_mut() {
                // Decay score toward zero
                if reputation.score.value() > 0 {
                    reputation.update_score(-self.decay_amount);
                } else if reputation.score.value() < 0 {
                    reputation.update_score(self.decay_amount);
                }
                debug!("Decayed reputation for {}: {}", addr, reputation.score.value());
            }
            *last_decay = Instant::now();
        }
    }
}

impl Default for ReputationManager {
    fn default() -> Self {
        Self::new(Duration::from_secs(300), 10)
    }
}

/// Reputation statistics for a peer
#[derive(Debug, Clone)]
pub struct PeerReputationStats {
    pub score: i32,
    pub successful_messages: u64,
    pub failed_messages: u64,
    pub invalid_messages: u64,
    pub uptime_seconds: u64,
    pub last_seen_seconds: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reputation_score() {
        let score = ReputationScore::new(100);
        assert_eq!(score.value(), 100);
        assert!(!score.is_banned());
        assert!(!score.is_warning());

        let banned_score = ReputationScore::new(-600);
        assert!(banned_score.is_banned());

        let warning_score = ReputationScore::new(-200);
        assert!(warning_score.is_warning());
        assert!(!warning_score.is_banned());
    }

    #[test]
    fn test_reputation_manager() {
        let manager = ReputationManager::new(Duration::from_secs(60), 10);
        
        let peer = "127.0.0.1:9000";
        
        // Initial score should be 0
        assert_eq!(manager.get_score(peer).value(), 0);
        
        // Record success should increase score
        manager.record_success(peer);
        assert_eq!(manager.get_score(peer).value(), 1);
        
        // Record failure should decrease score
        manager.record_failure(peer);
        assert_eq!(manager.get_score(peer).value(), -4);
        
        // Record invalid should decrease score significantly
        manager.record_invalid(peer);
        assert_eq!(manager.get_score(peer).value(), -24);
    }

    #[test]
    fn test_reputation_stats() {
        let manager = ReputationManager::new(Duration::from_secs(60), 10);
        
        let peer = "127.0.0.1:9000";
        
        manager.record_success(peer);
        manager.record_success(peer);
        manager.record_failure(peer);
        
        let stats = manager.get_stats(peer).unwrap();
        assert_eq!(stats.successful_messages, 2);
        assert_eq!(stats.failed_messages, 1);
        assert_eq!(stats.invalid_messages, 0);
    }

    #[test]
    fn test_should_ban() {
        let manager = ReputationManager::new(Duration::from_secs(60), 10);
        
        let peer = "127.0.0.1:9000";
        
        // Record many invalid messages to trigger ban
        for _ in 0..30 {
            manager.record_invalid(peer);
        }
        
        assert!(manager.should_ban(peer));
    }
}
