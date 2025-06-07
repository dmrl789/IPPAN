use crate::block::Block;

#[derive(Debug)]
pub struct Blockchain {
    pub chain: Vec<Block>,
    pub block_reward: u64, // New: block reward
}

impl Blockchain {
    pub fn new(genesis: Block, block_reward: u64) -> Self {
        Self {
            chain: vec![genesis],
            block_reward,
        }
    }

    pub fn add_block(&mut self, data: String, author: String) -> bool {
        let previous_hash = self.last_hash();
        let block = Block::new(data, previous_hash, author, self.block_reward);
        if self.is_valid_next_block(&block) {
            self.chain.push(block);
            true
        } else {
            false
        }
    }

    pub fn last_hash(&self) -> String {
        self.chain.last().map(|b| b.hash.clone()).unwrap_or_default()
    }

    // ... keep the rest as previously provided ...
    // is_valid_next_block(), is_chain_valid()...
}
