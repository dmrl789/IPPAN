//! Governance implementation for AI Registry

use crate::{
    errors::{RegistryError, Result},
    storage::RegistryStorage,
    types::*,
};
use chrono::{Duration, Utc};
use ippan_ai_core::types::ModelId;
use std::collections::HashMap;
use tracing::info;

/// Governance manager for AI Registry
pub struct GovernanceManager {
    /// Storage backend
    storage: RegistryStorage,
    /// Configuration
    config: RegistryConfig,
    /// Active proposals
    proposals: HashMap<String, GovernanceProposal>,
    /// Voting power by address
    voting_power: HashMap<String, u64>,
}

impl GovernanceManager {
    /// Create a new governance manager
    pub fn new(storage: RegistryStorage, config: RegistryConfig) -> Self {
        Self {
            storage,
            config,
            proposals: HashMap::new(),
            voting_power: HashMap::new(),
        }
    }

    /// Create a new governance proposal
    pub async fn create_proposal(
        &mut self,
        proposal_type: ProposalType,
        title: String,
        description: String,
        proposer: String,
        data: ProposalData,
    ) -> Result<GovernanceProposal> {
        info!("Creating governance proposal: {}", title);

        // Check if proposer has sufficient voting power
        let power = self.get_voting_power(&proposer)?;
        if power < self.config.min_voting_power {
            return Err(RegistryError::PermissionDenied(
                "Insufficient voting power to create proposal".to_string(),
            ));
        }

        // Create proposal
        let proposal_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();

        let proposal = GovernanceProposal {
            id: proposal_id.clone(),
            proposal_type,
            title,
            description,
            proposer,
            data,
            created_at: now,
            voting_deadline: now + Duration::seconds(self.config.voting_period_seconds as i64),
            execution_deadline: Some(
                now + Duration::seconds(self.config.execution_period_seconds as i64),
            ),
            status: ProposalStatus::Active,
            votes: Vec::new(),
        };

        // Store proposal
        self.storage.store_governance_proposal(&proposal).await?;

        // Update cache
        self.proposals.insert(proposal_id.clone(), proposal.clone());

        info!("Governance proposal created: {}", proposal_id);
        Ok(proposal)
    }

    /// Vote on a proposal
    pub async fn vote_on_proposal(
        &mut self,
        proposal_id: &str,
        voter: String,
        choice: VoteChoice,
        justification: Option<String>,
    ) -> Result<()> {
        info!("Voting on proposal {}: {:?}", proposal_id, choice);

        // Get proposal
        let proposal = self
            .get_proposal(proposal_id)
            .await?
            .ok_or_else(|| RegistryError::ModelNotFound(proposal_id.to_string()))?;

        // Check if proposal is still active
        if proposal.status != ProposalStatus::Active {
            return Err(RegistryError::GovernanceViolation(
                "Proposal is not active".to_string(),
            ));
        }

        // Check if voting deadline has passed
        if Utc::now() > proposal.voting_deadline {
            return Err(RegistryError::GovernanceViolation(
                "Voting deadline has passed".to_string(),
            ));
        }

        // Get voter's voting power
        let power = self.get_voting_power(&voter)?;
        if power == 0 {
            return Err(RegistryError::PermissionDenied(
                "No voting power".to_string(),
            ));
        }

        // Create vote
        let vote = Vote {
            voter,
            choice,
            weight: power,
            timestamp: Utc::now(),
            justification,
        };

        // Add vote to proposal
        let mut updated_proposal = proposal;
        updated_proposal.votes.push(vote);

        // Store updated proposal
        self.storage
            .store_governance_proposal(&updated_proposal)
            .await?;

        // Update cache
        self.proposals
            .insert(proposal_id.to_string(), updated_proposal);

        info!("Vote recorded successfully");
        Ok(())
    }

