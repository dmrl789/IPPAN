//! IPPAN Blockchain Command Line Interface
//!
//! A comprehensive CLI tool for interacting with IPPAN nodes.

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use ippan_wallet::keyfile::KeyFile;
use serde_json::{Map, Value};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "ippan-cli")]
#[command(about = "IPPAN Blockchain Command Line Interface", long_about = None)]
#[command(version)]
struct Cli {
    /// RPC endpoint URL
    #[arg(long, alias = "rpc", default_value = "http://localhost:8080")]
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
    /// Send an L1 payment via the node RPC
    Pay(PayCommand),
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
        address: String,
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
        hash: String,
    },
    /// Send a real signed payment (convenience wrapper over POST /tx/payment)
    Send(TxSendCommand),
    /// Send raw transaction (legacy)
    SendRaw {
        /// Raw transaction hex
        raw_tx: String,
    },
    /// List pending transactions
    Pending,
}

#[derive(Args)]
struct TxSendCommand {
    /// Path to the sender keyfile (ippan-wallet JSON keyfile)
    #[arg(long, alias = "from", value_name = "PATH")]
    from_key: PathBuf,
    /// Optional keyfile password (can also be provided via IPPAN_KEY_PASSWORD env)
    #[arg(long)]
    from_key_password: Option<String>,
    /// Recipient identifier (Base58Check, hex, or @handle)
    #[arg(long)]
    to: String,
    /// Amount in atomic units
    #[arg(long)]
    amount: u128,
    /// Optional fee limit in atomic units
    #[arg(long)]
    fee: Option<u128>,
    /// Optional explicit nonce (otherwise derived by the node)
    #[arg(long)]
    nonce: Option<u64>,
    /// Optional memo/topic
    #[arg(long)]
    memo: Option<String>,
}

#[derive(Subcommand)]
enum QueryCommands {
    /// Get block by height or hash
    Block {
        /// Block height or hash
        id: String,
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
        validator_id: String,
    },
}

#[derive(Args)]
struct PayCommand {
    /// Sender address (Base58Check or hex)
    #[arg(long)]
    from: String,
    /// Recipient address
    #[arg(long)]
    to: String,
    /// Amount in atomic IPN units (yocto-IPN)
    #[arg(long)]
    amount: u128,
    /// Signing key hex string (32 bytes)
    #[arg(long, conflicts_with = "key_file", value_name = "HEX")]
    signing_key_hex: Option<String>,
    /// Path to file containing the signing key hex
    #[arg(long, value_name = "PATH")]
    key_file: Option<PathBuf>,
    /// Optional fee limit (atomic units)
    #[arg(long)]
    fee: Option<u128>,
    /// Explicit nonce (otherwise fetched from node)
    #[arg(long)]
    nonce: Option<u64>,
    /// Optional memo/topic
    #[arg(long)]
    memo: Option<String>,
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
        Commands::Pay(cmd) => handle_pay_command(cmd, &cli.rpc_url).await,
    }
}

