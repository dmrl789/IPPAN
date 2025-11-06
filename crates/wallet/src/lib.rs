//! IPPAN Multi-Address Wallet
//!
//! A comprehensive wallet implementation for managing multiple IPPAN addresses,
//! private keys, and transactions with secure encryption and storage.

pub mod cli;
pub mod crypto;
pub mod errors;
pub mod operations;
pub mod storage;
pub mod types;

pub use errors::*;
pub use operations::*;
pub use types::*;

/// Re-export commonly used types
pub use ippan_types::{Address, Amount, Transaction};
