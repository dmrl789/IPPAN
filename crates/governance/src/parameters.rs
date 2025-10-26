use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use ippan_economics_core::{EconomicsParameterManager, EconomicsParams};

/// Governance and Economics parameter management
///
/// This module defines on-chain governance parameters that can be modified
/// through parameter proposals, subject to validator approval.
/// It reuses `EconomicsParams` from `ippan_economics_core` to avoid type drift.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceParameters {
    pub min_proposal_stake: u64,
    pub voting_threshold: f64,
    pub voting_duration: u64,
    pub max_active_proposals: usize,
    pub min_proposal_interval: u64,
    pub proposal_fee: u64,
    pub voting_fee: u64,
    pub economics: EconomicsParams,
}

impl Default for GovernanceParameters {
    fn default() -> Self {
        Self {
            min_proposal_stake: 1_000_000,
            voting_threshold: 0.67,
            voting_duration: 7 * 24 * 3600,
            max_active_proposals: 10,
            min_proposal_interval: 24 * 3600,
            proposal_fee: 10_000,
            voting_fee: 1_000,
            economics: EconomicsParams::default(),
        }
    }
}

/// Represents a proposal to change a governance or economic parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterChangeProposal {
    pub proposal_id: String,
    pub parameter_name: String,
    pub new_value: serde_json::Value,
    pub current_value: serde_json::Value,
    pub justification: String,
    pub proposer: [u8; 32],
    pub created_at: u64,
}

/// Parameter manager handling validation, proposal submission, and execution.
pub struct ParameterManager {
    parameters: GovernanceParameters,
    change_history: Vec<ParameterChangeProposal>,
    pending_changes: HashMap<String, ParameterChangeProposal>,
}

impl ParameterManager {
    pub fn new() -> Self {
        Self {
            parameters: GovernanceParameters::default(),
            change_history: Vec::new(),
            pending_changes: HashMap::new(),
        }
    }

    pub fn get_parameters(&self) -> &GovernanceParameters {
        &self.parameters
    }

    pub fn get_economics_params(&self) -> &EconomicsParams {
        &self.parameters.economics
    }

    pub fn update_economics_params(&mut self, params: EconomicsParams) {
        self.parameters.economics = params.clone();
    }

    pub fn submit_parameter_change(&mut self, proposal: ParameterChangeProposal) -> Result<()> {
        self.validate_parameter_name(&proposal.parameter_name)?;
        self.validate_parameter_value(&proposal.parameter_name, &proposal.new_value)?;

        if self.pending_changes.contains_key(&proposal.proposal_id) {
            return Err(anyhow::anyhow!(
                "Proposal ID {} already exists",
                proposal.proposal_id
            ));
        }
        self.pending_changes
            .insert(proposal.proposal_id.clone(), proposal);
        Ok(())
    }

    pub fn execute_parameter_change(&mut self, proposal_id: &str) -> Result<()> {
        if let Some(proposal) = self.pending_changes.remove(proposal_id) {
            self.apply_parameter_change(&proposal)?;
            self.change_history.push(proposal);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Proposal {} not found", proposal_id))
        }
    }

    pub fn get_change_history(&self) -> &[ParameterChangeProposal] {
        &self.change_history
    }

    pub fn get_pending_changes(&self) -> &HashMap<String, ParameterChangeProposal> {
        &self.pending_changes
    }

    fn validate_parameter_name(&self, name: &str) -> Result<()> {
        let valid = [
            // General governance
            "min_proposal_stake",
            "voting_threshold",
            "voting_duration",
            "max_active_proposals",
            "min_proposal_interval",
            "proposal_fee",
            "voting_fee",
            // Economics
            "economics.initial_round_reward_micro",
            "economics.halving_interval_rounds",
            "economics.max_supply_micro",
            "economics.fee_cap_numer",
            "economics.fee_cap_denom",
            "economics.proposer_weight_bps",
            "economics.verifier_weight_bps",
            "economics.fee_recycling_bps",
        ];
        if !valid.contains(&name) {
            return Err(anyhow::anyhow!("Invalid parameter: {}", name));
        }
        Ok(())
    }

