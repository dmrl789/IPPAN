mod wallet;
mod domain;
mod node;
mod utils;
mod types;

// (rest of your use lines, as before)
use std::io::{self, Write};

fn main() {
    println!("=== IPPAN Wallet Demo ===");

    // Load or create wallet
    let wallet = if std::path::Path::new("wallet.dat").exists() {
        wallet::Wallet::load_from_file("wallet.dat")
    } else {
        let w = wallet::Wallet::generate();
        w.save_to_file("wallet.dat");
        println!("New wallet generated and saved to wallet.dat");
        w
    };

    println!("Your wallet address: {}", wallet.address);

    loop {
        println!("\nOptions:");
        println!("1. Show private key as mnemonic words");
        println!("2. Show private key as hex");
        println!("3. Show both (for backup)");
        println!("4. Exit");
        print!("Choice: ");
        io::stdout().flush().unwrap();

        let mut choice = String::new();
        io::stdin().read_line(&mut choice).unwrap();
        match choice.trim() {
            "1" => wallet.print_mnemonic(),
            "2" => wallet.print_private_hex(),
            "3" => {
                wallet.print_mnemonic();
                wallet.print_private_hex();
            }
            "4" => break,
            _ => println!("Invalid choice!"),
        }
    }
}
