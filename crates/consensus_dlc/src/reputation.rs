//! Reputation tracking and management for validators
//! 
//! This module tracks validator reputation based on their behavior
//! and performance in the consensus process.

use crate::error::{DlcError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Reputation score for a validator
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ReputationScore {
    /// Total reputation points
    pub total: i64,
    /// Positive actions counter
    pub positive_actions: u64,
    /// Negative actions counter
    pub negative_actions: u64,
    /// Last update timestamp
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl Default for ReputationScore {
    fn default() -> Self {
        Self {
            total: 10000, // Start with neutral reputation
            positive_actions: 0,
            negative_actions: 0,
            last_updated: chrono::Utc::now(),
        }
    }
}

impl ReputationScore {
    /// Create a new reputation score
    pub fn new(initial_score: i64) -> Self {
        Self {
            total: initial_score,
            positive_actions: 0,
            negative_actions: 0,
            last_updated: chrono::Utc::now(),
        }
    }

    /// Apply a reputation delta
    pub fn apply_delta(&mut self, delta: i64) {
        self.total = (self.total + delta).max(0).min(100_000); // Clamp between 0 and 100k
        
        if delta > 0 {
            self.positive_actions += 1;
        } else if delta < 0 {
            self.negative_actions += 1;
        }
        
        self.last_updated = chrono::Utc::now();
    }

    /// Get normalized reputation (0.0 to 1.0)
    pub fn normalized(&self) -> f64 {
        (self.total as f64) / 100_000.0
    }

    /// Check if reputation is good standing (above threshold)
    pub fn is_good_standing(&self, threshold: i64) -> bool {
        self.total >= threshold
    }

    /// Calculate reputation trend (positive/negative ratio)
    pub fn trend(&self) -> f64 {
        let total_actions = self.positive_actions + self.negative_actions;
        if total_actions == 0 {
            return 0.0;
        }
        
        (self.positive_actions as f64) / (total_actions as f64)
    }
}

/// Reputation database for all validators
#[derive(Default, Debug, Serialize, Deserialize)]
pub struct ReputationDB {
    /// Reputation scores indexed by validator ID
    pub scores: HashMap<String, ReputationScore>,
    /// Reputation change history
    history: Vec<ReputationEvent>,
    /// Configuration
    config: ReputationConfig,
}

/// Configuration for reputation system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationConfig {
    /// Points for successfully proposing a block
    pub block_proposal_reward: i64,
    /// Points for successfully verifying a block
    pub block_verification_reward: i64,
    /// Penalty for missing block proposal
    pub missed_proposal_penalty: i64,
    /// Penalty for invalid block proposal
    pub invalid_proposal_penalty: i64,
    /// Penalty for downtime
    pub downtime_penalty: i64,
    /// Minimum reputation to remain active validator
    pub min_active_reputation: i64,
    /// Maximum reputation cap
    pub max_reputation: i64,
}

impl Default for ReputationConfig {
    fn default() -> Self {
        Self {
            block_proposal_reward: 100,
            block_verification_reward: 50,
            missed_proposal_penalty: -200,
            invalid_proposal_penalty: -500,
            downtime_penalty: -100,
            min_active_reputation: 5000,
            max_reputation: 100_000,
        }
    }
}

/// Reputation event for audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationEvent {
    /// Validator ID
    pub validator_id: String,
    /// Reputation delta applied
    pub delta: i64,
    /// Reason for change
    pub reason: String,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Round number
    pub round: u64,
}

impl ReputationDB {
    /// Create a new reputation database
    pub fn new(config: ReputationConfig) -> Self {
        Self {
            scores: HashMap::new(),
            history: Vec::new(),
            config,
        }
    }

    /// Get reputation score for a validator
    pub fn score(&self, validator_id: &str) -> i64 {
        self.scores
            .get(validator_id)
            .map(|s| s.total)
            .unwrap_or(0)
    }

    /// Get full reputation info for a validator
    pub fn get(&self, validator_id: &str) -> Option<&ReputationScore> {
        self.scores.get(validator_id)
    }

