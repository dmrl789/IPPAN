//! L1 AI-Integrated GBDT Consensus System
//!
//! This module implements AI-integrated GBDT (Gradient Boosting Decision Trees)
//! directly into the L1 consensus mechanism. L1 has NO smart contracts—only
//! deterministic consensus with AI-driven optimization.

use crate::reputation::ValidatorTelemetry;
use ippan_ai_core::{eval_gbdt, gbdt::GBDTModel};
use serde::{Deserialize, Serialize};
use tracing::info;

/// L1 AI-Integrated Consensus Engine
///
/// Uses GBDT models for:
/// - Validator selection optimization  
/// - Fee calculation optimization  
/// - Network health monitoring  
/// - Block ordering optimization
#[derive(Debug, Clone)]
pub struct L1AIConsensus {
    pub validator_selection_model: Option<GBDTModel>,
    pub fee_optimization_model: Option<GBDTModel>,
    pub network_health_model: Option<GBDTModel>,
    pub block_ordering_model: Option<GBDTModel>,
    pub config: L1AIConfig,
}

/// Configuration for L1 AI consensus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L1AIConfig {
    pub enable_validator_ai: bool,
    pub enable_fee_ai: bool,
    pub enable_health_ai: bool,
    pub enable_ordering_ai: bool,
    pub min_reputation_score: i32,
    pub max_fee_adjustment: f64,
}

impl Default for L1AIConfig {
    fn default() -> Self {
        Self {
            enable_validator_ai: true,
            enable_fee_ai: true,
            enable_health_ai: true,
            enable_ordering_ai: true,
            min_reputation_score: 5000, // 50% minimum reputation
            max_fee_adjustment: 2.0,    // Max 2x fee adjustment
        }
    }
}

/// Network state for AI optimization
#[derive(Debug, Clone)]
pub struct NetworkState {
    pub congestion_level: f64,
    pub avg_block_time_ms: f64,
    pub active_validators: usize,
    pub total_stake: u64,
    pub current_round: u64,
    pub recent_tx_volume: u64,
}

/// Validator candidate for AI selection
#[derive(Debug, Clone)]
pub struct ValidatorCandidate {
    pub id: [u8; 32],
    pub stake: u64,
    pub reputation_score: i32,
    pub uptime_percentage: f64,
    pub recent_performance: f64,
    pub network_contribution: f64,
}

/// AI-optimized validator selection result
#[derive(Debug, Clone)]
pub struct ValidatorSelectionResult {
    pub selected_validator: [u8; 32],
    pub confidence_score: f64,
    pub selection_reason: String,
    pub ai_features_used: Vec<String>,
}

/// AI-optimized fee calculation result
#[derive(Debug, Clone)]
pub struct FeeOptimizationResult {
    pub base_fee: u64,
    pub ai_adjusted_fee: u64,
    pub adjustment_factor: f64,
    pub optimization_reason: String,
    pub confidence_score: f64,
}

impl L1AIConsensus {
    pub fn new(config: L1AIConfig) -> Self {
        Self {
            validator_selection_model: None,
            fee_optimization_model: None,
            network_health_model: None,
            block_ordering_model: None,
            config,
        }
    }

    pub fn load_models(
        &mut self,
        validator_model: Option<GBDTModel>,
        fee_model: Option<GBDTModel>,
        health_model: Option<GBDTModel>,
        ordering_model: Option<GBDTModel>,
    ) -> Result<(), String> {
        self.validator_selection_model = validator_model;
        self.fee_optimization_model = fee_model;
        self.network_health_model = health_model;
        self.block_ordering_model = ordering_model;

        info!("L1 AI consensus models loaded successfully");
        Ok(())
    }

