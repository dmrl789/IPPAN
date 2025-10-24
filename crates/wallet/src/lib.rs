//! IPPAN Multi-Address Wallet
//!
//! A comprehensive wallet implementation for managing multiple IPPAN addresses,
//! private keys, and transactions with secure encryption and storage.

pub mod crypto;
pub mod storage;
pub mod operations;
pub mod types;
pub mod cli;
pub mod errors;

pub use types::*;
pub use operations::*;
pub use errors::*;

/// Re-export commonly used types
pub use ippan_types::{Address, Transaction, Amount};