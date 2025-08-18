//! IPPAN (Immutable Proof & Availability Network)
//! 
//! A fully decentralized Layer-1 blockchain with built-in global DHT storage.

pub mod config;
pub mod api;
pub mod blockchain;
pub mod consensus;
pub mod quantum;
pub mod iot;
// pub mod dht;
// pub mod domain;
pub mod error;
pub mod network;
pub mod node;
// pub mod staking; // TODO: Fix compilation errors before enabling
pub mod storage;
pub mod security;
pub mod crosschain;
pub mod dns;
pub mod transaction_types; // NEW - User-facing transaction types
// pub mod tests;
pub mod utils;
// pub mod wallet; // TODO: Fix compilation errors before enabling
pub mod monitoring;
pub mod performance_test;
pub mod testing_framework;
pub mod security_audit;

// Re-export commonly used types
pub use config::Config;
pub use error::IppanError;
pub use node::IppanNode;

// Common types used throughout the codebase
pub type Result<T> = std::result::Result<T, IppanError>;
pub type NodeId = [u8; 32];
pub type BlockHash = [u8; 32];
pub type TransactionHash = [u8; 32];

// Constants
pub const IPN_DECIMALS: u32 = 8;
pub const MAX_IPN_SUPPLY: u64 = 21_000_000_000_000_000; // 21M IPN in smallest units
pub const MIN_STAKE_AMOUNT: u64 = 10_000_000_000; // 10 IPN in smallest units
pub const MAX_STAKE_AMOUNT: u64 = 100_000_000_000; // 100 IPN in smallest units
pub const TRANSACTION_FEE_PERCENTAGE: u64 = 1; // 1%
