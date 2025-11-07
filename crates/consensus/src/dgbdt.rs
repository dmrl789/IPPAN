//! Deterministic Gradient-Boosted Decision Tree (D-GBDT) Engine
//!
//! Provides deterministic, AI-driven fairness for validator selection
//! and reputation scoring in the DLC consensus model.

use anyhow::Result;
use blake3::Hasher as Blake3;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use ippan_types::{RoundId, ValidatorId};

/// Validator metrics used for D-GBDT scoring
/// All scores are in fixed-point format (scaled by 1_000_000) for determinism
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorMetrics {
    pub blocks_proposed: u64,
    pub blocks_verified: u64,
    pub rounds_active: u64,
    pub avg_latency_us: u64,
    /// Uptime as fixed-point: 1_000_000 = 100%, 990_000 = 99%
    pub uptime_percentage: i64,
    pub slash_count: u32,
    /// Recent performance as fixed-point: 1_000_000 = 100%, 950_000 = 95%
    pub recent_performance: i64,
    /// Network contribution as fixed-point: 1_000_000 = 100%, 850_000 = 85%
    pub network_contribution: i64,
    pub stake_amount: u64,
}

impl Default for ValidatorMetrics {
    fn default() -> Self {
        Self {
            blocks_proposed: 0,
            blocks_verified: 0,
            rounds_active: 0,
            avg_latency_us: 50_000,
            uptime_percentage: 1_000_000, // 100%
            slash_count: 0,
            recent_performance: 1_000_000, // 100%
            network_contribution: 1_000_000, // 100%
            stake_amount: 0,
        }
    }
}

/// Verifier selection result from D-GBDT
#[derive(Debug, Clone)]
pub struct VerifierSelection {
    pub primary: ValidatorId,
    pub shadows: Vec<ValidatorId>,
    pub selection_scores: HashMap<ValidatorId, i32>,
    pub selection_seed: [u8; 32],
}

/// D-GBDT Engine for deterministic validator selection
/// All weights are stored as fixed-point integers (scaled by 1_000_000)
pub struct DGBDTEngine {
    /// Model weights for different factors (scaled by 1_000_000)
    weights: HashMap<String, i64>,

    /// Historical performance data
    history: Vec<SelectionHistory>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct SelectionHistory {
    round: RoundId,
    selected: ValidatorId,
    score: i32,
}

impl DGBDTEngine {
    /// Create a new D-GBDT engine with default weights
    /// All weights are fixed-point scaled by 1_000_000
    pub fn new() -> Self {
        let mut weights = HashMap::new();

        // Initialize weights for fairness model (scaled by 1_000_000)
        weights.insert("blocks_proposed".to_string(), 250_000); // 0.25
        weights.insert("blocks_verified".to_string(), 200_000); // 0.20
        weights.insert("uptime".to_string(), 150_000); // 0.15
        weights.insert("latency".to_string(), 150_000); // 0.15
        weights.insert("slash_penalty".to_string(), 100_000); // 0.10
        weights.insert("performance".to_string(), 100_000); // 0.10
        weights.insert("stake".to_string(), 50_000); // 0.05

        Self {
            weights,
            history: Vec::new(),
        }
    }

