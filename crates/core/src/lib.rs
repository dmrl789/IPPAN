//! IPPAN Core — BlockDAG primitives, synchronization, and ordering utilities.
//!
//! This crate contains the data structures and services that underpin the
//! IPPAN BlockDAG, including HashTimer-backed blocks, persistent DAG storage,
//! deterministic ordering helpers, and the libp2p synchronization runtime.

pub mod block;
pub mod dag;
pub mod dag_operations;
pub mod dag_sync;
pub mod order;
pub mod sync_manager;
pub mod zk_stark;

pub use block::{Block, BlockHeader};
pub use dag::BlockDAG;
pub use dag_operations::{
    DAGAnalysis, DAGOperations, DAGOptimizationConfig, DAGPath, DAGStatistics,
};
pub use dag_sync::{start_dag_sync, DagSyncService, GossipMsg};
pub use order::order_blocks;
pub use sync_manager::{
    ConflictResolutionStrategy, SyncConfig, SyncEvent, SyncManager, SyncPerformance, SyncState,
};
pub use zk_stark::{
    batch_verify_proofs, generate_stark_proof, verify_stark_proof, StarkConfig, StarkGenerator,
    StarkProof,
};
