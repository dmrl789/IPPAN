//! IPPAN Time Library
//!
//! Provides deterministic, median-based network time synchronization
//! for the IPPAN blockchain.
//!
//! # Features
//! - Microsecond precision
//! - Median drift correction
//! - Monotonic time advancement
//! - Thread-safe static state
//! - Smooth correction bounded at ±5 ms per update
//! - Optional libp2p-based peer synchronization service

pub mod hashtimer;
pub mod ippan_time;
pub mod sync;

pub use hashtimer::{
    generate_entropy, random_nonce, sign_hashtimer, verify_hashtimer, HashTimer, IppanTimeMicros,
};
pub use ippan_time::{ingest_sample, init, now, now_us, status};
pub use sync::{start_time_sync, TimeSyncService};
