//! Fee Schedule Governance Module
//!
//! Handles on-chain governance proposals for updating:
//! - Transaction fee schedules (FeeScheduleV1)
//! - Handle/domain registry price schedules (RegistryPriceScheduleV1)
//!
//! ## Key Features
//!
//! - **Timelock**: Minimum 2 epochs (NOTICE_EPOCHS) between proposal and activation
//! - **Rate Limits**: Each field can only change ±20% per epoch
//! - **Minimum Floors**: Base fees must be >= 1 kambei
//! - **Deterministic Activation**: Schedules take effect exactly at epoch start
//! - **Quorum + Supermajority**: Configurable voting thresholds

use anyhow::{anyhow, Result};
use ippan_types::{Epoch, FeeScheduleV1, RegistryPriceScheduleV1};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn};

// =============================================================================
// CONSTANTS
// =============================================================================

/// Minimum number of epochs between proposal creation and activation
pub const NOTICE_EPOCHS: u64 = 2;

/// Quorum required for fee schedule proposals (basis points, 10000 = 100%)
pub const QUORUM_BPS: u16 = 6000; // 60% of voting power must participate

/// Approval threshold for passing (basis points) - 2/3 supermajority
pub const APPROVAL_BPS: u16 = 6667; // 2/3 ≈ 66.67% of decisive votes must be "yes"

/// Maximum number of concurrent fee schedule proposals
pub const MAX_CONCURRENT_PROPOSALS: usize = 5;

// =============================================================================
// FEE SCHEDULE PROPOSAL
// =============================================================================

/// Proposal to change the transaction fee schedule
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeeScheduleProposal {
    /// Unique proposal identifier
    pub proposal_id: [u8; 32],
    /// Proposer's address/public key
    pub proposer: [u8; 32],
    /// New schedule to activate
    pub new_schedule: FeeScheduleV1,
    /// Current schedule at time of proposal (for rate limit validation)
    pub current_schedule: FeeScheduleV1,
    /// Epoch when proposal was created
    pub created_epoch: Epoch,
    /// Epoch when the new schedule will take effect (must be >= created_epoch + NOTICE_EPOCHS)
    pub activation_epoch: Epoch,
    /// Hash of the reason/justification document
    pub reason_hash: [u8; 32],
    /// Proposal status
    pub status: ProposalStatus,
    /// Vote tally
    pub votes: VoteTally,
}

/// Proposal to change the registry price schedule
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryPriceProposal {
    /// Unique proposal identifier
    pub proposal_id: [u8; 32],
    /// Proposer's address/public key
    pub proposer: [u8; 32],
    /// New schedule to activate
    pub new_schedule: RegistryPriceScheduleV1,
    /// Current schedule at time of proposal
    pub current_schedule: RegistryPriceScheduleV1,
    /// Epoch when proposal was created
    pub created_epoch: Epoch,
    /// Epoch when the new schedule will take effect
    pub activation_epoch: Epoch,
    /// Hash of the reason/justification document
    pub reason_hash: [u8; 32],
    /// Proposal status
    pub status: ProposalStatus,
    /// Vote tally
    pub votes: VoteTally,
}

/// Status of a fee schedule proposal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposalStatus {
    /// Proposal is open for voting
    Voting,
    /// Proposal passed and is awaiting activation
    Passed,
    /// Proposal was rejected
    Rejected,
    /// Proposal has been activated
    Activated,
    /// Proposal was cancelled by proposer
    Cancelled,
    /// Proposal expired without reaching quorum
    Expired,
}

/// Vote tally for a proposal
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoteTally {
    /// Total voting power of "yes" votes
    pub yes_power: u64,
    /// Total voting power of "no" votes
    pub no_power: u64,
    /// Total voting power of "abstain" votes
    pub abstain_power: u64,
    /// Map of voter -> vote choice
    pub voters: HashMap<[u8; 32], VoteChoice>,
}

/// Individual vote choice
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoteChoice {
    Yes,
    No,
    Abstain,
}

impl VoteTally {
    /// Total voting power that participated
    pub fn total_participated(&self) -> u64 {
        self.yes_power
            .saturating_add(self.no_power)
            .saturating_add(self.abstain_power)
    }

