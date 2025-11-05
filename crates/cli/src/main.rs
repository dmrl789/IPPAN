//! IPPAN Blockchain Command Line Interface
//! 
//! A comprehensive CLI tool for interacting with IPPAN nodes.

use anyhow::Result;
use clap::{Parser, Subcommand};
use serde_json::Value;

#[derive(Parser)]
#[command(name = "ippan-cli")]
#[command(about = "IPPAN Blockchain Command Line Interface", long_about = None)]
#[command(version)]
struct Cli {
    /// RPC endpoint URL
    #[arg(long, default_value = "http://localhost:8080")]
    rpc_url: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Node operations
    Node {
        #[command(subcommand)]
        action: NodeCommands,
    },
    /// Wallet operations
    Wallet {
        #[command(subcommand)]
        action: WalletCommands,
    },
    /// Transaction operations
    Transaction {
        #[command(subcommand)]
        action: TxCommands,
    },
    /// Query blockchain state
    Query {
        #[command(subcommand)]
        action: QueryCommands,
    },
    /// Validator operations
    Validator {
        #[command(subcommand)]
        action: ValidatorCommands,
    },
}

#[derive(Subcommand)]
enum NodeCommands {
    /// Get node status
    Status,
    /// Get node peers
    Peers,
    /// Get node version
    Version,
    /// Get node info
    Info,
}

#[derive(Subcommand)]
enum WalletCommands {
    /// Get wallet balance
    Balance { 
        /// Wallet address
        address: String 
    },
    /// Send transaction
    Send {
        /// From address
        from: String,
        /// To address
        to: String,
        /// Amount to send
        amount: u64,
    },
}

#[derive(Subcommand)]
enum TxCommands {
    /// Get transaction by hash
    Get { 
        /// Transaction hash
        hash: String 
    },
    /// Send raw transaction
    Send { 
        /// Raw transaction hex
        raw_tx: String 
    },
    /// List pending transactions
    Pending,
}

#[derive(Subcommand)]
enum QueryCommands {
    /// Get block by height or hash
    Block { 
        /// Block height or hash
        id: String 
    },
    /// Get latest block
    LatestBlock,
    /// Get blockchain info
    Info,
    /// Get blockchain statistics
    Stats,
}

#[derive(Subcommand)]
enum ValidatorCommands {
    /// Register as validator
    Register {
        /// Validator ID (hex)
        #[arg(long)]
        id: String,
        /// Stake amount
        #[arg(long)]
        stake: u64,
    },
    /// List all validators
    List,
    /// Get validator info
    Info { 
        /// Validator ID
        validator_id: String 
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Node { action } => handle_node_commands(action, &cli.rpc_url).await,
        Commands::Wallet { action } => handle_wallet_commands(action, &cli.rpc_url).await,
        Commands::Transaction { action } => handle_tx_commands(action, &cli.rpc_url).await,
        Commands::Query { action } => handle_query_commands(action, &cli.rpc_url).await,
        Commands::Validator { action } => handle_validator_commands(action, &cli.rpc_url).await,
    }
}

async fn handle_node_commands(cmd: NodeCommands, rpc_url: &str) -> Result<()> {
    let client = reqwest::Client::new();
    
    let response = match cmd {
        NodeCommands::Status => {
            client
                .get(format!("{}/node/status", rpc_url))
                .send()
                .await?
        }
        NodeCommands::Peers => {
            client
                .get(format!("{}/node/peers", rpc_url))
                .send()
                .await?
        }
        NodeCommands::Version => {
            client
                .get(format!("{}/node/version", rpc_url))
                .send()
                .await?
        }
        NodeCommands::Info => {
            client
                .get(format!("{}/node/info", rpc_url))
                .send()
                .await?
        }
    };
    
    let json: Value = response.json().await?;
    println!("{}", serde_json::to_string_pretty(&json)?);
    
    Ok(())
}

async fn handle_wallet_commands(cmd: WalletCommands, rpc_url: &str) -> Result<()> {
    let client = reqwest::Client::new();
    
    match cmd {
        WalletCommands::Balance { address } => {
            let response = client
                .get(format!("{}/wallet/{}/balance", rpc_url, address))
                .send()
                .await?;
            
            let json: Value = response.json().await?;
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        WalletCommands::Send { from, to, amount } => {
            let payload = serde_json::json!({
                "from": from,
                "to": to,
                "amount": amount,
            });
            
            let response = client
                .post(format!("{}/transaction", rpc_url))
                .json(&payload)
                .send()
                .await?;
            
            let json: Value = response.json().await?;
            println!("Transaction sent!");
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
    }
    
    Ok(())
}

async fn handle_tx_commands(cmd: TxCommands, rpc_url: &str) -> Result<()> {
    let client = reqwest::Client::new();
    
    match cmd {
        TxCommands::Get { hash } => {
            let response = client
                .get(format!("{}/transaction/{}", rpc_url, hash))
                .send()
                .await?;
            
            let json: Value = response.json().await?;
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        TxCommands::Send { raw_tx } => {
            let payload = serde_json::json!({
                "raw": raw_tx
            });
            
            let response = client
                .post(format!("{}/transaction", rpc_url))
                .json(&payload)
                .send()
                .await?;
            
            let json: Value = response.json().await?;
            println!("Transaction sent!");
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        TxCommands::Pending => {
            let response = client
                .get(format!("{}/transactions/pending", rpc_url))
                .send()
                .await?;
            
            let json: Value = response.json().await?;
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
    }
    
    Ok(())
}

async fn handle_query_commands(cmd: QueryCommands, rpc_url: &str) -> Result<()> {
    let client = reqwest::Client::new();
    
    let response = match cmd {
        QueryCommands::Block { id } => {
            client
                .get(format!("{}/block/{}", rpc_url, id))
                .send()
                .await?
        }
        QueryCommands::LatestBlock => {
            client
                .get(format!("{}/block/latest", rpc_url))
                .send()
                .await?
        }
        QueryCommands::Info => {
            client
                .get(format!("{}/blockchain/info", rpc_url))
                .send()
                .await?
        }
        QueryCommands::Stats => {
            client
                .get(format!("{}/blockchain/stats", rpc_url))
                .send()
                .await?
        }
    };
    
    let json: Value = response.json().await?;
    println!("{}", serde_json::to_string_pretty(&json)?);
    
    Ok(())
}

async fn handle_validator_commands(cmd: ValidatorCommands, rpc_url: &str) -> Result<()> {
    let client = reqwest::Client::new();
    
    match cmd {
        ValidatorCommands::Register { id, stake } => {
            let payload = serde_json::json!({
                "validator_id": id,
                "stake": stake,
            });
            
            let response = client
                .post(format!("{}/validator/register", rpc_url))
                .json(&payload)
                .send()
                .await?;
            
            let json: Value = response.json().await?;
            println!("Validator registered!");
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        ValidatorCommands::List => {
            let response = client
                .get(format!("{}/validators", rpc_url))
                .send()
                .await?;
            
            let json: Value = response.json().await?;
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        ValidatorCommands::Info { validator_id } => {
            let response = client
                .get(format!("{}/validator/{}", rpc_url, validator_id))
                .send()
                .await?;
            
            let json: Value = response.json().await?;
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
    }
    
    Ok(())
}
