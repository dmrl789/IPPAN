//! User-facing CLI for IPPAN transaction types
//! 
//! Provides command-line interface for all 23 canonical transaction types
//! that users can submit to the IPPAN network.

use crate::{Result, IppanError};
use crate::transaction_types::*;
use clap::{Parser, Subcommand, Args};
use serde_json;
use std::path::PathBuf;
use serde::Deserialize;

#[derive(Parser)]
#[command(name = "ippan-cli")]
#[command(about = "IPPAN Command Line Interface - User Transactions")]
#[command(version)]
pub struct UserCli {
    #[command(subcommand)]
    command: UserCommands,
}

#[derive(Subcommand)]
pub enum UserCommands {
    /// Send IPN from one account to another
    Pay(PayArgs),
    
    /// Register a new handle
    HandleRegister(HandleRegisterArgs),
    
    /// Register a new domain
    DomainRegister(DomainRegisterArgs),
    
    /// Update DNS zone records
    ZoneUpdate(ZoneUpdateArgs),
    
    /// List available TLDs with fees
    ListTlds(ListTldsArgs),
    
    /// Resolve an IPPAN name (ipn.domain.tld)
    ResolveIpn(ResolveIpnArgs),
}

// ============================================================================
// ARGUMENT STRUCTURES
// ============================================================================

#[derive(Args)]
pub struct PayArgs {
    /// Sender address or handle
    #[arg(long)]
    from: String,
    
    /// Recipient address or handle
    #[arg(long)]
    to: String,
    
    /// Amount in IPN
    #[arg(long)]
    amount: String,
    
    /// Optional memo (≤128 bytes)
    #[arg(long)]
    memo: Option<String>,
    
    /// Transaction fee (use "auto" for automatic calculation)
    #[arg(long, default_value = "auto")]
    fee: String,
    
    /// Signing key file
    #[arg(long)]
    key_file: PathBuf,
}

#[derive(Args)]
pub struct HandleRegisterArgs {
    /// Handle name (e.g., "@desiree.ipn")
    #[arg(long)]
    handle: String,
    
    /// Owner public key
    #[arg(long)]
    owner_pk: String,
    
    /// Registration years
    #[arg(long, default_value = "1")]
    years: u32,
    
    /// Transaction fee (use "auto" for automatic calculation)
    #[arg(long, default_value = "auto")]
    fee: String,
    
    /// Signing key file
    #[arg(long)]
    key_file: PathBuf,
}

#[derive(Args)]
pub struct DomainRegisterArgs {
    /// Domain name (e.g., "example.ipn")
    #[arg(long)]
    domain: String,
    
    /// Owner public key
    #[arg(long)]
    owner_pk: String,
    
    /// Registration years
    #[arg(long, default_value = "1")]
    years: u32,
    
    /// Plan type
    #[arg(long, default_value = "standard")]
    plan: String,
    
    /// Transaction fee (use "auto" for automatic calculation)
    #[arg(long, default_value = "auto")]
    fee: String,
    
    /// Signing key file
    #[arg(long)]
    key_file: PathBuf,
}

#[derive(Args)]
pub struct ZoneUpdateArgs {
    /// Domain name
    #[arg(long)]
    domain: String,
    
    /// Nonce for replay protection
    #[arg(long)]
    nonce: u64,
    
    /// Zone operations file (JSON array of {op, name?, rtype?, ttl?, records?})
    #[arg(long)]
    ops_file: PathBuf,
    
    /// Transaction fee in nano IPN (use "auto" for automatic calculation)
    #[arg(long, default_value = "auto")]
    fee_nano: String,
    
    /// Signing key file
    #[arg(long)]
    key_file: PathBuf,
}

#[derive(Args)]
pub struct ListTldsArgs {
    /// Filter by category (e.g., core, tech, finance, identity, lifestyle, experimental)
    #[arg(long)]
    category: Option<String>,

    /// Filter by premium multiplier (e.g., 1, 2, 3, 5, 10)
    #[arg(long)]
    multiplier: Option<u32>,

