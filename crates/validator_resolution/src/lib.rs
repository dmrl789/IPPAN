//! Validator ID Resolution Service
//!
//! Provides resolution of ValidatorId to public keys with support for:
//! - Direct public key identifiers
//! - Human-readable handles (resolved via L2 handle registry)
//! - Registry aliases

pub mod resolver;
pub mod errors;
pub mod types;

pub use resolver::*;
pub use errors::*;
pub use types::*;