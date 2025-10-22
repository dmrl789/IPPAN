use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// DAG-Fair emission parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmissionParams {
    /// Base reward per round (in micro-IPN)
    pub r0: u128,
    /// Halving interval in rounds
    pub halving_rounds: u64,
    /// Proposer bonus percentage (0.0 to 1.0)
    pub proposer_bonus: f64,
    /// Verifier reward percentage (0.0 to 1.0)
    pub verifier_reward: f64,
    /// Minimum reward per round
    pub min_reward: u128,
}

impl Default for EmissionParams {
    fn default() -> Self {
        Self {
            r0: 10_000, // 0.00001 IPN per round
            halving_rounds: 2_102_400, // ~2 years at 1 round per 5 seconds
            proposer_bonus: 0.20, // 20%
            verifier_reward: 0.80, // 80%
            min_reward: 1, // 0.000000001 IPN minimum
        }
    }
}

/// Round context for emission calculation
#[derive(Debug, Clone)]
pub struct RoundContext {
    /// Round number
    pub round: u64,
    /// Blocks in this round
    pub blocks: Vec<BlockInfo>,
    /// Total stake in the system
    pub total_stake: u128,
    /// Validator stakes
    pub validator_stakes: HashMap<[u8; 32], u128>,
}

/// Block information for emission calculation
#[derive(Debug, Clone)]
pub struct BlockInfo {
    /// Block proposer
    pub proposer: [u8; 32],
    /// Block verifiers
    pub verifiers: Vec<[u8; 32]>,
    /// Block size (bytes)
    pub size: u64,
    /// Block timestamp
    pub timestamp: u64,
}

/// Emission distribution result
#[derive(Debug, Clone)]
pub struct EmissionDistribution {
    /// Total reward for this round
    pub total_reward: u128,
    /// Proposer rewards
    pub proposer_rewards: HashMap<[u8; 32], u128>,
    /// Verifier rewards
    pub verifier_rewards: HashMap<[u8; 32], u128>,
    /// Remaining reward (if any)
    pub remaining: u128,
}

/// DAG-Fair emission calculator
pub struct EmissionCalculator {
    params: EmissionParams,
}

impl EmissionCalculator {
    /// Create a new emission calculator
    pub fn new(params: EmissionParams) -> Self {
        Self { params }
    }

    /// Calculate the base reward for a round
    pub fn calculate_round_reward(&self, round: u64) -> u128 {
        let halvings = (round / self.params.halving_rounds) as u32;
        let reward = self.params.r0 >> halvings;
        reward.max(self.params.min_reward)
    }

    /// Distribute rewards for a round
    pub fn distribute_rewards(&self, context: &RoundContext) -> Result<EmissionDistribution> {
        let total_reward = self.calculate_round_reward(context.round);
        
        if context.blocks.is_empty() {
            return Ok(EmissionDistribution {
                total_reward,
                proposer_rewards: HashMap::new(),
                verifier_rewards: HashMap::new(),
                remaining: total_reward,
            });
        }

        let mut proposer_rewards = HashMap::new();
        let mut verifier_rewards = HashMap::new();
        let mut remaining = total_reward;

        // Calculate rewards per block
        let reward_per_block = total_reward / context.blocks.len() as u128;
        let proposer_reward_per_block = (reward_per_block as f64 * self.params.proposer_bonus) as u128;
        let verifier_reward_per_block = (reward_per_block as f64 * self.params.verifier_reward) as u128;

        for block in &context.blocks {
            // Proposer reward
            *proposer_rewards.entry(block.proposer).or_insert(0) += proposer_reward_per_block;
            remaining = remaining.saturating_sub(proposer_reward_per_block);

            // Verifier rewards (split equally among verifiers)
            if !block.verifiers.is_empty() {
                let verifier_reward_per_verifier = verifier_reward_per_block / block.verifiers.len() as u128;
                for verifier in &block.verifiers {
                    *verifier_rewards.entry(*verifier).or_insert(0) += verifier_reward_per_verifier;
                    remaining = remaining.saturating_sub(verifier_reward_per_verifier);
                }
            } else {
                // If no verifiers, add to remaining
                remaining = remaining.saturating_add(verifier_reward_per_block);
            }
        }

        Ok(EmissionDistribution {
            total_reward,
            proposer_rewards,
            verifier_rewards,
            remaining,
        })
    }

