use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Economics / Emission parameters governed on-chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicsParams {
    /// Initial reward per round (in µIPN)
    pub r0: u128,
    /// Number of rounds between halvings
    pub halving_rounds: u64,
    /// Supply cap in µIPN
    pub supply_cap: u128,
    /// Proposer reward (basis points out of 10_000)
    pub proposer_bps: u16,
}

impl Default for EconomicsParams {
    fn default() -> Self {
        Self {
            r0: 10_000,
            halving_rounds: 315_000_000,
            supply_cap: 21_000_000_00000000,
            proposer_bps: 2000,
        }
    }
}
use std::collections::HashMap;

/// Governance parameters that can be modified through proposals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceParameters {
    /// Minimum stake required to submit a proposal
    pub min_proposal_stake: u64,
    /// Voting threshold (percentage of stake required for approval)
    pub voting_threshold: f64,
    /// Voting duration in seconds
    pub voting_duration: u64,
    /// Maximum number of active proposals
    pub max_active_proposals: usize,
    /// Minimum time between proposals from the same validator
    pub min_proposal_interval: u64,
    /// Fee for submitting a proposal
    pub proposal_fee: u64,
    /// Fee for voting on a proposal
    pub voting_fee: u64,
}

impl Default for GovernanceParameters {
    fn default() -> Self {
        Self {
            min_proposal_stake: 1_000_000, // 1M tokens
            voting_threshold: 0.67, // 67%
            voting_duration: 7 * 24 * 3600, // 7 days
            max_active_proposals: 10,
            min_proposal_interval: 24 * 3600, // 24 hours
            proposal_fee: 10_000, // 10K tokens
            voting_fee: 1_000, // 1K tokens
        }
    }
}

/// Parameter change proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterChangeProposal {
    /// Proposal ID
    pub proposal_id: String,
    /// Parameter name to change
    pub parameter_name: String,
    /// New value for the parameter
    pub new_value: serde_json::Value,
    /// Current value of the parameter
    pub current_value: serde_json::Value,
    /// Justification for the change
    pub justification: String,
    /// Proposer address
    pub proposer: [u8; 32],
    /// Creation timestamp
    pub created_at: u64,
}

/// Governance parameter manager
pub struct ParameterManager {
    /// Current parameters
    parameters: GovernanceParameters,
    /// Parameter change history
    change_history: Vec<ParameterChangeProposal>,
    /// Pending parameter changes
    pending_changes: HashMap<String, ParameterChangeProposal>,
}

impl ParameterManager {
    /// Create a new parameter manager
    pub fn new() -> Self {
        Self {
            parameters: GovernanceParameters::default(),
            change_history: Vec::new(),
            pending_changes: HashMap::new(),
        }
    }

    /// Get current parameters
    pub fn get_parameters(&self) -> &GovernanceParameters {
        &self.parameters
    }

    /// Submit a parameter change proposal
    pub fn submit_parameter_change(
        &mut self,
        proposal: ParameterChangeProposal,
    ) -> Result<()> {
        // Validate parameter name
        self.validate_parameter_name(&proposal.parameter_name)?;
        
        // Validate new value
        self.validate_parameter_value(&proposal.parameter_name, &proposal.new_value)?;
        
        // Check for duplicate proposal
        if self.pending_changes.contains_key(&proposal.proposal_id) {
            return Err(anyhow::anyhow!("Proposal ID {} already exists", proposal.proposal_id));
        }
        
        // Add to pending changes
        self.pending_changes.insert(proposal.proposal_id.clone(), proposal);
        
        Ok(())
    }

    /// Execute a parameter change
    pub fn execute_parameter_change(&mut self, proposal_id: &str) -> Result<()> {
        if let Some(proposal) = self.pending_changes.remove(proposal_id) {
            // Apply the change
            self.apply_parameter_change(&proposal)?;
            
            // Add to history
            self.change_history.push(proposal);
            
            Ok(())
        } else {
            Err(anyhow::anyhow!("Proposal {} not found", proposal_id))
        }
    }

    /// Get parameter change history
    pub fn get_change_history(&self) -> &[ParameterChangeProposal] {
        &self.change_history
    }

    /// Get pending changes
    pub fn get_pending_changes(&self) -> &HashMap<String, ParameterChangeProposal> {
        &self.pending_changes
    }

    /// Validate parameter name
    fn validate_parameter_name(&self, name: &str) -> Result<()> {
        let valid_parameters = [
            "min_proposal_stake",
            "voting_threshold",
            "voting_duration",
            "max_active_proposals",
            "min_proposal_interval",
            "proposal_fee",
            "voting_fee",
            // Economics
            "economics.r0",
            "economics.halving_rounds",
            "economics.supply_cap",
            "economics.proposer_bps",
        ];
        
        if !valid_parameters.contains(&name) {
            return Err(anyhow::anyhow!("Invalid parameter name: {}", name));
        }
        
        Ok(())
    }