    /// Execute a proposal
    pub async fn execute_proposal(&mut self, proposal_id: &str) -> Result<()> {
        info!("Executing proposal: {}", proposal_id);

        // Get proposal
        let proposal = self
            .get_proposal(proposal_id)
            .await?
            .ok_or_else(|| RegistryError::ModelNotFound(proposal_id.to_string()))?;

        // Check if proposal passed
        if !self.is_proposal_passed(&proposal)? {
            return Err(RegistryError::GovernanceViolation(
                "Proposal did not pass".to_string(),
            ));
        }

        // Check if execution deadline has passed
        if let Some(deadline) = proposal.execution_deadline {
            if Utc::now() > deadline {
                return Err(RegistryError::GovernanceViolation(
                    "Execution deadline has passed".to_string(),
                ));
            }
        }

        // Execute proposal based on type
        match &proposal.data {
            ProposalData::ModelApproval { model_id, .. } => {
                self.execute_model_approval(model_id).await?;
            }
            ProposalData::FeeChange {
                fee_type, new_fee, ..
            } => {
                self.execute_fee_change(fee_type, *new_fee).await?;
            }
            ProposalData::ParameterChange {
                parameter,
                new_value,
                ..
            } => {
                self.execute_parameter_change(parameter, new_value).await?;
            }
            ProposalData::Emergency { action, .. } => {
                self.execute_emergency_action(action).await?;
            }
        }

        // Update proposal status
        let mut updated_proposal = proposal;
        updated_proposal.status = ProposalStatus::Executed;

        // Store updated proposal
        self.storage
            .store_governance_proposal(&updated_proposal)
            .await?;

        // Update cache
        self.proposals
            .insert(proposal_id.to_string(), updated_proposal);

        info!("Proposal executed successfully");
        Ok(())
    }

    /// Get proposal by ID
    pub async fn get_proposal(&mut self, proposal_id: &str) -> Result<Option<GovernanceProposal>> {
        // Check cache first
        if let Some(proposal) = self.proposals.get(proposal_id) {
            return Ok(Some(proposal.clone()));
        }

        // Load from storage
        let proposal = self.storage.load_governance_proposal(proposal_id).await?;

        // Update cache
        if let Some(ref prop) = proposal {
            self.proposals.insert(proposal_id.to_string(), prop.clone());
        }

        Ok(proposal)
    }

    /// List active proposals
    pub async fn list_active_proposals(&self) -> Result<Vec<GovernanceProposal>> {
        self.storage.list_active_proposals().await
    }

    /// Check if proposal passed
    fn is_proposal_passed(&self, proposal: &GovernanceProposal) -> Result<bool> {
        let mut for_votes = 0u64;
        let mut against_votes = 0u64;
        let mut total_votes = 0u64;

        for vote in &proposal.votes {
            total_votes += vote.weight;
            match vote.choice {
                VoteChoice::For => for_votes += vote.weight,
                VoteChoice::Against => against_votes += vote.weight,
                VoteChoice::Abstain => {} // Abstain votes don't count toward pass/fail
            }
        }

        // Simple majority rule
        let passed = for_votes > against_votes && total_votes > 0;

        info!(
            "Proposal {} voting results: For={}, Against={}, Total={}, Passed={}",
            proposal.id, for_votes, against_votes, total_votes, passed
        );

        Ok(passed)
    }

    /// Execute model approval
    async fn execute_model_approval(&self, model_id: &ModelId) -> Result<()> {
        info!("Executing model approval for: {:?}", model_id);
        // This would update the model status to approved
        // Implementation depends on the registry structure
        Ok(())
    }

    /// Execute fee change
    async fn execute_fee_change(&self, fee_type: &FeeType, new_fee: u64) -> Result<()> {
        info!("Executing fee change: {:?} -> {}", fee_type, new_fee);
        // This would update the fee structure
        // Implementation depends on the fee management system
        Ok(())
    }

    /// Execute parameter change
    async fn execute_parameter_change(&self, parameter: &str, new_value: &str) -> Result<()> {
        info!("Executing parameter change: {} -> {}", parameter, new_value);
        // This would update the configuration parameter
        // Implementation depends on the configuration system
        Ok(())
    }

    /// Execute emergency action
    async fn execute_emergency_action(&self, action: &str) -> Result<()> {
        info!("Executing emergency action: {}", action);
        // This would execute the emergency action
        // Implementation depends on the specific action
        Ok(())
    }

    /// Get voting power for an address
    fn get_voting_power(&self, address: &str) -> Result<u64> {
        // This is a simplified implementation
        // In a real implementation, this would query the blockchain for actual voting power
        Ok(self.voting_power.get(address).copied().unwrap_or(0))
    }

    /// Set voting power for an address
    pub fn set_voting_power(&mut self, address: String, power: u64) {
        self.voting_power.insert(address, power);
    }
}
