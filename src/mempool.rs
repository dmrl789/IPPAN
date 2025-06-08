use crate::transaction::Transaction;
use std::collections::VecDeque;

pub struct Mempool {
    transactions: VecDeque<Transaction>,
}

impl Mempool {
    pub fn new() -> Self {
        Mempool { transactions: VecDeque::new() }
    }

    pub fn add_transaction(&mut self, tx: Transaction) {
        self.transactions.push_back(tx);
    }

    pub fn get_transactions(&self) -> &VecDeque<Transaction> {
        &self.transactions
    }

    pub fn take_all(&mut self) -> Vec<Transaction> {
        self.transactions.drain(..).collect()
    }
}