    /// Check if quorum is met given total voting power
    pub fn has_quorum(&self, total_voting_power: u64) -> bool {
        let participation_bps = if total_voting_power == 0 {
            0
        } else {
            (self.total_participated() as u128 * 10000 / total_voting_power as u128) as u16
        };
        participation_bps >= QUORUM_BPS
    }

    /// Check if approval threshold is met (2/3 supermajority)
    pub fn has_approval(&self) -> bool {
        let total_decisive = self.yes_power.saturating_add(self.no_power);
        if total_decisive == 0 {
            return false;
        }
        let yes_bps = (self.yes_power as u128 * 10000 / total_decisive as u128) as u16;
        yes_bps >= APPROVAL_BPS
    }

    /// Record a vote
    pub fn record_vote(&mut self, voter: [u8; 32], choice: VoteChoice, voting_power: u64) {
        // Remove previous vote if any
        if let Some(prev_choice) = self.voters.remove(&voter) {
            match prev_choice {
                VoteChoice::Yes => self.yes_power = self.yes_power.saturating_sub(voting_power),
                VoteChoice::No => self.no_power = self.no_power.saturating_sub(voting_power),
                VoteChoice::Abstain => {
                    self.abstain_power = self.abstain_power.saturating_sub(voting_power)
                }
            }
        }

        // Record new vote
        self.voters.insert(voter, choice);
        match choice {
            VoteChoice::Yes => self.yes_power = self.yes_power.saturating_add(voting_power),
            VoteChoice::No => self.no_power = self.no_power.saturating_add(voting_power),
            VoteChoice::Abstain => {
                self.abstain_power = self.abstain_power.saturating_add(voting_power)
            }
        }
    }
}

// =============================================================================
// PROPOSAL VALIDATION
// =============================================================================

impl FeeScheduleProposal {
    /// Create a new fee schedule proposal
    pub fn new(
        proposal_id: [u8; 32],
        proposer: [u8; 32],
        new_schedule: FeeScheduleV1,
        current_schedule: FeeScheduleV1,
        created_epoch: Epoch,
        activation_epoch: Epoch,
        reason_hash: [u8; 32],
    ) -> Result<Self> {
        let proposal = Self {
            proposal_id,
            proposer,
            new_schedule,
            current_schedule,
            created_epoch,
            activation_epoch,
            reason_hash,
            status: ProposalStatus::Voting,
            votes: VoteTally::default(),
        };
        proposal.validate()?;
        Ok(proposal)
    }

    /// Validate the proposal
    pub fn validate(&self) -> Result<()> {
        // Check timelock (minimum NOTICE_EPOCHS)
        if self.activation_epoch < self.created_epoch + NOTICE_EPOCHS {
            return Err(anyhow!(
                "Activation epoch {} must be at least {} epochs after creation {}",
                self.activation_epoch,
                NOTICE_EPOCHS,
                self.created_epoch
            ));
        }

        // Validate new schedule floors
        self.new_schedule.validate()?;

        // Check rate limits (±20% per epoch)
        self.new_schedule
            .validate_rate_limits(&self.current_schedule)?;

        // Ensure version incremented
        if self.new_schedule.version <= self.current_schedule.version {
            return Err(anyhow!(
                "New schedule version {} must be greater than current {}",
                self.new_schedule.version,
                self.current_schedule.version
            ));
        }

        Ok(())
    }

    /// Check if this proposal can be activated at the given epoch
    pub fn can_activate(&self, epoch: Epoch, total_voting_power: u64) -> bool {
        self.status == ProposalStatus::Passed
            && epoch >= self.activation_epoch
            && self.votes.has_quorum(total_voting_power)
            && self.votes.has_approval()
    }