    /// Calculate reputation score (0-10000) for a validator
    /// Uses only integer arithmetic for deterministic results across architectures
    pub fn calculate_reputation(&self, metrics: &ValidatorMetrics) -> i32 {
        // All calculations use fixed-point arithmetic (scaled by 1_000_000)
        let proposal_score = if metrics.rounds_active > 0 {
            ((metrics.blocks_proposed * 10000 / metrics.rounds_active).min(10000)) as i64
        } else {
            5000
        };

        let verification_score = if metrics.rounds_active > 0 {
            ((metrics.blocks_verified * 10000 / metrics.rounds_active).min(10000)) as i64
        } else {
            5000
        };

        // uptime_percentage is already scaled by 1_000_000, convert to 0-10000 scale
        let uptime_score = ((metrics.uptime_percentage * 10000) / 1_000_000).min(10000);

        // Latency score: lower is better
        let latency_score = {
            let clamped_latency = metrics.avg_latency_us.min(200_000);
            let normalized = ((200_000 - clamped_latency) * 10000) / 200_000;
            normalized.max(0) as i64
        };

        let slash_penalty = 10000 - ((metrics.slash_count as i64 * 1000).min(10000));
        
        // Performance and contribution are already scaled by 1_000_000
        let performance_score = ((metrics.recent_performance * 10000) / 1_000_000).min(10000);

        let stake_score = {
            let normalized = (metrics.stake_amount.min(100_000_000) * 10000) / 100_000_000;
            normalized as i64
        };

        // Weighted sum using fixed-point weights (scaled by 1_000_000)
        // Each score is 0-10000, weight is 0-1_000_000, so we divide by 1_000_000
        let weighted_score = 
            (proposal_score * self.weights.get("blocks_proposed").unwrap_or(&250_000)) / 1_000_000
            + (verification_score * self.weights.get("blocks_verified").unwrap_or(&200_000)) / 1_000_000
            + (uptime_score * self.weights.get("uptime").unwrap_or(&150_000)) / 1_000_000
            + (latency_score * self.weights.get("latency").unwrap_or(&150_000)) / 1_000_000
            + (slash_penalty * self.weights.get("slash_penalty").unwrap_or(&100_000)) / 1_000_000
            + (performance_score * self.weights.get("performance").unwrap_or(&100_000)) / 1_000_000
            + (stake_score * self.weights.get("stake").unwrap_or(&50_000)) / 1_000_000;

        (weighted_score as i32).clamp(0, 10000)
    }

    /// Select verifiers deterministically using round seed and metrics
    pub fn select_verifiers(
        &self,
        round_seed: RoundId,
        all_metrics: &HashMap<ValidatorId, ValidatorMetrics>,
        shadow_count: usize,
        min_reputation: i32,
    ) -> Result<VerifierSelection> {
        if all_metrics.is_empty() {
            return Err(anyhow::anyhow!("No validators available"));
        }

        // Calculate reputation scores for all validators
        let mut scores: HashMap<ValidatorId, i32> = all_metrics
            .iter()
            .map(|(id, metrics)| (*id, self.calculate_reputation(metrics)))
            .filter(|(_, score)| *score >= min_reputation)
            .collect();

        if scores.is_empty() {
            return Err(anyhow::anyhow!("No validators meet minimum reputation"));
        }

        // Generate deterministic selection seed from round
        let selection_seed = self.generate_selection_seed(round_seed);

        // Select primary verifier deterministically
        let primary = self.weighted_deterministic_selection(&scores, &selection_seed, 0)?;

        // Select shadow verifiers
        scores.remove(&primary); // Don't select same validator twice
        let mut shadows = Vec::new();

        for i in 0..shadow_count.min(scores.len()) {
            let shadow = self.weighted_deterministic_selection(&scores, &selection_seed, i + 1)?;
            shadows.push(shadow);
            scores.remove(&shadow);
        }

        Ok(VerifierSelection {
            primary,
            shadows,
            selection_scores: scores,
            selection_seed,
        })
    }

    /// Generate deterministic seed from round number
    fn generate_selection_seed(&self, round: RoundId) -> [u8; 32] {
        let mut hasher = Blake3::new();
        hasher.update(b"DLC_VERIFIER_SELECTION");
        hasher.update(&round.to_be_bytes());

        let hash = hasher.finalize();
        let mut seed = [0u8; 32];
        seed.copy_from_slice(hash.as_bytes());
        seed
    }

