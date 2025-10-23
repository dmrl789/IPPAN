//! IPPAN Economics — DAG-Fair Emission & Distribution
//!
//! Deterministic per-round emission with halving, hard-cap enforcement,
//! role-weighted fair distribution across validators for a BlockDAG.
//!
//! Monetary unit: micro-IPN (μIPN). 1 IPN = 1_000_000 μIPN.

pub mod distribution;
pub mod emission;
pub mod errors;
pub mod params;
pub mod types;
pub mod verify;

pub use distribution::*;
pub use emission::*;
pub use errors::*;
pub use params::*;
pub use types::*;
pub use verify::*;
