//! IPPAN Governance Module
//!
//! Provides on-chain governance primitives for:
//! - ✅ Proposal creation and voting
//! - 🤖 AI model approval (via `ippan-ai-registry`)
//! - ⚙️ Protocol parameter updates
//!
//! All governance actions are deterministic, time-bounded by HashTimer rounds,
//! and cryptographically signed by authorized validators or domain owners.

pub mod ai_models;
pub mod parameters;
pub mod voting;

pub use ai_models::*;
pub use parameters::*;
pub use voting::*;

/// Governance module version (for API introspection)
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
