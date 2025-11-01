use clap::{Parser, Subcommand};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Arc;

use crate::errors::*;
use crate::operations::*;
use crate::storage::WalletStorage;
use crate::WalletBackup;

/// IPPAN Multi-Address Wallet CLI
#[derive(Parser)]
#[command(name = "ippan-wallet")]
#[command(about = "A comprehensive wallet for managing multiple IPPAN addresses")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Wallet directory path
    #[arg(long, default_value = "./wallet")]
    pub wallet_dir: PathBuf,

    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new wallet
    Init {
        /// Wallet name
        #[arg(short, long, default_value = "My IPPAN Wallet")]
        name: String,

        /// Enable password protection
        #[arg(short, long)]
        password: bool,
    },

    /// Generate a new address
    NewAddress {
        /// Address label
        #[arg(short, long)]
        label: Option<String>,

        /// Generate multiple addresses
        #[arg(short, long, default_value = "1")]
        count: usize,
    },

    /// List all addresses
    ListAddresses,

    /// Get address balance
    Balance {
        /// Address to check balance for
        address: Option<String>,
    },

    /// Send transaction
    Send {
        /// Sender address
        from: String,

        /// Recipient address
        to: String,

        /// Amount to send (in atomic units)
        amount: u64,
    },

    /// Show transaction history
    History {
        /// Address to show history for
        address: Option<String>,
    },

    /// Create wallet backup
    Backup,

    /// Restore wallet from backup
    Restore {
        /// Backup file path
        backup_path: PathBuf,
    },

    /// List available backups
    ListBackups,

    /// Show wallet statistics
    Stats,

    /// Sync wallet with blockchain
    Sync,

    /// Export wallet data
    Export {
        /// Output file path
        output: PathBuf,
    },

    /// Import wallet data
    Import {
        /// Input file path
        input: PathBuf,
    },
}

/// CLI application runner
pub struct CliApp {
    wallet_manager: Option<WalletManager>,
    wallet_dir: PathBuf,
    _verbose: bool,
}

impl CliApp {
    pub fn new(wallet_dir: PathBuf, verbose: bool) -> Self {
        Self {
            wallet_manager: None,
            wallet_dir,
            _verbose: verbose,
        }
    }

    pub async fn run(&mut self, cli: Cli) -> Result<()> {
        match cli.command {
            Commands::Init { name, password } => self.init_wallet(name, password).await,
            Commands::NewAddress { label, count } => {
                self.load_wallet().await?;
                self.generate_addresses(label, count).await
            }
            Commands::ListAddresses => {
                self.load_wallet().await?;
                self.list_addresses().await
            }
            Commands::Balance { address } => {
                self.load_wallet().await?;
                self.show_balance(address).await
            }
            Commands::Send { from, to, amount } => {
                self.load_wallet().await?;
                self.send_transaction(from, to, amount).await
            }
            Commands::History { address } => {
                self.load_wallet().await?;
                self.show_history(address).await
            }
            Commands::Backup => {
                self.load_wallet().await?;
                self.create_backup().await
            }
            Commands::Restore { backup_path } => self.restore_wallet(backup_path).await,
            Commands::ListBackups => self.list_backups().await,
            Commands::Stats => {
                self.load_wallet().await?;
                self.show_stats().await
            }
            Commands::Sync => {
                self.load_wallet().await?;
                self.sync_wallet().await
            }
            Commands::Export { output } => {
                self.load_wallet().await?;
                self.export_wallet(output).await
            }
            Commands::Import { input } => self.import_wallet(input).await,
        }
    }

    async fn init_wallet(&mut self, name: String, password: bool) -> Result<()> {
        let storage = Arc::new(WalletStorage::new(&self.wallet_dir));
        let wallet_manager = WalletManager::new(storage, None);

        let password_input = if password {
            Some(self.prompt_password("Enter wallet password: ")?)
        } else {
            None
        };

        wallet_manager.create_wallet(name.clone(), password_input.as_deref())?;
        self.wallet_manager = Some(wallet_manager);

        println!("✅ Wallet '{}' created successfully!", name);
        if password {
            println!("🔒 Wallet is password protected");
        }

        Ok(())
    }

