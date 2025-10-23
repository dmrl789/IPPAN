//! IPPAN Treasury Module
//!
//! Handles reward distribution, emission tracking, and validator payouts
//! integrated with the DAG-Fair emission system.

pub mod reward_pool;

pub use reward_pool::{AccountLedger, RewardSink};
