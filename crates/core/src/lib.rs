//! IPPAN Core — BlockDAG primitives, synchronization, and ordering utilities.
//!
//! This crate contains the data structures and services that underpin the
//! IPPAN BlockDAG, including HashTimer-backed blocks, persistent DAG storage,
//! deterministic ordering helpers, and the libp2p synchronization runtime.

pub mod block;
pub mod dag;
pub mod dag_sync;
pub mod dag_operations;
pub mod sync_manager;
pub mod order;
pub mod zk_stark;

pub use block::{Block, BlockHeader};
pub use dag::BlockDAG;
pub use dag_sync::{start_dag_sync, DagSyncService, GossipMsg};
pub use dag_operations::{DAGOperations, DAGAnalysis, DAGPath, DAGOptimizationConfig, DAGStatistics};
pub use sync_manager::{SyncManager, SyncConfig, SyncState, SyncEvent, SyncPerformance, ConflictResolutionStrategy};
pub use order::order_blocks;
pub use zk_stark::{generate_stark_proof, verify_stark_proof, StarkProof, StarkGenerator, StarkConfig, batch_verify_proofs};
