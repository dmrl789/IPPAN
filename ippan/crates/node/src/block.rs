use ippan_common::{BlockId, Hash, Result, Transaction, merkle::compute_merkle_root, crypto::blake3_hash};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Block header structure
#[derive(Debug, Clone)]
pub struct BlockHeader {
    pub parents: Vec<BlockId>,      // Up to 4 parent blocks
    pub round_id: u64,              // Round ID
    pub block_time_us: u64,         // Block creation time
    pub builder_id: Hash,           // Builder's public key hash
    pub tx_count: u32,              // Number of transactions
    pub merkle_root: Hash,          // Merkle root of transaction IDs
    pub hashtimer: Hash,            // HashTimer for ordering
}

/// Block structure (contains only transaction IDs)
#[derive(Debug, Clone)]
pub struct Block {
    pub header: BlockHeader,
    pub tx_ids: Vec<Hash>,          // Transaction IDs only
}

/// Block builder implementation
pub struct BlockBuilder {
    builder_id: Hash,
    max_block_size: usize,          // Target 16KB
    max_parents: usize,             // Up to 4 parents
    block_interval_ms: u64,         // 10-50ms intervals
    last_block_time: u64,
    block_counter: u64,
}

impl BlockBuilder {
    pub fn new() -> Self {
        let builder_id = blake3_hash(b"default-builder");
        
        Self {
            builder_id,
            max_block_size: 16 * 1024, // 16KB target
            max_parents: 4,
            block_interval_ms: 25,     // 25ms default
            last_block_time: 0,
            block_counter: 0,
        }
    }

    /// Build a new block from transactions
    pub async fn build_block(&mut self, transactions: &[Transaction]) -> Result<BlockId> {
        if transactions.is_empty() {
            return Err(ippan_common::Error::Validation("No transactions to build block".to_string()));
        }

        // Get current time
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64;

        // Check if enough time has passed since last block
        if current_time - self.last_block_time < (self.block_interval_ms * 1000) {
            return Err(ippan_common::Error::Validation("Block interval not met".to_string()));
        }

        // Compute transaction IDs
        let mut tx_ids = Vec::new();
        for tx in transactions {
            let tx_id = tx.compute_id()?;
            tx_ids.push(tx_id);
        }

        // Compute merkle root
        let merkle_root = compute_merkle_root(&tx_ids)?;

        // Create block header
        let header = BlockHeader {
            parents: Vec::new(), // TODO: Implement parent selection
            round_id: self.block_counter / 4, // Simple round calculation
            block_time_us: current_time,
            builder_id: self.builder_id,
            tx_count: tx_ids.len() as u32,
            merkle_root,
            hashtimer: self.compute_block_hashtimer(&tx_ids, current_time),
        };

        // Create block
        let block = Block {
            header,
            tx_ids,
        };

        // Compute block ID
        let block_id = self.compute_block_id(&block)?;

        // Update state
        self.last_block_time = current_time;
        self.block_counter += 1;

        tracing::debug!("Built block {} with {} transactions", 
            hex::encode(block_id), block.header.tx_count);

        Ok(block_id)
    }

    /// Compute block hashtimer for ordering
    fn compute_block_hashtimer(&self, tx_ids: &[Hash], block_time: u64) -> Hash {
        let mut input = Vec::new();
        input.extend_from_slice(&block_time.to_le_bytes());
        input.extend_from_slice(&self.builder_id);
        
        for tx_id in tx_ids {
            input.extend_from_slice(tx_id);
        }
        
        blake3_hash(&input)
    }

    /// Compute block ID from block data
    fn compute_block_id(&self, block: &Block) -> Result<BlockId> {
        let mut data = Vec::new();
        
        // Serialize header
        data.extend_from_slice(&block.header.parents.len().to_le_bytes());
        for parent in &block.header.parents {
            data.extend_from_slice(parent);
        }
        data.extend_from_slice(&block.header.round_id.to_le_bytes());
        data.extend_from_slice(&block.header.block_time_us.to_le_bytes());
        data.extend_from_slice(&block.header.builder_id);
        data.extend_from_slice(&block.header.tx_count.to_le_bytes());
        data.extend_from_slice(&block.header.merkle_root);
        data.extend_from_slice(&block.header.hashtimer);
        
        // Add transaction count
        data.extend_from_slice(&block.tx_ids.len().to_le_bytes());
        
        Ok(blake3_hash(&data))
    }

    /// Get block size in bytes
    pub fn get_block_size(&self, block: &Block) -> usize {
        let header_size = 4 + // parents count
                         (block.header.parents.len() * 32) + // parent IDs
                         8 + // round_id
                         8 + // block_time_us
                         32 + // builder_id
                         4 + // tx_count
                         32 + // merkle_root
                         32; // hashtimer
        
        let tx_ids_size = 4 + (block.tx_ids.len() * 32); // tx_count + tx_ids
        
        header_size + tx_ids_size
    }

    /// Check if block size is within limits
    pub fn is_block_size_valid(&self, block: &Block) -> bool {
        self.get_block_size(block) <= self.max_block_size
    }

    /// Set block interval
    pub fn set_block_interval(&mut self, interval_ms: u64) {
        self.block_interval_ms = interval_ms;
    }

    /// Get block statistics
    pub fn get_stats(&self) -> BlockBuilderStats {
        BlockBuilderStats {
            blocks_built: self.block_counter,
            block_interval_ms: self.block_interval_ms,
            max_block_size: self.max_block_size,
            builder_id: self.builder_id,
        }
    }
}

#[derive(Debug)]
pub struct BlockBuilderStats {
    pub blocks_built: u64,
    pub block_interval_ms: u64,
    pub max_block_size: usize,
    pub builder_id: Hash,
}
