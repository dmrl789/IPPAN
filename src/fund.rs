use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct DomainFund {
    pub balance: u64,
}

impl DomainFund {
    pub fn new() -> Self {
        DomainFund { balance: 0 }
    }

    pub fn add_fee(&mut self, amount: u64) {
        self.balance += amount;
    }

    pub fn claim_reward(&mut self, num_nodes: u64) -> u64 {
        if num_nodes == 0 || self.balance == 0 {
            return 0;
        }
        let reward = self.balance / num_nodes;
        self.balance -= reward * num_nodes;
        reward
    }

    pub fn save_to_file(&self, path: &str) {
        let json = serde_json::to_string_pretty(&self).unwrap();
        fs::write(path, json).unwrap();
    }

    pub fn load_from_file(path: &str) -> Self {
        if Path::new(path).exists() {
            let data = fs::read_to_string(path).unwrap();
            serde_json::from_str(&data).unwrap()
        } else {
            DomainFund::new()
        }
    }
}
