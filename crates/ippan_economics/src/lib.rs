//! IPPAN Economics — DAG-Fair Emission & Distribution
//!
//! Deterministic per-round emission with halving, hard-cap enforcement,
//! role-weighted fair distribution across validators for a BlockDAG.
//!
//! Monetary unit: micro-IPN (μIPN). 1 IPN = 1_000_000 μIPN.

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