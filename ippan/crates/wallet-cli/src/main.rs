use clap::{Parser, Subcommand};
use ippan_common::{Transaction, KeyPair, crypto::derive_address, time::ippan_time_us, crypto::hashtimer};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use anyhow::{Result, Context};
use reqwest::Client;
use tokio;

/// IPPAN Wallet CLI
#[derive(Parser)]
#[command(name = "ippan-wallet")]
#[command(about = "IPPAN Wallet - Manage keys and send transactions")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new wallet
    New {
        /// Wallet name
        #[arg(short, long)]
        name: String,
        
        /// Password for encryption
        #[arg(short, long)]
        password: String,
    },
    
    /// Recover wallet from seed phrase
    Recover {
        /// Wallet name
        #[arg(short, long)]
        name: String,
        
        /// Seed phrase (space-separated words)
        #[arg(short, long)]
        seed: String,
        
        /// Password for encryption
        #[arg(short, long)]
        password: String,
    },
    
    /// Show wallet address
    Addr {
        /// Wallet name
        #[arg(short, long)]
        name: String,
    },
    
    /// Send transaction
    Send {
        /// Wallet name
        #[arg(short, long)]
        name: String,
        
        /// Recipient address
        #[arg(short, long)]
        to: String,
        
        /// Amount to send
        #[arg(short, long)]
        amount: u64,
        
        /// Nonce (use 'auto' for automatic)
        #[arg(short, long, default_value = "auto")]
        nonce: String,
        
        /// Node URL
        #[arg(short, long, default_value = "http://127.0.0.1:8080")]
        node: String,
    },
    
    /// List all wallets
    List,
    
    /// Show wallet balance
    Balance {
        /// Wallet name
        #[arg(short, long)]
        name: String,
        
        /// Node URL
        #[arg(short, long, default_value = "http://127.0.0.1:8080")]
        node: String,
    },
}

/// Wallet storage structure
#[derive(Debug, Serialize, Deserialize)]
struct WalletStore {
    wallets: HashMap<String, Wallet>,
}

/// Individual wallet
#[derive(Debug, Serialize, Deserialize, Clone)]
struct Wallet {
    name: String,
    encrypted_keypair: Vec<u8>, // TODO: Implement proper encryption
    address: String,
    created_at: u64,
}

impl WalletStore {
    fn load() -> Result<Self> {
        let path = Self::get_store_path()?;
        if path.exists() {
            let data = fs::read_to_string(&path)
                .context("Failed to read wallet store")?;
            let store: WalletStore = serde_json::from_str(&data)
                .context("Failed to parse wallet store")?;
            Ok(store)
        } else {
            Ok(WalletStore {
                wallets: HashMap::new(),
            })
        }
    }
    
    fn save(&self) -> Result<()> {
        let path = Self::get_store_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .context("Failed to create wallet directory")?;
        }
        
        let data = serde_json::to_string_pretty(self)
            .context("Failed to serialize wallet store")?;
        fs::write(&path, data)
            .context("Failed to write wallet store")?;
        Ok(())
    }
    
    fn get_store_path() -> Result<PathBuf> {
        let mut path = dirs::home_dir()
            .context("Could not find home directory")?;
        path.push(".ippan");
        path.push("wallets.json");
        Ok(path)
    }
    
    fn add_wallet(&mut self, wallet: Wallet) -> Result<()> {
        if self.wallets.contains_key(&wallet.name) {
            anyhow::bail!("Wallet '{}' already exists", wallet.name);
        }
        self.wallets.insert(wallet.name.clone(), wallet);
        self.save()?;
        Ok(())
    }
    
    fn get_wallet(&self, name: &str) -> Result<&Wallet> {
        self.wallets.get(name)
            .context(format!("Wallet '{}' not found", name))
    }
    
    fn list_wallets(&self) -> Vec<&Wallet> {
        self.wallets.values().collect()
    }
}

impl Wallet {
    fn new(name: String, keypair: KeyPair) -> Result<Self> {
        let address = derive_address(&keypair.public_key);
        let encrypted_keypair = Self::encrypt_keypair(&keypair, "temp_password")?; // TODO: Use proper password
        
        Ok(Self {
            name,
            encrypted_keypair,
            address,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        })
    }
    
    fn decrypt_keypair(&self, _password: &str) -> Result<KeyPair> {
        // TODO: Implement proper decryption
        // For now, just create a new keypair (this is not secure!)
        Ok(KeyPair::generate())
    }
    
