mod wallet;
mod block;
mod blockchain;
mod account;

use wallet::Wallet;
use block::Block;
use blockchain::Blockchain;

fn main() {
    println!("=== IPPAN Blockchain Demo: Accounts & Rewards ===");

    // Create a wallet (block author)
    let wallet = Wallet::generate();
    println!("Wallet address: {}", wallet.address);

    // Create genesis block
    let genesis = Block::genesis();
    println!("Genesis Block:\n{:#?}", genesis);

    // Create blockchain with a block reward of 100
    let mut chain = Blockchain::new(genesis, 100);

    // Add a block with reward
    let added = chain.add_block("first tx".to_string(), wallet.address.clone());
    println!("\nBlock added? {}", added);

    // Show blockchain
    for (i, block) in chain.chain.iter().enumerate() {
        println!(
            "Block {}: hash={} reward={} author={}",
            i,
            &block.hash[0..10],
            block.reward,
            block.author
        );
    }

    // Show balances
    println!("\nBalances:");
    println!("{}: {}", wallet.address, chain.get_balance(&wallet.address));
}
