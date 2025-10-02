use std::time::Duration;

/// Retention policy describing how long data should be kept.
#[derive(Debug, Clone, Copy, Default)]
pub struct RetentionPolicy {
    /// Number of most-recent heights to preserve. `None` disables height based retention.
    pub retain_latest_heights: Option<u64>,
    /// Duration to preserve data starting from its timestamp. `None` disables time based retention.
    pub retain_duration: Option<Duration>,
}

impl RetentionPolicy {
    /// Determine whether an item should be pruned based on height and timestamp metadata.
    pub fn should_prune(
        &self,
        height: u64,
        timestamp_us: u64,
        latest_height: u64,
        now_us: u64,
    ) -> bool {
        let prune_due_to_height = self
            .retain_latest_heights
            .map(|retain| {
                if retain == 0 {
                    // Only keep the latest height.
                    height < latest_height
                } else {
                    let keep_from = latest_height.saturating_sub(retain.saturating_sub(1));
                    height < keep_from
                }
            })
            .unwrap_or(false);

        let prune_due_to_time = self
            .retain_duration
            .map(|duration| {
                let retain_us = duration.as_micros().min(u64::MAX as u128) as u64;
                timestamp_us.saturating_add(retain_us) < now_us
            })
            .unwrap_or(false);

        prune_due_to_height || prune_due_to_time
    }

    /// Check if the policy is entirely disabled.
    pub fn is_disabled(&self) -> bool {
        self.retain_latest_heights.is_none() && self.retain_duration.is_none()
    }
}

/// Collection of retention policies covering the major data classes we track.
#[derive(Debug, Clone, Copy, Default)]
pub struct RetentionPolicies {
    pub block_bodies: Option<RetentionPolicy>,
    pub receipts: Option<RetentionPolicy>,
    pub snapshots: Option<RetentionPolicy>,
}

/// Enumeration of resources that can be pruned by the retention manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetentionTarget {
    BlockBodies,
    Receipts,
    Snapshots,
}

/// Outcome of a retention pass over a single target.
#[derive(Debug, Clone, Copy)]
pub struct PruneReport {
    pub target: RetentionTarget,
    pub pruned_entries: u64,
    pub retained_entries: u64,
}

impl PruneReport {
    pub fn new(target: RetentionTarget) -> Self {
        Self {
            target,
            pruned_entries: 0,
            retained_entries: 0,
        }
    }
}