async fn handle_node_commands(cmd: NodeCommands, rpc_url: &str) -> Result<()> {
    let client = reqwest::Client::new();

    let response = match cmd {
        NodeCommands::Status => client.get(format!("{rpc_url}/node/status")).send().await?,
        NodeCommands::Peers => client.get(format!("{rpc_url}/node/peers")).send().await?,
        NodeCommands::Version => client.get(format!("{rpc_url}/node/version")).send().await?,
        NodeCommands::Info => client.get(format!("{rpc_url}/node/info")).send().await?,
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
                .get(format!("{rpc_url}/wallet/{address}/balance"))
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
                .post(format!("{rpc_url}/transaction"))
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
                .get(format!("{rpc_url}/transaction/{hash}"))
                .send()
                .await?;

            let json: Value = response.json().await?;
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        TxCommands::Send(send) => {
            let password = send
                .from_key_password
                .or_else(|| std::env::var("IPPAN_KEY_PASSWORD").ok());
            let keyfile = KeyFile::load(&send.from_key)
                .with_context(|| format!("failed to load keyfile {}", send.from_key.display()))?;
            let unlocked = keyfile
                .unlock(password.as_deref())
                .context("failed to unlock keyfile")?;

            let mut payload = Map::new();
            payload.insert("from".into(), Value::String(unlocked.address));
            payload.insert("to".into(), Value::String(send.to));
            payload.insert("amount".into(), Value::String(send.amount.to_string()));
            payload.insert(
                "signing_key".into(),
                Value::String(hex::encode(unlocked.private_key)),
            );
            if let Some(fee_limit) = send.fee {
                payload.insert("fee".into(), Value::String(fee_limit.to_string()));
            }
            if let Some(nonce_value) = send.nonce {
                payload.insert(
                    "nonce".into(),
                    Value::Number(serde_json::Number::from(nonce_value)),
                );
            }
            if let Some(memo_value) = send.memo {
                payload.insert("memo".into(), Value::String(memo_value));
            }

            let response = client
                .post(format!("{rpc_url}/tx/payment"))
                .json(&Value::Object(payload))
                .send()
                .await?;

            let status = response.status();
            let body = response.json::<Value>().await.unwrap_or(Value::Null);

            if status.is_success() {
                if let Some(tx_hash) = body.get("tx_hash").and_then(|v| v.as_str()) {
                    println!("Payment accepted: {tx_hash}");
                } else {
                    println!("{}", serde_json::to_string_pretty(&body)?);
                }
                ()
            } else {
                anyhow::bail!(
                    "payment rejected (status {}): {}",
                    status,
                    serde_json::to_string_pretty(&body)?
                )
            }
        }
        TxCommands::SendRaw { raw_tx } => {
            let payload = serde_json::json!({
                "raw": raw_tx
            });

            let response = client
                .post(format!("{rpc_url}/transaction"))
                .json(&payload)
                .send()
                .await?;

            let json: Value = response.json().await?;
            println!("Transaction sent!");
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        TxCommands::Pending => {
            let response = client
                .get(format!("{rpc_url}/transactions/pending"))
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
        QueryCommands::Block { id } => client.get(format!("{rpc_url}/block/{id}")).send().await?,
        QueryCommands::LatestBlock => client.get(format!("{rpc_url}/block/latest")).send().await?,
        QueryCommands::Info => {
            client
                .get(format!("{rpc_url}/blockchain/info"))
                .send()
                .await?
        }
        QueryCommands::Stats => {
            client
                .get(format!("{rpc_url}/blockchain/stats"))
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
                .post(format!("{rpc_url}/validator/register"))
                .json(&payload)
                .send()
                .await?;

            let json: Value = response.json().await?;
            println!("Validator registered!");
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        ValidatorCommands::List => {
            let response = client.get(format!("{rpc_url}/validators")).send().await?;

            let json: Value = response.json().await?;
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        ValidatorCommands::Info { validator_id } => {
            let response = client
                .get(format!("{rpc_url}/validator/{validator_id}"))
                .send()
                .await?;

            let json: Value = response.json().await?;
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
    }

    Ok(())
}

async fn handle_pay_command(cmd: PayCommand, rpc_url: &str) -> Result<()> {
    let PayCommand {
        from,
        to,
        amount,
        signing_key_hex,
        key_file,
        fee,
        nonce,
        memo,
    } = cmd;

    let signing_key = if let Some(hex) = signing_key_hex {
        hex
    } else if let Some(path) = key_file {
        fs::read_to_string(&path)
            .with_context(|| format!("failed to read signing key file {}", path.display()))?
            .trim()
            .to_string()
    } else {
        anyhow::bail!("either --signing-key-hex or --key-file must be provided");
    };

    let mut payload = Map::new();
    payload.insert("from".into(), Value::String(from));
    payload.insert("to".into(), Value::String(to));
    payload.insert("amount".into(), Value::String(amount.to_string()));
    payload.insert(
        "signing_key".into(),
        Value::String(signing_key.trim().to_string()),
    );

    if let Some(fee_limit) = fee {
        payload.insert("fee".into(), Value::String(fee_limit.to_string()));
    }
    if let Some(nonce_value) = nonce {
        payload.insert(
            "nonce".into(),
            Value::Number(serde_json::Number::from(nonce_value)),
        );
    }
    if let Some(memo_value) = memo {
        payload.insert("memo".into(), Value::String(memo_value));
    }
    let payload_value = Value::Object(payload);

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{rpc_url}/tx/payment"))
        .json(&payload_value)
        .send()
        .await?;

    let status = response.status();
    let body = response.json::<Value>().await.unwrap_or(Value::Null);

    if status.is_success() {
        if let Some(tx_hash) = body.get("tx_hash").and_then(|v| v.as_str()) {
            println!("Payment accepted: {tx_hash}");
        } else {
            println!("{}", serde_json::to_string_pretty(&body)?);
        }
        Ok(())
    } else {
        anyhow::bail!(
            "payment rejected (status {}): {}",
            status,
            serde_json::to_string_pretty(&body)?
        )
    }
}
