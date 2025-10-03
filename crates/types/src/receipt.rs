use serde::{Deserialize, Serialize};

/// Execution receipt for a transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    /// ID of the transaction the receipt corresponds to.
    pub tx_id: String,
    /// Outcome/status code (e.g., success or application-specific error).
    pub status: u16,
    /// Metered resource consumption (e.g., gas used).
    pub resource_used: u64,
    /// Root/commitment of state keys touched by this transaction.
    pub touched_keys_root: String,
    /// Optional opaque proof bytes (e.g., Verkle/Merkle proofs).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proof_blob: Option<Vec<u8>>,
}
