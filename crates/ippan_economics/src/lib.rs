//! IPPAN Economics — DAG-Fair Emission & Distribution
//!
//! Core economic logic for the IPPAN blockchain.
//!
//! Provides deterministic, auditable mechanisms for:
//! - DAG-Fair emission and halving schedule
//! - Supply cap enforcement and automatic burn
//! - Role-weighted validator reward distribution
//! - Fee caps and recycling
//! - Parameter verification and consistency checks
//!
//! Monetary unit: **micro-IPN (μIPN)**  
//! `1 IPN = 1_000_000 μIPN`

pub mod types;
pub mod errors;
pub mod params;
pub mod emission;
pub mod distribution;
pub mod verify;

pub use types::*;
pub use errors::*;
pub use params::*;
pub use emission::*;
pub use distribution::*;
pub use verify::*;
