//! Economics parameter management and governance integration

use crate::types::EconomicsParams;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Economics parameter update proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicsParameterProposal {
    pub proposal_id: String,
    pub parameter_name: String,
    pub new_value: serde_json::Value,
    pub current_value: serde_json::Value,
    pub justification: String,
    pub proposer: [u8; 32],
    pub created_at: u64,
    pub voting_deadline: u64,
}

/// Economics parameter manager for governance control
pub struct EconomicsParameterManager {
    current_params: EconomicsParams,
    pending_proposals: HashMap<String, EconomicsParameterProposal>,
    parameter_history: Vec<EconomicsParameterProposal>,
}

impl EconomicsParameterManager {
    /// Create a new parameter manager with default parameters
    pub fn new() -> Self {
        Self {
            current_params: EconomicsParams::default(),
            pending_proposals: HashMap::new(),
            parameter_history: Vec::new(),
        }
    }

    /// Create a new parameter manager with custom parameters
    pub fn with_params(params: EconomicsParams) -> Self {
        Self {
            current_params: params,
            pending_proposals: HashMap::new(),
            parameter_history: Vec::new(),
        }
    }

    /// Get current economics parameters
    pub fn get_current_params(&self) -> &EconomicsParams {
        &self.current_params
    }

    /// Submit a parameter change proposal
    pub fn submit_parameter_proposal(
        &mut self,
        proposal: EconomicsParameterProposal,
    ) -> Result<()> {
        // Validate parameter name
        self.validate_parameter_name(&proposal.parameter_name)?;

        // Validate new value
        self.validate_parameter_value(&proposal.parameter_name, &proposal.new_value)?;

        // Check for duplicate proposal ID
        if self.pending_proposals.contains_key(&proposal.proposal_id) {
            return Err(anyhow!(
                "Proposal ID {} already exists",
                proposal.proposal_id
            ));
        }

        // Add to pending proposals
        self.pending_proposals
            .insert(proposal.proposal_id.clone(), proposal);

        Ok(())
    }

    /// Execute a parameter change (after successful governance vote)
    pub fn execute_parameter_change(&mut self, proposal_id: &str) -> Result<()> {
        if let Some(proposal) = self.pending_proposals.remove(proposal_id) {
            // Apply the parameter change
            self.apply_parameter_change(&proposal)?;

            // Add to history
            self.parameter_history.push(proposal);

            Ok(())
        } else {
            Err(anyhow!("Proposal {proposal_id} not found"))
        }
    }

    /// Get pending parameter proposals
    pub fn get_pending_proposals(&self) -> &HashMap<String, EconomicsParameterProposal> {
        &self.pending_proposals
    }

    /// Get parameter change history
    pub fn get_parameter_history(&self) -> &[EconomicsParameterProposal] {
        &self.parameter_history
    }

    /// Validate parameter name
    fn validate_parameter_name(&self, name: &str) -> Result<()> {
        let valid_parameters = [
            "initial_round_reward_micro",
            "halving_interval_rounds",
            "max_supply_micro",
            "fee_cap_numer",
            "fee_cap_denom",
            "proposer_weight_bps",
            "verifier_weight_bps",
            "fee_recycling_bps",
        ];

        if !valid_parameters.contains(&name) {
            return Err(anyhow!("Invalid parameter name: {name}"));
        }

        Ok(())
    }

    /// Validate parameter value
    fn validate_parameter_value(&self, name: &str, value: &serde_json::Value) -> Result<()> {
        match name {
            "initial_round_reward_micro" | "max_supply_micro" => {
                if let Some(val) = value.as_u64() {
                    if val == 0 {
                        return Err(anyhow!("Parameter {name} must be positive"));
                    }
                } else {
                    return Err(anyhow!("Parameter {name} must be a positive integer"));
                }
            }
            "halving_interval_rounds" => {
                if let Some(val) = value.as_u64() {
                    if val == 0 {
                        return Err(anyhow!("Halving interval must be positive"));
                    }
                } else {
                    return Err(anyhow!("Halving interval must be a positive integer"));
                }
            }
            "fee_cap_numer" | "fee_cap_denom" => {
                if let Some(val) = value.as_u64() {
                    if val == 0 {
                        return Err(anyhow!("Fee cap {name} must be positive"));
                    }
                } else {
                    return Err(anyhow!("Fee cap {name} must be a positive integer"));
                }
            }
            "proposer_weight_bps" | "verifier_weight_bps" | "fee_recycling_bps" => {
                if let Some(val) = value.as_u64() {
                    if val > 10000 {
                        return Err(anyhow!("Parameter {name} cannot exceed 10000 basis points"));
                    }
                } else {
                    return Err(anyhow!("Parameter {name} must be a positive integer"));
                }
            }
            _ => return Err(anyhow!("Unknown parameter: {name}")),
        }

        Ok(())
    }

