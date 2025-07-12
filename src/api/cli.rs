//! CLI interface module
//! 
//! Provides command-line interface for the IPPAN node.

use crate::node::IppanNode;
use clap::{Parser, Subcommand};
use std::sync::Arc;
use tokio::sync::RwLock;

/// IPPAN Node CLI
#[derive(Parser)]
#[command(name = "ippan")]
#[command(about = "IPPAN (Immutable Proof & Availability Network) Node")]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the IPPAN node
    Start {
        /// Configuration file path
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        
        /// Enable debug logging
        #[arg(short, long)]
        debug: bool,
    },
    
    /// Stop the IPPAN node
    Stop,
    
    /// Get node status
    Status,
    
    /// Wallet commands
    #[command(subcommand)]
    Wallet(WalletCommands),
    
    /// Storage commands
    #[command(subcommand)]
    Storage(StorageCommands),
    
    /// Network commands
    #[command(subcommand)]
    Network(NetworkCommands),
    
    /// DHT commands
    #[command(subcommand)]
    Dht(DhtCommands),
    
    /// Consensus commands
    #[command(subcommand)]
    Consensus(ConsensusCommands),
}

#[derive(Subcommand)]
enum WalletCommands {
    /// Get wallet balance
    Balance,
    
    /// List wallet addresses
    Addresses,
    
    /// Send payment
    Send {
        /// Recipient address
        #[arg(short, long)]
        to: String,
        
        /// Amount in IPN
        #[arg(short, long)]
        amount: u64,
    },
    
    /// Get transaction history
    Transactions,
    
    /// Stake tokens
    Stake {
        /// Amount to stake
        #[arg(short, long)]
        amount: u64,
    },
    
    /// Unstake tokens
    Unstake {
        /// Amount to unstake
        #[arg(short, long)]
        amount: u64,
    },
}

#[derive(Subcommand)]
enum StorageCommands {
    /// Get storage usage
    Usage,
    
    /// List stored files
    Files,
    
    /// Upload file
    Upload {
        /// File path
        #[arg(short, long)]
        file: String,
    },
    
    /// Download file
    Download {
        /// File hash
        #[arg(short, long)]
        hash: String,
        
        /// Output path
        #[arg(short, long)]
        output: String,
    },
}

#[derive(Subcommand)]
enum NetworkCommands {
    /// Get network stats
    Stats,
    
    /// List connected peers
    Peers,
    
    /// Connect to peer
    Connect {
        /// Peer address
        #[arg(short, long)]
        address: String,
    },
    
    /// Disconnect from peer
    Disconnect {
        /// Peer ID
        #[arg(short, long)]
        peer_id: String,
    },
}

#[derive(Subcommand)]
enum DhtCommands {
    /// List DHT keys
    Keys,
    
    /// Get DHT value
    Get {
        /// Key
        #[arg(short, long)]
        key: String,
    },
    
    /// Put DHT value
    Put {
        /// Key
        #[arg(short, long)]
        key: String,
        
        /// Value
        #[arg(short, long)]
        value: String,
    },
}

#[derive(Subcommand)]
enum ConsensusCommands {
    /// Get current round
    Round,
    
    /// Get recent blocks
    Blocks,
    
    /// Get validators
    Validators,
    
    /// Get consensus stats
    Stats,
}

/// CLI handler for IPPAN node
pub struct CliHandler {
    node: Arc<RwLock<IppanNode>>,
}

impl CliHandler {
    pub fn new(node: Arc<RwLock<IppanNode>>) -> Self {
        Self { node }
    }

    /// Handle CLI commands
    pub async fn handle(&self, cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
        match cli.command {
            Commands::Start { config, debug } => {
                self.handle_start(config, debug).await
            }
            Commands::Stop => {
                self.handle_stop().await
            }
            Commands::Status => {
                self.handle_status().await
            }
            Commands::Wallet(cmd) => {
                self.handle_wallet(cmd).await
            }
            Commands::Storage(cmd) => {
                self.handle_storage(cmd).await
            }
            Commands::Network(cmd) => {
                self.handle_network(cmd).await
            }
            Commands::Dht(cmd) => {
                self.handle_dht(cmd).await
            }
            Commands::Consensus(cmd) => {
                self.handle_consensus(cmd).await
            }
        }
    }

    async fn handle_start(&self, config: String, debug: bool) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting IPPAN node with config: {}", config);
        if debug {
            println!("Debug logging enabled");
        }
        
