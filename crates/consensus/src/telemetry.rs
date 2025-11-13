//! Validator telemetry tracking module
//!
//! Tracks validator performance metrics for AI-based reputation scoring

use anyhow::Result;
use ippan_storage::{Storage, ValidatorTelemetry};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, warn};

/// Telemetry manager for tracking validator performance
pub struct TelemetryManager {
    storage: Arc<dyn Storage + Send + Sync>,
    current_round: Arc<RwLock<u64>>,
    cache: Arc<RwLock<HashMap<[u8; 32], ValidatorTelemetry>>>,
}

impl TelemetryManager {
    pub fn new(storage: Arc<dyn Storage + Send + Sync>) -> Self {
        Self {
            storage,
            current_round: Arc::new(RwLock::new(0)),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Load all validator telemetry from storage into cache
    pub fn load_from_storage(&self) -> Result<()> {
        let all_telemetry = self.storage.get_all_validator_telemetry()?;
        let mut cache = self.cache.write();
        *cache = all_telemetry;
        debug!(
            "Loaded {} validator telemetry records from storage",
            cache.len()
        );
        Ok(())
    }

    /// Get telemetry for a specific validator
    pub fn get_telemetry(&self, validator_id: &[u8; 32]) -> Option<ValidatorTelemetry> {
        self.cache.read().get(validator_id).cloned()
    }

    /// Get telemetry for all validators, with defaults for missing ones
    pub fn get_all_telemetry_with_defaults(
        &self,
        validator_ids: &[[u8; 32]],
        stakes: &HashMap<[u8; 32], u64>,
    ) -> HashMap<[u8; 32], ValidatorTelemetry> {
        let cache = self.cache.read();
        let current_round = *self.current_round.read();

        validator_ids
            .iter()
            .map(|&id| {
                let telemetry = cache.get(&id).cloned().unwrap_or_else(|| {
                    // Create default telemetry for new validators
                    ValidatorTelemetry {
                        validator_id: id,
                        blocks_proposed: 0,
                        blocks_verified: 0,
                        rounds_active: 1,
                        avg_latency_us: 100_000, // 100ms default
                        slash_count: 0,
                        stake: stakes.get(&id).copied().unwrap_or(0),
                        age_rounds: 1,
                        last_active_round: current_round,
                        uptime_percentage_scaled: 10000, // 100%
                        recent_performance_scaled: 10000, // 100%
                        network_contribution_scaled: 5000, // 50%
                    }
                });
                (id, telemetry)
            })
            .collect()
    }

    /// Record a block proposal by a validator
    pub fn record_block_proposal(&self, validator_id: &[u8; 32]) -> Result<()> {
        let mut cache = self.cache.write();
        let current_round = *self.current_round.read();

        let telemetry = cache
            .entry(*validator_id)
            .or_insert_with(|| ValidatorTelemetry {
                validator_id: *validator_id,
                blocks_proposed: 0,
                blocks_verified: 0,
                rounds_active: 1,
                avg_latency_us: 100_000,
                slash_count: 0,
                stake: 0,
                age_rounds: 1,
                last_active_round: current_round,
                uptime_percentage_scaled: 10000,
                recent_performance_scaled: 10000,
                network_contribution_scaled: 5000,
            });

        telemetry.blocks_proposed += 1;
        telemetry.last_active_round = current_round;
        // recent_performance = old * 0.9 + 0.1 = (old * 9000 + 1000) / 10000
        telemetry.recent_performance_scaled = (telemetry.recent_performance_scaled * 9000 + 1000) / 10000;

        // Persist to storage
        self.storage
            .store_validator_telemetry(validator_id, telemetry)?;

        debug!(
            "Recorded block proposal for validator {}",
            hex::encode(validator_id)
        );
        Ok(())
    }

    /// Record a block verification by a validator
    pub fn record_block_verification(&self, validator_id: &[u8; 32]) -> Result<()> {
        let mut cache = self.cache.write();
        let current_round = *self.current_round.read();

        let telemetry = cache
            .entry(*validator_id)
            .or_insert_with(|| ValidatorTelemetry {
                validator_id: *validator_id,
                blocks_proposed: 0,
                blocks_verified: 0,
                rounds_active: 1,
                avg_latency_us: 100_000,
                slash_count: 0,
                stake: 0,
                age_rounds: 1,
                last_active_round: current_round,
                uptime_percentage_scaled: 10000,
                recent_performance_scaled: 10000,
                network_contribution_scaled: 5000,
            });

        telemetry.blocks_verified += 1;
        telemetry.last_active_round = current_round;

        // Persist to storage
        self.storage
            .store_validator_telemetry(validator_id, telemetry)?;

        Ok(())
    }

    /// Update round and calculate uptime metrics
    pub fn advance_round(&self) -> Result<()> {
        let mut current_round = self.current_round.write();
        *current_round += 1;
        let round = *current_round;

        // Update uptime metrics for all validators
        let mut cache = self.cache.write();
        for (validator_id, telemetry) in cache.iter_mut() {
            let rounds_since_active = round.saturating_sub(telemetry.last_active_round);
            telemetry.age_rounds = telemetry.age_rounds.saturating_add(1);

            if rounds_since_active > 0 {
                // Adjust uptime percentage based on inactivity (using integer math)
                // activity_rate = 10000 / (rounds_since_active + 1) scaled by 10000
                let activity_rate_scaled = (10000 / (rounds_since_active + 1)) as i64;
                // uptime = old_uptime * 0.95 + activity_rate * 0.05
                // = (old_uptime * 9500 + activity_rate_scaled * 5) / 10000
                telemetry.uptime_percentage_scaled = ((telemetry.uptime_percentage_scaled * 9500 + activity_rate_scaled * 5) / 10000).min(10000);

                // Decay recent performance: perf = perf * 0.9
                telemetry.recent_performance_scaled = ((telemetry.recent_performance_scaled * 9000) / 10000).max(0);
            }

            // Persist updates
            if let Err(e) = self
                .storage
                .store_validator_telemetry(validator_id, telemetry)
            {
                warn!(
                    "Failed to persist telemetry for validator {}: {}",
                    hex::encode(validator_id),
                    e
                );
            }
        }

        debug!(
            "Advanced to round {}, updated {} validator telemetry records",
            round,
            cache.len()
        );
        Ok(())
    }

    /// Update validator stake (called when stake changes)
    pub fn update_stake(&self, validator_id: &[u8; 32], new_stake: u64) -> Result<()> {
        let mut cache = self.cache.write();
        if let Some(telemetry) = cache.get_mut(validator_id) {
            telemetry.stake = new_stake;
            self.storage
                .store_validator_telemetry(validator_id, telemetry)?;
        }
        Ok(())
    }

    /// Record a slashing event
    pub fn record_slash(&self, validator_id: &[u8; 32]) -> Result<()> {
        let mut cache = self.cache.write();
        if let Some(telemetry) = cache.get_mut(validator_id) {
            telemetry.slash_count += 1;
            // Slash penalty: recent_performance = recent_performance * 0.5
            telemetry.recent_performance_scaled = ((telemetry.recent_performance_scaled * 5000) / 10000).max(0);
            self.storage
                .store_validator_telemetry(validator_id, telemetry)?;
            warn!(
                "Recorded slash for validator {}, total slashes: {}",
                hex::encode(validator_id),
                telemetry.slash_count
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ippan_storage::SledStorage;
    use tempfile::tempdir;

    #[test]
    fn test_telemetry_manager() {
        let dir = tempdir().unwrap();
        let storage = Arc::new(SledStorage::new(dir.path()).unwrap());
        let manager = TelemetryManager::new(storage);

        let validator_id = [1u8; 32];

        // Record proposal
        manager.record_block_proposal(&validator_id).unwrap();

        // Check telemetry
        let telemetry = manager.get_telemetry(&validator_id).unwrap();
        assert_eq!(telemetry.blocks_proposed, 1);
        assert_eq!(telemetry.blocks_verified, 0);

        // Record verification
        manager.record_block_verification(&validator_id).unwrap();
        let telemetry = manager.get_telemetry(&validator_id).unwrap();
        assert_eq!(telemetry.blocks_verified, 1);

        // Advance round
        manager.advance_round().unwrap();
        let round = *manager.current_round.read();
        assert_eq!(round, 1);
    }
}
