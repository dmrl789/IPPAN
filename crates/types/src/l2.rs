use serde::{Deserialize, Serialize};

/// Registration and runtime metadata for an L2 network bridged to IPPAN.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2Network {
    /// Globally unique identifier for the L2 (e.g. rollup name).
    pub id: String,
    /// Proof system used by the L2 (zk, optimistic, etc.).
    pub proof_type: String,
    /// Data availability mode advertised by the L2 (inline/external).
    pub da_mode: String,
    /// Human friendly network status flag.
    pub status: L2NetworkStatus,
    /// Epoch number included in the most recent commit observed on L1.
    pub last_epoch: u64,
    /// Total number of commits that have been accepted for this L2.
    pub total_commits: u64,
    /// Total number of exits processed for this L2.
    pub total_exits: u64,
    /// Unix timestamp in microseconds when the most recent commit was submitted.
    pub last_commit_time: Option<u64>,
    /// Unix timestamp in microseconds when the network was registered.
    pub registered_at: u64,
    /// Optional optimistic challenge window length in milliseconds.
    pub challenge_window_ms: Option<u64>,
}

/// Status flag for an L2 network.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum L2NetworkStatus {
    Active,
    Inactive,
    Challenged,
}

impl Default for L2NetworkStatus {
    fn default() -> Self {
        Self::Active
    }
}

/// Record describing a single L2 state commitment posted to IPPAN.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2Commit {
    /// Unique identifier derived from a HashTimer.
    pub id: String,
    /// L2 identifier the commit belongs to.
    pub l2_id: String,
    /// Epoch the commitment covers.
    pub epoch: u64,
    /// Root hash representing the committed state.
    pub state_root: String,
    /// Hash of the associated data availability payload.
    pub da_hash: String,
    /// Proof system used for this commitment.
    pub proof_type: String,
    /// Optional serialized proof payload.
    pub proof: Option<String>,
    /// Optional inline data payload when the DA mode is inline.
    pub inline_data: Option<String>,
    /// Unix timestamp in microseconds when the commit was accepted.
    pub submitted_at: u64,
    /// HashTimer string used to uniquely identify the commit.
    pub hashtimer: String,
}

/// Status of an exit request that originated on an L2.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum L2ExitStatus {
    Pending,
    Finalized,
    Rejected,
    ChallengeWindow,
}

impl Default for L2ExitStatus {
    fn default() -> Self {
        Self::Pending
    }
}

/// Exit request originating from an L2 network that is tracked on IPPAN.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2ExitRecord {
    /// Unique identifier derived from a HashTimer.
    pub id: String,
    /// L2 identifier the exit originated from.
    pub l2_id: String,
    /// Epoch number the exit proof references.
    pub epoch: u64,
    /// Address/account initiating the exit on the L2.
    pub account: String,
    /// Amount being withdrawn, expressed in native IPPAN units.
    pub amount: f64,
    /// Optional replay protection nonce supplied by the L2.
    pub nonce: Option<u64>,
    /// Raw proof of inclusion that accompanies the exit.
    pub proof_of_inclusion: String,
    /// Status flag representing the current lifecycle state of the exit.
    pub status: L2ExitStatus,
    /// Unix timestamp in microseconds when the exit was first submitted.
    pub submitted_at: u64,
    /// Optional timestamp for when the exit was finalized on L1.
    pub finalized_at: Option<u64>,
    /// Optional rejection reason when an exit is refused.
    pub rejection_reason: Option<String>,
    /// Optional timestamp in microseconds when an optimistic challenge window ends.
    pub challenge_window_ends_at: Option<u64>,
}