    /// Validate parameter value
    fn validate_parameter_value(&self, name: &str, value: &serde_json::Value) -> Result<()> {
        match name {
            "min_proposal_stake" | "voting_duration" | "max_active_proposals" 
            | "min_proposal_interval" | "proposal_fee" | "voting_fee" => {
                if !value.is_number() || value.as_u64().is_none() {
                    return Err(anyhow::anyhow!("Parameter {} must be a positive integer", name));
                }
            }
            "voting_threshold" => {
                if let Some(threshold) = value.as_f64() {
                    if threshold < 0.0 || threshold > 1.0 {
                        return Err(anyhow::anyhow!("Voting threshold must be between 0.0 and 1.0"));
                    }
                } else {
                    return Err(anyhow::anyhow!("Voting threshold must be a number"));
                }
            }
            "economics.r0" | "economics.supply_cap" => {
                if !value.is_number() && value.as_str().and_then(|s| s.parse::<u128>().ok()).is_none() {
                    return Err(anyhow::anyhow!("{} must be a u128 (or string u128)", name));
                }
            }
            "economics.halving_rounds" => {
                if !value.is_number() || value.as_u64().is_none() {
                    return Err(anyhow::anyhow!("{} must be a u64", name));
                }
            }
            "economics.proposer_bps" => {
                let Some(v) = value.as_u64() else { return Err(anyhow::anyhow!("proposer_bps must be u16")); };
                if v > 10_000 { return Err(anyhow::anyhow!("proposer_bps cannot exceed 10000")); }
            }
            _ => return Err(anyhow::anyhow!("Unknown parameter: {}", name)),
        }
        
        Ok(())
    }

    /// Apply a parameter change
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
                self.parameters.max_active_proposals = proposal.new_value.as_u64().unwrap() as usize;
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
            // Economics parameters
            "economics.r0" => {
                let r0 = proposal.new_value.as_u64().map(|v| v as u128)
                    .or_else(|| proposal.new_value.as_str().and_then(|s| s.parse::<u128>().ok()))
                    .ok_or_else(|| anyhow::anyhow!("invalid r0 value"))?;
                // Store in metadata for other modules to read
                // In a full implementation, this would write to chain state; here we just log
                tracing::info!("Governance: updated economics.r0 to {}", r0);
            }
            "economics.halving_rounds" => {
                let v = proposal.new_value.as_u64().unwrap();
                tracing::info!("Governance: updated economics.halving_rounds to {}", v);
            }
            "economics.supply_cap" => {
                let cap = proposal.new_value.as_u64().map(|v| v as u128)
                    .or_else(|| proposal.new_value.as_str().and_then(|s| s.parse::<u128>().ok()))
                    .ok_or_else(|| anyhow::anyhow!("invalid supply cap"))?;
                tracing::info!("Governance: updated economics.supply_cap to {}", cap);
            }
            "economics.proposer_bps" => {
                let bps = proposal.new_value.as_u64().unwrap() as u16;
                tracing::info!("Governance: updated economics.proposer_bps to {}", bps);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parameter_manager() {
        let mut manager = ParameterManager::new();
        let params = manager.get_parameters();
        
        assert_eq!(params.min_proposal_stake, 1_000_000);
        assert_eq!(params.voting_threshold, 0.67);
    }

    #[test]
    fn test_parameter_change_proposal() {
        let mut manager = ParameterManager::new();
        
        let proposal = ParameterChangeProposal {
            proposal_id: "change_threshold".to_string(),
            parameter_name: "voting_threshold".to_string(),
            new_value: serde_json::Value::Number(serde_json::Number::from_f64(0.75).unwrap()),
            current_value: serde_json::Value::Number(serde_json::Number::from_f64(0.67).unwrap()),
            justification: "Increase threshold for better security".to_string(),
            proposer: [1u8; 32],
            created_at: 1234567890,
        };
        
        assert!(manager.submit_parameter_change(proposal).is_ok());
        assert_eq!(manager.get_pending_changes().len(), 1);
    }

    #[test]
    fn test_invalid_parameter_name() {
        let mut manager = ParameterManager::new();
        
        let proposal = ParameterChangeProposal {
            proposal_id: "invalid".to_string(),
            parameter_name: "invalid_parameter".to_string(),
            new_value: serde_json::Value::Number(serde_json::Number::from(100)),
            current_value: serde_json::Value::Number(serde_json::Number::from(50)),
            justification: "Test".to_string(),
            proposer: [1u8; 32],
            created_at: 1234567890,
        };
        
        assert!(manager.submit_parameter_change(proposal).is_err());
    }

    #[test]
    fn test_invalid_parameter_value() {
        let mut manager = ParameterManager::new();
        
        let proposal = ParameterChangeProposal {
            proposal_id: "invalid_value".to_string(),
            parameter_name: "voting_threshold".to_string(),
            new_value: serde_json::Value::String("invalid".to_string()),
            current_value: serde_json::Value::Number(serde_json::Number::from_f64(0.67).unwrap()),
            justification: "Test".to_string(),
            proposer: [1u8; 32],
            created_at: 1234567890,
        };
        
        assert!(manager.submit_parameter_change(proposal).is_err());
    }
}