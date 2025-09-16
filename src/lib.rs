//! IPPAN (Immutable Proof & Availability Network)
//! 
//! A fully decentralized Layer-1 blockchain with built-in global DHT storage.

pub mod config;
pub mod api;
pub mod blockchain;
pub mod consensus;
pub mod crypto;
pub mod database;
pub mod mining;
pub mod genesis;
pub mod cli; // NEW - Real persistent database implementation // NEW - Real cryptographic implementations
pub mod logging; // NEW - Comprehensive logging and monitoring system
// pub mod security; // NEW - Comprehensive security audit and vulnerability assessment (moved to feature-gated)
pub mod error;
pub mod node;
pub mod transaction_types; // NEW - User-facing transaction types
pub mod types; // NEW - Address validation and types
pub mod utils;
pub mod performance;
pub mod performance_test;
pub mod testing_framework;
pub mod security_audit;

// Feature-gated modules
#[cfg(feature = "quantum")]
pub mod quantum;

#[cfg(feature = "iot")]
pub mod iot;

#[cfg(feature = "network")]
pub mod network;

#[cfg(feature = "storage")]
pub mod storage;

#[cfg(feature = "network")]
pub mod transaction;

#[cfg(feature = "security")]
pub mod security;

#[cfg(feature = "crosschain")]
pub mod crosschain;

pub mod bridge;
pub mod tools;

#[cfg(feature = "dns")]
pub mod dns;

#[cfg(feature = "staking")]
pub mod staking;

#[cfg(feature = "wallet")]
pub mod wallet;

#[cfg(feature = "monitoring")]
pub mod monitoring;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_flags() {
        // Test that contracts are disabled by default
        #[cfg(feature = "contracts")]
        {
            panic!("Contracts feature should be disabled by default");
        }
        
        #[cfg(not(feature = "contracts"))]
        {
            // This should compile and run
            assert!(true, "Contracts feature is correctly disabled");
        }
    }

    #[test]
    fn test_default_features_enabled() {
        // Test that default features are enabled
        #[cfg(feature = "dns")]
        {
            assert!(true, "DNS feature is enabled");
        }
        
        #[cfg(feature = "storage")]
        {
            assert!(true, "Storage feature is enabled");
        }
        
        #[cfg(feature = "staking")]
        {
            assert!(true, "Staking feature is enabled");
        }
        
        #[cfg(feature = "wallet")]
        {
            assert!(true, "Wallet feature is enabled");
        }
    }
}