    /// Finalize voting and update status
    pub fn finalize_voting(&mut self, total_voting_power: u64) {
        if self.status != ProposalStatus::Voting {
            return;
        }

        if !self.votes.has_quorum(total_voting_power) {
            self.status = ProposalStatus::Expired;
            warn!(
                proposal_id = hex::encode(self.proposal_id),
                "Fee schedule proposal expired - quorum not reached"
            );
        } else if self.votes.has_approval() {
            self.status = ProposalStatus::Passed;
            info!(
                proposal_id = hex::encode(self.proposal_id),
                activation_epoch = self.activation_epoch,
                "Fee schedule proposal passed"
            );
        } else {
            self.status = ProposalStatus::Rejected;
            info!(
                proposal_id = hex::encode(self.proposal_id),
                "Fee schedule proposal rejected - supermajority not reached"
            );
        }
    }

    /// Mark the proposal as activated
    pub fn mark_activated(&mut self) {
        self.status = ProposalStatus::Activated;
        info!(
            proposal_id = hex::encode(self.proposal_id),
            version = self.new_schedule.version,
            "Fee schedule activated"
        );
    }
}

impl RegistryPriceProposal {
    /// Create a new registry price proposal
    pub fn new(
        proposal_id: [u8; 32],
        proposer: [u8; 32],
        new_schedule: RegistryPriceScheduleV1,
        current_schedule: RegistryPriceScheduleV1,
        created_epoch: Epoch,
        activation_epoch: Epoch,
        reason_hash: [u8; 32],
    ) -> Result<Self> {
        let proposal = Self {
            proposal_id,
            proposer,
            new_schedule,
            current_schedule,
            created_epoch,
            activation_epoch,
            reason_hash,
            status: ProposalStatus::Voting,
            votes: VoteTally::default(),
        };
        proposal.validate()?;
        Ok(proposal)
    }

    /// Validate the proposal
    pub fn validate(&self) -> Result<()> {
        // Check timelock
        if self.activation_epoch < self.created_epoch + NOTICE_EPOCHS {
            return Err(anyhow!(
                "Activation epoch {} must be at least {} epochs after creation {}",
                self.activation_epoch,
                NOTICE_EPOCHS,
                self.created_epoch
            ));
        }

        // Validate new schedule floors
        self.new_schedule.validate()?;

        // Ensure version incremented
        if self.new_schedule.version <= self.current_schedule.version {
            return Err(anyhow!(
                "New schedule version {} must be greater than current {}",
                self.new_schedule.version,
                self.current_schedule.version
            ));
        }

        Ok(())
    }

    /// Check if this proposal can be activated at the given epoch
    pub fn can_activate(&self, epoch: Epoch, total_voting_power: u64) -> bool {
        self.status == ProposalStatus::Passed
            && epoch >= self.activation_epoch
            && self.votes.has_quorum(total_voting_power)
            && self.votes.has_approval()
    }

    /// Finalize voting and update status
    pub fn finalize_voting(&mut self, total_voting_power: u64) {
        if self.status != ProposalStatus::Voting {
            return;
        }

        if !self.votes.has_quorum(total_voting_power) {
            self.status = ProposalStatus::Expired;
        } else if self.votes.has_approval() {
            self.status = ProposalStatus::Passed;
        } else {
            self.status = ProposalStatus::Rejected;
        }
    }

    /// Mark the proposal as activated
    pub fn mark_activated(&mut self) {
        self.status = ProposalStatus::Activated;
    }
}

// =============================================================================
// FEE SCHEDULE MANAGER
// =============================================================================

/// Manages the active fee schedules and pending proposals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeScheduleManager {
    /// Current active transaction fee schedule
    pub active_fee_schedule: FeeScheduleV1,
    /// Current active registry price schedule
    pub active_registry_schedule: RegistryPriceScheduleV1,
    /// Pending fee schedule proposals
    pub pending_fee_proposals: HashMap<[u8; 32], FeeScheduleProposal>,
    /// Pending registry price proposals
    pub pending_registry_proposals: HashMap<[u8; 32], RegistryPriceProposal>,
    /// Historical fee schedule changes (for audit)
    pub fee_schedule_history: Vec<(Epoch, FeeScheduleV1)>,
    /// Historical registry price changes
    pub registry_schedule_history: Vec<(Epoch, RegistryPriceScheduleV1)>,
}

impl Default for FeeScheduleManager {
    fn default() -> Self {
        Self::new()
    }
}

