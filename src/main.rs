mod wallet;
mod block;
mod blockchain;
mod account;
mod transaction;

use wallet::Wallet;
use block::Block;
use blockchain::Blockchain;
use transaction::Transaction;

fn main() {
    println!("=== IPPAN Blockchain Demo: Transactions ===");

    // Create wallets
    let wallet1 = Wallet::generate();
    let wallet2 = Wallet::generate();
    println!("Wallet1 address: {}", wallet1.address);
    println!("Wallet2 address: {}", wallet2.address);

    // Create genesis block & blockchain
    let genesis = Block::genesis();
    let mut chain = Blockchain::new(genesis, 100);

    // Reward wallet1 for block authoring
    chain.add_block(wallet1.address.clone());

    // Print balances
    println!("Balance of wallet1: {}", chain.get_balance(&wallet1.address));
    println!("Balance of wallet2: {}", chain.get_balance(&wallet2.address));

    // Create and add a transaction from wallet1 to wallet2
    let tx = Transaction::new(
        wallet1.address.clone(),
        wallet2.address.clone(),
        50,
        vec![], // Not actually signed for now
    );
    chain.add_transaction(tx);

    // Author new block with transaction (by wallet1)
    chain.add_block(wallet1.address.clone());

    // Print balances again
    println!("Balance of wallet1: {}", chain.get_balance(&wallet1.address));
    println!("Balance of wallet2: {}", chain.get_balance(&wallet2.address));
}