    /// Apply a parameter change
    fn apply_parameter_change(&mut self, proposal: &EconomicsParameterProposal) -> Result<()> {
        match proposal.parameter_name.as_str() {
            "initial_round_reward_micro" => {
                self.current_params.initial_round_reward_micro =
                    proposal.new_value.as_u64().unwrap() as u128;
            }
            "halving_interval_rounds" => {
                self.current_params.halving_interval_rounds = proposal.new_value.as_u64().unwrap();
            }
            "max_supply_micro" => {
                self.current_params.max_supply_micro = proposal.new_value.as_u64().unwrap() as u128;
            }
            "fee_cap_numer" => {
                self.current_params.fee_cap_numer = proposal.new_value.as_u64().unwrap();
            }
            "fee_cap_denom" => {
                self.current_params.fee_cap_denom = proposal.new_value.as_u64().unwrap();
            }
            "proposer_weight_bps" => {
                self.current_params.proposer_weight_bps =
                    proposal.new_value.as_u64().unwrap() as u16;
            }
            "verifier_weight_bps" => {
                self.current_params.verifier_weight_bps =
                    proposal.new_value.as_u64().unwrap() as u16;
            }
            "fee_recycling_bps" => {
                self.current_params.fee_recycling_bps = proposal.new_value.as_u64().unwrap() as u16;
            }
            _ => return Err(anyhow!("Unknown parameter: {}", proposal.parameter_name)),
        }

        Ok(())
    }
}

impl Default for EconomicsParameterManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to create parameter change proposals
pub fn create_parameter_proposal(
    parameter_name: &str,
    new_value: serde_json::Value,
    current_value: serde_json::Value,
    justification: String,
    proposer: [u8; 32],
    voting_duration_seconds: u64,
) -> EconomicsParameterProposal {
    let proposal_id = format!("{}_{}", parameter_name, chrono::Utc::now().timestamp());
    let created_at = chrono::Utc::now().timestamp() as u64;

    EconomicsParameterProposal {
        proposal_id,
        parameter_name: parameter_name.to_string(),
        new_value,
        current_value,
        justification,
        proposer,
        created_at,
        voting_deadline: created_at + voting_duration_seconds,
    }
}

#[cfg(all(test, feature = "enable-tests"))]
mod tests {
    use super::*;

    #[test]
    fn test_parameter_manager_creation() {
        let manager = EconomicsParameterManager::new();
        let params = manager.get_current_params();

        assert_eq!(params.initial_round_reward_micro, 10_000_000);
        assert_eq!(params.halving_interval_rounds, 315_000_000);
    }

    #[test]
    fn test_parameter_proposal_submission() {
        let mut manager = EconomicsParameterManager::new();

        let proposal = create_parameter_proposal(
            "initial_round_reward_micro",
            serde_json::Value::Number(serde_json::Number::from(20_000_000)),
            serde_json::Value::Number(serde_json::Number::from(10_000_000)),
            "Double the emission rate".to_string(),
            [1u8; 32],
            7 * 24 * 3600, // 7 days
        );

        assert!(manager.submit_parameter_proposal(proposal).is_ok());
        assert_eq!(manager.get_pending_proposals().len(), 1);
    }

    #[test]
    fn test_invalid_parameter_name() {
        let mut manager = EconomicsParameterManager::new();

        let proposal = create_parameter_proposal(
            "invalid_parameter",
            serde_json::Value::Number(serde_json::Number::from(100)),
            serde_json::Value::Number(serde_json::Number::from(50)),
            "Test".to_string(),
            [1u8; 32],
            3600,
        );

        assert!(manager.submit_parameter_proposal(proposal).is_err());
    }

    #[test]
    fn test_parameter_execution() {
        let mut manager = EconomicsParameterManager::new();

        let proposal = create_parameter_proposal(
            "initial_round_reward_micro",
            serde_json::Value::Number(serde_json::Number::from(20_000_000)),
            serde_json::Value::Number(serde_json::Number::from(10_000_000)),
            "Double the emission rate".to_string(),
            [1u8; 32],
            3600,
        );

        let proposal_id = proposal.proposal_id.clone();
        manager.submit_parameter_proposal(proposal).unwrap();

        // Execute the change
        manager.execute_parameter_change(&proposal_id).unwrap();

        // Check that the parameter was updated
        let params = manager.get_current_params();
        assert_eq!(params.initial_round_reward_micro, 20_000_000);

        // Check that it was moved to history
        assert_eq!(manager.get_pending_proposals().len(), 0);
        assert_eq!(manager.get_parameter_history().len(), 1);
    }
}
