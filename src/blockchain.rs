use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use serde::{Serialize, Deserialize};

use crate::block::Block;
use crate::transaction::Transaction;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Blockchain {
    pub chain: Vec<Block>,
    pub mempool: Vec<Transaction>,
    pub miner_address: String,
}

impl Blockchain {
    /// Loads the blockchain from disk, or creates a new one with a genesis block.
    pub fn load_or_new(path: &str, miner_address: String) -> Self {
        if Path::new(path).exists() {
            let mut file = File::open(path).expect("Unable to open blockchain file");
            let mut data = Vec::new();
            file.read_to_end(&mut data).expect("Unable to read blockchain file");
            bincode::deserialize(&data).expect("Unable to parse blockchain file")
        } else {
            let mut bc = Self::new(miner_address);
            bc.save(path);
            bc
        }
    }

    /// Creates a new blockchain with a genesis block.
    pub fn new(miner_address: String) -> Self {
        let genesis_block = Block::new(
            0,
            "0".to_string(),
            vec![],
            0,
            miner_address.clone(),
        );
        Blockchain {
            chain: vec![genesis_block],
            mempool: vec![],
            miner_address,
        }
    }

    /// Adds a new transaction to the mempool (not yet mined).
    pub fn add_transaction(&mut self, tx: Transaction) {
        self.mempool.push(tx);
    }

    /// Mines a new block with current mempool transactions.
    pub fn mine_block(&mut self, reward: u64) -> &Block {
        let last_block = self.chain.last().expect("Blockchain should never be empty");
        let new_block = Block::new(
            last_block.index + 1,
            last_block.hash.clone(),
            self.mempool.clone(),
            reward,
            self.miner_address.clone(),
        );
        self.chain.push(new_block);
        self.mempool.clear();
        self.chain.last().unwrap()
    }

    /// Validates the entire chain.
    pub fn is_valid(&self) -> bool {
        for i in 1..self.chain.len() {
            if !self.chain[i].is_valid(&self.chain[i - 1]) {
                return false;
            }
        }
        true
    }

    /// Saves the blockchain to disk as binary.
    pub fn save(&self, path: &str) {
        let data = bincode::serialize(self).expect("Unable to serialize blockchain");
        let mut file = File::create(path).expect("Unable to create blockchain file");
        file.write_all(&data).expect("Unable to write blockchain file");
    }

    /// Loads blockchain from disk (shortcut).
    pub fn load(path: &str) -> Self {
        let mut file = File::open(path).expect("Unable to open blockchain file");
        let mut data = Vec::new();
        file.read_to_end(&mut data).expect("Unable to read blockchain file");
        bincode::deserialize(&data).expect("Unable to parse blockchain file")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transaction::Transaction;

    #[test]
    fn test_blockchain() {
        let mut bc = Blockchain::new("miner".to_string());
        assert_eq!(bc.chain.len(), 1);

        // Add a tx and mine
        let tx = Transaction {
            from: "A".to_string(),
            to: "B".to_string(),
            amount: 10,
            signature: vec![0; 64],
        };
        bc.add_transaction(tx);
        bc.mine_block(50);
        assert_eq!(bc.chain.len(), 2);
        assert_eq!(bc.mempool.len(), 0);
        assert!(bc.is_valid());
    }
}
