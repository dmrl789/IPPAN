use anyhow::Result;
use ippan_types::{Block, Transaction, ippan_time_now, HashTimer, IppanTimeMicros};
use std::collections::HashMap;

/// Consensus engine for IPPAN blockchain
pub trait ConsensusEngine {
    /// Start the consensus engine
    async fn start(&mut self) -> Result<()>;
    
    /// Stop the consensus engine
    async fn stop(&mut self) -> Result<()>;
    
    /// Propose a new block
    async fn propose_block(&self, transactions: Vec<Transaction>) -> Result<Block>;
    
    /// Validate a proposed block
    async fn validate_block(&self, block: &Block) -> Result<bool>;
    
    /// Get current consensus state
    fn get_state(&self) -> ConsensusState;
}

/// Consensus state information
#[derive(Debug, Clone)]
pub struct ConsensusState {
    pub current_round: u64,
    pub is_proposer: bool,
    pub validator_count: usize,
}

/// Simple consensus implementation (for testing/development)
pub struct SimpleConsensus {
    current_round: u64,
    validator_id: [u8; 32],
    is_running: bool,
}

impl SimpleConsensus {
    pub fn new(validator_id: [u8; 32]) -> Self {
        Self {
            current_round: 0,
            validator_id,
            is_running: false,
        }
    }
}

impl ConsensusEngine for SimpleConsensus {
    async fn start(&mut self) -> Result<()> {
        self.is_running = true;
        tracing::info!("Consensus engine started");
        Ok(())
    }
    
    async fn stop(&mut self) -> Result<()> {
        self.is_running = false;
        tracing::info!("Consensus engine stopped");
        Ok(())
    }
    
    async fn propose_block(&self, transactions: Vec<Transaction>) -> Result<Block> {
        if !self.is_running {
            return Err(anyhow::anyhow!("Consensus engine not running"));
        }
        
        // Create a new block with the given transactions
        let prev_hash = if self.current_round == 0 {
            [0u8; 32] // Genesis block
        } else {
            // In a real implementation, this would be the hash of the previous block
            [1u8; 32]
        };
        
        let block = Block::new(
            prev_hash,
            transactions,
            self.current_round + 1,
            self.validator_id,
        );
        
        tracing::info!("Proposed block {} at round {}", 
            hex::encode(block.hash()), self.current_round + 1);
        
        Ok(block)
    }
    
    async fn validate_block(&self, block: &Block) -> Result<bool> {
        if !self.is_running {
            return Err(anyhow::anyhow!("Consensus engine not running"));
        }
        
        // Basic validation
        if !block.is_valid() {
            return Ok(false);
        }
        
        // Check round number
        if block.header.round_id != self.current_round + 1 {
            return Ok(false);
        }
        
        // Check proposer
        if block.header.proposer_id != self.validator_id {
            return Ok(false);
        }
        
        tracing::info!("Validated block {} at round {}", 
            hex::encode(block.hash()), block.header.round_id);
        
        Ok(true)
    }
    
    fn get_state(&self) -> ConsensusState {
        ConsensusState {
            current_round: self.current_round,
            is_proposer: true, // Simple consensus always allows proposing
            validator_count: 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ippan_types::Transaction;

    #[tokio::test]
    async fn test_consensus_engine() {
        let mut consensus = SimpleConsensus::new([1u8; 32]);
        
        // Test starting and stopping
        consensus.start().await.unwrap();
        assert!(consensus.is_running);
        
        consensus.stop().await.unwrap();
        assert!(!consensus.is_running);
    }

    #[tokio::test]
    async fn test_block_proposal() {
        let mut consensus = SimpleConsensus::new([1u8; 32]);
        consensus.start().await.unwrap();
        
        let transactions = vec![
            Transaction::new([1u8; 32], [2u8; 32], 1000, 1),
        ];
        
        let block = consensus.propose_block(transactions).await.unwrap();
        assert_eq!(block.header.round_id, 1);
        assert_eq!(block.header.proposer_id, [1u8; 32]);
    }
}