    /// Calculate total supply at a given round
    pub fn calculate_total_supply(&self, round: u64) -> u128 {
        let mut total_supply = 0u128;
        let mut current_round = 0u64;

        while current_round <= round {
            let reward = self.calculate_round_reward(current_round);
            total_supply = total_supply.saturating_add(reward);
            current_round += 1;
        }

        total_supply
    }

    /// Get emission parameters
    pub fn get_params(&self) -> &EmissionParams {
        &self.params
    }

    /// Update emission parameters
    pub fn update_params(&mut self, params: EmissionParams) {
        self.params = params;
    }
}

/// Convenience function to calculate round reward
pub fn round_reward(round: u64, params: &EmissionParams) -> u128 {
    let halvings = (round / params.halving_rounds) as u32;
    let reward = params.r0 >> halvings;
    reward.max(params.min_reward)
}

/// Convenience function to distribute rewards
pub fn distribute(round_ctx: &RoundContext, params: &EmissionParams) -> Result<EmissionDistribution> {
    let calculator = EmissionCalculator::new(params.clone());
    calculator.distribute_rewards(round_ctx)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_context() -> RoundContext {
        RoundContext {
            round: 100,
            blocks: vec![
                BlockInfo {
                    proposer: [1u8; 32],
                    verifiers: vec![[2u8; 32], [3u8; 32]],
                    size: 1000,
                    timestamp: 1234567890,
                },
                BlockInfo {
                    proposer: [2u8; 32],
                    verifiers: vec![[1u8; 32], [3u8; 32]],
                    size: 1200,
                    timestamp: 1234567891,
                },
            ],
            total_stake: 1_000_000,
            validator_stakes: HashMap::new(),
        }
    }

    #[test]
    fn test_round_reward_calculation() {
        let params = EmissionParams::default();
        let calculator = EmissionCalculator::new(params.clone());
        
        // Round 0 should give full reward
        assert_eq!(calculator.calculate_round_reward(0), params.r0);
        
        // Round before halving should give full reward
        assert_eq!(calculator.calculate_round_reward(params.halving_rounds - 1), params.r0);
        
        // Round after first halving should give half reward
        assert_eq!(calculator.calculate_round_reward(params.halving_rounds), params.r0 / 2);
        
        // Round after second halving should give quarter reward
        assert_eq!(calculator.calculate_round_reward(params.halving_rounds * 2), params.r0 / 4);
    }

    #[test]
    fn test_reward_distribution() {
        let params = EmissionParams::default();
        let calculator = EmissionCalculator::new(params);
        let context = create_test_context();
        
        let distribution = calculator.distribute_rewards(&context).unwrap();
        
        // Should have rewards for both proposers
        assert!(distribution.proposer_rewards.contains_key(&[1u8; 32]));
        assert!(distribution.proposer_rewards.contains_key(&[2u8; 32]));
        
        // Should have rewards for all verifiers
        assert!(distribution.verifier_rewards.contains_key(&[1u8; 32]));
        assert!(distribution.verifier_rewards.contains_key(&[2u8; 32]));
        assert!(distribution.verifier_rewards.contains_key(&[3u8; 32]));
        
        // Total distributed should not exceed total reward
        let total_distributed = distribution.proposer_rewards.values().sum::<u128>()
            + distribution.verifier_rewards.values().sum::<u128>();
        assert!(total_distributed <= distribution.total_reward);
    }

    #[test]
    fn test_empty_round() {
        let params = EmissionParams::default();
        let calculator = EmissionCalculator::new(params);
        let context = RoundContext {
            round: 100,
            blocks: vec![],
            total_stake: 1_000_000,
            validator_stakes: HashMap::new(),
        };
        
        let distribution = calculator.distribute_rewards(&context).unwrap();
        
        assert_eq!(distribution.proposer_rewards.len(), 0);
        assert_eq!(distribution.verifier_rewards.len(), 0);
        assert_eq!(distribution.remaining, distribution.total_reward);
    }

    #[test]
    fn test_total_supply_calculation() {
        let params = EmissionParams::default();
        let calculator = EmissionCalculator::new(params);
        
        // Total supply should be sum of all rewards up to the given round
        let total_supply = calculator.calculate_total_supply(10);
        let mut expected = 0u128;
        for round in 0..=10 {
            expected += calculator.calculate_round_reward(round);
        }
        assert_eq!(total_supply, expected);
    }
}