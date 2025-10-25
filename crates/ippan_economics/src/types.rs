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

/// Validator identifier
/// 
/// Can be either:
/// - Ed25519 public key (32 bytes, hex-encoded)
/// - Human-readable handle (e.g., @user.ipn) - resolved via L2 handle registry
/// - Registry alias (custom identifier)
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ValidatorId(pub String);

impl ValidatorId {
    /// Create a new ValidatorId from string
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
    
    /// Get the ID as string
    pub fn as_str(&self) -> &str {
        &self.0
    }
    
    /// Check if this is a human-readable handle (starts with @)
    pub fn is_handle(&self) -> bool {
        self.0.starts_with('@')
    }
    
    /// Check if this is a public key (64 hex characters)
    pub fn is_public_key(&self) -> bool {
        self.0.len() == 64 && self.0.chars().all(|c| c.is_ascii_hexdigit())
    }
    
    /// Check if this is a registry alias (not a handle or public key)
    pub fn is_alias(&self) -> bool {
        !self.is_handle() && !self.is_public_key()
    }
}

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
