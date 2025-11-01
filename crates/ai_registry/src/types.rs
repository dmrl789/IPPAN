//! Type definitions for the AI Registry module

use chrono::{DateTime, Utc};
use ippan_ai_core::types::{ModelId, ModelMetadata};
use serde::{Deserialize, Serialize};

/// Model registration status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegistrationStatus {
    /// Model is pending approval
    Pending,
    /// Model is approved and active
    Approved,
    /// Model is rejected
    Rejected,
    /// Model is suspended
    Suspended,
    /// Model is deprecated
    Deprecated,
}

/// Model registration record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRegistration {
    /// Model identifier
    pub model_id: ModelId,
    /// Model metadata
    pub metadata: ModelMetadata,
    /// Registration status
    pub status: RegistrationStatus,
    /// Registrant address
    pub registrant: String,
    /// Registration timestamp
    pub registered_at: DateTime<Utc>,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
    /// Registration fee paid
    pub registration_fee: u64,
    /// Model category
    pub category: ModelCategory,
    /// Model tags
    pub tags: Vec<String>,
    /// Model description
    pub description: Option<String>,
    /// Model license
    pub license: Option<String>,
    /// Model source URL
    pub source_url: Option<String>,
}

/// Model category
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ModelCategory {
    #[default]
    /// Natural language processing
    NLP,
    /// Computer vision
    ComputerVision,
    /// Speech processing
    Speech,
    /// Recommendation systems
    Recommendation,
    /// Time series analysis
    TimeSeries,
    /// Generative models
    Generative,
    /// Other
    Other,
}

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
    /// Model approval proposal
    ModelApproval,
    /// Fee change proposal
    FeeChange,
    /// Governance parameter change
    ParameterChange,
    /// Emergency proposal
    Emergency,
}

/// Proposal data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProposalData {
    /// Model approval data
    ModelApproval {
        model_id: ModelId,
        approval_reason: String,
    },
    /// Fee change data
    FeeChange {
        fee_type: FeeType,
        new_fee: u64,
        reason: String,
    },
    /// Parameter change data
    ParameterChange {
        parameter: String,
        old_value: String,
        new_value: String,
        reason: String,
    },
    /// Emergency data
    Emergency { action: String, reason: String },
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

/// Fee type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FeeType {
    /// Model registration fee
    Registration,
    /// Model execution fee
    Execution,
    /// Model storage fee
    Storage,
    /// Governance proposal fee
    Proposal,
    /// Model update fee
    Update,
}

/// Fee structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeStructure {
    /// Fee type
    pub fee_type: FeeType,
    /// Base fee amount
    pub base_fee: u64,
    /// Fee per unit (e.g., per parameter, per execution)
    pub unit_fee: u64,
    /// Minimum fee
    pub min_fee: u64,
    /// Maximum fee
    pub max_fee: u64,
    /// Fee calculation method
    pub calculation_method: FeeCalculationMethod,
}

/// Fee calculation method
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeeCalculationMethod {
    /// Fixed fee
    Fixed,
    /// Linear scaling
    Linear,
    /// Logarithmic scaling
    Logarithmic,
    /// Step function
    Step,
}

/// Model usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsageStats {
    /// Model ID
    pub model_id: ModelId,
    /// Total executions
    pub total_executions: u64,
    /// Successful executions
    pub successful_executions: u64,
    /// Failed executions
    pub failed_executions: u64,
    /// Total fees collected
    pub total_fees_collected: u64,
    /// Last execution timestamp
    pub last_execution: Option<DateTime<Utc>>,
    /// Average execution time in microseconds
    pub avg_execution_time_us: u64,
    /// Unique users count
    pub unique_users: u64,
}

/// Registry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// Minimum registration fee
    pub min_registration_fee: u64,
    /// Maximum registration fee
    pub max_registration_fee: u64,
    /// Default execution fee
    pub default_execution_fee: u64,
    /// Storage fee per byte per day
    pub storage_fee_per_byte_per_day: u64,
    /// Governance proposal fee
    pub proposal_fee: u64,
    /// Voting period in seconds
    pub voting_period_seconds: u64,
    /// Execution period in seconds
    pub execution_period_seconds: u64,
    /// Minimum voting power required
    pub min_voting_power: u64,
    /// Maximum model size in bytes
    pub max_model_size: u64,
    /// Maximum parameter count
    pub max_parameter_count: u64,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            min_registration_fee: 1000,
            max_registration_fee: 1_000_000,
            default_execution_fee: 100,
            storage_fee_per_byte_per_day: 1,
            proposal_fee: 10_000,
            voting_period_seconds: 86400 * 7,     // 7 days
            execution_period_seconds: 86400 * 14, // 14 days
            min_voting_power: 1000,
            max_model_size: 1024 * 1024 * 100,  // 100MB
            max_parameter_count: 1_000_000_000, // 1B parameters
        }
    }
}

/// Registry statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryStats {
    /// Total registered models
    pub total_models: u64,
    /// Active models
    pub active_models: u64,
    /// Pending models
    pub pending_models: u64,
    /// Total executions
    pub total_executions: u64,
    /// Total fees collected
    pub total_fees_collected: u64,
    /// Total registrants
    pub total_registrants: u64,
    /// Average model size
    pub avg_model_size: u64,
    /// Most popular category
    pub most_popular_category: Option<ModelCategory>,
}
