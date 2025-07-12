//! IppanNode orchestrator
//!
//! Main entry point for running an IPPAN node. Coordinates all subsystems.

use crate::{
    consensus,
    storage,
    network,
    wallet,
    dht,
    staking,
    domain,
    config,
    api::ApiLayer,
};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct IppanNode {
    pub config: config::Config,
    pub consensus: Arc<RwLock<consensus::ConsensusEngine>>,
    pub storage: Arc<RwLock<storage::StorageOrchestrator>>,
    pub network: Arc<RwLock<network::NetworkManager>>,
    pub wallet: Arc<RwLock<wallet::WalletManager>>,
    pub dht: Arc<RwLock<dht::DhtManager>>,
    pub staking: Arc<RwLock<staking::StakingSystem>>,
    pub domain: Arc<RwLock<domain::DomainSystem>>,
    pub api: Option<ApiLayer>,
    start_time: std::time::Instant,
    is_running: bool,
}

impl IppanNode {
    pub async fn new(config: config::Config) -> Result<Self, Box<dyn std::error::Error>> {
        let node_id = utils::crypto::generate_node_id();
        
        let consensus = Arc::new(RwLock::new(consensus::ConsensusEngine::new(config.consensus.clone())?));
        let storage = Arc::new(RwLock::new(storage::StorageOrchestrator::new(config.storage.clone())?));
        let network = Arc::new(RwLock::new(network::NetworkManager::new(config.network.clone())?));
        let wallet = Arc::new(RwLock::new(wallet::WalletManager::new(config.wallet.clone())?));
        let dht = Arc::new(RwLock::new(dht::DhtManager::new(config.dht.clone(), node_id).await?));
        
        // Create staking system
        let staking = Arc::new(RwLock::new(staking::StakingSystem::new(
            wallet.clone(),
            consensus.clone(),
        )?));
        
        // Create domain system
        let domain = Arc::new(RwLock::new(domain::DomainSystem::new(
            wallet.clone(),
        )?));
        
        Ok(Self {
            config,
            consensus,
            storage,
            network,
            wallet,
            dht,
            staking,
            domain,
            api: None, // Will be initialized after node creation
            start_time: std::time::Instant::now(),
            is_running: false,
        })
    }

    /// Initialize the API layer (called after node creation to avoid circular reference)
    pub fn init_api(&mut self) {
        let node_arc = Arc::new(RwLock::new(self.clone_for_api()));
        self.api = Some(ApiLayer::new(node_arc));
    }

    /// Create a clone of the node for API access (without the API field to avoid circular reference)
    fn clone_for_api(&self) -> IppanNodeForApi {
        IppanNodeForApi {
            consensus: Arc::clone(&self.consensus),
            storage: Arc::clone(&self.storage),
            network: Arc::clone(&self.network),
            wallet: Arc::clone(&self.wallet),
            dht: Arc::clone(&self.dht),
            staking: Arc::clone(&self.staking),
            domain: Arc::clone(&self.domain),
            start_time: self.start_time,
        }
    }

    /// Start the IPPAN node
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.is_running {
            return Ok(());
        }

        log::info!("Starting IPPAN node...");
        
        // Start consensus engine
        self.consensus.write().await.start().await?;
        log::info!("Consensus engine started");
        
        // Start storage orchestrator
        self.storage.write().await.start().await?;
        log::info!("Storage orchestrator started");
        
        // Start network manager
        self.network.write().await.start().await?;
        log::info!("Network manager started");
        
        // Start wallet manager
        self.wallet.write().await.start().await?;
        log::info!("Wallet manager started");
        
        // Start DHT manager
        self.dht.write().await.start().await?;
        log::info!("DHT manager started");
        
        // Start staking system
        self.staking.write().await.start().await?;
        log::info!("Staking system started");
        
        // Start domain system
        self.domain.write().await.start().await?;
        log::info!("Domain system started");
        
        // Start API layer if initialized
        if let Some(ref mut api) = self.api {
            api.start().await?;
            log::info!("API layer started");
        }
        
