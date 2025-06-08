use crate::transaction::Transaction;

pub struct Mempool {
    transactions: Vec<Transaction>,
}

impl Mempool {
    pub fn new() -> Self {
        Mempool { transactions: vec![] }
    }
    pub fn add_transaction(&mut self, tx: Transaction) {
        self.transactions.push(tx);
    }
    pub fn take_all(&mut self) -> Vec<Transaction> {
        std::mem::take(&mut self.transactions)
    }
    pub fn get_transactions(&self) -> &Vec<Transaction> {
        &self.transactions
    }
}