        let mut node = self.node.write().await;
        node.start().await?;
        println!("IPPAN node started successfully");
        Ok(())
    }

    async fn handle_stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Stopping IPPAN node...");
        let mut node = self.node.write().await;
        node.stop().await?;
        println!("IPPAN node stopped");
        Ok(())
    }

    async fn handle_status(&self) -> Result<(), Box<dyn std::error::Error>> {
        let node = self.node.read().await;
        let status = node.get_status();
        
        println!("=== IPPAN Node Status ===");
        println!("Node ID: {:?}", node.node_id());
        println!("Peer ID: {}", node.peer_id());
        println!("Version: {}", env!("CARGO_PKG_VERSION"));
        println!("Uptime: {:?}", node.get_uptime());
        println!("Consensus Round: {}", node.consensus.get_current_round());
        println!("Connected Peers: {}", node.network.get_peer_count());
        println!("Storage Used: {} / {} bytes", 
            node.storage.get_usage().used_bytes,
            node.storage.get_usage().total_bytes);
        println!("Wallet Balance: {} IPN", node.wallet.get_balance());
        println!("DHT Keys: {}", node.dht.get_key_count());
        Ok(())
    }

    async fn handle_wallet(&self, cmd: WalletCommands) -> Result<(), Box<dyn std::error::Error>> {
        match cmd {
            WalletCommands::Balance => {
                let node = self.node.read().await;
                let balance = node.wallet.get_balance();
                let staked = node.wallet.get_staked_amount();
                println!("Balance: {} IPN", balance);
                println!("Staked: {} IPN", staked);
                println!("Available: {} IPN", balance - staked);
            }
            WalletCommands::Addresses => {
                let node = self.node.read().await;
                let addresses = node.wallet.get_addresses();
                println!("Wallet Addresses:");
                for (i, addr) in addresses.iter().enumerate() {
                    println!("  {}. {}", i + 1, addr);
                }
            }
            WalletCommands::Send { to, amount } => {
                let mut node = self.node.write().await;
                match node.wallet.send_payment(&to, amount).await {
                    Ok(tx_hash) => {
                        println!("Payment sent successfully!");
                        println!("Transaction Hash: {:?}", tx_hash);
                        println!("Amount: {} IPN", amount);
                        println!("To: {}", to);
                    }
                    Err(e) => {
                        eprintln!("Payment failed: {}", e);
                    }
                }
            }
            WalletCommands::Transactions => {
                let node = self.node.read().await;
                let transactions = node.wallet.get_transactions();
                println!("Transaction History:");
                for tx in transactions {
                    println!("  Hash: {:?}", tx.hash);
                    println!("  Amount: {} IPN", tx.amount);
                    println!("  To: {}", tx.to_address);
                    println!("  Time: {}", tx.timestamp);
                    println!();
                }
            }
            WalletCommands::Stake { amount } => {
                let mut node = self.node.write().await;
                match node.wallet.stake(amount).await {
                    Ok(_) => {
                        println!("Staked {} IPN successfully", amount);
                    }
                    Err(e) => {
                        eprintln!("Staking failed: {}", e);
                    }
                }
            }
            WalletCommands::Unstake { amount } => {
                let mut node = self.node.write().await;
                match node.wallet.unstake(amount).await {
                    Ok(_) => {
                        println!("Unstaked {} IPN successfully", amount);
                    }
                    Err(e) => {
                        eprintln!("Unstaking failed: {}", e);
                    }
                }
            }
        }
        Ok(())
    }

    async fn handle_storage(&self, cmd: StorageCommands) -> Result<(), Box<dyn std::error::Error>> {
        match cmd {
            StorageCommands::Usage => {
                let node = self.node.read().await;
                let usage = node.storage.get_usage();
                println!("Storage Usage:");
                println!("  Used: {} bytes", usage.used_bytes);
                println!("  Total: {} bytes", usage.total_bytes);
                println!("  Available: {} bytes", usage.total_bytes - usage.used_bytes);
                println!("  Shards: {}", usage.shard_count);
            }
            StorageCommands::Files => {
                let node = self.node.read().await;
                let files = node.storage.get_files();
                println!("Stored Files:");
                for file in files {
                    println!("  Hash: {:?}", file.hash);
                    println!("  Size: {} bytes", file.size);
                    println!("  Shards: {}", file.shard_count);
                    println!("  Uploaded: {:?}", file.uploaded_at);
                    println!();
                }
            }
            StorageCommands::Upload { file } => {
                let mut node = self.node.write().await;
                match std::fs::read(&file) {
                    Ok(data) => {
                        match node.storage.store_file(&data, &file).await {
                            Ok(hash) => {
                                println!("File uploaded successfully!");
                                println!("Hash: {:?}", hash);
                                println!("Size: {} bytes", data.len());
                            }
                            Err(e) => {
                                eprintln!("Upload failed: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to read file: {}", e);
                    }
                }
            }
            StorageCommands::Download { hash, output } => {
                let node = self.node.read().await;
                // Implementation would decode hash and retrieve file
                println!("Download functionality not yet implemented");
                println!("Hash: {}", hash);
                println!("Output: {}", output);
            }
        }
        Ok(())
    }

    async fn handle_network(&self, cmd: NetworkCommands) -> Result<(), Box<dyn std::error::Error>> {
        match cmd {
            NetworkCommands::Stats => {
                let node = self.node.read().await;
                println!("Network Statistics:");
                println!("  Connected Peers: {}", node.network.get_peer_count());
                println!("  Total Nodes: {}", node.network.get_total_nodes());
                println!("  Active Nodes: {}", node.network.get_active_nodes());
            }
            NetworkCommands::Peers => {
                let node = self.node.read().await;
                let peers = node.network.get_peers();
                println!("Connected Peers:");
                for peer in peers {
                    println!("  Peer ID: {}", peer.peer_id);
                    println!("  Address: {}", peer.address);
                    println!("  Last Seen: {:?}", peer.last_seen);
                    println!();
                }
            }
            NetworkCommands::Connect { address } => {
                let mut node = self.node.write().await;
                match node.network.connect_peer(&address).await {
                    Ok(_) => {
                        println!("Connected to peer: {}", address);
                    }
                    Err(e) => {
                        eprintln!("Connection failed: {}", e);
                    }
                }
            }
            NetworkCommands::Disconnect { peer_id } => {
                let mut node = self.node.write().await;
                match node.network.disconnect_peer(&peer_id).await {
                    Ok(_) => {
                        println!("Disconnected from peer: {}", peer_id);
                    }
                    Err(e) => {
                        eprintln!("Disconnection failed: {}", e);
                    }
                }
            }
        }
        Ok(())
    }

    async fn handle_dht(&self, cmd: DhtCommands) -> Result<(), Box<dyn std::error::Error>> {
        match cmd {
            DhtCommands::Keys => {
                let node = self.node.read().await;
                let keys = node.dht.get_keys();
                println!("DHT Keys:");
                for key in keys {
                    println!("  {}", key);
                }
            }
            DhtCommands::Get { key } => {
                let node = self.node.read().await;
                match node.dht.get(&key) {
                    Some(value) => {
                        println!("Key: {}", key);
                        println!("Value: {}", value);
                    }
                    None => {
                        println!("Key '{}' not found", key);
                    }
                }
            }
            DhtCommands::Put { key, value } => {
                let mut node = self.node.write().await;
                match node.dht.put(&key, &value).await {
                    Ok(_) => {
                        println!("Stored key-value pair successfully");
                        println!("Key: {}", key);
                        println!("Value: {}", value);
                    }
                    Err(e) => {
                        eprintln!("Failed to store key-value pair: {}", e);
                    }
                }
            }
        }
        Ok(())
    }

    async fn handle_consensus(&self, cmd: ConsensusCommands) -> Result<(), Box<dyn std::error::Error>> {
        match cmd {
            ConsensusCommands::Round => {
                let node = self.node.read().await;
                let round = node.consensus.get_current_round();
                println!("Current Consensus Round: {}", round);
            }
            ConsensusCommands::Blocks => {
                let node = self.node.read().await;
                let blocks = node.consensus.get_recent_blocks();
                println!("Recent Blocks:");
                for block in blocks {
                    println!("  Hash: {:?}", block.hash());
                    println!("  Round: {}", block.round());
                    println!("  Timestamp: {}", block.timestamp());
                    println!("  Transactions: {}", block.transactions().len());
                    println!();
                }
            }
            ConsensusCommands::Validators => {
                let node = self.node.read().await;
                let validators = node.consensus.get_validators();
                println!("Validators:");
                for validator in validators {
                    println!("  Node ID: {:?}", validator.node_id);
                    println!("  Stake: {} IPN", validator.stake_amount);
                    println!("  Active: {}", validator.is_active);
                    println!();
                }
            }
            ConsensusCommands::Stats => {
                let node = self.node.read().await;
                println!("Consensus Statistics:");
                println!("  Current Round: {}", node.consensus.get_current_round());
                println!("  Is Validator: {}", node.consensus.is_validator());
                println!("  Stake Amount: {} IPN", node.consensus.get_stake_amount());
            }
        }
        Ok(())
    }
}
