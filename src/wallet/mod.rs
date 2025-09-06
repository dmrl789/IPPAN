//! Wallet subsystem for IPPAN
//!
//! Handles Ed25519 key management, payments, staking, and M2M payments.

use crate::config::Config;
use crate::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod ed25519;
pub mod payments;
pub mod stake;
pub mod m2m_payments;

pub struct WalletManager {
    pub config: Config,
    /// M2M payment system
    pub m2m_payments: Arc<RwLock<m2m_payments::M2MPaymentSystem>>,
    /// Ed25519 key manager
    pub keys: Arc<RwLock<ed25519::Ed25519Manager>>,
    /// Payment processor
    pub payments: Arc<RwLock<payments::PaymentProcessor>>,
    /// Staking manager
    pub staking: Arc<RwLock<stake::StakeManager>>,
    running: bool,
}

impl WalletManager {
    /// Create a new wallet manager
    pub async fn new(config: Config) -> Result<Self> {
        let m2m_payments = Arc::new(RwLock::new(m2m_payments::M2MPaymentSystem::new()));
        let keys = Arc::new(RwLock::new(ed25519::Ed25519Manager::new()));
        let payments = Arc::new(RwLock::new(payments::PaymentProcessor::new()?));
        let staking = Arc::new(RwLock::new(stake::StakeManager::new()?));

        Ok(Self {
            config,
            m2m_payments,
            keys,
            payments,
            staking,
            running: false,
        })
    }

    /// Start the wallet subsystem
    pub async fn start(&mut self) -> Result<()> {
        log::info!("Starting wallet subsystem...");
        
        // Initialize key management
        let mut keys = self.keys.write().await;
        keys.initialize().await?;
        
        // Initialize payment processor
        let mut payments = self.payments.write().await;
        payments.initialize().await?;
        
        // Initialize staking manager
        let mut staking = self.staking.write().await;
        staking.initialize().await?;
        
        self.running = true;
        log::info!("Wallet subsystem started");
        Ok(())
    }

    /// Stop the wallet subsystem
    pub async fn stop(&mut self) -> Result<()> {
        log::info!("Stopping wallet subsystem...");
        
        // Stop all wallet components
        let mut keys = self.keys.write().await;
        keys.shutdown().await?;
        
        let mut payments = self.payments.write().await;
        payments.shutdown().await?;
        
        let mut staking = self.staking.write().await;
        staking.shutdown().await?;
        
        self.running = false;
        log::info!("Wallet subsystem stopped");
        Ok(())
    }

    /// Create a new payment channel for M2M payments
    pub async fn create_payment_channel(
        &self,
        sender: String,
        recipient: String,
        deposit_amount: u64,
        timeout_hours: u64,
    ) -> Result<m2m_payments::PaymentChannel> {
        let mut m2m = self.m2m_payments.write().await;
        m2m.create_payment_channel(sender, recipient, deposit_amount, Some(timeout_hours)).await
    }

    /// Process a micro-payment
    pub async fn process_micro_payment(
        &self,
        channel_id: &str,
        amount: u64,
        tx_type: m2m_payments::MicroTransactionType,
    ) -> Result<m2m_payments::MicroTransaction> {
        let mut m2m = self.m2m_payments.write().await;
        // TODO: Fix this - method doesn't exist
        // For now, return an error
        Err(crate::error::IppanError::Validation("Method not implemented".to_string()))
    }

    /// Get payment channel information
    pub async fn get_payment_channel(&self, channel_id: &str) -> Option<m2m_payments::PaymentChannel> {
        let m2m = self.m2m_payments.read().await;
        m2m.get_payment_channel(channel_id).cloned()
    }

    /// Get all payment channels for an address
    pub async fn get_channels_for_address(&self, address: &str) -> Vec<m2m_payments::PaymentChannel> {
        let m2m = self.m2m_payments.read().await;
        m2m.get_channels_for_address(address)
            .iter()
            .map(|c| (*c).clone())
            .collect()
    }

    /// Get M2M payment statistics
    pub async fn get_m2m_statistics(&self) -> m2m_payments::PaymentStatistics {
        let m2m = self.m2m_payments.read().await;
        m2m.get_payment_statistics()
    }

    /// Clean up expired payment channels
    pub async fn cleanup_expired_channels(&self) -> usize {
        let mut m2m = self.m2m_payments.write().await;
        m2m.cleanup_expired_channels()
    }

    /// Get total fees collected from M2M payments
    pub async fn get_total_m2m_fees(&self) -> u64 {
        let m2m = self.m2m_payments.read().await;
        m2m.get_total_fees_collected()
    }
}
