//! L1 AI-Integrated GBDT Consensus System
//!
//! This module implements AI-integrated GBDT (Gradient Boosting Decision Trees)
//! directly into the L1 consensus mechanism. L1 has NO smart contracts - only
//! pure consensus with AI optimization.

use crate::reputation::{ValidatorTelemetry, ReputationScore};
use ippan_ai_core::{GBDTModel, eval_gbdt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// L1 AI-Integrated Consensus Engine
/// 
/// This is the core AI system integrated into L1 consensus.
/// It uses GBDT models for:
/// - Validator selection optimization
/// - Fee calculation optimization  
/// - Network health monitoring
/// - Block ordering optimization
#[derive(Debug, Clone)]
pub struct L1AIConsensus {
    /// GBDT model for validator selection optimization
    pub validator_selection_model: Option<GBDTModel>,
    
    /// GBDT model for dynamic fee optimization
    pub fee_optimization_model: Option<GBDTModel>,
    
    /// GBDT model for network health monitoring
    pub network_health_model: Option<GBDTModel>,
    
    /// GBDT model for block ordering optimization
    pub block_ordering_model: Option<GBDTModel>,
    
    /// Configuration for AI consensus
    pub config: L1AIConfig,
}

/// Configuration for L1 AI consensus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L1AIConfig {
    /// Enable AI-optimized validator selection
    pub enable_validator_ai: bool,
    
    /// Enable AI-optimized fee calculation
    pub enable_fee_ai: bool,
    
    /// Enable AI network health monitoring
    pub enable_health_ai: bool,
    
    /// Enable AI-optimized block ordering
    pub enable_ordering_ai: bool,
    
    /// Minimum reputation score for AI selection
    pub min_reputation_score: i32,
    
    /// Maximum fee adjustment factor (e.g., 2.0 = 2x max)
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
    /// Current network congestion level (0.0 to 1.0)
    pub congestion_level: f64,
    
    /// Average block time in milliseconds
    pub avg_block_time_ms: f64,
    
    /// Number of active validators
    pub active_validators: usize,
    
    /// Total stake in the network
    pub total_stake: u64,
    
    /// Current round number
    pub current_round: u64,
    
    /// Recent transaction volume
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
    /// Create a new L1 AI consensus engine
    pub fn new(config: L1AIConfig) -> Self {
        Self {
            validator_selection_model: None,
            fee_optimization_model: None,
            network_health_model: None,
            block_ordering_model: None,
            config,
        }
    }
    
    /// Load GBDT models for AI consensus
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
    
    /// AI-optimized validator selection
    /// 
    /// Uses GBDT to select the best validator based on:
    /// - Reputation score
    /// - Stake weight
    /// - Recent performance
    /// - Network contribution
    /// - Uptime percentage
    pub fn select_validator(
        &self,
        candidates: &[ValidatorCandidate],
        network_state: &NetworkState,
    ) -> Result<ValidatorSelectionResult, String> {
        if !self.config.enable_validator_ai || self.validator_selection_model.is_none() {
            return self.fallback_validator_selection(candidates);
        }
        
        let model = self.validator_selection_model.as_ref().unwrap();
        
        // Extract features for each candidate
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
        
        // Select the highest scoring candidate
        scored_candidates.sort_by(|a, b| b.1.cmp(&a.1));
        let (best_candidate, best_score) = &scored_candidates[0];
        
        let confidence_score = (*best_score as f64) / 10000.0; // Normalize to 0-1
        
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
                "reputation_score".to_string(),
                "stake_weight".to_string(),
                "recent_performance".to_string(),
                "uptime_percentage".to_string(),
                "network_contribution".to_string(),
            ],
        })
    }
    
    /// AI-optimized fee calculation
    /// 
    /// Uses GBDT to dynamically adjust fees based on:
    /// - Network congestion
    /// - Transaction volume
    /// - Validator performance
    /// - Historical patterns
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
                optimization_reason: "AI disabled, using base fee".to_string(),
                confidence_score: 0.0,
            });
        }
        
        let model = self.fee_optimization_model.as_ref().unwrap();
        
        // Extract features for fee optimization
        let features = self.extract_fee_features(base_fee, tx_type, network_state);
        let adjustment_score = eval_gbdt(model, &features);
        
        // Convert score to adjustment factor (0.5 to 2.0 range)
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
    
    /// AI network health monitoring
    /// 
    /// Uses GBDT to monitor and predict network health issues
    pub fn monitor_network_health(
        &self,
        network_state: &NetworkState,
        validator_telemetry: &[ValidatorTelemetry],
    ) -> Result<NetworkHealthReport, String> {
        if !self.config.enable_health_ai || self.network_health_model.is_none() {
            return Ok(NetworkHealthReport::default());
        }
        
        let model = self.network_health_model.as_ref().unwrap();
        
        // Extract features for health monitoring
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
    
    /// Extract features for validator selection
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
    
    /// Extract features for fee optimization
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
            (base_fee as f64 / 1000.0) as i64, // Normalize base fee
            tx_type_encoding,
            (network_state.congestion_level * 10000.0) as i64,
            (network_state.avg_block_time_ms / 100.0) as i64, // Normalize block time
            network_state.active_validators as i64,
            (network_state.recent_tx_volume as f64 / 1000.0) as i64, // Normalize volume
            network_state.current_round as i64,
        ]
    }
    
    /// Extract features for health monitoring
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
    
    /// Calculate average validator performance
    fn calculate_avg_validator_performance(&self, telemetry: &[ValidatorTelemetry]) -> f64 {
        if telemetry.is_empty() {
            return 0.0;
        }
        
        // Calculate reputation score based on available telemetry fields
        let total_score: i32 = telemetry.iter().map(|t| {
            // Use a combination of blocks proposed, verified, and age as reputation score
            (t.blocks_proposed + t.blocks_verified + t.age_rounds) as i32
        }).sum();
        (total_score as f64) / (telemetry.len() as f64 * 10000.0)
    }
    
    /// Generate health recommendations
    fn generate_health_recommendations(
        &self,
        health_level: f64,
        network_state: &NetworkState,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        if health_level < 0.3 {
            recommendations.push("Critical: Network health is very low".to_string());
        } else if health_level < 0.6 {
            recommendations.push("Warning: Network health is below optimal".to_string());
        }
        
        if network_state.congestion_level > 0.8 {
            recommendations.push("High congestion detected - consider fee adjustment".to_string());
        }
        
        if network_state.avg_block_time_ms > 300.0 {
            recommendations.push("Slow block times - check validator performance".to_string());
        }
        
        if network_state.active_validators < 10 {
            recommendations.push("Low validator count - consider adding more validators".to_string());
        }
        
        if recommendations.is_empty() {
            recommendations.push("Network health is optimal".to_string());
        }
        
        recommendations
    }
    
    /// Fallback validator selection when AI is disabled
    fn fallback_validator_selection(
        &self,
        candidates: &[ValidatorCandidate],
    ) -> Result<ValidatorSelectionResult, String> {
        if candidates.is_empty() {
            return Err("No validator candidates available".to_string());
        }
        
        // Simple stake-weighted selection
        let total_stake: u64 = candidates.iter().map(|c| c.stake).sum();
        if total_stake == 0 {
            return Err("No stake available for selection".to_string());
        }
        
        // Select based on highest stake
        let best_candidate = candidates
            .iter()
            .max_by_key(|c| c.stake)
            .unwrap();
        
        Ok(ValidatorSelectionResult {
            selected_validator: best_candidate.id,
            confidence_score: 0.5, // Lower confidence for fallback
            selection_reason: "Fallback selection based on highest stake".to_string(),
            ai_features_used: vec!["stake_weight".to_string()],
        })
    }
}

