//! Type definitions for the Governance module

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Governance proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceProposal {
    /// Proposal ID
    pub id: String,
    /// Proposal type
    pub proposal_type: ProposalType,
    /// Proposal title
    pub title: String,
    /// Proposal description
    pub description: String,
    /// Proposer address
    pub proposer: String,
    /// Proposal data
    pub data: ProposalData,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Voting deadline
    pub voting_deadline: DateTime<Utc>,
    /// Execution deadline
    pub execution_deadline: Option<DateTime<Utc>>,
    /// Proposal status
    pub status: ProposalStatus,
    /// Votes
    pub votes: Vec<Vote>,
}

/// Proposal type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposalType {
    /// Parameter change proposal
    ParameterChange,
    /// Treasury spending proposal
    TreasurySpending,
    /// Upgrade proposal
    Upgrade,
    /// Emergency proposal
    Emergency,
    /// AI model governance proposal
    AiModelGovernance,
}

/// Proposal data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProposalData {
    /// Parameter change data
    ParameterChange {
        parameter: String,
        old_value: String,
        new_value: String,
        reason: String,
    },
    /// Treasury spending data
    TreasurySpending {
        recipient: String,
        amount: u64,
        purpose: String,
    },
    /// Upgrade data
    Upgrade {
        version: String,
        description: String,
        migration_required: bool,
    },
    /// Emergency data
    Emergency {
        action: String,
        reason: String,
    },
    /// AI model governance data
    AiModelGovernance {
        model_id: String,
        action: String,
        reason: String,
    },
}

/// Proposal status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposalStatus {
    /// Proposal is active and accepting votes
    Active,
    /// Proposal passed
    Passed,
    /// Proposal failed
    Failed,
    /// Proposal executed
    Executed,
    /// Proposal expired
    Expired,
}

/// Vote on a proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    /// Voter address
    pub voter: String,
    /// Vote choice
    pub choice: VoteChoice,
    /// Vote weight
    pub weight: u64,
    /// Vote timestamp
    pub timestamp: DateTime<Utc>,
    /// Vote justification
    pub justification: Option<String>,
}

/// Vote choice
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoteChoice {
    /// Vote for the proposal
    For,
    /// Vote against the proposal
    Against,
    /// Abstain from voting
    Abstain,
}

/// Voting power delegation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delegation {
    /// Delegator address
    pub delegator: String,
    /// Delegate address
    pub delegate: String,
    /// Delegated voting power
    pub power: u64,
    /// Delegation timestamp
    pub timestamp: DateTime<Utc>,
    /// Delegation status
    pub status: DelegationStatus,
}

/// Delegation status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DelegationStatus {
    /// Active delegation
    Active,
    /// Revoked delegation
    Revoked,
    /// Expired delegation
    Expired,
}

/// Treasury transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreasuryTransaction {
    /// Transaction ID
    pub id: String,
    /// Transaction type
    pub transaction_type: TreasuryTransactionType,
    /// Amount
    pub amount: u64,
    /// Recipient
    pub recipient: String,
    /// Purpose
    pub purpose: String,
    /// Execution timestamp
    pub executed_at: DateTime<Utc>,
    /// Executor
    pub executor: String,
    /// Related proposal ID
    pub proposal_id: Option<String>,
}

/// Treasury transaction type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TreasuryTransactionType {
    /// Spending transaction
    Spending,
    /// Revenue transaction
    Revenue,
    /// Transfer transaction
    Transfer,
}

/// Governance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceConfig {
    /// Minimum voting power required to create a proposal
    pub min_proposal_power: u64,
    /// Minimum voting power required to vote
    pub min_voting_power: u64,
    /// Voting period in seconds
    pub voting_period_seconds: u64,
    /// Execution period in seconds
    pub execution_period_seconds: u64,
    /// Proposal fee
    pub proposal_fee: u64,
    /// Maximum proposal description length
    pub max_description_length: usize,
    /// Maximum proposal title length
    pub max_title_length: usize,
}

/// Governance statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceStats {
    /// Total proposals
    pub total_proposals: u64,
    /// Active proposals
    pub active_proposals: u64,
    /// Passed proposals
    pub passed_proposals: u64,
    /// Failed proposals
    pub failed_proposals: u64,
    /// Executed proposals
    pub executed_proposals: u64,
    /// Total voters
    pub total_voters: u64,
    /// Total voting power
    pub total_voting_power: u64,
    /// Treasury balance
    pub treasury_balance: u64,
}