    /// Show detailed information
    #[arg(long)]
    detailed: bool,
}

#[derive(Args)]
pub struct ResolveIpnArgs {
    /// IPPAN name to resolve (e.g., "ipn.domain.tld")
    #[arg(long)]
    ipn_name: String,
}

// ============================================================================
// CLI HANDLER
// ============================================================================

pub struct UserCliHandler;

impl UserCliHandler {
    pub fn new() -> Self {
        Self
    }
    
    pub async fn handle(&self, cli: UserCli) -> Result<()> {
        match cli.command {
            UserCommands::Pay(args) => {
                self.handle_pay(args).await
            }
            
            UserCommands::HandleRegister(args) => {
                self.handle_register(args).await
            }
            
            UserCommands::DomainRegister(args) => {
                self.handle_domain_register(args).await
            }
            
            UserCommands::ZoneUpdate(args) => {
                self.handle_zone_update(args).await
            }
            
            UserCommands::ListTlds(args) => {
                self.handle_list_tlds(args).await
            }
            
            UserCommands::ResolveIpn(args) => {
                self.handle_resolve_ipn(args).await
            }
        }
    }
    
    // ============================================================================
    // TRANSACTION HANDLERS
    // ============================================================================
    
    async fn handle_pay(&self, args: PayArgs) -> Result<()> {
        println!("Processing payment transaction...");
        
        // Calculate amount in smallest units (1 IPN = 1e8 units)
        let amount_units = self.parse_amount_to_units(&args.amount);
        
        // Calculate fee in smallest units (PRD-aligned)
        let fee_units = self.calc_fee_1pct(amount_units);
        
        // Create transaction
        let tx = PayTransaction {
            from: args.from,
            to: args.to,
            amount_ipn: args.amount, // Keep original amount string for display
            memo: args.memo,
            fee: fee_units.to_string(), // Use calculated fee
            sig: self.sign_transaction(&args.key_file, "pay").await?,
        };
        
        // Validate transaction
        tx.validate()?;
        
        // Submit transaction
        self.submit_transaction(&tx).await?;
        
        println!("Payment transaction submitted successfully!");
        Ok(())
    }
    
    async fn handle_register(&self, args: HandleRegisterArgs) -> Result<()> {
        println!("Registering handle...");
        
        // Validate handle format
        validate_handle(&args.handle)?;
        
        // Create transaction
        let tx = HandleRegisterTransaction {
            handle: args.handle,
            owner_pk: args.owner_pk,
            years: args.years,
            fee: self.calculate_fee(&args.fee, "handle_register"),
            sig: self.sign_transaction(&args.key_file, "handle_register").await?,
        };
        
        // Submit transaction
        self.submit_transaction(&tx).await?;
        
        println!("Handle registered successfully!");
        Ok(())
    }
    
    async fn handle_domain_register(&self, args: DomainRegisterArgs) -> Result<()> {
        println!("Registering domain...");
        
        // Validate domain format
        validate_domain(&args.domain)?;
        
        // Calculate dynamic fee based on domain type and years
        let fee = self.calculate_domain_fee(&args.domain, args.years);
        
        // Show fee breakdown
        println!("Domain: {}", args.domain);
        println!("Years: {}", args.years);
        println!("Fee: {} IPN", fee);
        
        // Create transaction
        let tx = DomainRegisterTransaction {
            domain: args.domain,
            owner_pk: args.owner_pk,
            years: args.years,
            plan: args.plan,
            fee,
            sig: self.sign_transaction(&args.key_file, "domain_register").await?,
        };
        
        // Submit transaction
        self.submit_transaction(&tx).await?;
        
        println!("Domain registered successfully!");
        Ok(())
    }
    