    /// AI-driven validator selection
    pub fn select_validator(
        &self,
        candidates: &[ValidatorCandidate],
        network_state: &NetworkState,
    ) -> Result<ValidatorSelectionResult, String> {
        if !self.config.enable_validator_ai || self.validator_selection_model.is_none() {
            return self.fallback_validator_selection(candidates);
        }

        let model = self.validator_selection_model.as_ref().unwrap();
        let mut scored_candidates = Vec::new();

        for candidate in candidates {
            let features = self.extract_validator_features(candidate, network_state);
            let score = eval_gbdt(model, &features);
            if score >= self.config.min_reputation_score {
                scored_candidates.push((candidate.clone(), score));
            }
        }

        if scored_candidates.is_empty() {
            return self.fallback_validator_selection(candidates);
        }

        scored_candidates.sort_by(|a, b| b.1.cmp(&a.1));
        let (best_candidate, best_score) = &scored_candidates[0];
        let confidence_score = (*best_score as f64) / 10000.0;

        Ok(ValidatorSelectionResult {
            selected_validator: best_candidate.id,
            confidence_score,
            selection_reason: format!(
                "AI-selected based on reputation: {}, performance: {:.2}, uptime: {:.2}%",
                best_candidate.reputation_score,
                best_candidate.recent_performance,
                best_candidate.uptime_percentage
            ),
            ai_features_used: vec![
                "reputation_score".into(),
                "stake_weight".into(),
                "recent_performance".into(),
                "uptime_percentage".into(),
                "network_contribution".into(),
            ],
        })
    }

    /// AI-based dynamic fee optimization
    pub fn optimize_fee(
        &self,
        base_fee: u64,
        tx_type: &str,
        network_state: &NetworkState,
    ) -> Result<FeeOptimizationResult, String> {
        if !self.config.enable_fee_ai || self.fee_optimization_model.is_none() {
            return Ok(FeeOptimizationResult {
                base_fee,
                ai_adjusted_fee: base_fee,
                adjustment_factor: 1.0,
                optimization_reason: "AI disabled, using base fee".into(),
                confidence_score: 0.0,
            });
        }

        let model = self.fee_optimization_model.as_ref().unwrap();
        let features = self.extract_fee_features(base_fee, tx_type, network_state);
        let adjustment_score = eval_gbdt(model, &features);

        let adjustment_factor = (adjustment_score as f64 / 10000.0) * 1.5 + 0.5;
        let clamped_factor = adjustment_factor.clamp(0.5, self.config.max_fee_adjustment);
        let ai_adjusted_fee = (base_fee as f64 * clamped_factor) as u64;
        let confidence_score = (adjustment_score as f64) / 10000.0;

        Ok(FeeOptimizationResult {
            base_fee,
            ai_adjusted_fee,
            adjustment_factor: clamped_factor,
            optimization_reason: format!(
                "AI-adjusted based on congestion: {:.2}, volume: {}, validators: {}",
                network_state.congestion_level,
                network_state.recent_tx_volume,
                network_state.active_validators
            ),
            confidence_score,
        })
    }

    /// AI-driven network health monitoring
    pub fn monitor_network_health(
        &self,
        network_state: &NetworkState,
        validator_telemetry: &[ValidatorTelemetry],
    ) -> Result<NetworkHealthReport, String> {
        if !self.config.enable_health_ai || self.network_health_model.is_none() {
            return Ok(NetworkHealthReport::default());
        }

        let model = self.network_health_model.as_ref().unwrap();
        let features = self.extract_health_features(network_state, validator_telemetry);
        let health_score = eval_gbdt(model, &features);
        let health_level = (health_score as f64) / 10000.0;

        Ok(NetworkHealthReport {
            overall_health: health_level,
            congestion_level: network_state.congestion_level,
            validator_performance: self.calculate_avg_validator_performance(validator_telemetry),
            recommendations: self.generate_health_recommendations(health_level, network_state),
            confidence_score: health_level,
        })
    }

