use crate::block::Block;
use crate::account::Ledger;
use crate::transaction::Transaction;

pub struct Blockchain {
    pub chain: Vec<Block>,
    pub block_reward: u64,
    pub ledger: Ledger,
    pub mempool: Vec<Transaction>, // Add mempool!
}

impl Blockchain {
    pub fn new(genesis: Block, block_reward: u64) -> Self {
        let mut ledger = Ledger::new();
        ledger.credit("network", 0);
        Self {
            chain: vec![genesis],
            block_reward,
            ledger,
            mempool: vec![],
        }
    }

    pub fn add_transaction(&mut self, tx: Transaction) -> bool {
        // TODO: Signature verification here if needed
        self.mempool.push(tx);
        true
    }

    pub fn add_block(&mut self, author: String) -> bool {
        let previous_hash = self.last_hash();
        let block_transactions = self.mempool.clone();
        let block = Block::new(block_transactions, previous_hash, author.clone(), self.block_reward);
        if self.is_valid_next_block(&block) {
            // Process txs
            for tx in &block.transactions {
                if self.ledger.debit(&tx.sender, tx.amount) {
                    self.ledger.credit(&tx.recipient, tx.amount);
                }
            }
            // Reward author
            self.ledger.credit(&author, self.block_reward);
            self.chain.push(block);
            self.mempool.clear();
            true
        } else {
            false
        }
    }

    // ...other functions
}
