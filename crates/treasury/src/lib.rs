//! IPPAN Treasury Module
//!
//! Manages reward distribution, fee collection, and emission tracking
//! integrated with the DAG-Fair Emission system. Provides abstractions
//! for reward sinks, account ledgers, fee management, and weekly fee pools.
//!
//! ## Key Components
//!
//! - [`account_ledger`]: Interface for crediting/debiting validator accounts
//! - [`fee_collector`]: Per-round fee accumulation and statistics
//! - [`reward_pool`]: Round-based reward distribution
//! - [`weekly_pool`]: Epoch-based fee pool with weekly redistribution

pub mod account_ledger;
pub mod fee_collector;
pub mod reward_pool;
pub mod weekly_pool;

pub use account_ledger::*;
pub use fee_collector::*;
pub use reward_pool::*;
pub use weekly_pool::*;

/// Module version for API introspection
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
