//! Canonical IPPAN data types re-export. Aggregates address, block, transaction,
//! round, and time primitives so applications use a single crate for DAG
//! structures, HashTimer utilities, L2 records, and currency denominations.

// Module declarations - alphabetical order
pub mod address;
pub mod block;
pub mod chain_state;
pub mod currency;
pub mod file_descriptor;
pub mod handle;
pub mod health;
pub mod l2;
pub mod receipt;
pub mod round;
pub mod scalars;
pub mod snapshot;
pub mod time_service;
pub mod transaction;

// Re-exports - grouped by category
// Address types
pub use address::*;

// Block and round types
pub use block::*;
pub use round::*;

// Chain state
pub use chain_state::*;

// Currency and amount types
pub use currency::{denominations, Amount, AtomicIPN, ATOMIC_PER_IPN, IPN_DECIMALS, SUPPLY_CAP};

// File descriptor metadata
pub use file_descriptor::*;

// Health/observability payloads
pub use health::*;

// Handle transaction helpers
pub use handle::*;

// HashTimer from ippan-time for unified implementation
pub use ippan_time::{random_nonce, HashTimer, IppanTimeMicros};

// L2 types
pub use l2::*;

// Receipt types
pub use receipt::*;

// Snapshot types
pub use snapshot::*;

// Scalar helpers
pub use scalars::*;

// Time service utilities
pub use time_service::{
    generate_entropy, ingest_sample, init, ippan_time_ingest_sample, ippan_time_init,
    ippan_time_now, ippan_time_start_sync, now, now_us, sign_hashtimer, start_time_sync, status,
    verify_hashtimer, TimeSyncService,
};

// Transaction types
pub use transaction::*;

#[cfg(test)]
mod tests;
