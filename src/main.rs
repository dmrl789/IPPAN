mod wallet;
mod block;
mod blockchain;

use wallet::Wallet;
use block::Block;
use blockchain::Blockchain;

fn main() {
    println!("=== IPPAN Blockchain Demo ===");

    // Wallet
    let wallet = Wallet::generate();

    // Genesis Block
    let genesis = Block::genesis();
    println!("Genesis Block:\n{:#?}", genesis);

    // Blockchain with block reward 100
    let mut chain = Blockchain::new(genesis, 100);

    // Add a valid block with reward
    println!("\nAdding valid block (rewarded)...");
    let added = chain.add_block("tx1".to_string(), wallet.address.clone());
    println!("Block added? {}", added);

    // Show block rewards in the chain
    println!("\nBlockchain blocks (showing rewards):");
    for (i, block) in chain.chain.iter().enumerate() {
        println!(
            "Block {}: hash={} reward={} author={}",
            i,
            &block.hash[0..10],
            block.reward,
            block.author
        );
    }
}
