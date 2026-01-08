//! IPPAN Governance Module
//!
//! Provides on-chain governance primitives for:
//! - ✅ Proposal creation and voting
//! - 🤖 AI model approval (via `ippan-ai-registry`)
//! - ⚙️ Protocol parameter updates
//! - 💰 Fee schedule governance with timelock + rate limits
//!
//! All governance actions are deterministic, time-bounded by HashTimer rounds,
//! and cryptographically signed by authorized validators or domain owners.

// V1-BLOCKER: wire the minimal governance/upgrade path for mainnet (config-gated
// proposals and validator authorization) once the launch policy is finalized.

pub mod ai_models;
pub mod fee_schedule;
pub mod parameters;
pub mod voting;

pub use ai_models::*;
pub use fee_schedule::*;
pub use parameters::*;
pub use voting::*;

/// Governance module version (for API introspection)
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