impl FeeScheduleManager {
    /// Create a new manager with default schedules
    pub fn new() -> Self {
        Self {
            active_fee_schedule: FeeScheduleV1::default(),
            active_registry_schedule: RegistryPriceScheduleV1::default(),
            pending_fee_proposals: HashMap::new(),
            pending_registry_proposals: HashMap::new(),
            fee_schedule_history: vec![(0, FeeScheduleV1::default())],
            registry_schedule_history: vec![(0, RegistryPriceScheduleV1::default())],
        }
    }

    /// Submit a fee schedule proposal
    pub fn submit_fee_proposal(&mut self, proposal: FeeScheduleProposal) -> Result<()> {
        if self.pending_fee_proposals.len() >= MAX_CONCURRENT_PROPOSALS {
            return Err(anyhow!(
                "Maximum {} concurrent proposals reached",
                MAX_CONCURRENT_PROPOSALS
            ));
        }

        if self
            .pending_fee_proposals
            .contains_key(&proposal.proposal_id)
        {
            return Err(anyhow!("Proposal ID already exists"));
        }

        // Ensure proposal uses current schedule for rate limit validation
        if proposal.current_schedule != self.active_fee_schedule {
            return Err(anyhow!(
                "Proposal current_schedule does not match active schedule"
            ));
        }

        proposal.validate()?;
        self.pending_fee_proposals
            .insert(proposal.proposal_id, proposal);
        Ok(())
    }

    /// Submit a registry price proposal
    pub fn submit_registry_proposal(&mut self, proposal: RegistryPriceProposal) -> Result<()> {
        if self.pending_registry_proposals.len() >= MAX_CONCURRENT_PROPOSALS {
            return Err(anyhow!(
                "Maximum {} concurrent proposals reached",
                MAX_CONCURRENT_PROPOSALS
            ));
        }

        if self
            .pending_registry_proposals
            .contains_key(&proposal.proposal_id)
        {
            return Err(anyhow!("Proposal ID already exists"));
        }

        proposal.validate()?;
        self.pending_registry_proposals
            .insert(proposal.proposal_id, proposal);
        Ok(())
    }

    /// Vote on a fee schedule proposal
    pub fn vote_fee_proposal(
        &mut self,
        proposal_id: &[u8; 32],
        voter: [u8; 32],
        choice: VoteChoice,
        voting_power: u64,
    ) -> Result<()> {
        let proposal = self
            .pending_fee_proposals
            .get_mut(proposal_id)
            .ok_or_else(|| anyhow!("Proposal not found"))?;

        if proposal.status != ProposalStatus::Voting {
            return Err(anyhow!("Proposal is not in voting status"));
        }

        proposal.votes.record_vote(voter, choice, voting_power);
        Ok(())
    }

    /// Vote on a registry price proposal
    pub fn vote_registry_proposal(
        &mut self,
        proposal_id: &[u8; 32],
        voter: [u8; 32],
        choice: VoteChoice,
        voting_power: u64,
    ) -> Result<()> {
        let proposal = self
            .pending_registry_proposals
            .get_mut(proposal_id)
            .ok_or_else(|| anyhow!("Proposal not found"))?;

        if proposal.status != ProposalStatus::Voting {
            return Err(anyhow!("Proposal is not in voting status"));
        }

        proposal.votes.record_vote(voter, choice, voting_power);
        Ok(())
    }

