use crate::crypto::sign_message;
use crate::errors::*;
use crate::keyfile::KeyFile;
use crate::rpc::{AccountState, WalletRpcClient};
use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};
use ippan_types::address::decode_address;
use ippan_types::currency::Amount;
use rpassword::prompt_password;
use serde_json::json;
use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;

/// IPPAN Wallet and Signing CLI
#[derive(Parser, Debug)]
#[command(name = "ippan-wallet")]
#[command(about = "Key management, signing, and payment flows for IPPAN")]
#[command(version)]
pub struct Cli {
    /// Node RPC base URL (used by send-payment)
    #[arg(long, global = true, default_value = "http://127.0.0.1:8080")]
    pub rpc_url: String,

    /// Network profile associated with generated keys
    #[arg(long, global = true, value_enum, default_value = "devnet")]
    pub network: NetworkProfile,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum NetworkProfile {
    Devnet,
    Testnet,
    Mainnet,
}

impl NetworkProfile {
    pub fn as_str(&self) -> &'static str {
        match self {
            NetworkProfile::Devnet => "devnet",
            NetworkProfile::Testnet => "testnet",
            NetworkProfile::Mainnet => "mainnet",
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Generate a new key file and print the derived address/public key
    GenerateKey(GenerateKeyArgs),
    /// Show address and metadata for a stored key
    ShowAddress(ShowAddressArgs),
    /// Sign an arbitrary payload or file and print the signature
    Sign(SignArgs),
    /// Construct and submit a signed payment transaction to the RPC node
    SendPayment(SendPaymentArgs),
}

#[derive(Args, Debug)]
pub struct GenerateKeyArgs {
    /// Output path for the key file
    #[arg(long, default_value = "./keys/ippan-devnet.key")]
    pub out: PathBuf,

    /// Overwrite the key file if it already exists
    #[arg(long, action = ArgAction::SetTrue)]
    pub force: bool,

    /// Store the private key unencrypted (requires explicit opt-in)
    #[arg(long, action = ArgAction::SetTrue)]
    pub insecure_plaintext: bool,

    /// Print the private key to stdout (dev/test only)
    #[arg(long, action = ArgAction::SetTrue)]
    pub print_private_key: bool,

    /// Optional free-form notes embedded in the key metadata
    #[arg(long)]
    pub notes: Option<String>,

    #[command(flatten)]
    pub password: PasswordArgs,
}

#[derive(Args, Debug, Default)]
pub struct PasswordArgs {
    /// Password to encrypt/decrypt the key (insecure on shared terminals)
    #[arg(long)]
    pub password: Option<String>,

    /// Read password from a file (first line is used)
    #[arg(long)]
    pub password_file: Option<PathBuf>,

    /// Prompt interactively (password is not echoed)
    #[arg(long, action = ArgAction::SetTrue)]
    pub prompt_password: bool,
}

#[derive(Args, Debug)]
pub struct ShowAddressArgs {
    /// Path to the key file
    #[arg(long)]
    pub key: PathBuf,

    #[command(flatten)]
    pub password: PasswordArgs,

    /// Emit machine-readable JSON instead of human-friendly text
    #[arg(long, action = ArgAction::SetTrue)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct SignArgs {
    /// Path to the key file
    #[arg(long)]
    pub key: PathBuf,

    #[command(flatten)]
    pub password: PasswordArgs,

    /// ASCII/UTF-8 message to sign
    #[arg(long, conflicts_with_all = ["message_hex", "file"])]
    pub message: Option<String>,

    /// Hex-encoded payload to sign
    #[arg(long, conflicts_with_all = ["message", "file"])]
    pub message_hex: Option<String>,

    /// Path to a file whose raw bytes should be signed
    #[arg(long, conflicts_with_all = ["message", "message_hex"])]
    pub file: Option<PathBuf>,

    /// Optional output file for the signature (defaults to stdout)
    #[arg(long)]
    pub out: Option<PathBuf>,

    /// Emit raw binary signature bytes instead of hex
    #[arg(long, action = ArgAction::SetTrue)]
    pub raw: bool,
}

#[derive(Args, Debug)]
pub struct AmountArg {
    /// Payment amount in IPN (supports up to 24 decimals)
    #[arg(long)]
    pub amount: Option<String>,

    /// Payment amount expressed in atomic units
    #[arg(long)]
    pub amount_atomic: Option<u128>,
}

impl AmountArg {
    pub fn to_atomic(&self, label: &str) -> Result<u128> {
        if let Some(value) = self.amount_atomic {
            return Ok(value);
        }
        if let Some(text) = &self.amount {
            let amount =
                Amount::from_str_ipn(text).map_err(|err| WalletError::InvalidCliUsage(err))?;
            return Ok(amount.atomic());
        }
        Err(WalletError::InvalidCliUsage(format!(
            "missing {}; specify --{} or --{}-atomic",
            label, label, label
        )))
    }
}

#[derive(Args, Debug, Default)]
pub struct FeeArg {
    /// Optional fee limit in IPN (24 decimals)
    #[arg(long)]
    pub fee: Option<String>,

