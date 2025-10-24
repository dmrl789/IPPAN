//! Core types for the IPPAN Economics Engine
//!
//! Defines monetary units, validator identifiers, participation records,
//! and system-wide economic parameters for DAG-Fair emission and distribution.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Round index (HashTimer-anchored, monotonically increasing)
pub type RoundIndex = u64;

/// Monetary amount in micro-IPN (μIPN)
/// 1 IPN = 1 000 000 μIPN
pub type MicroIPN = u128;

/// Validator identifier (Ed25519 public key, .ipn handle, or registry alias)
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

/// Per-validator participation record for a round.
/// `blocks` = number of micro-blocks, votes, or attestations credited.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Participation {
    pub role: Role,
    pub blocks: u32,
}

/// Mapping validator → participation
pub type ParticipationSet = HashMap<ValidatorId, Participation>;

/// Per-validator payout result (μIPN)
pub type Payouts = HashMap<ValidatorId, MicroIPN>;

/// Helper constant — 1 IPN = 1 000 000 μIPN
pub const MICRO_PER_IPN: MicroIPN = 1_000_000;
