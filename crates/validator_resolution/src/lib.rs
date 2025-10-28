//! Validator ID Resolution Service
//!
//! Provides resolution of ValidatorId to public keys with support for:
//! - Direct public key identifiers
//! - Human-readable handles (resolved via L2 handle registry)
//! - Registry aliases

pub mod errors;
pub mod resolver;
pub mod types;

pub use errors::*;
pub use resolver::*;
pub use types::*;