    /// Initialize a new validator with default reputation
    pub fn initialize_validator(&mut self, validator_id: String) -> Result<()> {
        if self.scores.contains_key(&validator_id) {
            return Err(DlcError::ReputationUpdate(format!(
                "Validator {} already has reputation",
                validator_id
            )));
        }

        self.scores
            .insert(validator_id, ReputationScore::default());
        Ok(())
    }

    /// Update reputation with a delta
    pub fn update(
        &mut self,
        validator_id: &str,
        delta: i64,
        reason: String,
        round: u64,
    ) -> Result<()> {
        let score = self
            .scores
            .entry(validator_id.to_string())
            .or_insert_with(ReputationScore::default);

        score.apply_delta(delta);

        // Record event
        self.history.push(ReputationEvent {
            validator_id: validator_id.to_string(),
            delta,
            reason,
            timestamp: chrono::Utc::now(),
            round,
        });

        tracing::debug!(
            "Updated reputation for {}: delta={}, new_total={}",
            validator_id,
            delta,
            score.total
        );

        Ok(())
    }

    /// Reward validator for proposing a block
    pub fn reward_proposal(&mut self, validator_id: &str, round: u64) -> Result<()> {
        self.update(
            validator_id,
            self.config.block_proposal_reward,
            "Block proposal".to_string(),
            round,
        )
    }

    /// Reward validator for verifying a block
    pub fn reward_verification(&mut self, validator_id: &str, round: u64) -> Result<()> {
        self.update(
            validator_id,
            self.config.block_verification_reward,
            "Block verification".to_string(),
            round,
        )
    }

    /// Penalize validator for missing proposal
    pub fn penalize_missed_proposal(&mut self, validator_id: &str, round: u64) -> Result<()> {
        self.update(
            validator_id,
            self.config.missed_proposal_penalty,
            "Missed proposal".to_string(),
            round,
        )
    }

    /// Penalize validator for invalid proposal
    pub fn penalize_invalid_proposal(&mut self, validator_id: &str, round: u64) -> Result<()> {
        self.update(
            validator_id,
            self.config.invalid_proposal_penalty,
            "Invalid proposal".to_string(),
            round,
        )
    }

    /// Penalize validator for downtime
    pub fn penalize_downtime(&mut self, validator_id: &str, round: u64) -> Result<()> {
        self.update(
            validator_id,
            self.config.downtime_penalty,
            "Downtime".to_string(),
            round,
        )
    }

    /// Check if validator has sufficient reputation to participate
    pub fn can_participate(&self, validator_id: &str) -> bool {
        self.score(validator_id) >= self.config.min_active_reputation
    }

