use anyhow::Result;
use ippan_economics::EmissionParams;
use ippan_types::{ratio_from_bps, RatioMicros, RATIO_SCALE};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Governance and Economics parameter management
///
/// Defines all tunable parameters subject to on-chain proposals and validator approval.
/// Integrates directly with `EmissionParams` to keep token economics in sync.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceParameters {
    /// Minimum IPN stake to submit a proposal
    pub min_proposal_stake: u64,
    /// Minimum approval threshold (0–1)
    pub voting_threshold: RatioMicros,
    /// Duration of voting window (seconds)
    pub voting_duration: u64,
    /// Maximum number of concurrent proposals
    pub max_active_proposals: usize,
    /// Minimum interval between proposals from same proposer (seconds)
    pub min_proposal_interval: u64,
    /// Fee to submit a proposal (micro-IPN)
    pub proposal_fee: u64,
    /// Fee to cast a vote (micro-IPN)
    pub voting_fee: u64,
    /// Core economics configuration
    pub economics: EmissionParams,
}

impl Default for GovernanceParameters {
    fn default() -> Self {
        Self {
            min_proposal_stake: 1_000_000,
            voting_threshold: ratio_from_bps(6_700),
            voting_duration: 7 * 24 * 3600,
            max_active_proposals: 10,
            min_proposal_interval: 24 * 3600,
            proposal_fee: 10_000,
            voting_fee: 1_000,
            economics: EmissionParams::default(),
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

    pub fn get_economics_params(&self) -> &EmissionParams {
        &self.parameters.economics
    }

    pub fn update_economics_params(&mut self, params: EmissionParams) {
        self.parameters.economics = params;
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

    // ------------------------------------------------------------
    // Internal validation and application
    // ------------------------------------------------------------

    fn validate_parameter_name(&self, name: &str) -> Result<()> {
        let valid = [
            // Governance core
            "min_proposal_stake",
            "voting_threshold",
            "voting_duration",
            "max_active_proposals",
            "min_proposal_interval",
            "proposal_fee",
            "voting_fee",
            // Economics (EmissionParams)
            "economics.initial_round_reward_micro",
            "economics.halving_interval_rounds",
            "economics.max_supply_micro",
            "economics.proposer_weight_bps",
            "economics.verifier_weight_bps",
        ];
        if !valid.contains(&name) {
            return Err(anyhow::anyhow!("Invalid parameter: {}", name));
        }
        Ok(())
    }

    fn validate_parameter_value(&self, name: &str, value: &serde_json::Value) -> Result<()> {
        match name {
            "voting_threshold" => {
                parse_ratio_from_json(value)?;
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
            // Governance
            "min_proposal_stake" => {
                self.parameters.min_proposal_stake = proposal
                    .new_value
                    .as_u64()
                    .ok_or_else(|| anyhow::anyhow!("Invalid value type for min_proposal_stake"))?;
            }
            "voting_threshold" => {
                self.parameters.voting_threshold = parse_ratio_from_json(&proposal.new_value)?;
            }
            "voting_duration" => {
                self.parameters.voting_duration = proposal
                    .new_value
                    .as_u64()
                    .ok_or_else(|| anyhow::anyhow!("Invalid value type for voting_duration"))?;
            }
            "max_active_proposals" => {
                self.parameters.max_active_proposals =
                    proposal.new_value.as_u64().ok_or_else(|| {
                        anyhow::anyhow!("Invalid value type for max_active_proposals")
                    })? as usize;
            }
            "min_proposal_interval" => {
                self.parameters.min_proposal_interval =
                    proposal.new_value.as_u64().ok_or_else(|| {
                        anyhow::anyhow!("Invalid value type for min_proposal_interval")
                    })?;
            }
            "proposal_fee" => {
                self.parameters.proposal_fee = proposal
                    .new_value
                    .as_u64()
                    .ok_or_else(|| anyhow::anyhow!("Invalid value type for proposal_fee"))?;
            }
            "voting_fee" => {
                self.parameters.voting_fee = proposal
                    .new_value
                    .as_u64()
                    .ok_or_else(|| anyhow::anyhow!("Invalid value type for voting_fee"))?;
            }

            // Economics
            "economics.initial_round_reward_micro" => {
                self.parameters.economics.initial_round_reward_micro =
                    proposal.new_value.as_u64().ok_or_else(|| {
                        anyhow::anyhow!("Invalid value type for initial_round_reward_micro")
                    })?;
            }
            "economics.halving_interval_rounds" => {
                self.parameters.economics.halving_interval_rounds =
                    proposal.new_value.as_u64().ok_or_else(|| {
                        anyhow::anyhow!("Invalid value type for halving_interval_rounds")
                    })?;
            }
            "economics.max_supply_micro" => {
                self.parameters.economics.max_supply_micro = proposal
                    .new_value
                    .as_u64()
                    .ok_or_else(|| anyhow::anyhow!("Invalid value type for max_supply_micro"))?;
            }
            "economics.proposer_weight_bps" => {
                self.parameters.economics.proposer_weight_bps =
                    proposal.new_value.as_u64().ok_or_else(|| {
                        anyhow::anyhow!("Invalid value type for proposer_weight_bps")
                    })? as u32;
            }
            "economics.verifier_weight_bps" => {
                self.parameters.economics.verifier_weight_bps =
                    proposal.new_value.as_u64().ok_or_else(|| {
                        anyhow::anyhow!("Invalid value type for verifier_weight_bps")
                    })? as u32;
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Unknown parameter: {}",
                    proposal.parameter_name
                ))
            }
        }
        Ok(())
    }
}

impl Default for ParameterManager {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_ratio_from_json(value: &serde_json::Value) -> Result<RatioMicros> {
    match value {
        serde_json::Value::Number(num) => parse_ratio_str(&num.to_string()),
        serde_json::Value::String(s) => parse_ratio_str(s),
        _ => Err(anyhow::anyhow!(
            "Voting threshold must be provided as a number or string"
        )),
    }
}

fn parse_ratio_str(raw: &str) -> Result<RatioMicros> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(anyhow::anyhow!("Voting threshold cannot be empty"));
    }
    if trimmed.starts_with('-') {
        return Err(anyhow::anyhow!(
            "Voting threshold cannot be negative: {trimmed}"
        ));
    }

    let mut parts = trimmed.split('.');
    let whole_part = parts.next().unwrap_or("");
    let fractional_part = parts.next();
    if parts.next().is_some() {
        return Err(anyhow::anyhow!(
            "Voting threshold may contain at most one decimal point: {trimmed}"
        ));
    }

    let whole = if whole_part.is_empty() {
        0
    } else {
        whole_part.parse::<u64>().map_err(|_| {
            anyhow::anyhow!("Invalid whole number component for voting threshold: {trimmed}")
        })?
    };

    if whole > 1 {
        return Err(anyhow::anyhow!(
            "Voting threshold cannot exceed 1.0 (100%): {trimmed}"
        ));
    }

    let mut ratio = (whole as u128) * (RATIO_SCALE as u128);
    if let Some(frac) = fractional_part {
        if !frac.chars().all(|c| c.is_ascii_digit()) {
            return Err(anyhow::anyhow!(
                "Voting threshold fractional component must be numeric: {trimmed}"
            ));
        }
        let mut fractional = frac.to_string();
        if fractional.len() > 6 {
            fractional.truncate(6);
        }
        while fractional.len() < 6 {
            fractional.push('0');
        }
        if !fractional.is_empty() {
            let frac_value = fractional.parse::<u64>().map_err(|_| {
                anyhow::anyhow!("Invalid fractional component for voting threshold: {trimmed}")
            })?;
            ratio += frac_value as u128;
        }
    }

    Ok(ratio.min(RATIO_SCALE as u128) as RatioMicros)
}

// -----------------------------------------------------------------------------
// ✅ Tests
// -----------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_default_governance_params() {
        let g = GovernanceParameters::default();
        assert_eq!(g.voting_threshold, ratio_from_bps(6_700));
        assert_eq!(g.economics.proposer_weight_bps, 5455);
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