/// Network health report from AI monitoring
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
    use ippan_ai_core::gbdt::{GBDTModel, Tree, Node};
    
    fn create_test_gbdt_model() -> GBDTModel {
        GBDTModel {
            trees: vec![Tree {
                nodes: vec![
                    Node::Internal {
                        feature: 0,
                        threshold: 5000,
                        left: 1,
                        right: 2,
                    },
                    Node::Leaf { value: 8000 },
                    Node::Leaf { value: 2000 },
                ],
            }],
            bias: 0,
            scale: 10000,
        }
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
        
        let network_state = NetworkState {
            congestion_level: 0.3,
            avg_block_time_ms: 200.0,
            active_validators: 2,
            total_stake: 3000,
            current_round: 100,
            recent_tx_volume: 1000,
        };
        
        let result = ai_consensus.select_validator(&candidates, &network_state).unwrap();
        assert_eq!(result.selected_validator, [2u8; 32]); // Highest stake
        assert_eq!(result.ai_features_used, vec!["stake_weight"]);
    }
    
    #[test]
    fn test_fee_optimization_fallback() {
        let config = L1AIConfig::default();
        let ai_consensus = L1AIConsensus::new(config);
        
        let network_state = NetworkState {
            congestion_level: 0.5,
            avg_block_time_ms: 250.0,
            active_validators: 5,
            total_stake: 10000,
            current_round: 200,
            recent_tx_volume: 5000,
        };
        
        let result = ai_consensus.optimize_fee(1000, "transfer", &network_state).unwrap();
        assert_eq!(result.base_fee, 1000);
        assert_eq!(result.ai_adjusted_fee, 1000);
        assert_eq!(result.adjustment_factor, 1.0);
    }
    
    #[test]
    fn test_network_health_monitoring() {
        let config = L1AIConfig::default();
        let ai_consensus = L1AIConsensus::new(config);
        
        let network_state = NetworkState {
            congestion_level: 0.2,
            avg_block_time_ms: 200.0,
            active_validators: 10,
            total_stake: 50000,
            current_round: 500,
            recent_tx_volume: 2000,
        };
        
        let telemetry = vec![
            ValidatorTelemetry {
                validator_id: [1u8; 32],
                reputation_score: 8000,
                uptime_percentage: 99.0,
                blocks_proposed: 100,
                blocks_verified: 200,
            },
        ];
        
        let report = ai_consensus.monitor_network_health(&network_state, &telemetry).unwrap();
        assert!(report.overall_health >= 0.0);
        assert!(report.overall_health <= 1.0);
        assert!(!report.recommendations.is_empty());
    }
}