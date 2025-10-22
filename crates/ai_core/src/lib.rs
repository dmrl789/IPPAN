//! AI Core module for IPPAN blockchain
//! 
//! This module provides deterministic AI model execution and validation capabilities
//! for the IPPAN blockchain. It ensures that AI model inference is reproducible
//! and verifiable across all nodes in the network.

pub mod execution;
pub mod models;
pub mod validation;
pub mod determinism;
pub mod types;
pub mod errors;

pub use execution::*;
pub use models::*;
pub use validation::*;
pub use determinism::*;
pub use types::*;
pub use errors::*;

/// Version of the AI Core module
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Feature flags for the AI Core module
pub mod features {
    /// Enable deterministic execution mode
    pub const DETERMINISTIC: &str = "deterministic";
    /// Enable GPU acceleration
    pub const GPU: &str = "gpu";
    /// Enable ONNX model support
    pub const ONNX: &str = "onnx";
    /// Enable HDF5 data format support
    pub const HDF5: &str = "hdf5";
    /// Enable Protocol Buffers support
    pub const PROTOBUF: &str = "protobuf";
}