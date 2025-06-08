mod block;
mod transaction;
mod wallet;
mod blockchain;
mod handle;

use blockchain::Blockchain;
use wallet::Wallet;
use std::sync::{Arc, Mutex};

fn main() {
    println!("=== IPPAN Wallet Demo ===");

    // Generate wallet
    let wallet = Wallet::generate();
    println!("Your wallet address: {}", wallet.address());

    // Load or create blockchain
    let blockchain_file = "blockchain.json";
    let blockchain = if let Some(bc) = Blockchain::load_from_file(blockchain_file) {
        bc
    } else {
        Blockchain::new(wallet.address())
    };
    let blockchain = Arc::new(Mutex::new(blockchain));

    loop {
        println!("\nOptions:");
        println!("1. Register handle (e.g. @alice.ipn)");
        println!("2. Show your wallet address");
        println!("3. Find pubkey by handle");
        println!("4. Save blockchain");
        println!("5. Exit");

        let mut choice = String::new();
        std::io::stdin().read_line(&mut choice).unwrap();
        match choice.trim() {
            "1" => {
                println!("Enter handle (e.g. @alice.ipn): ");
                let mut handle = String::new();
                std::io::stdin().read_line(&mut handle).unwrap();
                let handle = handle.trim();

                let pubkey_hex = wallet.address();
                let mut bc = blockchain.lock().unwrap();
                match bc.register_handle(handle, &pubkey_hex) {
                    Ok(_) => println!("Handle registered!"),
                    Err(e) => println!("Failed to register: {}", e),
                }
            },
            "2" => {
                println!("Your wallet address: {}", wallet.address());
            },
            "3" => {
                println!("Enter handle to look up: ");
                let mut handle = String::new();
                std::io::stdin().read_line(&mut handle).unwrap();
                let handle = handle.trim();
                let bc = blockchain.lock().unwrap();
                match bc.get_pubkey_by_handle(handle) {
                    Some(pk) => println!("Pubkey: {}", pk),
                    None => println!("Handle not found!"),
                }
            },
            "4" => {
                let bc = blockchain.lock().unwrap();
                bc.save_to_file(blockchain_file);
                println!("Blockchain saved.");
            },
            "5" => {
                println!("Exiting...");
                break;
            },
            _ => println!("Invalid choice."),
        }
    }
}
