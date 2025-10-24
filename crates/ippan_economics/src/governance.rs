//! Governance controls for emission parameters

use crate::types::*;
use crate::errors::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Governance parameters for emission control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceParams {
    /// Current emission parameters
    pub emission_params: EmissionParams,
    /// Voting power required for parameter changes (as percentage)
    pub voting_threshold: u64, // e.g., 66 for 66%
    /// Minimum voting period in rounds
    pub min_voting_period: RoundIndex,
    /// Maximum voting period in rounds
    pub max_voting_period: RoundIndex,
    /// Validator voting power (validator_id -> power)
    pub validator_power: HashMap<ValidatorId, u64>,
    /// Active proposals
    pub active_proposals: HashMap<ProposalId, EmissionProposal>,
    /// Proposal counter
    pub next_proposal_id: ProposalId,
}

/// Unique proposal identifier
pub type ProposalId = u64;

/// Emission parameter change proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmissionProposal {
    /// Proposal ID
    pub id: ProposalId,
    /// Proposed emission parameters
    pub proposed_params: EmissionParams,
    /// Proposer validator ID
    pub proposer: ValidatorId,
    /// Round when proposal was created
    pub creation_round: RoundIndex,
    /// Round when voting ends
    pub voting_end_round: RoundIndex,
    /// Current votes (validator_id -> vote)
    pub votes: HashMap<ValidatorId, Vote>,
    /// Proposal status
    pub status: ProposalStatus,
    /// Justification for the change
    pub justification: String,
}

/// Vote on a proposal
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Vote {
    /// Approve the proposal
    Approve,
    /// Reject the proposal
    Reject,
    /// Abstain from voting
    Abstain,
}

/// Status of a proposal
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ProposalStatus {
    /// Proposal is active and accepting votes
    Active,
    /// Proposal has been approved
    Approved,
    /// Proposal has been rejected
    Rejected,
    /// Proposal has expired
    Expired,
    /// Proposal has been executed
    Executed,
}

impl GovernanceParams {
    /// Create new governance parameters
    pub fn new(emission_params: EmissionParams) -> Self {
        Self {
            emission_params,
            voting_threshold: 66, // 66% supermajority
            min_voting_period: 100, // 10 seconds at 10 rounds/second
            max_voting_period: 1000, // 100 seconds
            validator_power: HashMap::new(),
            active_proposals: HashMap::new(),
            next_proposal_id: 1,
        }
    }

    /// Add or update validator voting power
    pub fn set_validator_power(&mut self, validator_id: ValidatorId, power: u64) {
        self.validator_power.insert(validator_id, power);
    }

    /// Create a new emission parameter proposal
    pub fn create_proposal(
        &mut self,
        proposer: ValidatorId,
        proposed_params: EmissionParams,
        voting_period: RoundIndex,
        justification: String,
        current_round: RoundIndex,
    ) -> Result<ProposalId, GovernanceError> {
        // Validate proposer has voting power
        if !self.validator_power.contains_key(&proposer) {
            return Err(GovernanceError::InsufficientVotingPower {
                required: 1,
                actual: 0,
            });
        }

        // Validate voting period
        if voting_period < self.min_voting_period || voting_period > self.max_voting_period {
            return Err(GovernanceError::InvalidParameter {
                param: "voting_period".to_string(),
                value: voting_period.to_string(),
                reason: format!(
                    "Must be between {} and {} rounds",
                    self.min_voting_period, self.max_voting_period
                ),
            });
        }

        // Validate proposed parameters
        self.validate_emission_params(&proposed_params)?;

        let proposal_id = self.next_proposal_id;
        self.next_proposal_id += 1;

        let proposal = EmissionProposal {
            id: proposal_id,
            proposed_params,
            proposer: proposer.clone(),
            creation_round: current_round,
            voting_end_round: current_round + voting_period,
            votes: HashMap::new(),
            status: ProposalStatus::Active,
            justification,
        };

        self.active_proposals.insert(proposal_id, proposal);

        info!(
            "Created emission proposal {} by validator {}",
            proposal_id, proposer.clone()
        );

        Ok(proposal_id)
    }

