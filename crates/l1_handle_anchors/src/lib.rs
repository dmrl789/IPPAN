//! L1 Handle Ownership Anchors
//!
//! This crate provides minimal L1 storage for handle ownership proofs.
//! The actual human-readable handle mappings are stored on L2, while
//! L1 only stores ownership anchors for global ordering and interoperability.

pub mod anchors;
pub mod errors;
pub mod types;

pub use anchors::*;
pub use errors::*;
pub use types::*;