    /// Get validators in good standing
    pub fn get_active_validators(&self) -> Vec<String> {
        self.scores
            .iter()
            .filter(|(_, score)| score.total >= self.config.min_active_reputation)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get reputation history for a validator
    pub fn get_history(&self, validator_id: &str, limit: usize) -> Vec<ReputationEvent> {
        self.history
            .iter()
            .filter(|e| e.validator_id == validator_id)
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get recent reputation changes
    pub fn recent_changes(&self, limit: usize) -> Vec<ReputationEvent> {
        self.history
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get reputation statistics
    pub fn stats(&self) -> ReputationStats {
        let scores: Vec<i64> = self.scores.values().map(|s| s.total).collect();
        
        let total = scores.len();
        let avg = if total > 0 {
            scores.iter().sum::<i64>() / total as i64
        } else {
            0
        };
        
        let min = scores.iter().min().copied().unwrap_or(0);
        let max = scores.iter().max().copied().unwrap_or(0);
        
        let active = self.get_active_validators().len();

        ReputationStats {
            total_validators: total,
            active_validators: active,
            average_reputation: avg,
            min_reputation: min,
            max_reputation: max,
            total_events: self.history.len(),
        }
    }

    /// Clear history older than specified rounds
    pub fn prune_history(&mut self, keep_rounds: u64, current_round: u64) {
        if current_round <= keep_rounds {
            return;
        }

        let cutoff_round = current_round - keep_rounds;
        self.history.retain(|event| event.round > cutoff_round);
        
        tracing::debug!("Pruned reputation history, kept {} events", self.history.len());
    }

    /// Reset reputation for a validator (admin function)
    pub fn reset_validator(&mut self, validator_id: &str, new_score: i64) -> Result<()> {
        let score = self
            .scores
            .get_mut(validator_id)
            .ok_or_else(|| DlcError::ValidatorNotFound(validator_id.to_string()))?;

        score.total = new_score;
        score.last_updated = chrono::Utc::now();

        Ok(())
    }
}

/// Reputation statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationStats {
    pub total_validators: usize,
    pub active_validators: usize,
    pub average_reputation: i64,
    pub min_reputation: i64,
    pub max_reputation: i64,
    pub total_events: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reputation_score() {
        let mut score = ReputationScore::default();
        assert_eq!(score.total, 10000);

        score.apply_delta(500);
        assert_eq!(score.total, 10500);
        assert_eq!(score.positive_actions, 1);

        score.apply_delta(-200);
        assert_eq!(score.total, 10300);
        assert_eq!(score.negative_actions, 1);
    }

    #[test]
    fn test_reputation_normalization() {
        let score = ReputationScore::new(50000);
        assert_eq!(score.normalized(), 0.5);
    }

    #[test]
    fn test_reputation_db() {
        let mut db = ReputationDB::default();
        
        db.initialize_validator("val1".to_string()).unwrap();
        assert_eq!(db.score("val1"), 10000);

        db.update("val1", 100, "Test".to_string(), 1).unwrap();
        assert_eq!(db.score("val1"), 10100);
    }

    #[test]
    fn test_reputation_rewards() {
        let mut db = ReputationDB::default();
        db.initialize_validator("val1".to_string()).unwrap();

        db.reward_proposal("val1", 1).unwrap();
        assert_eq!(db.score("val1"), 10100);

        db.reward_verification("val1", 1).unwrap();
        assert_eq!(db.score("val1"), 10150);
    }

    #[test]
    fn test_reputation_penalties() {
        let mut db = ReputationDB::default();
        db.initialize_validator("val1".to_string()).unwrap();

        db.penalize_missed_proposal("val1", 1).unwrap();
        assert_eq!(db.score("val1"), 9800);

        db.penalize_invalid_proposal("val1", 1).unwrap();
        assert_eq!(db.score("val1"), 9300);
    }

    #[test]
    fn test_active_validators() {
        let mut db = ReputationDB::default();
        
        db.initialize_validator("val1".to_string()).unwrap();
        db.initialize_validator("val2".to_string()).unwrap();
        
        db.update("val2", -6000, "Test".to_string(), 1).unwrap();

        let active = db.get_active_validators();
        assert_eq!(active.len(), 1);
        assert!(active.contains(&"val1".to_string()));
    }

    #[test]
    fn test_reputation_history() {
        let mut db = ReputationDB::default();
        db.initialize_validator("val1".to_string()).unwrap();

        db.update("val1", 100, "Event 1".to_string(), 1).unwrap();
        db.update("val1", 200, "Event 2".to_string(), 2).unwrap();

        let history = db.get_history("val1", 10);
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn test_reputation_stats() {
        let mut db = ReputationDB::default();
        
        db.initialize_validator("val1".to_string()).unwrap();
        db.initialize_validator("val2".to_string()).unwrap();
        db.update("val1", 500, "Test".to_string(), 1).unwrap();

        let stats = db.stats();
        assert_eq!(stats.total_validators, 2);
        assert!(stats.average_reputation > 0);
    }

    #[test]
    fn test_reputation_clamping() {
        let mut score = ReputationScore::new(99000);
        score.apply_delta(5000);
        assert_eq!(score.total, 100_000); // Clamped to max

        let mut score2 = ReputationScore::new(100);
        score2.apply_delta(-500);
        assert_eq!(score2.total, 0); // Clamped to min
    }
}
