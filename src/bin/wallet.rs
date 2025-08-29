use clap::{Parser, Subcommand};
use ippan::{crypto::KeyPair, transaction::Transaction, wallet::WalletManager, time::IppanTime};
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "ippan-wallet")]
#[command(about = "IPPAN Wallet CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new wallet
    New {
        /// Wallet name
        name: String,
    },
    /// Recover wallet from secret key
    Recover {
        /// Wallet name
        name: String,
        /// Secret key (hex-encoded)
        secret_key: String,
    },
    /// List all wallets
    List,
    /// Show wallet address
    Addr {
        /// Wallet name (optional, uses default if not specified)
        name: Option<String>,
    },
    /// Show wallet balance
    Balance {
        /// Wallet name (optional, uses default if not specified)
        name: Option<String>,
    },
    /// Send a payment transaction
    Send {
        /// Recipient address
        #[arg(long)]
        to: String,
        /// Amount to send
        #[arg(long)]
        amount: u64,
        /// Wallet name (optional, uses default if not specified)
        #[arg(long)]
        wallet: Option<String>,
        /// Node URL for transaction submission
        #[arg(long, default_value = "http://127.0.0.1:8080")]
        node: String,
    },
    /// Set default wallet
    SetDefault {
        /// Wallet name
        name: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    let wallet_manager = WalletManager::new();
    
    match cli.command {
        Commands::New { name } => {
            wallet_manager.create_wallet(name).await?;
            println!("Wallet created successfully!");
        }
        
        Commands::Recover { name, secret_key } => {
            let secret_key_bytes = hex::decode(&secret_key)?;
            wallet_manager.import_wallet(name, &secret_key_bytes).await?;
            println!("Wallet recovered successfully!");
        }
        
        Commands::List => {
            let wallets = wallet_manager.list_wallets().await;
            if wallets.is_empty() {
                println!("No wallets found.");
            } else {
                println!("Wallets:");
                for wallet_name in wallets {
                    let default = wallet_manager.get_default_wallet_name().await;
                    let marker = if default.as_ref() == Some(&wallet_name) { " (default)" } else { "" };
                    println!("  {}{}", wallet_name, marker);
                }
            }
        }
        
        Commands::Addr { name } => {
            let wallet = wallet_manager.get_wallet(name.as_deref()).await?;
            println!("Address: {}", wallet.get_address());
        }
        
        Commands::Balance { name } => {
            let wallet = wallet_manager.get_wallet(name.as_deref()).await?;
            println!("Balance: {}", wallet.get_balance());
        }
        
        Commands::Send { to, amount, wallet, node } => {
            let wallet_name = wallet.as_deref();
            let wallet_obj = wallet_manager.get_wallet(wallet_name).await?;
            
            // Create IPPAN time instance
            let ippan_time = Arc::new(IppanTime::new());
            
            // Create transaction
            let transaction = wallet_obj.create_payment_tx(&to, amount, ippan_time)?;
            
            // Serialize and encode transaction
            let tx_data = transaction.serialize()?;
            let tx_hex = hex::encode(&tx_data);
            
            // Submit to node
            let client = reqwest::Client::new();
            let response = client
                .post(&format!("{}/tx", node))
                .json(&serde_json::json!({
                    "transaction": tx_hex
                }))
                .send()
                .await?;
            
            if response.status().is_success() {
                let result: serde_json::Value = response.json().await?;
                if result["success"].as_bool().unwrap_or(false) {
                    println!("Transaction submitted successfully!");
                    if let Some(tx_id) = result["tx_id"].as_str() {
                        println!("Transaction ID: {}", tx_id);
                    }
                } else {
                    eprintln!("Transaction failed: {}", result["message"].as_str().unwrap_or("Unknown error"));
                }
            } else {
                eprintln!("Failed to submit transaction: HTTP {}", response.status());
            }
        }
        
        Commands::SetDefault { name } => {
            wallet_manager.set_default_wallet(&name).await?;
            println!("Default wallet set to: {}", name);
        }
    }
    
    Ok(())
}