    fn encrypt_keypair(_keypair: &KeyPair, _password: &str) -> Result<Vec<u8>> {
        // TODO: Implement proper encryption
        // For now, just return empty vector
        Ok(Vec::new())
    }
}

/// Transaction sender
struct TransactionSender {
    client: Client,
    node_url: String,
}

impl TransactionSender {
    fn new(node_url: String) -> Self {
        Self {
            client: Client::new(),
            node_url,
        }
    }
    
    async fn send_transaction(&self, tx: &Transaction) -> Result<String> {
        let tx_data = bincode::serialize(tx)
            .context("Failed to serialize transaction")?;
        
        let response = self.client
            .post(&format!("{}/tx", self.node_url))
            .body(tx_data)
            .header("Content-Type", "application/octet-stream")
            .send()
            .await
            .context("Failed to send transaction")?;
        
        if response.status().is_success() {
            let result = response.text().await
                .context("Failed to read response")?;
            Ok(result)
        } else {
            let error = response.text().await
                .context("Failed to read error response")?;
            anyhow::bail!("Transaction failed: {}", error)
        }
    }
    
    async fn get_balance(&self, address: &str) -> Result<u64> {
        let response = self.client
            .get(&format!("{}/balance/{}", self.node_url, address))
            .send()
            .await
            .context("Failed to get balance")?;
        
        if response.status().is_success() {
            let balance: u64 = response.json().await
                .context("Failed to parse balance")?;
            Ok(balance)
        } else {
            anyhow::bail!("Failed to get balance: {}", response.status())
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::New { name, password: _ } => {
            let keypair = KeyPair::generate();
            let wallet = Wallet::new(name, keypair)?;
            
            let mut store = WalletStore::load()?;
            store.add_wallet(wallet.clone())?;
            
            println!("✅ Wallet created successfully!");
            println!("Address: {}", wallet.address);
            println!("Store your seed phrase safely (not implemented in this MVP)");
        }
        
        Commands::Recover { name: _, seed, password: _ } => {
            // TODO: Implement seed phrase recovery
            println!("⚠️  Seed phrase recovery not implemented in this MVP");
            println!("Seed: {}", seed);
        }
        
        Commands::Addr { name } => {
            let store = WalletStore::load()?;
            let wallet = store.get_wallet(&name)?;
            println!("Address: {}", wallet.address);
        }
        
        Commands::Send { name, to, amount, nonce, node } => {
            let store = WalletStore::load()?;
            let wallet = store.get_wallet(&name)?;
            
            // Decrypt keypair (simplified)
            let keypair = wallet.decrypt_keypair("temp_password")?;
            
            // Parse recipient address
            let to_pubkey = if to.starts_with("i") {
                // TODO: Implement address to public key conversion
                // For now, use a placeholder
                [0u8; 32]
            } else {
                anyhow::bail!("Invalid recipient address format. Expected 'i...'");
            };
            
            // Get current nonce
            let current_nonce = if nonce == "auto" {
                // TODO: Get nonce from node
                0
            } else {
                u64::from_str(&nonce)
                    .context("Invalid nonce value")?
            };
            
            // Create transaction
            let ippan_time = ippan_time_us();
            let tx_id = [0u8; 32]; // Placeholder
            let hashtimer = hashtimer(&tx_id);
            
            let mut tx = Transaction {
                ver: 1,
                from_pub: keypair.public_key,
                to_addr: to_pubkey,
                amount,
                nonce: current_nonce,
                ippan_time_us: ippan_time,
                hashtimer,
                sig: [0u8; 64], // Will be set after signing
            };
            
            // Sign transaction
            tx.sig = keypair.sign(&bincode::serialize(&tx)?)?;
            
            // Send transaction
            let sender = TransactionSender::new(node);
            let result = sender.send_transaction(&tx).await?;
            
            println!("✅ Transaction sent successfully!");
            println!("Result: {}", result);
            println!("Transaction ID: {}", hex::encode(tx_id));
        }
        
        Commands::List => {
            let store = WalletStore::load()?;
            let wallets = store.list_wallets();
            
            if wallets.is_empty() {
                println!("No wallets found");
            } else {
                println!("Available wallets:");
                for wallet in wallets {
                    println!("  {}: {}", wallet.name, wallet.address);
                }
            }
        }
        
        Commands::Balance { name, node } => {
            let store = WalletStore::load()?;
            let wallet = store.get_wallet(&name)?;
            
            let sender = TransactionSender::new(node);
            let balance = sender.get_balance(&wallet.address).await?;
            
            println!("Balance: {} IPPAN", balance);
        }
    }
    
    Ok(())
}
