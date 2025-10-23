//! IPPAN Economics Engine
//!
//! This crate provides the core economics functionality for IPPAN, including:
//! - DAG-Fair Emission with deterministic halving schedules
//! - Validator reward distribution based on participation
//! - Supply cap enforcement and automatic burn mechanisms
//! - Fee recycling and economic parameter management

pub mod emission;
pub mod distribution;
pub mod types;

pub use emission::*;
pub use distribution::*;
pub use types::*;