    async fn load_wallet(&mut self) -> Result<()> {
        let storage = Arc::new(WalletStorage::new(&self.wallet_dir));

        if !storage.wallet_exists() {
            return Err(WalletError::WalletNotInitialized);
        }

        let password = self
            .prompt_password_optional("Enter wallet password (or press Enter for no password): ")?;
        let wallet_manager = WalletManager::new(storage, None);
        wallet_manager.load_wallet(password.as_deref())?;

        self.wallet_manager = Some(wallet_manager);
        Ok(())
    }

    async fn generate_addresses(&self, label: Option<String>, count: usize) -> Result<()> {
        let wallet = self.wallet_manager.as_ref().unwrap();

        let password = self
            .prompt_password_optional("Enter wallet password (or press Enter for no password): ")?;

        if count == 1 {
            let address = wallet.generate_address(label.clone(), password.as_deref())?;
            println!("✅ Generated new address:");
            println!("   Address: {}", address);
            if let Some(label_value) = label.as_ref() {
                println!("   Label: {}", label_value);
            }
        } else {
            let label_prefix = label.unwrap_or_else(|| "Address".to_string());
            let addresses =
                wallet.generate_addresses(count, Some(label_prefix), password.as_deref())?;

            println!("✅ Generated {} new addresses:", count);
            for (i, address) in addresses.iter().enumerate() {
                println!("   {}. {}", i + 1, address);
            }
        }

        Ok(())
    }

    async fn list_addresses(&self) -> Result<()> {
        let wallet = self.wallet_manager.as_ref().unwrap();
        let addresses = wallet.list_addresses()?;

        if addresses.is_empty() {
            println!("No addresses found in wallet");
            return Ok(());
        }

        println!("📋 Wallet Addresses ({}):", addresses.len());
        println!(
            "{:<4} {:<65} {:<20} {:<15}",
            "#", "Address", "Label", "Balance"
        );
        println!("{}", "-".repeat(110));

        for (i, addr) in addresses.iter().enumerate() {
            let balance = wallet.get_address_balance(&addr.address).unwrap_or(0);
            let label = addr.label.as_deref().unwrap_or("No label");
            println!(
                "{:<4} {:<65} {:<20} {:<15}",
                i + 1,
                addr.address,
                label,
                balance
            );
        }

        Ok(())
    }

    async fn show_balance(&self, address: Option<String>) -> Result<()> {
        let wallet = self.wallet_manager.as_ref().unwrap();

        if let Some(addr) = address {
            let balance = wallet.get_address_balance(&addr)?;
            println!("💰 Balance for {}: {}", addr, balance);
        } else {
            let total_balance = wallet.get_total_balance()?;
            println!("💰 Total wallet balance: {}", total_balance);
        }

        Ok(())
    }

    async fn send_transaction(&self, from: String, to: String, amount: u64) -> Result<()> {
        let wallet = self.wallet_manager.as_ref().unwrap();

        let password = self
            .prompt_password_optional("Enter wallet password (or press Enter for no password): ")?;

        println!("📤 Sending {} from {} to {}...", amount, from, to);

        let tx_hash = wallet.send_transaction(&from, &to, amount, password.as_deref())?;

        println!("✅ Transaction sent successfully!");
        println!("   Transaction Hash: {}", tx_hash);

        Ok(())
    }

    async fn show_history(&self, address: Option<String>) -> Result<()> {
        let wallet = self.wallet_manager.as_ref().unwrap();

        let transactions = if let Some(addr) = address {
            wallet.get_address_transactions(&addr)?
        } else {
            wallet.get_all_transactions()?
        };

        if transactions.is_empty() {
            println!("No transactions found");
            return Ok(());
        }

        println!(
            "📜 Transaction History ({} transactions):",
            transactions.len()
        );
        println!(
            "{:<20} {:<65} {:<65} {:<15} {:<10}",
            "Time", "From", "To", "Amount", "Status"
        );
        println!("{}", "-".repeat(200));

        for tx in transactions.iter().take(20) {
            // Show last 20 transactions
            let from = tx.from_address.as_deref().unwrap_or("Unknown");
            let to = tx.to_address.as_deref().unwrap_or("Unknown");
            let time = tx.timestamp.format("%Y-%m-%d %H:%M:%S");
            let status = format!("{:?}", tx.status);

            println!(
                "{:<20} {:<65} {:<65} {:<15} {:<10}",
                time, from, to, tx.amount, status
            );
        }

        if transactions.len() > 20 {
            println!("... and {} more transactions", transactions.len() - 20);
        }

        Ok(())
    }

