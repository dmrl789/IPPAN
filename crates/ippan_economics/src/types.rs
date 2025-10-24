use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Round index (HashTimer-anchored, monotonically increasing)
pub type RoundIndex = u64;

/// Monetary amount in micro-IPN (μIPN)
/// 1 IPN = 1 000 000 μIPN
pub type MicroIPN = u128;

/// Validator identifier (public key or `.ipn` handle)
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ValidatorId(pub String);

/// Participation role within a round
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    /// Default role — validator verifying others’ work
    #[default]
    Verifier,
    /// Validator proposing a block or DAG event
    Proposer,
}

/// Per-validator participation in a round.
/// `blocks` = number of micro-blocks, votes, or attestations credited to this validator.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Participation {
    pub role: Role,
    pub blocks: u32,
}

/// Mapping of validator → participation
pub type ParticipationSet = HashMap<ValidatorId, Participation>;

/// Per-validator payout result (in μIPN)
pub type Payouts = HashMap<ValidatorId, MicroIPN>;

/// Helper constant — 1 IPN = 1 000 000 μIPN
pub const MICRO_PER_IPN: MicroIPN = 1_000_000;
