//! Hard limits for IPPAN consensus system
//!
//! This module defines the hard maximum limits that cannot be exceeded
//! regardless of configuration. These are the single source of truth for
//! block size enforcement.

/// Hard maximum block size in bytes (32 KB).
pub const MAX_BLOCK_SIZE_BYTES: usize = 32 * 1024; // 32,768

/// Recommended typical range for telemetry/docs (not enforced).
pub const TYPICAL_BLOCK_SIZE_MIN_BYTES: usize = 4 * 1024;   // 4 KB
pub const TYPICAL_BLOCK_SIZE_MAX_BYTES: usize = 32 * 1024;  // 32 KB

/// Maximum number of transactions per block (approximate, depends on tx size)
pub const MAX_TRANSACTIONS_PER_BLOCK: usize = 2000;

/// Maximum number of parent blocks in DAG structure
pub const MAX_PARENT_BLOCKS: usize = 8;

/// Minimum number of parent blocks (except genesis)
pub const MIN_PARENT_BLOCKS: usize = 1;
