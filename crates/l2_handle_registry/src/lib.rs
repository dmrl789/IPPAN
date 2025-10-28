//! L2 Handle Registry for Human-Readable ID Management
//!
//! This crate provides L2 storage and resolution for human-readable identifiers
//! such as `@user.ipn`, `@device.iot`, `@bank.fin`, etc.
//!
//! Layer 1 (L1) only stores the ownership anchors and root commitments,
//! while Layer 2 (L2) manages the actual handle mappings, metadata, and renewals.

pub mod errors;
pub mod registry;
pub mod resolution;
pub mod types;

pub use errors::*;
pub use registry::*;
pub use resolution::*;
pub use types::*;
