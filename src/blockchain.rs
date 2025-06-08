use crate::block::Block;
use crate::transaction::Transaction;
use std::fs::{File};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blockchain {
    pub chain: Vec<Block>,
}

impl Blockchain {
    pub fn new(miner_address: String) -> Self {
        let genesis_block = Block::new(0, "0".to_string(), vec![], 0, miner_address);
        Blockchain {
            chain: vec![genesis_block],
        }
    }

    pub fn add_block(&mut self, transactions: Vec<Transaction>, reward: u64, miner_address: String) {
        let last_block = self.chain.last().unwrap();
        let new_block = Block::new(
            last_block.index + 1,
            last_block.hash.clone(),
            transactions,
            reward,
            miner_address,
        );
        self.chain.push(new_block);
    }

    pub fn to_file(&self, path: &str) {
        if let Ok(encoded) = bincode::serialize(&self) {
            let _ = File::create(path).and_then(|mut file| file.write_all(&encoded));
        }
    }

    pub fn from_file(path: &str) -> Option<Blockchain> {
        let mut file = File::open(path).ok()?;
        let mut data = vec![];
        file.read_to_end(&mut data).ok()?;
        bincode::deserialize(&data).ok()
    }
}

pub type SharedBlockchain = Arc<Mutex<Blockchain>>;
