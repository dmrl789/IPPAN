//! Cryptographic implementations for IPPAN
//! 
//! This module provides real cryptographic functions to replace placeholder implementations.

pub mod real_implementations;

// Re-export the main cryptographic functions for easy access
pub use real_implementations::{
    RealHashFunctions,
    RealEd25519,
    RealAES256GCM,
    RealZkStarkProof,
    RealTransactionSigner,
};
