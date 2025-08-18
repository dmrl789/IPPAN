use clap::{Parser, Subcommand};
use neuro_core::*;
use reqwest::Client;
use serde_json::json;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    #[arg(long, default_value = "http://localhost:3000")]
    api_url: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new model
    CreateModel {
        #[arg(long)]
        owner: String,
        #[arg(long)]
        arch_id: u32,
        #[arg(long)]
        version: u32,
        #[arg(long)]
        weights_hash: String,
        #[arg(long)]
        size_bytes: u64,
        #[arg(long)]
        license_id: u32,
    },
    /// Get model by ID
    GetModel {
        #[arg(long)]
        id: String,
    },
    /// Create a new inference job
    CreateJob {
        #[arg(long)]
        model_ref: String,
        #[arg(long)]
        input_commit: String,
        #[arg(long)]
        max_latency_ms: u32,
        #[arg(long)]
        region: String,
        #[arg(long)]
        max_price_ipn: u128,
        #[arg(long)]
        escrow_ipn: u128,
        #[arg(long)]
        privacy: String,
        #[arg(long)]
        bid_window_ms: u16,
    },
    /// Place a bid on a job
    PlaceBid {
        #[arg(long)]
        job_id: String,
        #[arg(long)]
        executor_id: String,
        #[arg(long)]
        price_ipn: u128,
        #[arg(long)]
        est_latency_ms: u32,
        #[arg(long)]
        tee: bool,
    },
    /// Health check
    Health,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let client = Client::new();
    
    match &cli.command {
        Commands::CreateModel { owner, arch_id, version, weights_hash, size_bytes, license_id } => {
            let response = client
                .post(&format!("{}/models", cli.api_url))
                .json(&json!({
                    "owner": owner,
                    "arch_id": arch_id,
                    "version": version,
                    "weights_hash": weights_hash,
                    "size_bytes": size_bytes,
                    "license_id": license_id,
                }))
                .send()
                .await?;
            
            let result: serde_json::Value = response.json().await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        
        Commands::GetModel { id } => {
            let response = client
                .get(&format!("{}/models/{}", cli.api_url, id))
                .send()
                .await?;
            
            let result: serde_json::Value = response.json().await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        
        Commands::CreateJob { model_ref, input_commit, max_latency_ms, region, max_price_ipn, escrow_ipn, privacy, bid_window_ms } => {
            let response = client
                .post(&format!("{}/jobs", cli.api_url))
                .json(&json!({
                    "model_ref": model_ref,
                    "input_commit": input_commit,
                    "max_latency_ms": max_latency_ms,
                    "region": region,
                    "max_price_ipn": max_price_ipn,
                    "escrow_ipn": escrow_ipn,
                    "privacy": privacy,
                    "bid_window_ms": bid_window_ms,
                }))
                .send()
                .await?;
            
            let result: serde_json::Value = response.json().await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        
        Commands::PlaceBid { job_id, executor_id, price_ipn, est_latency_ms, tee } => {
            let response = client
                .post(&format!("{}/bids", cli.api_url))
                .json(&json!({
                    "job_id": job_id,
                    "executor_id": executor_id,
                    "price_ipn": price_ipn,
                    "est_latency_ms": est_latency_ms,
                    "tee": tee,
                }))
                .send()
                .await?;
            
            let result: serde_json::Value = response.json().await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        
        Commands::Health => {
            let response = client
                .get(&format!("{}/health", cli.api_url))
                .send()
                .await?;
            
            println!("Health check status: {}", response.status());
        }
    }
    
    Ok(())
}