    /// Process proposals at epoch boundary
    /// - Finalize voting for proposals whose voting period ended
    /// - Activate passed proposals whose activation_epoch arrived
    pub fn process_epoch(&mut self, current_epoch: Epoch, total_voting_power: u64) {
        // Process fee schedule proposals
        let fee_proposal_ids: Vec<_> = self.pending_fee_proposals.keys().copied().collect();
        for proposal_id in fee_proposal_ids {
            if let Some(proposal) = self.pending_fee_proposals.get_mut(&proposal_id) {
                // Finalize voting if needed
                if proposal.status == ProposalStatus::Voting {
                    proposal.finalize_voting(total_voting_power);
                }

                // Activate if ready
                if proposal.can_activate(current_epoch, total_voting_power) {
                    self.active_fee_schedule = proposal.new_schedule;
                    self.fee_schedule_history
                        .push((current_epoch, proposal.new_schedule));
                    proposal.mark_activated();

                    info!(
                        epoch = current_epoch,
                        version = proposal.new_schedule.version,
                        "Fee schedule activated at epoch boundary"
                    );
                }
            }
        }

        // Process registry price proposals
        let registry_proposal_ids: Vec<_> =
            self.pending_registry_proposals.keys().copied().collect();
        for proposal_id in registry_proposal_ids {
            if let Some(proposal) = self.pending_registry_proposals.get_mut(&proposal_id) {
                if proposal.status == ProposalStatus::Voting {
                    proposal.finalize_voting(total_voting_power);
                }

                if proposal.can_activate(current_epoch, total_voting_power) {
                    self.active_registry_schedule = proposal.new_schedule;
                    self.registry_schedule_history
                        .push((current_epoch, proposal.new_schedule));
                    proposal.mark_activated();
                }
            }
        }

        // Clean up completed proposals (keep only last 100)
        self.pending_fee_proposals
            .retain(|_, p| matches!(p.status, ProposalStatus::Voting | ProposalStatus::Passed));
        self.pending_registry_proposals
            .retain(|_, p| matches!(p.status, ProposalStatus::Voting | ProposalStatus::Passed));
    }

    /// Get the active fee schedule
    pub fn fee_schedule(&self) -> &FeeScheduleV1 {
        &self.active_fee_schedule
    }

    /// Get the active registry price schedule
    pub fn registry_schedule(&self) -> &RegistryPriceScheduleV1 {
        &self.active_registry_schedule
    }

