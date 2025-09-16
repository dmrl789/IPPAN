//! Cryptographic implementations for IPPAN
//! 
//! This module provides real cryptographic functions to replace placeholder implementations.

pub mod real_implementations;
pub mod ed25519_signer;

// Re-export the main cryptographic functions for easy access
pub use real_implementations::{
    RealHashFunctions,
    RealEd25519,
    RealAES256GCM,
    RealZkStarkProof,
    RealTransactionSigner,
};

// Re-export ed25519 signer utilities
pub use ed25519_signer::{
    Ed25519Keypair,
    CanonicalTransaction,
    SignedTransaction,
    TransactionSigner,
};