    /// Vote on a proposal
    pub fn vote_on_proposal(
        &mut self,
        proposal_id: ProposalId,
        validator_id: ValidatorId,
        vote: Vote,
        current_round: RoundIndex,
    ) -> Result<(), GovernanceError> {
        let proposal = self.active_proposals.get_mut(&proposal_id)
            .ok_or_else(|| GovernanceError::GovernanceError(format!("Proposal {} not found", proposal_id)))?;

        // Check if proposal is still active
        if proposal.status != ProposalStatus::Active {
            return Err(GovernanceError::GovernanceError(format!(
                "Proposal {} is not active (status: {:?})",
                proposal_id, proposal.status
            )));
        }

        // Check if voting period has ended
        if current_round > proposal.voting_end_round {
            if let Some(proposal) = self.active_proposals.get_mut(&proposal_id) {
                proposal.status = ProposalStatus::Expired;
            }
            return Err(GovernanceError::GovernanceError(format!(
                "Voting period for proposal {} has ended",
                proposal_id
            )));
        }

        // Check if validator has voting power
        let power = self.validator_power.get(&validator_id)
            .ok_or_else(|| GovernanceError::InsufficientVotingPower {
                required: 1,
                actual: 0,
            })?;

        if *power == 0 {
            return Err(GovernanceError::InsufficientVotingPower {
                required: 1,
                actual: 0,
            });
        }

        // Record the vote
        proposal.votes.insert(validator_id.clone(), vote);

        debug!(
            "Validator {} voted {:?} on proposal {}",
            validator_id.clone(), vote, proposal_id
        );

        Ok(())
    }

    /// Process proposal results and update parameters if approved
    pub fn process_proposal_results(&mut self, proposal_id: ProposalId) -> Result<bool, GovernanceError> {
        let proposal = self.active_proposals.get(&proposal_id)
            .ok_or_else(|| GovernanceError::GovernanceError(format!("Proposal {} not found", proposal_id)))?;

        if proposal.status != ProposalStatus::Active {
            return Ok(false); // Already processed
        }

        // Calculate voting results
        let (approve_power, reject_power, _voting_power) = self.calculate_voting_power(proposal);
        
        // Calculate total power of ALL validators (not just those who voted)
        let total_validator_power: u64 = self.validator_power.values().sum();

        // Check if we have enough participation (50% of total validator power)
        let participation_threshold = (total_validator_power * 50) / 100;
        if (approve_power + reject_power) < participation_threshold {
            if let Some(proposal) = self.active_proposals.get_mut(&proposal_id) {
                proposal.status = ProposalStatus::Expired;
            }
            return Ok(false);
        }

        // Check if proposal is approved (66% of total validator power)
        let approval_threshold = (total_validator_power * self.voting_threshold) / 100;
        if approve_power >= approval_threshold {
            if let Some(proposal) = self.active_proposals.get_mut(&proposal_id) {
                proposal.status = ProposalStatus::Approved;
            }
            info!("Proposal {} approved with {}% approval of total power", proposal_id, (approve_power * 100) / total_validator_power);
            return Ok(true);
        } else {
            if let Some(proposal) = self.active_proposals.get_mut(&proposal_id) {
                proposal.status = ProposalStatus::Rejected;
            }
            info!("Proposal {} rejected with {}% approval of total power", proposal_id, (approve_power * 100) / total_validator_power);
            return Ok(false);
        }
    }

    /// Execute an approved proposal
    pub fn execute_proposal(&mut self, proposal_id: ProposalId) -> Result<(), GovernanceError> {
        let proposal = self.active_proposals.get_mut(&proposal_id)
            .ok_or_else(|| GovernanceError::GovernanceError(format!("Proposal {} not found", proposal_id)))?;

        if proposal.status != ProposalStatus::Approved {
            return Err(GovernanceError::GovernanceError(format!(
                "Proposal {} is not approved (status: {:?})",
                proposal_id, proposal.status
            )));
        }

        // Update emission parameters
        self.emission_params = proposal.proposed_params.clone();
        proposal.status = ProposalStatus::Executed;

        info!(
            "Executed proposal {}: updated emission parameters to {:?}",
            proposal_id, self.emission_params
        );

        Ok(())
    }

    /// Calculate voting power for a proposal
    fn calculate_voting_power(&self, proposal: &EmissionProposal) -> (u64, u64, u64) {
        let mut approve_power = 0;
        let mut reject_power = 0;
        let mut total_power = 0;

        for (validator_id, vote) in &proposal.votes {
            if let Some(power) = self.validator_power.get(validator_id) {
                total_power += power;
                match vote {
                    Vote::Approve => approve_power += power,
                    Vote::Reject => reject_power += power,
                    Vote::Abstain => {} // Abstain doesn't count toward approval/rejection
                }
            }
        }

        (approve_power, reject_power, total_power)
    }

