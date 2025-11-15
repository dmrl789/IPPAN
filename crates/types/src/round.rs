use crate::block::{BlockId, RoundId};
use crate::IppanTimeMicros;
use serde::{Deserialize, Serialize};
use serde_bytes;

/// Time window metadata for a consensus round.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RoundWindow {
    pub id: RoundId,
    pub start_us: IppanTimeMicros,
    pub end_us: IppanTimeMicros,
}

/// Aggregated attestation that a round's blocks have been observed.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RoundCertificate {
    pub round: RoundId,
    pub block_ids: Vec<BlockId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty", with = "serde_bytes")]
    pub agg_sig: Vec<u8>,
}

/// Finalization record describing the ordered execution of a round.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RoundFinalizationRecord {
    pub round: RoundId,
    pub window: RoundWindow,
    pub ordered_tx_ids: Vec<[u8; 32]>,
    pub fork_drops: Vec<[u8; 32]>,
    pub state_root: [u8; 32],
    pub proof: RoundCertificate,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_fees_atomic: Option<u128>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub treasury_fees_atomic: Option<u128>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applied_payments: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejected_payments: Option<u64>,
}
