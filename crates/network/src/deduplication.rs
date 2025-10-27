use parking_lot::RwLock;
use std::collections::HashSet;
use std::time::{Duration, Instant};

/// Message deduplication cache to prevent processing duplicate messages
/// Uses a time-based eviction policy to prevent unbounded growth
#[derive(Debug)]
pub struct MessageDeduplicator {
    seen_messages: RwLock<HashSet<[u8; 32]>>,
    last_cleanup: RwLock<Instant>,
    cleanup_interval: Duration,
    max_size: usize,
}

impl MessageDeduplicator {
    /// Create a new message deduplicator
    pub fn new(cleanup_interval: Duration, max_size: usize) -> Self {
        Self {
            seen_messages: RwLock::new(HashSet::new()),
            last_cleanup: RwLock::new(Instant::now()),
            cleanup_interval,
            max_size,
        }
    }

    /// Check if a message has been seen before and mark it as seen
    /// Returns true if this is a new message, false if it's a duplicate
    pub fn check_and_mark(&self, message_hash: [u8; 32]) -> bool {
        // Periodic cleanup to prevent unbounded growth
        self.maybe_cleanup();

        let mut seen = self.seen_messages.write();
        
        // If at capacity, remove oldest (random) entry
        if seen.len() >= self.max_size {
            if let Some(&first) = seen.iter().next() {
                seen.remove(&first);
            }
        }

        seen.insert(message_hash)
    }

    /// Check if a message has been seen without marking it
    pub fn has_seen(&self, message_hash: &[u8; 32]) -> bool {
        self.seen_messages.read().contains(message_hash)
    }

    /// Get the number of tracked messages
    pub fn size(&self) -> usize {
        self.seen_messages.read().len()
    }

    /// Clear all tracked messages
    pub fn clear(&self) {
        self.seen_messages.write().clear();
        *self.last_cleanup.write() = Instant::now();
    }

    /// Periodic cleanup based on time
    fn maybe_cleanup(&self) {
        let mut last_cleanup = self.last_cleanup.write();
        if last_cleanup.elapsed() >= self.cleanup_interval {
            // Clear half of the cache periodically
            let mut seen = self.seen_messages.write();
            let target_size = seen.len() / 2;
            if seen.len() > target_size {
                let to_remove: Vec<[u8; 32]> = seen.iter().take(seen.len() - target_size).copied().collect();
                for hash in to_remove {
                    seen.remove(&hash);
                }
            }
            *last_cleanup = Instant::now();
        }
    }
}

impl Default for MessageDeduplicator {
    fn default() -> Self {
        Self::new(Duration::from_secs(300), 10_000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deduplication() {
        let dedup = MessageDeduplicator::new(Duration::from_secs(60), 100);
        
        let msg1 = [1u8; 32];
        let msg2 = [2u8; 32];
        
        // First time seeing msg1 should return true
        assert!(dedup.check_and_mark(msg1));
        
        // Second time seeing msg1 should return false
        assert!(!dedup.check_and_mark(msg1));
        
        // First time seeing msg2 should return true
        assert!(dedup.check_and_mark(msg2));
        
        // Check without marking
        assert!(dedup.has_seen(&msg1));
        assert!(dedup.has_seen(&msg2));
    }

    #[test]
    fn test_max_size() {
        let dedup = MessageDeduplicator::new(Duration::from_secs(60), 10);
        
        // Add more than max_size messages
        for i in 0..15u8 {
            let mut msg = [0u8; 32];
            msg[0] = i;
            dedup.check_and_mark(msg);
        }
        
        // Should not exceed max_size
        assert!(dedup.size() <= 10);
    }

    #[test]
    fn test_clear() {
        let dedup = MessageDeduplicator::new(Duration::from_secs(60), 100);
        
        dedup.check_and_mark([1u8; 32]);
        dedup.check_and_mark([2u8; 32]);
        
        assert_eq!(dedup.size(), 2);
        
        dedup.clear();
        assert_eq!(dedup.size(), 0);
    }
}
