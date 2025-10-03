use serde::{Deserialize, Serialize};

/// Signed snapshot checkpoint used for fast sync bootstrapping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotCheckpoint {
    /// Verkle root of the global state at the checkpoint height.
    pub state_root: String,
    /// Block height associated with the snapshot.
    pub block_height: u64,
    /// Round identifier for the consensus layer.
    pub round_id: String,
    /// Validator set that signed the checkpoint.
    pub validator_set: Vec<String>,
    /// Aggregate signature over the checkpoint payload.
    pub signature: String,
}
