//! IPPAN Treasury Module
//!
//! Manages reward distribution, fee collection, and economic operations
//! for the IPPAN blockchain. Integrates with the DAG-Fair Emission system.

pub mod reward_pool;
pub mod fee_collector;
pub mod account_ledger;

pub use reward_pool::*;
pub use fee_collector::*;
pub use account_ledger::*;

/// Module version for API introspection
pub const VERSION: &str = env!("CARGO_PKG_VERSION");