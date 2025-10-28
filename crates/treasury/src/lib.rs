//! IPPAN Treasury Module
//!
//! Manages reward distribution, fee collection, and emission tracking
//! integrated with the DAG-Fair Emission system. Provides abstractions
//! for reward sinks, account ledgers, and fee management.

pub mod account_ledger;
pub mod fee_collector;
pub mod reward_pool;

pub use account_ledger::*;
pub use fee_collector::*;
pub use reward_pool::*;

/// Module version for API introspection
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
