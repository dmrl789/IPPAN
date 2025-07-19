//! CLI interface module
//! 
//! Provides command-line interface for the IPPAN node.

use crate::Result;
use crate::crosschain::{CrossChainManager, CrossChainConfig, AnchorTx, ProofType};
use crate::consensus::hashtimer::HashTimer;
use clap::{Parser, Subcommand};
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "ippan-cli")]
#[command(about = "IPPAN Command Line Interface")]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Submit an anchor transaction
    SubmitAnchor {
        /// External chain ID
        #[arg(long)]
        chain_id: String,
        /// External state root
        #[arg(long)]
        state_root: String,
        /// Proof type (none, signature, zk, merkle, multisig)
        #[arg(long)]
        proof_type: Option<String>,
        /// Proof data (hex encoded)
        #[arg(long)]
        proof_data: Option<String>,
        /// Transaction fee
        #[arg(long, default_value = "1000000")]
        fee: u64,
    },
    /// Get the latest anchor for a chain
    GetLatestAnchor {
        /// Chain ID
        #[arg(long)]
        chain_id: String,
    },
    /// Verify external inclusion proof
    VerifyInclusion {
        /// Chain ID
        #[arg(long)]
        chain_id: String,
        /// Transaction hash
        #[arg(long)]
        tx_hash: String,
        /// Merkle proof (hex encoded)
        #[arg(long)]
        merkle_proof: String,
    },
    /// Register a bridge endpoint
    RegisterBridge {
        /// Chain ID
        #[arg(long)]
        chain_id: String,
        /// Accepted proof types (comma separated)
        #[arg(long)]
        proof_types: String,
        /// Trust level (0-100)
        #[arg(long, default_value = "50")]
        trust_level: u8,
    },
    /// Get cross-chain report
    GetReport,
}

pub struct CliHandler {
    manager: Arc<CrossChainManager>,
}

impl CliHandler {
    pub async fn new() -> Result<Self> {
        let config = CrossChainConfig::default();
        let manager = CrossChainManager::new(config).await?;
        Ok(Self {
            manager: Arc::new(manager),
        })
    }

    pub async fn handle(&self, cli: Cli) -> Result<()> {
        match cli.command {
            Commands::SubmitAnchor {
                chain_id,
                state_root,
                proof_type,
                proof_data,
                fee,
            } => {
                self.handle_submit_anchor(chain_id, state_root, proof_type, proof_data, fee).await
            }
            Commands::GetLatestAnchor { chain_id } => {
                self.handle_get_latest_anchor(chain_id).await
            }
            Commands::VerifyInclusion {
                chain_id,
                tx_hash,
                merkle_proof,
            } => {
                self.handle_verify_inclusion(chain_id, tx_hash, merkle_proof).await
            }
            Commands::RegisterBridge {
                chain_id,
                proof_types,
                trust_level,
            } => {
                self.handle_register_bridge(chain_id, proof_types, trust_level).await
            }
            Commands::GetReport => {
                self.handle_get_report().await
            }
        }
    }

    async fn handle_submit_anchor(
        &self,
        chain_id: String,
        state_root: String,
        proof_type: Option<String>,
        proof_data: Option<String>,
        fee: u64,
    ) -> Result<()> {
        let proof_type_enum = proof_type.as_ref().map(|pt| match pt.as_str() {
            "none" => ProofType::None,
            "signature" => ProofType::Signature,
            "zk" => ProofType::ZK,
            "merkle" => ProofType::Merkle,
            "multisig" => ProofType::MultiSig,
            _ => return Err(crate::error::IppanError::Validation(
                format!("Invalid proof type: {}", pt)
            )),
        }).transpose()?;

        let proof_data_bytes = proof_data
            .map(|pd| hex::decode(pd))
            .transpose()
            .map_err(|e| crate::error::IppanError::Validation(
                format!("Invalid proof data hex: {}", e)
            ))?;

        let anchor_tx = AnchorTx {
            external_chain_id: chain_id,
            external_state_root: state_root,
            timestamp: HashTimer::new([0u8; 32], [0u8; 32]),
            proof_type: proof_type_enum,
            proof_data: proof_data_bytes.unwrap_or_default(),
        };

        let anchor_id = self.manager.submit_anchor(anchor_tx).await?;
        println!("✅ Anchor submitted successfully!");
        println!("   Anchor ID: {}", anchor_id);
        println!("   Chain ID: {}", anchor_tx.external_chain_id);
        println!("   State Root: {}", anchor_tx.external_state_root);
        println!("   Fee: {} IPN", fee as f64 / 100_000_000.0);

        Ok(())
    }