    async fn create_backup(&self) -> Result<()> {
        let wallet = self.wallet_manager.as_ref().unwrap();
        let backup_path = wallet.create_backup()?;

        println!("✅ Wallet backup created successfully!");
        println!("   Backup file: {}", backup_path.display());

        Ok(())
    }

    async fn restore_wallet(&mut self, backup_path: PathBuf) -> Result<()> {
        let storage = Arc::new(WalletStorage::new(&self.wallet_dir));
        let wallet_manager = WalletManager::new(storage, None);

        let password = self
            .prompt_password_optional("Enter wallet password (or press Enter for no password): ")?;

        wallet_manager.restore_from_backup(&backup_path, password.as_deref())?;
        self.wallet_manager = Some(wallet_manager);

        println!(
            "✅ Wallet restored successfully from: {}",
            backup_path.display()
        );

        Ok(())
    }

    async fn list_backups(&self) -> Result<()> {
        let storage = WalletStorage::new(&self.wallet_dir);
        let backups = storage.list_backups()?;

        if backups.is_empty() {
            println!("No backups found");
            return Ok(());
        }

        println!("📦 Available Backups ({}):", backups.len());
        for (i, backup) in backups.iter().enumerate() {
            let metadata = std::fs::metadata(backup)?;
            let modified = metadata.modified()?;
            let datetime: chrono::DateTime<chrono::Utc> = modified.into();

            println!(
                "   {}. {} ({})",
                i + 1,
                backup.file_name().unwrap().to_string_lossy(),
                datetime.format("%Y-%m-%d %H:%M:%S")
            );
        }

        Ok(())
    }

    async fn show_stats(&self) -> Result<()> {
        let wallet = self.wallet_manager.as_ref().unwrap();
        let stats = wallet.get_wallet_stats()?;

        println!("📊 Wallet Statistics:");
        println!("   Name: {}", stats.name);
        println!("   Addresses: {}", stats.address_count);
        println!("   Total Balance: {}", stats.total_balance);
        println!("   Transactions: {}", stats.transaction_count);
        println!(
            "   Created: {}",
            stats.created_at.format("%Y-%m-%d %H:%M:%S")
        );

        if let Some(last_sync) = stats.last_sync {
            println!("   Last Sync: {}", last_sync.format("%Y-%m-%d %H:%M:%S"));
        } else {
            println!("   Last Sync: Never");
        }

        Ok(())
    }

    async fn sync_wallet(&self) -> Result<()> {
        let wallet = self.wallet_manager.as_ref().unwrap();

        println!("🔄 Syncing wallet with blockchain...");
        wallet.sync_wallet()?;

        println!("✅ Wallet synced successfully!");

        Ok(())
    }

    async fn export_wallet(&self, output: PathBuf) -> Result<()> {
        let wallet = self.wallet_manager.as_ref().unwrap();
        let backup = wallet.export_wallet()?;

        let data = serde_json::to_string_pretty(&backup)?;
        std::fs::write(&output, data)?;

        println!("✅ Wallet exported to: {}", output.display());

        Ok(())
    }

    async fn import_wallet(&mut self, input: PathBuf) -> Result<()> {
        let storage = Arc::new(WalletStorage::new(&self.wallet_dir));
        let wallet_manager = WalletManager::new(storage, None);

        let data = std::fs::read_to_string(&input)?;
        let backup: WalletBackup = serde_json::from_str(&data)?;

        let password = self
            .prompt_password_optional("Enter wallet password (or press Enter for no password): ")?;

        wallet_manager.import_wallet(backup, password.as_deref())?;
        self.wallet_manager = Some(wallet_manager);

        println!("✅ Wallet imported successfully from: {}", input.display());

        Ok(())
    }

    fn prompt_password(&self, prompt: &str) -> Result<String> {
        print!("{}", prompt);
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        Ok(input.trim().to_string())
    }

    fn prompt_password_optional(&self, prompt: &str) -> Result<Option<String>> {
        print!("{}", prompt);
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let trimmed = input.trim();
        if trimmed.is_empty() {
            Ok(None)
        } else {
            Ok(Some(trimmed.to_string()))
        }
    }
}

/// Main CLI entry point
pub async fn run_cli() -> Result<()> {
    let cli = Cli::parse();
    let mut app = CliApp::new(cli.wallet_dir.clone(), cli.verbose);
    app.run(cli).await
}