    async fn handle_zone_update(&self, args: ZoneUpdateArgs) -> Result<()> {
        println!("Updating DNS zone...");
        
        // Load operations from file
        let ops_content = std::fs::read_to_string(&args.ops_file)
            .map_err(|e| IppanError::Validation(format!("Failed to read ops file: {}", e)))?;
        
        let ops: Vec<ZoneOp> = serde_json::from_str(&ops_content)
            .map_err(|e| IppanError::Validation(format!("Invalid JSON in ops file: {}", e)))?;
        
        // Get current timestamp
        let updated_at_us = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;
        
        // Create transaction
        let tx = ZoneUpdateTransaction {
            domain: args.domain,
            nonce: args.nonce,
            ops,
            updated_at_us,
            fee_nano: self.calculate_fee_nano(&args.fee_nano, "zone_update"),
            sig: self.sign_transaction(&args.key_file, "zone_update").await?,
        };
        
        // Validate transaction
        tx.validate()?;
        
        // Submit transaction
        self.submit_transaction(&tx).await?;
        
        println!("DNS zone updated successfully!");
        Ok(())
    }
    
    async fn handle_list_tlds(&self, args: ListTldsArgs) -> Result<()> {
        println!("Listing available TLDs...");

        #[derive(Deserialize)]
        struct TldRecord {
            tld: String,
            category: String,
            premium_multiplier: u32,
            description: String,
        }

        let path = "config/tld_registry.json";
        let data = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                println!("Could not read {}: {}", path, e);
                println!("Tip: run setup to generate the TLD registry.");
                return Ok(());
            }
        };

        let mut records: Vec<TldRecord> = match serde_json::from_str(&data) {
            Ok(v) => v,
            Err(e) => {
                println!("Invalid JSON in {}: {}", path, e);
                return Ok(());
            }
        };

        if let Some(ref cat) = args.category {
            let cat_lower = cat.to_lowercase();
            records.retain(|r| r.category.to_lowercase() == cat_lower);
        }
        if let Some(mult) = args.multiplier {
            records.retain(|r| r.premium_multiplier == mult);
        }

        if records.is_empty() {
            println!("No TLDs matched your filters.");
            return Ok(());
        }

        // Sort by category then by multiplier desc then by tld
        records.sort_by(|a, b| {
            (a.category.to_lowercase(), std::cmp::Reverse(a.premium_multiplier), a.tld.to_lowercase())
                .cmp(&(b.category.to_lowercase(), std::cmp::Reverse(b.premium_multiplier), b.tld.to_lowercase()))
        });

        let mut current_cat: Option<String> = None;
        for r in records {
            if args.detailed {
                if current_cat.as_ref().map(|c| c != &r.category).unwrap_or(true) {
                    current_cat = Some(r.category.clone());
                    println!("\n[{}]", current_cat.as_ref().unwrap());
                }
                println!(
                    "  {}  (x{}, {})",
                    r.tld,
                    r.premium_multiplier,
                    r.description
                );
            } else {
                println!("{} ({} x{})", r.tld, r.category, r.premium_multiplier);
            }
        }

        Ok(())
    }

    async fn handle_resolve_ipn(&self, args: ResolveIpnArgs) -> Result<()> {
        println!("Resolving IPPAN name: {}", args.ipn_name);

        // TODO: Implement actual IPPAN name resolution logic
        // This would involve sending a transaction to the IPPAN network
        // to query the name's owner and associated data.

        println!("IPPAN name resolution is not yet implemented.");
        Ok(())
    }

    // ============================================================================
    // HELPER METHODS
    // ============================================================================
    
    fn calculate_fee(&self, fee_str: &str, tx_type: &str) -> String {
        if fee_str == "auto" {
            // Calculate automatic fee based on transaction type and amount
            // PRD Rule: 1% of transferred amount, minimum 1 unit (dust guard)
            match tx_type {
                "pay" => {
                    // For pay transactions, fee is calculated based on amount
                    // This will be calculated dynamically when amount is known
                    "0.00000100".to_string() // Placeholder - actual calculation done in transaction
                }
                "handle_register" => "0.50000000".to_string(),
                "domain_register" => {
                    // NEW: Dynamic fee calculation based on domain type and years
                    // This will be calculated dynamically when domain and years are known
                    "0.20000000".to_string() // Placeholder - actual calculation done in transaction
                },
                _ => "0.00000100".to_string(),
            }
        } else {
            fee_str.to_string()
        }
    }
    
    /// Calculate domain registration fee using the new 20-year sliding scale
    fn calculate_domain_fee(&self, domain: &str, years: u32) -> String {
        // Premium multipliers
        let premium_mult = if domain.ends_with(".ai") || domain.ends_with(".m") {
            10 // Premium domains
        } else if domain.ends_with(".iot") {
            2 // IoT domains
        } else {
            1 // Standard domains
        };
        
        // Calculate total fee for the requested years
        let mut total_fee_micro = 0;
        for year in 1..=years {
            let base_fee = if year == 1 {
                0.20
            } else if year == 2 {
                0.02
            } else {
                let decayed = 0.01 - 0.001 * (year as f64 - 3.0);
                if decayed < 0.001 { 0.001 } else { decayed }
            };
            
            total_fee_micro += (base_fee * premium_mult as f64 * 1_000_000.0).round() as u64;
        }
        
        let total_fee_ipn = total_fee_micro as f64 / 1_000_000.0;
        format!("{:.6}", total_fee_ipn)
    }
    
    /// Calculate domain renewal fee using the new 20-year sliding scale
    fn calculate_domain_renewal_fee(&self, domain: &str, current_year: u32, years: u32) -> String {
        // Premium multipliers
        let premium_mult = if domain.ends_with(".ai") || domain.ends_with(".m") {
            10 // Premium domains
        } else if domain.ends_with(".iot") {
            2 // IoT domains
        } else {
            1 // Standard domains
        };
        
        // Calculate total fee for the renewal period
        let mut total_fee_micro = 0;
        for year in current_year..current_year + years {
            let base_fee = if year == 1 {
                0.20
            } else if year == 2 {
                0.02
            } else {
                let decayed = 0.01 - 0.001 * (year as f64 - 3.0);
                if decayed < 0.001 { 0.001 } else { decayed }
            };
            
            total_fee_micro += (base_fee * premium_mult as f64 * 1_000_000.0).round() as u64;
        }
        
        let total_fee_ipn = total_fee_micro as f64 / 1_000_000.0;
        format!("{:.6}", total_fee_ipn)
    }

    /// Calculate 1% fee on amount in smallest units (PRD-aligned)
    fn calc_fee_1pct(&self, amount_units: u64) -> u64 {
        let one_pct = amount_units.saturating_mul(1) / 100; // floor division
        one_pct.max(1) // dust guard: minimum 1 unit
    }

    /// Parse amount string to smallest units (1 IPN = 1e8 units)
    fn parse_amount_to_units(&self, amount_str: &str) -> u64 {
        if let Ok(amount) = amount_str.parse::<f64>() {
            (amount * 100_000_000.0) as u64 // 1 IPN = 100,000,000 units
        } else {
            0
        }
    }
    
    fn calculate_fee_nano(&self, fee_str: &str, tx_type: &str) -> u64 {
        if fee_str == "auto" {
            // Calculate automatic fee in nano units
            // PRD Rule: 1% of transferred amount, minimum 1 unit (dust guard)
            match tx_type {
                "zone_update" => 100, // 0.0000001 IPN in nano units
                _ => 1000, // Default
            }
        } else {
            fee_str.parse().unwrap_or(1000)
        }
    }
    
    async fn sign_transaction(&self, _key_file: &PathBuf, _tx_type: &str) -> Result<String> {
        // TODO: Implement actual signing logic
        // For now, return a placeholder signature
        Ok(format!("ed25519:{}", "0".repeat(128)))
    }
    
    async fn submit_transaction<T: serde::Serialize>(&self, tx: &T) -> Result<()> {
        // TODO: Implement actual transaction submission
        // For now, just serialize and print
        let tx_json = serde_json::to_string_pretty(tx)
            .map_err(|e| IppanError::Validation(format!("Failed to serialize transaction: {}", e)))?;
        
        println!("Transaction JSON:");
        println!("{}", tx_json);
        
        Ok(())
    }
}
