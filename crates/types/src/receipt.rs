use crate::hashtimer::IppanTimeMicros;
use crate::transaction::Transaction;
use crate::BlockId;
use serde::{Deserialize, Serialize};

/// Receipt generated for a single transaction once it has been executed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransactionReceipt {
    /// Canonical transaction hash the receipt corresponds to.
    pub transaction_hash: [u8; 32],
    /// Whether the transaction execution completed successfully.
    pub success: bool,
    /// Amount of gas consumed during execution (opaque unit today).
    pub gas_used: u64,
    /// Arbitrary log messages or events emitted by the execution engine.
    pub logs: Vec<String>,
}

impl TransactionReceipt {
    /// Convenience helper that creates a successful receipt from a transaction.
    pub fn success(tx: &Transaction, gas_used: u64, logs: Vec<String>) -> Self {
        Self {
            transaction_hash: tx.hash(),
            success: true,
            gas_used,
            logs,
        }
    }

    /// Convenience helper that creates a failed receipt from a transaction.
    pub fn failed(tx: &Transaction, gas_used: u64, logs: Vec<String>) -> Self {
        Self {
            transaction_hash: tx.hash(),
            success: false,
            gas_used,
            logs,
        }
    }
}

/// Bundle of receipts emitted by executing all transactions in a block.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlockReceipts {
    /// Block identifier the receipts belong to.
    pub block_hash: BlockId,
    /// Height of the block in the canonical chain.
    pub block_height: u64,
    /// Root commitment to the post-state after processing the block.
    pub state_root: [u8; 32],
    /// Timestamp inherited from the block's HashTimer.
    pub timestamp: IppanTimeMicros,
    /// Receipts ordered following the transaction list in the block body.
    pub receipts: Vec<TransactionReceipt>,
}

impl BlockReceipts {
    /// Create receipts bundle from block metadata and execution output.
    pub fn new(
        block_hash: BlockId,
        block_height: u64,
        state_root: [u8; 32],
        timestamp: IppanTimeMicros,
        receipts: Vec<TransactionReceipt>,
    ) -> Self {
        Self {
            block_hash,
            block_height,
            state_root,
            timestamp,
            receipts,
        }
    }
}