    /// Weighted deterministic selection using seed
    fn weighted_deterministic_selection(
        &self,
        scores: &HashMap<ValidatorId, i32>,
        seed: &[u8; 32],
        index: usize,
    ) -> Result<ValidatorId> {
        if scores.is_empty() {
            return Err(anyhow::anyhow!("No candidates available"));
        }

        // Create deterministic ordering based on seed and index
        let mut hasher = Blake3::new();
        hasher.update(seed);
        hasher.update(&index.to_be_bytes());
        let selection_hash = hasher.finalize();

        // Convert hash to selection value
        let mut selection_bytes = [0u8; 8];
        selection_bytes.copy_from_slice(&selection_hash.as_bytes()[..8]);
        let selection_value = u64::from_be_bytes(selection_bytes);

        // Calculate total weighted score
        let total_score: i64 = scores.values().map(|&s| s as i64).sum();

        if total_score == 0 {
            // Fallback to first validator if all scores are 0
            return Ok(*scores.keys().next().unwrap());
        }

        // Weighted random selection using the deterministic value
        let target = (selection_value % total_score as u64) as i64;
        let mut cumulative = 0i64;

        let mut ordered: Vec<(ValidatorId, i32)> =
            scores.iter().map(|(id, score)| (*id, *score)).collect();
        ordered.sort_by(|(a, _), (b, _)| a.cmp(b));

        for (validator_id, score) in &ordered {
            cumulative += *score as i64;
            if target < cumulative {
                return Ok(*validator_id);
            }
        }

        // Fallback (shouldn't reach here)
        Ok(ordered
            .last()
            .map(|(id, _)| *id)
            .unwrap_or_else(|| scores.keys().next().copied().unwrap()))
    }

    /// Record selection in history for learning
    pub fn record_selection(&mut self, round: RoundId, selected: ValidatorId, score: i32) {
        self.history.push(SelectionHistory {
            round,
            selected,
            score,
        });

        // Keep history bounded
        if self.history.len() > 10_000 {
            self.history.drain(0..1_000);
        }
    }

    /// Update model weights (for adaptive learning)
    /// new_weight should be scaled by 1_000_000 (e.g., 500_000 = 0.5)
    pub fn update_weights(&mut self, factor: &str, new_weight: i64) {
        if let Some(weight) = self.weights.get_mut(factor) {
            *weight = new_weight.clamp(0, 1_000_000);
        }
    }
}

impl Default for DGBDTEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reputation_calculation() {
        let engine = DGBDTEngine::new();
        let metrics = ValidatorMetrics {
            blocks_proposed: 100,
            blocks_verified: 200,
            rounds_active: 100,
            avg_latency_us: 50_000,
            uptime_percentage: 990_000, // 99% in fixed-point
            slash_count: 0,
            recent_performance: 950_000, // 95% in fixed-point
            network_contribution: 900_000, // 90% in fixed-point
            stake_amount: 10_000_000,
        };

        let score = engine.calculate_reputation(&metrics);
        assert!((0..=10000).contains(&score));
        assert!(score > 8000); // Should be high for good metrics
    }

    #[test]
    fn test_deterministic_selection() {
        let engine = DGBDTEngine::new();
        let mut metrics = HashMap::new();

        for i in 0..5 {
            let mut id = [0u8; 32];
            id[0] = i;
            metrics.insert(id, ValidatorMetrics::default());
        }

        let result1 = engine.select_verifiers(1, &metrics, 3, 0).unwrap();
        let result2 = engine.select_verifiers(1, &metrics, 3, 0).unwrap();

        // Same round should give same result (deterministic)
        assert_eq!(result1.primary, result2.primary);
        assert_eq!(result1.shadows, result2.shadows);
    }

    #[test]
    fn test_selection_seed_generation() {
        let engine = DGBDTEngine::new();
        let seed1 = engine.generate_selection_seed(1);
        let seed2 = engine.generate_selection_seed(1);
        let seed3 = engine.generate_selection_seed(2);

        assert_eq!(seed1, seed2); // Same round = same seed
        assert_ne!(seed1, seed3); // Different round = different seed
    }
}