    /// Get fee schedule at a specific epoch (for historical queries)
    pub fn fee_schedule_at_epoch(&self, epoch: Epoch) -> FeeScheduleV1 {
        self.fee_schedule_history
            .iter()
            .rev()
            .find(|(e, _)| *e <= epoch)
            .map(|(_, s)| *s)
            .unwrap_or_default()
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_proposal_id(seed: u8) -> [u8; 32] {
        let mut id = [0u8; 32];
        id[0] = seed;
        id
    }

    fn make_proposer(seed: u8) -> [u8; 32] {
        let mut id = [0u8; 32];
        id[0] = seed;
        id
    }

    fn make_reason_hash() -> [u8; 32] {
        [0xABu8; 32]
    }

    #[test]
    fn test_fee_schedule_proposal_creation() {
        let current = FeeScheduleV1::default();
        let mut new = current;
        new.version = 2;
        new.tx_base_fee = current.tx_base_fee * 11 / 10; // +10%

        let proposal = FeeScheduleProposal::new(
            make_proposal_id(1),
            make_proposer(1),
            new,
            current,
            0,
            2, // activation at epoch 2 (>= 0 + NOTICE_EPOCHS)
            make_reason_hash(),
        );

        assert!(proposal.is_ok());
    }

    #[test]
    fn test_fee_schedule_proposal_timelock_violation() {
        let current = FeeScheduleV1::default();
        let mut new = current;
        new.version = 2;

        // Try to activate too soon
        let proposal = FeeScheduleProposal::new(
            make_proposal_id(1),
            make_proposer(1),
            new,
            current,
            5,
            6, // Only 1 epoch notice, need NOTICE_EPOCHS (2)
            make_reason_hash(),
        );

        assert!(proposal.is_err());
    }

    #[test]
    fn test_fee_schedule_rate_limit_violation() {
        let current = FeeScheduleV1::default();
        let mut new = current;
        new.version = 2;
        new.tx_base_fee = current.tx_base_fee * 2; // +100% - exceeds ±20%

        let proposal = FeeScheduleProposal::new(
            make_proposal_id(1),
            make_proposer(1),
            new,
            current,
            0,
            2,
            make_reason_hash(),
        );

        assert!(proposal.is_err());
    }

    #[test]
    fn test_vote_tally_quorum() {
        let mut tally = VoteTally::default();
        let total_voting_power = 1000;

        // 50% participation - below quorum (60%)
        tally.record_vote([1u8; 32], VoteChoice::Yes, 500);
        assert!(!tally.has_quorum(total_voting_power));

        // 60% participation - meets quorum exactly
        tally.record_vote([2u8; 32], VoteChoice::No, 100);
        assert!(tally.has_quorum(total_voting_power));
    }

    #[test]
    fn test_vote_tally_approval() {
        let mut tally = VoteTally::default();

        // 60% yes - below 2/3 approval threshold
        tally.record_vote([1u8; 32], VoteChoice::Yes, 600);
        tally.record_vote([2u8; 32], VoteChoice::No, 400);
        assert!(!tally.has_approval());

        // 66.5% yes - still below 2/3 (66.67%)
        let mut tally2 = VoteTally::default();
        tally2.record_vote([1u8; 32], VoteChoice::Yes, 665);
        tally2.record_vote([2u8; 32], VoteChoice::No, 335);
        assert!(!tally2.has_approval());

        // 66.7% yes - meets 2/3 approval threshold
        let mut tally3 = VoteTally::default();
        tally3.record_vote([1u8; 32], VoteChoice::Yes, 667);
        tally3.record_vote([2u8; 32], VoteChoice::No, 333);
        assert!(tally3.has_approval());

        // 70% yes - above approval threshold
        let mut tally4 = VoteTally::default();
        tally4.record_vote([1u8; 32], VoteChoice::Yes, 700);
        tally4.record_vote([2u8; 32], VoteChoice::No, 300);
        assert!(tally4.has_approval());
    }

    #[test]
    fn test_fee_schedule_manager_flow() {
        let mut manager = FeeScheduleManager::new();
        let current = manager.active_fee_schedule;

        // Create a valid proposal
        let mut new = current;
        new.version = 2;
        new.tx_base_fee = current.tx_base_fee * 11 / 10;

        let proposal = FeeScheduleProposal::new(
            make_proposal_id(1),
            make_proposer(1),
            new,
            current,
            0,
            2,
            make_reason_hash(),
        )
        .unwrap();

        // Submit proposal
        manager.submit_fee_proposal(proposal).unwrap();

        // Vote: 60% quorum + 2/3 approval
        // Need 600/1000 participation for quorum (60%)
        // Need 66.67% of decisive votes for approval
        // 500 yes + 100 no = 600 participation (60%) with 500/600 = 83.3% approval
        manager
            .vote_fee_proposal(&make_proposal_id(1), [10u8; 32], VoteChoice::Yes, 500)
            .unwrap();
        manager
            .vote_fee_proposal(&make_proposal_id(1), [11u8; 32], VoteChoice::No, 100)
            .unwrap();

        // Process at activation epoch
        manager.process_epoch(2, 1000);

        // Check schedule was activated
        assert_eq!(manager.active_fee_schedule.version, 2);
        assert_eq!(manager.active_fee_schedule.tx_base_fee, new.tx_base_fee);
    }

    #[test]
    fn test_max_concurrent_proposals() {
        let mut manager = FeeScheduleManager::new();
        let current = manager.active_fee_schedule;

        // Submit max proposals
        for i in 0..MAX_CONCURRENT_PROPOSALS {
            let mut new = current;
            new.version = (i + 2) as u32;
            new.tx_base_fee = current.tx_base_fee * 11 / 10;

            let proposal = FeeScheduleProposal::new(
                make_proposal_id(i as u8),
                make_proposer(1),
                new,
                current,
                0,
                2,
                make_reason_hash(),
            )
            .unwrap();

            manager.submit_fee_proposal(proposal).unwrap();
        }

        // Next one should fail
        let mut new = current;
        new.version = 100;
        let proposal = FeeScheduleProposal::new(
            make_proposal_id(99),
            make_proposer(1),
            new,
            current,
            0,
            2,
            make_reason_hash(),
        )
        .unwrap();

        assert!(manager.submit_fee_proposal(proposal).is_err());
    }

    #[test]
    fn test_fee_schedule_at_epoch() {
        let manager = FeeScheduleManager::new();

        // Initial schedule at epoch 0
        let v1 = manager.fee_schedule_at_epoch(0);
        assert_eq!(v1.version, 1);

        // Check any epoch before changes
        let v1_later = manager.fee_schedule_at_epoch(10);
        assert_eq!(v1_later.version, 1);
    }
}