    /// --- Feature Extraction Helpers ---
    fn extract_validator_features(
        &self,
        candidate: &ValidatorCandidate,
        network_state: &NetworkState,
    ) -> Vec<i64> {
        vec![
            candidate.reputation_score as i64,
            (candidate.stake as f64 / network_state.total_stake as f64 * 10000.0) as i64,
            (candidate.uptime_percentage * 100.0) as i64,
            (candidate.recent_performance * 10000.0) as i64,
            (candidate.network_contribution * 10000.0) as i64,
            (network_state.congestion_level * 10000.0) as i64,
            network_state.active_validators as i64,
        ]
    }

    fn extract_fee_features(
        &self,
        base_fee: u64,
        tx_type: &str,
        network_state: &NetworkState,
    ) -> Vec<i64> {
        let tx_type_encoding = match tx_type {
            "transfer" => 0,
            "l2_anchor" => 1,
            "l2_exit" => 2,
            "governance" => 3,
            "validator" => 4,
            _ => 0,
        };

        vec![
            (base_fee as f64 / 1000.0) as i64,
            tx_type_encoding,
            (network_state.congestion_level * 10000.0) as i64,
            (network_state.avg_block_time_ms / 100.0) as i64,
            network_state.active_validators as i64,
            (network_state.recent_tx_volume as f64 / 1000.0) as i64,
            network_state.current_round as i64,
        ]
    }

    fn extract_health_features(
        &self,
        network_state: &NetworkState,
        validator_telemetry: &[ValidatorTelemetry],
    ) -> Vec<i64> {
        let avg_performance = self.calculate_avg_validator_performance(validator_telemetry);
        vec![
            (network_state.congestion_level * 10000.0) as i64,
            (network_state.avg_block_time_ms / 100.0) as i64,
            network_state.active_validators as i64,
            (network_state.recent_tx_volume as f64 / 1000.0) as i64,
            (avg_performance * 10000.0) as i64,
            network_state.current_round as i64,
        ]
    }

    fn calculate_avg_validator_performance(&self, telemetry: &[ValidatorTelemetry]) -> f64 {
        if telemetry.is_empty() {
            return 0.0;
        }

        let total_score: i32 = telemetry
            .iter()
            .map(|t| (t.blocks_proposed + t.blocks_verified + t.age_rounds) as i32)
            .sum();

        (total_score as f64) / (telemetry.len() as f64 * 10000.0)
    }

    fn generate_health_recommendations(
        &self,
        health_level: f64,
        network_state: &NetworkState,
    ) -> Vec<String> {
        let mut recs = Vec::new();

        if health_level < 0.3 {
            recs.push("Critical: Network health is very low".into());
        } else if health_level < 0.6 {
            recs.push("Warning: Network health below optimal".into());
        }

        if network_state.congestion_level > 0.8 {
            recs.push("High congestion detected – consider fee adjustment".into());
        }
        if network_state.avg_block_time_ms > 300.0 {
            recs.push("Slow block times – check validator performance".into());
        }
        if network_state.active_validators < 10 {
            recs.push("Low validator count – add more validators".into());
        }

        if recs.is_empty() {
            recs.push("Network health is optimal".into());
        }
        recs
    }

    fn fallback_validator_selection(
        &self,
        candidates: &[ValidatorCandidate],
    ) -> Result<ValidatorSelectionResult, String> {
        if candidates.is_empty() {
            return Err("No validator candidates available".into());
        }

        let total_stake: u64 = candidates.iter().map(|c| c.stake).sum();
        if total_stake == 0 {
            return Err("No stake available for selection".into());
        }

        let best_candidate = candidates.iter().max_by_key(|c| c.stake).unwrap();
        Ok(ValidatorSelectionResult {
            selected_validator: best_candidate.id,
            confidence_score: 0.5,
            selection_reason: "Fallback selection based on highest stake".into(),
            ai_features_used: vec!["stake_weight".into()],
        })
    }
}