    /// Optional fee limit in atomic units
    #[arg(long)]
    pub fee_atomic: Option<u128>,
}

impl FeeArg {
    pub fn to_atomic(&self) -> Result<Option<u128>> {
        if let Some(value) = self.fee_atomic {
            return Ok(Some(value));
        }
        if let Some(text) = &self.fee {
            let amount =
                Amount::from_str_ipn(text).map_err(|err| WalletError::InvalidCliUsage(err))?;
            return Ok(Some(amount.atomic()));
        }
        Ok(None)
    }
}

#[derive(Args, Debug)]
pub struct SendPaymentArgs {
    /// Path to the signing key file
    #[arg(long)]
    pub key: PathBuf,

    #[command(flatten)]
    pub password: PasswordArgs,

    /// Recipient address (Base58Check or hex)
    #[arg(long)]
    pub to: String,

    #[command(flatten)]
    pub amount: AmountArg,

    #[command(flatten)]
    pub fee: FeeArg,

    /// Explicit nonce (defaults to querying the RPC for the next nonce)
    #[arg(long)]
    pub nonce: Option<u64>,

    /// Optional memo/topic (<=256 bytes)
    #[arg(long)]
    pub memo: Option<String>,

    /// Skip the interactive confirmation prompt
    #[arg(long, action = ArgAction::SetTrue)]
    pub yes: bool,
}

/// Entrypoint invoked by `src/bin/ippan-wallet.rs`.
pub async fn run_cli() -> Result<()> {
    let Cli {
        rpc_url,
        network,
        command,
    } = Cli::parse();

    match command {
        Commands::GenerateKey(args) => handle_generate_key(network, args).await,
        Commands::ShowAddress(args) => handle_show_address(args).await,
        Commands::Sign(args) => handle_sign(args).await,
        Commands::SendPayment(args) => handle_send_payment(&rpc_url, args).await,
    }
}

async fn handle_generate_key(network: NetworkProfile, args: GenerateKeyArgs) -> Result<()> {
    let password = resolve_password(&args.password, true)?;
    if password.is_none() && !args.insecure_plaintext {
        return Err(WalletError::InvalidCliUsage(
            "set a password (e.g. --prompt-password) or pass --insecure-plaintext".into(),
        ));
    }

    let (mut keyfile, unlocked) = KeyFile::generate(
        password.as_deref(),
        Some(network.as_str().to_string()),
        args.insecure_plaintext,
    )?;
    if let Some(notes) = args.notes {
        keyfile.metadata.notes = Some(notes);
    }
    keyfile.save(&args.out, args.force)?;

    println!("✅ Key generated");
    println!("   Address: {}", unlocked.address);
    println!("   Public key: {}", hex::encode(unlocked.public_key));
    println!("   File: {}", args.out.display());
    println!("   Network: {}", network.as_str());
    if args.insecure_plaintext {
        println!("⚠️  Stored WITHOUT encryption (dev/test only)");
    }
    if args.print_private_key {
        println!(
            "⚠️  PRIVATE KEY (do not share): {}",
            hex::encode(unlocked.private_key)
        );
    }

    Ok(())
}

async fn handle_show_address(args: ShowAddressArgs) -> Result<()> {
    let keyfile = KeyFile::load(&args.key)?;
    let password = resolve_password(&args.password, false)?;
    let unlocked = keyfile.unlock(password.as_deref())?;

    if args.json {
        let payload = json!({
            "address": unlocked.address,
            "public_key_hex": hex::encode(unlocked.public_key),
            "created_at": unlocked.metadata.created_at.to_rfc3339(),
            "network_profile": unlocked.metadata.network_profile,
            "warning": unlocked.metadata.warning,
            "notes": unlocked.metadata.notes,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!("Address: {}", unlocked.address);
        println!("Public key: {}", hex::encode(unlocked.public_key));
        println!(
            "Created: {}",
            unlocked.metadata.created_at.format("%Y-%m-%d %H:%M:%S")
        );
        if let Some(network) = &unlocked.metadata.network_profile {
            println!("Network: {network}");
        }
        if let Some(warning) = &unlocked.metadata.warning {
            println!("Warning: {warning}");
        }
        if let Some(notes) = &unlocked.metadata.notes {
            println!("Notes: {notes}");
        }
    }

    Ok(())
}

async fn handle_sign(args: SignArgs) -> Result<()> {
    let payload = resolve_message_payload(&args)?;
    let keyfile = KeyFile::load(&args.key)?;
    let password = resolve_password(&args.password, false)?;
    let unlocked = keyfile.unlock(password.as_deref())?;
    let signature = sign_message(&payload, &unlocked.private_key)?;

    let output_bytes = signature.to_vec();
    if let Some(out_path) = args.out {
        fs::write(
            out_path,
            if args.raw {
                output_bytes
            } else {
                hex::encode(output_bytes).as_bytes().to_vec()
            },
        )?;
    } else if args.raw {
        io::stdout().write_all(&output_bytes)?;
    } else {
        println!("{}", hex::encode(output_bytes));
    }

    Ok(())
}

async fn handle_send_payment(rpc_url: &str, args: SendPaymentArgs) -> Result<()> {
    let rpc = WalletRpcClient::new(rpc_url);
    let keyfile = KeyFile::load(&args.key)?;
    let password = resolve_password(&args.password, false)?;
    let unlocked = keyfile.unlock(password.as_deref())?;
    let amount_atomic = args.amount.to_atomic("amount")?;
    let fee_atomic = args.fee.to_atomic()?;
    decode_any_address(&args.to)?;
    let memo = args.memo.clone();
    if let Some(memo_value) = &memo {
        if memo_value.as_bytes().len() > 256 {
            return Err(WalletError::InvalidCliUsage(
                "memo length exceeds 256 bytes".into(),
            ));
        }
    }

    let derived_nonce = if args.nonce.is_none() {
        derive_nonce(&rpc, &unlocked.address).await?
    } else {
        None
    };
    let next_nonce = args.nonce.or(derived_nonce);

    println!("📤 Preparing payment");
    println!("   From: {}", unlocked.address);
    println!("   To: {}", args.to);
    println!("   Amount (atomic): {}", amount_atomic);
    if let Some(fee) = fee_atomic {
        println!("   Fee limit (atomic): {fee}");
    }
    if let Some(nonce) = next_nonce {
        println!("   Nonce: {nonce}");
    } else {
        println!("   Nonce: auto (RPC)");
    }
    println!("   RPC: {}", rpc.base_url());
    if memo.is_some() {
        println!("   Memo: {}", memo.clone().unwrap());
    }

    if !args.yes && !confirm("Proceed? [y/N] ")? {
        return Err(WalletError::InvalidCliUsage("operation cancelled".into()));
    }

    let payload = json!({
        "from": unlocked.address,
        "to": args.to,
        "amount": amount_atomic.to_string(),
        "fee": fee_atomic.map(|fee| fee.to_string()),
        "nonce": next_nonce,
        "memo": memo,
        "signing_key": hex::encode(unlocked.private_key),
    });

    let response = rpc.submit_payment(&payload).await?;
    if let Some(tx_hash) = response.get("tx_hash").and_then(|v| v.as_str()) {
        println!("✅ Payment accepted");
        println!("   Tx hash: {tx_hash}");
    } else {
        println!("{}", serde_json::to_string_pretty(&response)?);
    }

    Ok(())
}

fn resolve_message_payload(args: &SignArgs) -> Result<Vec<u8>> {
    if let Some(text) = &args.message {
        return Ok(text.as_bytes().to_vec());
    }
    if let Some(hex_str) = &args.message_hex {
        return hex::decode(hex_str.trim())
            .map_err(|err| WalletError::InvalidCliUsage(err.to_string()));
    }
    if let Some(path) = &args.file {
        let mut file = fs::File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        return Ok(buffer);
    }
    Err(WalletError::InvalidCliUsage(
        "provide --message, --message-hex, or --file".into(),
    ))
}

fn resolve_password(args: &PasswordArgs, confirm: bool) -> Result<Option<String>> {
    if let Some(pwd) = &args.password {
        return Ok(Some(pwd.clone()));
    }
    if let Some(path) = &args.password_file {
        let data = fs::read_to_string(path)?;
        return Ok(Some(data.trim().to_string()));
    }
    if args.prompt_password {
        let first = prompt_password("Password: ").map_err(WalletError::IoError)?;
        if confirm {
            let second = prompt_password("Confirm password: ").map_err(WalletError::IoError)?;
            if first != second {
                return Err(WalletError::InvalidPassword);
            }
        }
        return Ok(Some(first));
    }
    Ok(None)
}

fn confirm(prompt: &str) -> Result<bool> {
    print!("{prompt}");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(matches!(input.trim(), "y" | "Y" | "yes" | "YES"))
}

fn decode_any_address(input: &str) -> Result<[u8; 32]> {
    decode_address(input).or_else(|err| {
        let normalized = input
            .trim()
            .trim_start_matches("0x")
            .trim_start_matches("0X");
        let mut bytes = [0u8; 32];
        hex::decode_to_slice(normalized, &mut bytes)
            .map(|_| bytes)
            .map_err(|hex_err| WalletError::InvalidAddress(format!("{err}; hex error: {hex_err}")))
    })
}

async fn derive_nonce(rpc: &WalletRpcClient, address: &str) -> Result<Option<u64>> {
    let address_bytes = decode_any_address(address)?;
    let address_hex = hex::encode(address_bytes);
    match rpc.fetch_account(&address_hex).await? {
        Some(AccountState { nonce, .. }) => Ok(Some(nonce.saturating_add(1))),
        None => Ok(Some(0)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_amount_decimal() {
        let arg = AmountArg {
            amount: Some("1.5".into()),
            amount_atomic: None,
        };
        let value = arg.to_atomic("amount").unwrap();
        assert_eq!(value, Amount::from_str_ipn("1.5").unwrap().atomic());
    }

    #[test]
    fn parse_amount_atomic() {
        let arg = AmountArg {
            amount: None,
            amount_atomic: Some(42),
        };
        assert_eq!(arg.to_atomic("amount").unwrap(), 42);
    }
}
