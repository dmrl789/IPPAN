//! IPPAN Key Generation and Management Tool
//!
//! Secure Ed25519 keypair generation and management for IPPAN validators and wallets.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use ed25519_dalek::{SigningKey, VerifyingKey};
use rand_core::OsRng;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "ippan-keygen")]
#[command(about = "IPPAN Key Generation and Management Tool")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a new Ed25519 keypair
    Generate {
        /// Output directory for keys
        #[arg(short, long, default_value = ".")]
        output: PathBuf,

        /// Key name/prefix
        #[arg(short, long, default_value = "validator")]
        name: String,

        /// Print keys to stdout (insecure, for development only)
        #[arg(long)]
        stdout: bool,
    },

    /// Derive public key from private key
    PubKey {
        /// Path to private key file
        private_key: PathBuf,
    },

    /// Verify keypair
    Verify {
        /// Path to private key file
        private_key: PathBuf,

        /// Path to public key file
        public_key: PathBuf,
    },

    /// Get validator ID from public key
    ValidatorId {
        /// Path to public key file or hex string
        public_key: String,
    },
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Generate {
            output,
            name,
            stdout,
        } => {
            generate_keypair(&output, &name, stdout)?;
        }
        Commands::PubKey { private_key } => {
            derive_public_key(&private_key)?;
        }
        Commands::Verify {
            private_key,
            public_key,
        } => {
            verify_keypair(&private_key, &public_key)?;
        }
        Commands::ValidatorId { public_key } => {
            compute_validator_id(&public_key)?;
        }
    }

    Ok(())
}

fn generate_keypair(output: &PathBuf, name: &str, stdout: bool) -> Result<()> {
    println!("🔐 Generating new Ed25519 keypair...");

    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();

    let private_hex = hex::encode(signing_key.to_bytes());
    let public_hex = hex::encode(verifying_key.to_bytes());

    if stdout {
        println!("\n⚠️  WARNING: Printing keys to stdout is insecure!");
        println!("Private key: {}", private_hex);
        println!("Public key:  {}", public_hex);
    } else {
        let private_path = output.join(format!("{}_private.key", name));
        let public_path = output.join(format!("{}_public.key", name));

        // Write private key
        fs::write(&private_path, &private_hex).context("Failed to write private key")?;

        // Write public key
        fs::write(&public_path, &public_hex).context("Failed to write public key")?;

        println!("\n✓ Keypair generated successfully!");
        println!("  Private key: {}", private_path.display());
        println!("  Public key:  {}", public_path.display());
    }

    // Calculate and display validator ID
    let validator_id = blake3::hash(&verifying_key.to_bytes());
    println!(
        "\n📋 Validator ID: {}",
        hex::encode(validator_id.as_bytes())
    );

    if !stdout {
        println!("\n⚠️  IMPORTANT: Keep your private key secure!");
        println!("   Never share your private key with anyone.");
        println!("   Store it in a secure location with restricted permissions.");
    }

    Ok(())
}

fn derive_public_key(private_key_path: &PathBuf) -> Result<()> {
    let private_hex =
        fs::read_to_string(private_key_path).context("Failed to read private key file")?;

    let private_hex = private_hex.trim();
    let private_bytes =
        hex::decode(private_hex).context("Invalid hex format in private key file")?;

    if private_bytes.len() != 32 {
        anyhow::bail!(
            "Invalid private key length: expected 32 bytes, got {}",
            private_bytes.len()
        );
    }

    let mut key_bytes = [0u8; 32];
    key_bytes.copy_from_slice(&private_bytes);

    let signing_key = SigningKey::from_bytes(&key_bytes);
    let verifying_key = signing_key.verifying_key();

    println!("Public key: {}", hex::encode(verifying_key.to_bytes()));

    // Also show validator ID
    let validator_id = blake3::hash(&verifying_key.to_bytes());
    println!("Validator ID: {}", hex::encode(validator_id.as_bytes()));

    Ok(())
}

fn verify_keypair(private_key_path: &PathBuf, public_key_path: &PathBuf) -> Result<()> {
    let private_hex =
        fs::read_to_string(private_key_path).context("Failed to read private key file")?;
    let public_hex =
        fs::read_to_string(public_key_path).context("Failed to read public key file")?;

    let private_hex = private_hex.trim();
    let public_hex = public_hex.trim();

    let private_bytes =
        hex::decode(private_hex).context("Invalid hex format in private key file")?;
    let public_bytes = hex::decode(public_hex).context("Invalid hex format in public key file")?;

    if private_bytes.len() != 32 {
        anyhow::bail!(
            "Invalid private key length: expected 32 bytes, got {}",
            private_bytes.len()
        );
    }

    if public_bytes.len() != 32 {
        anyhow::bail!(
            "Invalid public key length: expected 32 bytes, got {}",
            public_bytes.len()
        );
    }

    let mut private_key_bytes = [0u8; 32];
    private_key_bytes.copy_from_slice(&private_bytes);

    let mut public_key_bytes = [0u8; 32];
    public_key_bytes.copy_from_slice(&public_bytes);

    let signing_key = SigningKey::from_bytes(&private_key_bytes);
    let derived_public = signing_key.verifying_key();
    let provided_public =
        VerifyingKey::from_bytes(&public_key_bytes).context("Invalid public key")?;

    if derived_public == provided_public {
        println!("✓ Keypair is valid!");
        println!("  Keys match and are properly paired.");

        let validator_id = blake3::hash(&provided_public.to_bytes());
        println!(
            "\n📋 Validator ID: {}",
            hex::encode(validator_id.as_bytes())
        );

        Ok(())
    } else {
        anyhow::bail!("✗ Keypair mismatch! The private and public keys do not match.");
    }
}

fn compute_validator_id(public_key_input: &str) -> Result<()> {
    // Try to read as file first, otherwise treat as hex string
    let public_hex = if PathBuf::from(public_key_input).exists() {
        fs::read_to_string(public_key_input).context("Failed to read public key file")?
    } else {
        public_key_input.to_string()
    };

    let public_hex = public_hex.trim();
    let public_bytes = hex::decode(public_hex).context("Invalid hex format")?;

    if public_bytes.len() != 32 {
        anyhow::bail!(
            "Invalid public key length: expected 32 bytes, got {}",
            public_bytes.len()
        );
    }

    let validator_id = blake3::hash(&public_bytes);
    println!("Validator ID: {}", hex::encode(validator_id.as_bytes()));

    Ok(())
}
