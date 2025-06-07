use crate::block::Block;

pub struct Blockchain {
    pub blocks: Vec<Block>,
    pub block_reward: u64,
}

impl Blockchain {
    pub fn new() -> Self {
        let genesis_block = Block::new(
            0,
            String::from("0"),
            vec![],
            String::from("genesis"),
            50,
        );
        Blockchain {
            blocks: vec![genesis_block],
            block_reward: 50,
        }
    }

    pub fn mine_block(&mut self, miner_address: &str) {
        let prev_block = self.blocks.last().unwrap();
        let new_block = Block::new(
            prev_block.index + 1,
            prev_block.hash.clone(),
            vec![], // No txs in demo
            miner_address.to_string(),
            self.block_reward,
        );
        self.blocks.push(new_block);
    }

    pub fn total_reward_for(&self, address: &str) -> u64 {
        self.blocks.iter().filter(|block| block.miner == address).map(|b| b.reward).sum()
    }
}
