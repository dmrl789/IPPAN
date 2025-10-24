use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Round index (HashTimer-anchored, monotonically increasing)
pub type RoundIndex = u64;

/// Monetary amount in micro-IPN (μIPN)
/// 1 IPN = 1_000_000 μIPN
pub type MicroIPN = u128;

/// Validator identifier (your system likely uses ed25519 pubkey or .ipn handle)
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ValidatorId(pub String);

/// Participation role within a round
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    #[default]
    Verifier,
    Proposer,
}

/// Per-validator participation in a round.
/// `blocks` is the number of micro-blocks (or votes/attestations) attributable to the validator.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Participation {
    pub role: Role,
    pub blocks: u32,
}

/// Mapping validator -> participation
pub type ParticipationSet = HashMap<ValidatorId, Participation>;

/// Per-validator payout result (μIPN)
pub type Payouts = HashMap<ValidatorId, MicroIPN>;

/// Helper constants
pub const MICRO_PER_IPN: MicroIPN = 1_000_000;
