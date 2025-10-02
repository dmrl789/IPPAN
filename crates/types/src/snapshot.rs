use crate::hashtimer::IppanTimeMicros;
use crate::{BlockId, RoundId, ValidatorId};
use serde::{Deserialize, Serialize};
use serde_bytes;

/// Metadata describing a validator's weight inside a snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnapshotValidator {
    /// Validator identifier.
    pub validator_id: ValidatorId,
    /// Stake weight (opaque unit, defaults to 1 for PoA setups).
    pub weight: u64,
}

impl SnapshotValidator {
    /// Create a new validator entry with the provided weight.
    pub fn new(validator_id: ValidatorId, weight: u64) -> Self {
        Self {
            validator_id,
            weight,
        }
    }
}

/// Signed snapshot checkpoint anchoring the chain state for fast sync.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateSnapshot {
    /// Commitment to the global state after executing the referenced block.
    pub state_root: [u8; 32],
    /// Canonical block identifier backing the snapshot.
    pub block_hash: BlockId,
    /// Block height the snapshot corresponds to.
    pub block_height: u64,
    /// Consensus round identifier for the block.
    pub round_id: RoundId,
    /// Active validator set that co-signed the checkpoint.
    pub validator_set: Vec<SnapshotValidator>,
    /// Aggregated signature attesting to the snapshot contents.
    #[serde(with = "serde_bytes")]
    pub signature: Vec<u8>,
    /// Time the snapshot was produced.
    pub produced_at: IppanTimeMicros,
}

impl StateSnapshot {
    /// Create a new snapshot with an empty validator signature.
    pub fn unsigned(
        state_root: [u8; 32],
        block_hash: BlockId,
        block_height: u64,
        round_id: RoundId,
        validator_set: Vec<SnapshotValidator>,
        produced_at: IppanTimeMicros,
    ) -> Self {
        Self {
            state_root,
            block_hash,
            block_height,
            round_id,
            validator_set,
            signature: Vec::new(),
            produced_at,
        }
    }

    /// Attach an aggregated signature to the snapshot.
    pub fn with_signature(mut self, signature: Vec<u8>) -> Self {
        self.signature = signature;
        self
    }
}
