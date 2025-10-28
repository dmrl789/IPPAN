//! L2 Handle Registry for Human-Readable ID Management
//!
//! This crate provides L2 storage and resolution for human-readable identifiers
//! like `@user.ipn`, `@device.iot`, etc. The L1 only stores ownership anchors,
//! while the actual handle mappings and metadata live on L2.

pub mod errors;
pub mod registry;
pub mod resolution;
pub mod types;

pub use errors::*;
pub use registry::*;
pub use resolution::*;
pub use types::*;