/// Network health report structure
#[derive(Debug, Clone, Default)]
pub struct NetworkHealthReport {
    pub overall_health: f64,
    pub congestion_level: f64,
    pub validator_performance: f64,
    pub recommendations: Vec<String>,
    pub confidence_score: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ippan_ai_core::gbdt::{GBDTModel, Node, Tree};

    fn create_test_gbdt_model() -> GBDTModel {
        GBDTModel::new(
            vec![Tree {
                nodes: vec![
                    Node {
                        feature_index: 0,
                        threshold: 5000,
                        left: 1,
                        right: 2,
                        value: None,
                    },
                    Node {
                        feature_index: 0,
                        threshold: 0,
                        left: 0,
                        right: 0,
                        value: Some(8000),
                    },
                    Node {
                        feature_index: 0,
                        threshold: 0,
                        left: 0,
                        right: 0,
                        value: Some(2000),
                    },
                ],
            }],
            0,
            10000,
            1,
        )
        .expect("valid test model")
    }

    #[test]
    fn test_l1_ai_consensus_creation() {
        let config = L1AIConfig::default();
        let ai_consensus = L1AIConsensus::new(config);
        assert!(ai_consensus.validator_selection_model.is_none());
        assert!(ai_consensus.fee_optimization_model.is_none());
        assert!(ai_consensus.network_health_model.is_none());
        assert!(ai_consensus.block_ordering_model.is_none());
    }

    #[test]
    fn test_validator_selection_fallback() {
        let config = L1AIConfig::default();
        let ai_consensus = L1AIConsensus::new(config);

        let candidates = vec![
            ValidatorCandidate {
                id: [1u8; 32],
                stake: 1000,
                reputation_score: 8000,
                uptime_percentage: 99.5,
                recent_performance: 0.9,
                network_contribution: 0.8,
            },
            ValidatorCandidate {
                id: [2u8; 32],
                stake: 2000,
                reputation_score: 7000,
                uptime_percentage: 98.0,
                recent_performance: 0.8,
                network_contribution: 0.7,
            },
        ];

        let state = NetworkState {
            congestion_level: 0.3,
            avg_block_time_ms: 200.0,
            active_validators: 2,
            total_stake: 3000,
            current_round: 100,
            recent_tx_volume: 1000,
        };

        let result = ai_consensus.select_validator(&candidates, &state).unwrap();
        assert_eq!(result.selected_validator, [2u8; 32]);
        assert_eq!(result.ai_features_used, vec!["stake_weight"]);
    }

    #[test]
    fn test_fee_optimization_fallback() {
        let config = L1AIConfig::default();
        let ai_consensus = L1AIConsensus::new(config);

        let state = NetworkState {
            congestion_level: 0.5,
            avg_block_time_ms: 250.0,
            active_validators: 5,
            total_stake: 10000,
            current_round: 200,
            recent_tx_volume: 5000,
        };

        let result = ai_consensus.optimize_fee(1000, "transfer", &state).unwrap();
        assert_eq!(result.base_fee, 1000);
        assert_eq!(result.ai_adjusted_fee, 1000);
        assert_eq!(result.adjustment_factor, 1.0);
    }

    #[test]
    fn test_network_health_monitoring() {
        let config = L1AIConfig::default();
        let ai_consensus = L1AIConsensus::new(config);

        let state = NetworkState {
            congestion_level: 0.2,
            avg_block_time_ms: 200.0,
            active_validators: 10,
            total_stake: 50000,
            current_round: 500,
            recent_tx_volume: 2000,
        };

        let telemetry = vec![ValidatorTelemetry {
            blocks_proposed: 100,
            blocks_verified: 200,
            rounds_active: 500,
            avg_latency_us: 150_000,
            slash_count: 0,
            stake: 1_000_000,
            age_rounds: 10_000,
        }];

        let report = ai_consensus
            .monitor_network_health(&state, &telemetry)
            .unwrap();
        assert!(report.overall_health >= 0.0);
        assert!(report.overall_health <= 1.0);
        assert!(!report.recommendations.is_empty());
    }
}