    fn validate_parameter_value(&self, name: &str, value: &serde_json::Value) -> Result<()> {
        match name {
            "voting_threshold" => {
                let v = value
                    .as_f64()
                    .ok_or_else(|| anyhow::anyhow!("must be f64"))?;
                if !(0.0..=1.0).contains(&v) {
                    return Err(anyhow::anyhow!("Voting threshold must be between 0.0 and 1.0"));
                }
            }
            _ => {
                if !value.is_number() {
                    return Err(anyhow::anyhow!("{} must be numeric", name));
                }
            }
        }
        Ok(())
    }

    fn apply_parameter_change(&mut self, proposal: &ParameterChangeProposal) -> Result<()> {
        match proposal.parameter_name.as_str() {
            "min_proposal_stake" => {
                self.parameters.min_proposal_stake = proposal.new_value.as_u64().unwrap();
            }
            "voting_threshold" => {
                self.parameters.voting_threshold = proposal.new_value.as_f64().unwrap();
            }
            "voting_duration" => {
                self.parameters.voting_duration = proposal.new_value.as_u64().unwrap();
            }
            "max_active_proposals" => {
                self.parameters.max_active_proposals =
                    proposal.new_value.as_u64().unwrap() as usize;
            }
            "min_proposal_interval" => {
                self.parameters.min_proposal_interval = proposal.new_value.as_u64().unwrap();
            }
            "proposal_fee" => {
                self.parameters.proposal_fee = proposal.new_value.as_u64().unwrap();
            }
            "voting_fee" => {
                self.parameters.voting_fee = proposal.new_value.as_u64().unwrap();
            }
            "economics.initial_round_reward_micro" => {
                self.parameters.economics.initial_round_reward_micro =
                    proposal.new_value.as_u64().unwrap() as u128;
            }
            "economics.halving_interval_rounds" => {
                self.parameters.economics.halving_interval_rounds =
                    proposal.new_value.as_u64().unwrap();
            }
            "economics.max_supply_micro" => {
                self.parameters.economics.max_supply_micro =
                    proposal.new_value.as_u64().unwrap() as u128;
            }
            "economics.fee_cap_numer" => {
                self.parameters.economics.fee_cap_numer = proposal.new_value.as_u64().unwrap();
            }
            "economics.fee_cap_denom" => {
                self.parameters.economics.fee_cap_denom = proposal.new_value.as_u64().unwrap();
            }
            "economics.proposer_weight_bps" => {
                self.parameters.economics.proposer_weight_bps =
                    proposal.new_value.as_u64().unwrap() as u16;
            }
            "economics.verifier_weight_bps" => {
                self.parameters.economics.verifier_weight_bps =
                    proposal.new_value.as_u64().unwrap() as u16;
            }
            "economics.fee_recycling_bps" => {
                self.parameters.economics.fee_recycling_bps =
                    proposal.new_value.as_u64().unwrap() as u16;
            }
            _ => return Err(anyhow::anyhow!("Unknown parameter: {}", proposal.parameter_name)),
        }
        Ok(())
    }
}

impl Default for ParameterManager {
    fn default() -> Self {
        Self::new()
    }
}

// -----------------------------------------------------------------------------
// ✅ Tests
// -----------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_governance_params() {
        let g = GovernanceParameters::default();
        assert_eq!(g.voting_threshold, 0.67);
        assert_eq!(g.economics.proposer_weight_bps, 2000);
    }

    #[test]
    fn test_submit_and_execute_change() {
        let mut m = ParameterManager::new();
        let proposal = ParameterChangeProposal {
            proposal_id: "change_reward".into(),
            parameter_name: "economics.initial_round_reward_micro".into(),
            new_value: json!(20000),
            current_value: json!(10000),
            justification: "Test doubling reward".into(),
            proposer: [1u8; 32],
            created_at: 123,
        };
        assert!(m.submit_parameter_change(proposal.clone()).is_ok());
        assert!(m.execute_parameter_change("change_reward").is_ok());
        assert_eq!(
            m.get_parameters().economics.initial_round_reward_micro,
            20000
        );
    }
}
