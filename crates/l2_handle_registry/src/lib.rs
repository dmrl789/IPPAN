//! L2 Handle Registry for Human-Readable ID Management
//!
//! This crate provides L2 storage and resolution for human-readable identifiers
//! like `@user.ipn`, `@device.iot`, etc. The L1 only stores ownership anchors,
//! while the actual handle mappings and metadata live on L2.

use std::time::SystemTime;

pub mod registry;
pub mod resolution;
pub mod types;
pub mod errors;

pub use registry::L2HandleRegistry;
pub use resolution::HandleResolver;
pub use types::*;
pub use errors::*;