    /// Validate emission parameters
    fn validate_emission_params(&self, params: &EmissionParams) -> Result<(), GovernanceError> {
        if params.initial_round_reward == 0 {
            return Err(GovernanceError::InvalidParameter {
                param: "initial_round_reward".to_string(),
                value: params.initial_round_reward.to_string(),
                reason: "Must be greater than 0".to_string(),
            });
        }

        if params.halving_interval == 0 {
            return Err(GovernanceError::InvalidParameter {
                param: "halving_interval".to_string(),
                value: params.halving_interval.to_string(),
                reason: "Must be greater than 0".to_string(),
            });
        }

        if params.total_supply_cap == 0 {
            return Err(GovernanceError::InvalidParameter {
                param: "total_supply_cap".to_string(),
                value: params.total_supply_cap.to_string(),
                reason: "Must be greater than 0".to_string(),
            });
        }

        if params.fee_cap_fraction < rust_decimal::Decimal::ZERO || params.fee_cap_fraction > rust_decimal::Decimal::ONE {
            return Err(GovernanceError::InvalidParameter {
                param: "fee_cap_fraction".to_string(),
                value: params.fee_cap_fraction.to_string(),
                reason: "Must be between 0 and 1".to_string(),
            });
        }

        Ok(())
    }

    /// Get current emission parameters
    pub fn emission_params(&self) -> &EmissionParams {
        &self.emission_params
    }

    /// Get active proposals
    pub fn active_proposals(&self) -> &HashMap<ProposalId, EmissionProposal> {
        &self.active_proposals
    }

    /// Get proposal by ID
    pub fn get_proposal(&self, proposal_id: ProposalId) -> Option<&EmissionProposal> {
        self.active_proposals.get(&proposal_id)
    }

    /// Clean up expired proposals
    pub fn cleanup_expired_proposals(&mut self, current_round: RoundIndex) {
        let expired_proposals: Vec<ProposalId> = self.active_proposals
            .iter()
            .filter(|(_, proposal)| {
                proposal.status == ProposalStatus::Active && current_round > proposal.voting_end_round
            })
            .map(|(id, _)| *id)
            .collect();

        for proposal_id in expired_proposals {
            if let Some(proposal) = self.active_proposals.get_mut(&proposal_id) {
                proposal.status = ProposalStatus::Expired;
                warn!("Proposal {} expired at round {}", proposal_id, current_round);
            }
        }
    }
}

impl Default for GovernanceParams {
    fn default() -> Self {
        Self::new(EmissionParams::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proposal_creation() {
        let mut governance = GovernanceParams::new(EmissionParams::default());
        governance.set_validator_power("validator1".to_string(), 100);

        let proposal_id = governance.create_proposal(
            "validator1".to_string(),
            EmissionParams::default(),
            100,
            "Test proposal".to_string(),
            1,
        ).unwrap();

        assert_eq!(proposal_id, 1);
        assert!(governance.active_proposals.contains_key(&proposal_id));
    }

    #[test]
    fn test_voting() {
        let mut governance = GovernanceParams::new(EmissionParams::default());
        governance.set_validator_power("validator1".to_string(), 100);
        governance.set_validator_power("validator2".to_string(), 50);

        let proposal_id = governance.create_proposal(
            "validator1".to_string(),
            EmissionParams::default(),
            100,
            "Test proposal".to_string(),
            1,
        ).unwrap();

        governance.vote_on_proposal(proposal_id, "validator1".to_string(), Vote::Approve, 1).unwrap();
        governance.vote_on_proposal(proposal_id, "validator2".to_string(), Vote::Approve, 1).unwrap();

        let proposal = governance.get_proposal(proposal_id).unwrap();
        assert_eq!(proposal.votes.len(), 2);
    }

    #[test]
    fn test_proposal_execution() {
        let mut governance = GovernanceParams::new(EmissionParams::default());
        governance.set_validator_power("validator1".to_string(), 100);

        let mut new_params = EmissionParams::default();
        new_params.initial_round_reward = 20_000;

        let proposal_id = governance.create_proposal(
            "validator1".to_string(),
            new_params.clone(),
            100,
            "Test proposal".to_string(),
            1,
        ).unwrap();

        governance.vote_on_proposal(proposal_id, "validator1".to_string(), Vote::Approve, 1).unwrap();
        governance.process_proposal_results(proposal_id).unwrap();
        governance.execute_proposal(proposal_id).unwrap();

        assert_eq!(governance.emission_params().initial_round_reward, 20_000);
    }
}