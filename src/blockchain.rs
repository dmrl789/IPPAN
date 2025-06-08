use crate::block::Block;
use crate::transaction::Transaction;
use crate::handle::HandleRegistry;
use serde::{Serialize, Deserialize};
use std::fs::{File};
use std::io::{Read, Write};

#[derive(Debug, Serialize, Deserialize)]
pub struct Blockchain {
    pub chain: Vec<Block>,
    pub pending_transactions: Vec<Transaction>,
    pub handle_registry: HandleRegistry,
    pub miner_address: String,
}

impl Blockchain {
    pub fn new(miner_address: String) -> Self {
        let genesis = Block::genesis();
        Blockchain {
            chain: vec![genesis],
            pending_transactions: Vec::new(),
            handle_registry: HandleRegistry::new(),
            miner_address,
        }
    }

    pub fn add_block(&mut self, block: Block) {
        self.chain.push(block);
    }

    pub fn add_transaction(&mut self, tx: Transaction) {
        self.pending_transactions.push(tx);
    }

    pub fn register_handle(&mut self, handle: &str, pubkey_hex: &str) -> Result<(), String> {
        self.handle_registry.register(handle, pubkey_hex)
    }

    pub fn save_to_file(&self, filename: &str) {
        let data = serde_json::to_string(self).expect("Failed to serialize blockchain");
        let mut file = File::create(filename).expect("Failed to create file");
        file.write_all(data.as_bytes()).expect("Failed to write to file");
    }

    pub fn load_from_file(filename: &str) -> Option<Self> {
        let mut file = File::open(filename).ok()?;
        let mut data = String::new();
        file.read_to_string(&mut data).ok()?;
        serde_json::from_str(&data).ok()
    }

    pub fn get_pubkey_by_handle(&self, handle: &str) -> Option<&String> {
        self.handle_registry.get_pubkey(handle)
    }
}
