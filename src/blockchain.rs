use crate::block::Block;
use crate::mempool::Mempool;
use crate::transaction::Transaction;

pub struct Blockchain {
    pub chain: Vec<Block>,
}

impl Blockchain {
    pub fn new() -> Self {
        let genesis_block = Block::genesis();
        Blockchain {
            chain: vec![genesis_block],
        }
    }

    pub fn add_block(&mut self, block: Block) {
        self.chain.push(block);
    }

    pub fn create_block_from_mempool(&self, mempool: &mut Mempool, reward: u64, miner: String) -> Block {
        let transactions = mempool.take_all();
        let prev_hash = self.chain.last().unwrap().hash.clone();
        Block::new(
            self.chain.len() as u64,
            prev_hash,
            transactions,
            reward,
            miner,
        )
    }
}
