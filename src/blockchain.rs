use crate::block::Block;
use crate::account::Ledger;

#[derive(Debug)]
pub struct Blockchain {
    pub chain: Vec<Block>,
    pub block_reward: u64,
    pub ledger: Ledger, // New: ledger field
}

impl Blockchain {
    pub fn new(genesis: Block, block_reward: u64) -> Self {
        let mut ledger = Ledger::new();
        ledger.credit("network", 0); // Initialize genesis
        Self {
            chain: vec![genesis],
            block_reward,
            ledger,
        }
    }

    pub fn add_block(&mut self, data: String, author: String) -> bool {
        let previous_hash = self.last_hash();
        let block = Block::new(data, previous_hash, author.clone(), self.block_reward);
        if self.is_valid_next_block(&block) {
            self.ledger.credit(&author, self.block_reward); // Reward the block author!
            self.chain.push(block);
            true
        } else {
            false
        }
    }

    pub fn get_balance(&self, address: &str) -> u64 {
        self.ledger.get_balance(address)
    }

    pub fn last_hash(&self) -> String {
        self.chain.last().map(|b| b.hash.clone()).unwrap_or_default()
    }

    // is_valid_next_block(), is_chain_valid()...
}
