mod wallet;
mod block;
mod blockchain;
mod transaction;

use block::Block;
use blockchain::Blockchain;

fn main() {
    let chain_file = "blockchain.bin";
    let mut chain = Blockchain {
        chain: Blockchain::load_from_bin(chain_file),
    };

    if chain.chain.is_empty() {
        let genesis = Block::new(0, String::from("0"), vec![], 0, 50);
        chain.chain.push(genesis);
        println!("Genesis block created.");
    }

    // Add a new block example (normally, after some transactions)
    // let new_block = Block::new(...);
    // chain.add_block(new_block);

    chain.save_to_bin(chain_file);
    println!("Blockchain (binary) saved to {chain_file}.");
}
