//! Transaction signing utility for IPPAN
//! 
//! Command-line tool to create and sign transactions

use ippan::crypto::{Ed25519Keypair, CanonicalTransaction, TransactionSigner};
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 4 {
        eprintln!("Usage: {} <to_address> <amount> <fee> [nonce]", args[0]);
        eprintln!("Example: {} iRbDqSo0H4NxPGC0q55ohG36JrvlcYGvM3DpS4Q 25000 10 1", args[0]);
        std::process::exit(1);
    }
    
    let to_address = &args[1];
    let amount = &args[2];
    let fee = &args[3];
    let nonce = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(1);
    
    // Create canonical transaction
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    let tx = CanonicalTransaction::new(
        "ippan-devnet-001".to_string(),
        "iSender1111111111111111111111111111111111111".to_string(),
        to_address.to_string(),
        amount.to_string(),
        fee.to_string(),
        nonce,
        timestamp.to_string(),
    );
    
    // Generate keypair (in real implementation, this would load from file)
    let keypair = Ed25519Keypair::generate();
    let signer = TransactionSigner::new(keypair);
    
    // Sign transaction
    let signed_tx = signer.sign_transaction(tx)?;
    
    // Output signed transaction as JSON
    let json = serde_json::to_string_pretty(&signed_tx)?;
    println!("{}", json);
    
    Ok(())
}

