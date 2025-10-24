//! IPPAN Treasury Module
//!
//! Manages reward distribution, fee collection, and emission tracking
//! integrated with the DAG-Fair Emission system. Provides abstractions
//! for reward sinks, account ledgers, and fee management.

pub mod reward_pool;
pub mod fee_collector;
pub mod account_ledger;

pub use reward_pool::*;
pub use fee_collector::*;
pub use account_ledger::*;

/// Module version for API introspection
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
