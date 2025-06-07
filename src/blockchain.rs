use crate::block::Block;
use serde::{Serialize, Deserialize};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};

#[derive(Debug, Serialize, Deserialize)]
pub struct Blockchain {
    pub chain: Vec<Block>,
}

impl Blockchain {
    pub fn new(genesis_block: Block) -> Self {
        Blockchain { chain: vec![genesis_block] }
    }

    pub fn add_block(&mut self, block: Block) {
        self.chain.push(block);
    }

    // --- Binary persistent storage ---
    pub fn save_to_bin(&self, filename: &str) {
        let bytes = bincode::serialize(&self.chain).expect("Chain serialize failed");
        let mut file = File::create(filename).expect("Could not create blockchain file");
        file.write_all(&bytes).expect("Write failed");
    }

    pub fn load_from_bin(filename: &str) -> Vec<Block> {
        let mut file = match File::open(filename) {
            Ok(f) => f,
            Err(_) => return Vec::new(),
        };
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();
        bincode::deserialize(&bytes).unwrap_or_else(|_| Vec::new())
    }

    // --- (Optional) JSON version for debugging ---
    pub fn save_to_json(&self, filename: &str) {
        let json = serde_json::to_string_pretty(&self.chain).unwrap();
        let mut file = File::create(filename).expect("Could not create blockchain file");
        file.write_all(json.as_bytes()).expect("Write failed");
    }

    pub fn load_from_json(filename: &str) -> Vec<Block> {
        let mut file = match File::open(filename) {
            Ok(f) => f,
            Err(_) => return Vec::new(),
        };
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        serde_json::from_str(&contents).unwrap_or_else(|_| Vec::new())
    }
}