        self.is_running = true;
        log::info!("IPPAN node started successfully");
        Ok(())
    }

    /// Stop the IPPAN node
    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.is_running {
            return Ok(());
        }

        log::info!("Stopping IPPAN node...");
        
        // Stop API layer first if initialized
        if let Some(ref mut api) = self.api {
            api.stop().await?;
            log::info!("API layer stopped");
        }
        
        // Stop domain system
        self.domain.write().await.stop().await?;
        log::info!("Domain system stopped");
        
        // Stop staking system
        self.staking.write().await.stop().await?;
        log::info!("Staking system stopped");
        
        // Stop DHT manager
        self.dht.write().await.stop().await?;
        log::info!("DHT manager stopped");
        
        // Stop wallet manager
        self.wallet.write().await.stop().await?;
        log::info!("Wallet manager stopped");
        
        // Stop network manager
        self.network.write().await.stop().await?;
        log::info!("Network manager stopped");
        
        // Stop storage orchestrator
        self.storage.write().await.stop().await?;
        log::info!("Storage orchestrator stopped");
        
        // Stop consensus engine last
        self.consensus.write().await.stop().await?;
        log::info!("Consensus engine stopped");
        
        self.is_running = false;
        log::info!("IPPAN node stopped");
        Ok(())
    }

    /// Get node status
    pub fn get_status(&self) -> NodeStatus {
        NodeStatus {
            is_running: self.is_running,
            uptime: self.start_time.elapsed(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Get node ID
    pub fn node_id(&self) -> [u8; 32] {
        // Implementation would return the actual node ID
        [0u8; 32]
    }

    /// Get peer ID
    pub fn peer_id(&self) -> String {
        // Implementation would return the actual peer ID
        "peer_id".to_string()
    }

    /// Get uptime
    pub fn get_uptime(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }

    /// Add transaction fee to global fund
    pub async fn add_transaction_fee(&self, fee_amount: u64) -> Result<(), Box<dyn std::error::Error>> {
        self.staking.write().await.add_transaction_fee(fee_amount).await;
        Ok(())
    }

    /// Add domain fee to global fund
    pub async fn add_domain_fee(&self, fee_amount: u64) -> Result<(), Box<dyn std::error::Error>> {
        self.staking.write().await.add_domain_fee(fee_amount).await;
        Ok(())
    }

    /// Update node metrics for global fund
    pub async fn update_node_metrics(&self, node_id: String, metrics: staking::global_fund::NodeMetrics) -> Result<(), Box<dyn std::error::Error>> {
        self.staking.write().await.update_node_metrics(node_id, metrics).await;
        Ok(())
    }

    /// Check if weekly global fund distribution should occur
    pub async fn should_distribute_global_fund(&self) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(self.staking.read().await.should_distribute_global_fund().await)
    }

    /// Perform weekly global fund distribution
    pub async fn perform_weekly_distribution(&self) -> Result<staking::global_fund::WeeklyDistribution, Box<dyn std::error::Error>> {
        self.staking.write().await.perform_weekly_distribution().await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    /// Get global fund statistics
    pub async fn get_global_fund_stats(&self) -> Result<staking::global_fund::FundStatistics, Box<dyn std::error::Error>> {
        Ok(self.staking.read().await.get_global_fund_stats().await)
    }

    /// Create M2M payment channel
    pub async fn create_m2m_payment_channel(
        &self,
        sender: String,
        recipient: String,
        deposit_amount: u64,
        timeout_hours: u64,
    ) -> Result<wallet::m2m_payments::PaymentChannel, Box<dyn std::error::Error>> {
        self.wallet.read().await.create_payment_channel(sender, recipient, deposit_amount, timeout_hours).await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    /// Process M2M micro-payment
    pub async fn process_m2m_micro_payment(
        &self,
        channel_id: &str,
        amount: u64,
        tx_type: wallet::m2m_payments::MicroTransactionType,
    ) -> Result<wallet::m2m_payments::MicroTransaction, Box<dyn std::error::Error>> {
        self.wallet.read().await.process_micro_payment(channel_id, amount, tx_type).await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    /// Get M2M payment channel
    pub async fn get_m2m_payment_channel(&self, channel_id: &str) -> Result<Option<wallet::m2m_payments::PaymentChannel>, Box<dyn std::error::Error>> {
        Ok(self.wallet.read().await.get_payment_channel(channel_id).await)
    }

    /// Get M2M payment statistics
    pub async fn get_m2m_statistics(&self) -> Result<wallet::m2m_payments::PaymentStatistics, Box<dyn std::error::Error>> {
        Ok(self.wallet.read().await.get_m2m_statistics().await)
    }

    /// Clean up expired M2M payment channels
    pub async fn cleanup_expired_m2m_channels(&self) -> Result<usize, Box<dyn std::error::Error>> {
        Ok(self.wallet.read().await.cleanup_expired_channels().await)
    }

    /// Get total M2M fees collected
    pub async fn get_total_m2m_fees(&self) -> Result<u64, Box<dyn std::error::Error>> {
        Ok(self.wallet.read().await.get_total_m2m_fees().await)
    }

    /// Run the main node loop
    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.start().await?;
        
        // Main event loop
        loop {
            // Check for global fund distribution
            if self.should_distribute_global_fund().await? {
                match self.perform_weekly_distribution().await {
                    Ok(distribution) => {
                        log::info!("Weekly global fund distribution completed: {} distributed to {} nodes", 
                            distribution.total_distributed, distribution.eligible_nodes);
                    }
                    Err(e) => {
                        log::error!("Global fund distribution failed: {}", e);
                    }
                }
            }

            // Clean up expired M2M payment channels
            let expired_count = self.cleanup_expired_m2m_channels().await?;
            if expired_count > 0 {
                log::info!("Cleaned up {} expired M2M payment channels", expired_count);
            }

            // Sleep for a short interval
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        }
    }
}

/// Node structure for API access (without circular reference)
pub struct IppanNodeForApi {
    pub consensus: Arc<RwLock<consensus::ConsensusEngine>>,
    pub storage: Arc<RwLock<storage::StorageOrchestrator>>,
    pub network: Arc<RwLock<network::NetworkManager>>,
    pub wallet: Arc<RwLock<wallet::WalletManager>>,
    pub dht: Arc<RwLock<dht::DhtManager>>,
    pub staking: Arc<RwLock<staking::StakingSystem>>,
    pub domain: Arc<RwLock<domain::DomainSystem>>,
    start_time: std::time::Instant,
}

impl IppanNodeForApi {
    /// Get node ID
    pub fn node_id(&self) -> [u8; 32] {
        // Implementation would return the actual node ID
        [0u8; 32]
    }

    /// Get peer ID
    pub fn peer_id(&self) -> String {
        // Implementation would return the actual peer ID
        "peer_id".to_string()
    }

    /// Get uptime
    pub fn get_uptime(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }
}

/// Node status information
#[derive(Debug)]
pub struct NodeStatus {
    pub is_running: bool,
    pub uptime: std::time::Duration,
    pub version: String,
} 