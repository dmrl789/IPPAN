//! Fee collection and management for the treasury
//!
//! Tracks per-round transaction fees, aggregates statistics, and supports
//! deterministic recycling back into validator rewards or system funds.

use anyhow::Result;
use ippan_types::MicroIPN;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info};

/// Fee collection statistics for monitoring and analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeCollectionStats {
    pub total_collected_micro: MicroIPN,
    pub total_rounds: u64,
    pub average_per_round: MicroIPN,
    pub highest_round: MicroIPN,
    pub lowest_round: MicroIPN,
}

/// Fee collector for managing transaction fees
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FeeCollector {
    /// Round → total fees collected (in µIPN)
    round_fees: HashMap<u64, MicroIPN>,
    /// Total fees collected across all rounds
    total_collected_micro: MicroIPN,
}

impl FeeCollector {
    /// Create a new fee collector
    pub fn new() -> Self {
        Self {
            round_fees: HashMap::new(),
            total_collected_micro: 0,
        }
    }

    /// Collect fees for a round
    pub fn collect_round_fees(&mut self, round: u64, fees_micro: MicroIPN) -> Result<()> {
        if fees_micro == 0 {
            debug!(target: "treasury", "Round {}: No fees to collect", round);
            return Ok(());
        }

        self.round_fees.insert(round, fees_micro);
        self.total_collected_micro = self.total_collected_micro.saturating_add(fees_micro);

        info!(
            target: "treasury",
            "Round {}: Collected {} µIPN in fees",
            round,
            fees_micro
        );

        Ok(())
    }

    /// Get fees collected in a specific round
    pub fn get_round_fees(&self, round: u64) -> MicroIPN {
        self.round_fees.get(&round).copied().unwrap_or(0)
    }

    /// Get total fees collected
    pub fn get_total_collected(&self) -> MicroIPN {
        self.total_collected_micro
    }

    /// Return all rounds with recorded fees
    pub fn get_rounds_with_fees(&self) -> Vec<u64> {
        self.round_fees.keys().copied().collect()
    }

    /// Generate summary statistics
    pub fn get_statistics(&self) -> FeeCollectionStats {
        let total_rounds = self.round_fees.len() as u64;
        let fees: Vec<MicroIPN> = self.round_fees.values().copied().collect();

        let average_per_round = if total_rounds > 0 {
            self.total_collected_micro / total_rounds as u128
        } else {
            0
        };

        let highest_round = fees.iter().max().copied().unwrap_or(0);
        let lowest_round = fees.iter().min().copied().unwrap_or(0);

        FeeCollectionStats {
            total_collected_micro: self.total_collected_micro,
            total_rounds,
            average_per_round,
            highest_round,
            lowest_round,
        }
    }

    /// Clear older rounds from the map to save memory
    pub fn clear_old_fees(&mut self, keep_from_round: u64) {
        let rounds_to_remove: Vec<u64> = self
            .round_fees
            .keys()
            .filter(|&&round| round < keep_from_round)
            .copied()
            .collect();

        for round in rounds_to_remove.iter() {
            self.round_fees.remove(round);
        }

        debug!(
            target: "treasury",
            "Cleared {} outdated round fee entries (kept ≥ {})",
            rounds_to_remove.len(),
            keep_from_round
        );
    }

    /// Get cumulative fees within a round range
    pub fn get_fees_for_range(&self, start_round: u64, end_round: u64) -> MicroIPN {
        (start_round..=end_round)
            .map(|r| self.get_round_fees(r))
            .sum()
    }
}

/// Manages recycling of collected fees into treasury or reward pools
#[derive(Debug, Clone)]
pub struct FeeRecyclingManager {
    collector: FeeCollector,
    recycling_rate_bps: u16, // Basis points (e.g., 10000 = 100%)
}

impl FeeRecyclingManager {
    /// Create a new fee recycling manager
    pub fn new(recycling_rate_bps: u16) -> Self {
        Self {
            collector: FeeCollector::new(),
            recycling_rate_bps,
        }
    }

    /// Collect fees for a round
    pub fn collect_round_fees(&mut self, round: u64, fees_micro: MicroIPN) -> Result<()> {
        self.collector.collect_round_fees(round, fees_micro)
    }

    /// Calculate how much of the collected fees should be recycled
    pub fn calculate_recycling_amount(&self, total_fees_micro: MicroIPN) -> MicroIPN {
        (total_fees_micro * self.recycling_rate_bps as u128) / 10_000
    }

    /// Get total amount available for recycling (based on all collected fees)
    pub fn get_available_for_recycling(&self) -> MicroIPN {
        self.calculate_recycling_amount(self.collector.get_total_collected())
    }

    /// Immutable access to the fee collector
    pub fn get_collector(&self) -> &FeeCollector {
        &self.collector
    }

    /// Mutable access to the fee collector
    pub fn get_collector_mut(&mut self) -> &mut FeeCollector {
        &mut self.collector
    }

    /// Update recycling rate dynamically (in basis points)
    pub fn set_recycling_rate(&mut self, rate_bps: u16) {
        self.recycling_rate_bps = rate_bps;
        info!(
            target: "treasury",
            "Updated fee recycling rate to {} bps",
            rate_bps
        );
    }
}

// -----------------------------------------------------------------------------
// ✅ Unit Tests
// -----------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fee_collector_creation() {
        let collector = FeeCollector::new();
        assert_eq!(collector.get_total_collected(), 0);
        assert!(collector.get_rounds_with_fees().is_empty());
    }

    #[test]
    fn test_collect_round_fees() {
        let mut collector = FeeCollector::new();

        collector.collect_round_fees(1, 1000).unwrap();
        collector.collect_round_fees(2, 2000).unwrap();

        assert_eq!(collector.get_total_collected(), 3000);
        assert_eq!(collector.get_round_fees(1), 1000);
        assert_eq!(collector.get_round_fees(2), 2000);
        assert_eq!(collector.get_rounds_with_fees().len(), 2);
    }

    #[test]
    fn test_fee_statistics() {
        let mut collector = FeeCollector::new();
        collector.collect_round_fees(1, 1000).unwrap();
        collector.collect_round_fees(2, 2000).unwrap();
        collector.collect_round_fees(3, 500).unwrap();

        let stats = collector.get_statistics();
        assert_eq!(stats.total_collected_micro, 3500);
        assert_eq!(stats.total_rounds, 3);
        assert_eq!(stats.average_per_round, 1166);
        assert_eq!(stats.highest_round, 2000);
        assert_eq!(stats.lowest_round, 500);
    }

    #[test]
    fn test_fee_recycling_manager() {
        let mut manager = FeeRecyclingManager::new(5000); // 50% recycling

        manager.collect_round_fees(1, 1000).unwrap();
        manager.collect_round_fees(2, 2000).unwrap();

        let available = manager.get_available_for_recycling();
        assert_eq!(available, 1500); // 50% of 3000

        manager.set_recycling_rate(10000); // 100% recycling
        let available = manager.get_available_for_recycling();
        assert_eq!(available, 3000);
    }

    #[test]
    fn test_fee_range_query() {
        let mut collector = FeeCollector::new();
        collector.collect_round_fees(1, 1000).unwrap();
        collector.collect_round_fees(2, 2000).unwrap();
        collector.collect_round_fees(4, 500).unwrap(); // skip round 3

        assert_eq!(collector.get_fees_for_range(1, 4), 3500);
        assert_eq!(collector.get_fees_for_range(2, 3), 2000);
    }
}
