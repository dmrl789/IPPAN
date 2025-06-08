mod wallet;
mod transaction;
mod block;
mod blockchain;
mod mempool;

use wallet::Wallet;
use transaction::Transaction;
use blockchain::Blockchain;
use mempool::Mempool;

fn main() {
    println!("=== IPPAN Blockchain Node Demo ===");

    // Initialize components
    let mut blockchain = Blockchain::new();
    let mut mempool = Mempool::new();

    // Demo: Add a sample transaction to the mempool
    let tx = Transaction {
        from: "Alice".to_string(),
        to: "Bob".to_string(),
        amount: 42,
        signature: vec![0; 64], // Replace with real signatures in production!
    };
    mempool.add_transaction(tx);

    println!("Transactions in mempool: {}", mempool.get_transactions().len());

    // Mine/create a block using transactions in the mempool
    let miner = "Miner1".to_string();
    let reward = 1;
    let new_block = blockchain.create_block_from_mempool(&mut mempool, reward, miner.clone());
    println!("New block created by {}: {:?}", miner, new_block);

    // Add the new block to the chain
    blockchain.add_block(new_block);

    // Print chain info
    println!("Current blockchain length: {}", blockchain.chain.len());
    for (i, block) in blockchain.chain.iter().enumerate() {
        println!("Block {}: hash {}", i, block.hash);
    }
}