    async fn handle_get_latest_anchor(&self, chain_id: String) -> Result<()> {
        match self.manager.get_latest_anchor(&chain_id).await? {
            Some(anchor) => {
                println!("✅ Latest anchor found for chain: {}", chain_id);
                println!("   State Root: {}", anchor.external_state_root);
                println!("   Proof Type: {:?}", anchor.proof_type);
                println!("   Proof Data Size: {} bytes", anchor.proof_data.len());
                println!("   Timestamp: {:?}", anchor.timestamp);
            }
            None => {
                println!("❌ No anchor found for chain: {}", chain_id);
            }
        }

        Ok(())
    }

    async fn handle_verify_inclusion(
        &self,
        chain_id: String,
        tx_hash: String,
        merkle_proof: String,
    ) -> Result<()> {
        let proof_bytes = hex::decode(&merkle_proof)
            .map_err(|e| crate::error::IppanError::Validation(
                format!("Invalid merkle proof hex: {}", e)
            ))?;

        let result = self.manager.verify_external_inclusion(
            &chain_id,
            &tx_hash,
            &proof_bytes,
        ).await?;

        if result.success {
            println!("✅ Inclusion proof verified successfully!");
            println!("   Chain ID: {}", chain_id);
            println!("   Transaction Hash: {}", tx_hash);
            if let Some(timestamp) = result.anchor_timestamp {
                println!("   Anchor Timestamp: {:?}", timestamp);
            }
            if let Some(round) = result.anchor_round {
                println!("   Anchor Round: {}", round);
            }
            println!("   Details: {}", result.details);
        } else {
            println!("❌ Inclusion proof verification failed!");
            println!("   Chain ID: {}", chain_id);
            println!("   Transaction Hash: {}", tx_hash);
            println!("   Details: {}", result.details);
        }

        Ok(())
    }

    async fn handle_register_bridge(
        &self,
        chain_id: String,
        proof_types: String,
        trust_level: u8,
    ) -> Result<()> {
        let proof_type_list: Vec<ProofType> = proof_types
            .split(',')
            .map(|pt| match pt.trim() {
                "none" => Ok(ProofType::None),
                "signature" => Ok(ProofType::Signature),
                "zk" => Ok(ProofType::ZK),
                "merkle" => Ok(ProofType::Merkle),
                "multisig" => Ok(ProofType::MultiSig),
                _ => Err(crate::error::IppanError::Validation(
                    format!("Invalid proof type: {}", pt)
                )),
            })
            .collect::<Result<Vec<_>>>()?;

        let endpoint = crate::crosschain::bridge::BridgeEndpoint {
            chain_id: chain_id.clone(),
            accepted_anchor_types: proof_type_list,
            latest_anchor: None,
            config: crate::crosschain::bridge::BridgeConfig {
                trust_level,
                ..Default::default()
            },
            status: crate::crosschain::bridge::BridgeStatus::Active,
            last_activity: chrono::Utc::now(),
        };

        self.manager.register_bridge(endpoint).await?;
        println!("✅ Bridge registered successfully!");
        println!("   Chain ID: {}", chain_id);
        println!("   Trust Level: {}", trust_level);
        println!("   Accepted Proof Types: {}", proof_types);

        Ok(())
    }

    async fn handle_get_report(&self) -> Result<()> {
        let report = self.manager.generate_cross_chain_report().await?;
        
        println!("📊 Cross-Chain Report");
        println!("   Generated at: {}", report.generated_at.format("%Y-%m-%d %H:%M:%S UTC"));
        println!("   Total Anchors: {}", report.total_anchors);
        println!("   Active Bridges: {}", report.active_bridges);
        println!("   Verification Success Rate: {:.2}%", report.verification_success_rate * 100.0);
        
        if !report.recent_anchors.is_empty() {
            println!("\n📌 Recent Anchors:");
            for anchor in report.recent_anchors.iter().take(5) {
                println!("   - {}: {} (Proof: {:?})", 
                    anchor.external_chain_id, 
                    anchor.external_state_root,
                    anchor.proof_type);
            }
        }
        
        if !report.bridge_endpoints.is_empty() {
            println!("\n🌉 Bridge Endpoints:");
            for bridge in report.bridge_endpoints.iter().take(5) {
                println!("   - {}: {:?} (Trust: {})", 
                    bridge.chain_id, 
                    bridge.status,
                    bridge.config.trust_level);
            }
        }

        Ok(())
    }
}

pub async fn run_cli() -> Result<()> {
    let cli = Cli::parse();
    let handler = CliHandler::new().await?;
    handler.handle(cli).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cli_handler_creation() {
        let handler = CliHandler::new().await.unwrap();
        // Test that handler was created successfully
        assert!(true);
    }

    #[tokio::test]
    async fn test_submit_anchor_command() {
        let handler = CliHandler::new().await.unwrap();
        
        // Test anchor submission
        let result = handler.handle_submit_anchor(
            "testchain".to_string(),
            "0x1234567890abcdef".to_string(),
            Some("signature".to_string()),
            Some("0102030405060708".to_string()),
            1000000,
        ).await;
        
        assert!(result.is_ok());
    }
}
