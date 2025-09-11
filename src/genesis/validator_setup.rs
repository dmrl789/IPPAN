//! Validator setup for IPPAN
//! 
//! Implements validator setup and management including validator registration,
//! stake management, and validator configuration.

use crate::{Result, IppanError, TransactionHash};
use crate::crypto::{RealHashFunctions, RealEd25519, RealTransactionSigner};
use super::{ValidatorInfo, NetworkParameters};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};
use tracing::{info, warn, error, debug};

/// Validator setup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorSetupConfig {
    /// Enable validator setup
    pub enable_validator_setup: bool,
    /// Minimum validators required
    pub min_validators_required: usize,
    /// Maximum validators allowed
    pub max_validators_allowed: usize,
    /// Minimum stake required
    pub min_stake_required: u64,
    /// Maximum stake allowed
    pub max_stake_allowed: u64,
    /// Default commission rate (basis points)
    pub default_commission_rate: u16,
    /// Maximum commission rate (basis points)
    pub max_commission_rate: u16,
    /// Enable validator rotation
    pub enable_validator_rotation: bool,
    /// Validator rotation interval in seconds
    pub validator_rotation_interval_seconds: u64,
    /// Enable validator slashing
    pub enable_validator_slashing: bool,
    /// Slashing penalty (basis points)
    pub slashing_penalty: u16,
}

impl Default for ValidatorSetupConfig {
    fn default() -> Self {
        Self {
            enable_validator_setup: true,
            min_validators_required: 4,
            max_validators_allowed: 100,
            min_stake_required: 10_000_000_000, // 10 IPN
            max_stake_allowed: 100_000_000_000, // 100 IPN
            default_commission_rate: 500, // 5%
            max_commission_rate: 2000, // 20%
            enable_validator_rotation: true,
            validator_rotation_interval_seconds: 24 * 3600, // 24 hours
            enable_validator_slashing: true,
            slashing_penalty: 1000, // 10%
        }
    }
}

/// Validator setup request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorSetupRequest {
    /// Validator name
    pub validator_name: String,
    /// Validator description
    pub validator_description: String,
    /// Validator website
    pub validator_website: String,
    /// Validator contact
    pub validator_contact: String,
    /// Stake amount
    pub stake_amount: u64,
    /// Commission rate (basis points)
    pub commission_rate: u16,
    /// Validator location
    pub validator_location: String,
    /// Validator operator
    pub validator_operator: String,
    /// Public key
    pub public_key: [u8; 32],
    /// Validator address
    pub validator_address: String,
}

/// Validator setup result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorSetupResult {
    /// Validator info
    pub validator_info: ValidatorInfo,
    /// Setup success
    pub success: bool,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Setup time in milliseconds
    pub setup_time_ms: u64,
    /// Validator ID
    pub validator_id: [u8; 32],
    /// Validator address
    pub validator_address: String,
    /// Stake amount
    pub stake_amount: u64,
    /// Commission rate
    pub commission_rate: u16,
}

/// Validator setup statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorSetupStats {
    /// Total validators set up
    pub total_validators_setup: u64,
    /// Successful setups
    pub successful_setups: u64,
    /// Failed setups
    pub failed_setups: u64,
    /// Average setup time in milliseconds
    pub average_setup_time_ms: f64,
    /// Average stake amount
    pub average_stake_amount: f64,
    /// Average commission rate
    pub average_commission_rate: f64,
    /// Setup success rate
    pub setup_success_rate: f64,
    /// Active validators
    pub active_validators: u64,
    /// Inactive validators
    pub inactive_validators: u64,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Last setup timestamp
    pub last_setup: Option<u64>,
}

impl Default for ValidatorSetupStats {
    fn default() -> Self {
        Self {
            total_validators_setup: 0,
            successful_setups: 0,
            failed_setups: 0,
            average_setup_time_ms: 0.0,
            average_stake_amount: 0.0,
            average_commission_rate: 0.0,
            setup_success_rate: 0.0,
            active_validators: 0,
            inactive_validators: 0,
            uptime_seconds: 0,
            last_setup: None,
        }
    }
}

/// Validator setup manager
pub struct ValidatorSetupManager {
    /// Configuration
    config: ValidatorSetupConfig,
    /// Validators
    validators: Arc<RwLock<HashMap<[u8; 32], ValidatorInfo>>>,
    /// Statistics
    stats: Arc<RwLock<ValidatorSetupStats>>,
    /// Is running
    is_running: Arc<RwLock<bool>>,
    /// Start time
    start_time: Instant,
}

impl ValidatorSetupManager {
    /// Create a new validator setup manager
    pub fn new(config: ValidatorSetupConfig) -> Self {
        Self {
            config,
            validators: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(ValidatorSetupStats::default())),
            is_running: Arc::new(RwLock::new(false)),
            start_time: Instant::now(),
        }
    }
    
    /// Start the validator setup manager
    pub async fn start(&self) -> Result<()> {
        info!("Starting validator setup manager");
        
        let mut is_running = self.is_running.write().await;
        *is_running = true;
        drop(is_running);
        
        info!("Validator setup manager started successfully");
        Ok(())
    }
    
    /// Stop the validator setup manager
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping validator setup manager");
        
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        
        info!("Validator setup manager stopped");
        Ok(())
    }
    
    /// Setup a new validator
    pub async fn setup_validator(&self, request: ValidatorSetupRequest) -> Result<ValidatorSetupResult> {
        let start_time = Instant::now();
        
        info!("Setting up validator: {}", request.validator_name);
        
        // Validate request
        self.validate_setup_request(&request).await?;
        
        // Generate validator ID
        let validator_id = self.generate_validator_id(&request).await?;
        
        // Create validator info
        let validator_info = ValidatorInfo {
            validator_id,
            validator_address: request.validator_address.clone(),
            public_key: request.public_key,
            stake_amount: request.stake_amount,
            commission_rate: request.commission_rate,
            is_active: true,
            validator_name: request.validator_name.clone(),
            validator_description: request.validator_description,
            validator_website: request.validator_website,
            validator_contact: request.validator_contact,
        };
        
        // Add validator
        {
            let mut validators = self.validators.write().await;
            validators.insert(validator_id, validator_info.clone());
        }
        
        let setup_time = start_time.elapsed().as_millis() as u64;
        
        let result = ValidatorSetupResult {
            validator_info,
            success: true,
            error_message: None,
            setup_time_ms: setup_time,
            validator_id,
            validator_address: request.validator_address,
            stake_amount: request.stake_amount,
            commission_rate: request.commission_rate,
        };
        
        // Update statistics
        self.update_stats(setup_time, request.stake_amount, request.commission_rate, true).await;
        
        info!("Validator setup completed successfully in {}ms", setup_time);
        Ok(result)
    }
    
    /// Get validator by ID
    pub async fn get_validator(&self, validator_id: &[u8; 32]) -> Result<Option<ValidatorInfo>> {
        let validators = self.validators.read().await;
        Ok(validators.get(validator_id).cloned())
    }
    
    /// Get all validators
    pub async fn get_all_validators(&self) -> Result<Vec<ValidatorInfo>> {
        let validators = self.validators.read().await;
        Ok(validators.values().cloned().collect())
    }
    
    /// Get active validators
    pub async fn get_active_validators(&self) -> Result<Vec<ValidatorInfo>> {
        let validators = self.validators.read().await;
        Ok(validators.values().filter(|v| v.is_active).cloned().collect())
    }
    
    /// Update validator stake
    pub async fn update_validator_stake(&self, validator_id: &[u8; 32], new_stake: u64) -> Result<()> {
        info!("Updating validator stake for validator {:02x?}", validator_id);
        
        // Validate stake amount
        if new_stake < self.config.min_stake_required {
            return Err(IppanError::Validator("Stake amount below minimum".to_string()));
        }
        
        if new_stake > self.config.max_stake_allowed {
            return Err(IppanError::Validator("Stake amount exceeds maximum".to_string()));
        }
        
        // Update validator
        {
            let mut validators = self.validators.write().await;
            if let Some(validator) = validators.get_mut(validator_id) {
                validator.stake_amount = new_stake;
            } else {
                return Err(IppanError::Validator("Validator not found".to_string()));
            }
        }
        
        info!("Validator stake updated successfully");
        Ok(())
    }
    
    /// Update validator commission
    pub async fn update_validator_commission(&self, validator_id: &[u8; 32], new_commission: u16) -> Result<()> {
        info!("Updating validator commission for validator {:02x?}", validator_id);
        
        // Validate commission rate
        if new_commission > self.config.max_commission_rate {
            return Err(IppanError::Validator("Commission rate exceeds maximum".to_string()));
        }
        
        // Update validator
        {
            let mut validators = self.validators.write().await;
            if let Some(validator) = validators.get_mut(validator_id) {
                validator.commission_rate = new_commission;
            } else {
                return Err(IppanError::Validator("Validator not found".to_string()));
            }
        }
        
        info!("Validator commission updated successfully");
        Ok(())
    }
    
    /// Activate validator
    pub async fn activate_validator(&self, validator_id: &[u8; 32]) -> Result<()> {
        info!("Activating validator {:02x?}", validator_id);
        
        {
            let mut validators = self.validators.write().await;
            if let Some(validator) = validators.get_mut(validator_id) {
                validator.is_active = true;
            } else {
                return Err(IppanError::Validator("Validator not found".to_string()));
            }
        }
        
        info!("Validator activated successfully");
        Ok(())
    }
    
    /// Deactivate validator
    pub async fn deactivate_validator(&self, validator_id: &[u8; 32]) -> Result<()> {
        info!("Deactivating validator {:02x?}", validator_id);
        
        {
            let mut validators = self.validators.write().await;
            if let Some(validator) = validators.get_mut(validator_id) {
                validator.is_active = false;
            } else {
                return Err(IppanError::Validator("Validator not found".to_string()));
            }
        }
        
        info!("Validator deactivated successfully");
        Ok(())
    }
    
    /// Remove validator
    pub async fn remove_validator(&self, validator_id: &[u8; 32]) -> Result<()> {
        info!("Removing validator {:02x?}", validator_id);
        
        {
            let mut validators = self.validators.write().await;
            validators.remove(validator_id);
        }
        
        info!("Validator removed successfully");
        Ok(())
    }
    
    /// Validate setup request
    async fn validate_setup_request(&self, request: &ValidatorSetupRequest) -> Result<()> {
        // Validate validator name
        if request.validator_name.is_empty() {
            return Err(IppanError::Validator("Validator name cannot be empty".to_string()));
        }
        
        // Validate stake amount
        if request.stake_amount < self.config.min_stake_required {
            return Err(IppanError::Validator(
                format!("Stake amount {} is below minimum {}", 
                    request.stake_amount, self.config.min_stake_required)
            ));
        }
        
        if request.stake_amount > self.config.max_stake_allowed {
            return Err(IppanError::Validator(
                format!("Stake amount {} exceeds maximum {}", 
                    request.stake_amount, self.config.max_stake_allowed)
            ));
        }
        
        // Validate commission rate
        if request.commission_rate > self.config.max_commission_rate {
            return Err(IppanError::Validator(
                format!("Commission rate {} exceeds maximum {}", 
                    request.commission_rate, self.config.max_commission_rate)
            ));
        }
        
        // Validate public key
        if request.public_key == [0u8; 32] {
            return Err(IppanError::Validator("Public key cannot be zero".to_string()));
        }
        
        // Validate validator address
        if request.validator_address.is_empty() {
            return Err(IppanError::Validator("Validator address cannot be empty".to_string()));
        }
        
        // Check validator count limit
        let validators = self.validators.read().await;
        if validators.len() >= self.config.max_validators_allowed {
            return Err(IppanError::Validator(
                format!("Maximum validators {} reached", self.config.max_validators_allowed)
            ));
        }
        
        Ok(())
    }
    
    /// Generate validator ID
    async fn generate_validator_id(&self, request: &ValidatorSetupRequest) -> Result<[u8; 32]> {
        let data = format!("{}{}{}", request.validator_name, request.validator_address, hex::encode(request.public_key));
        Ok(RealHashFunctions::sha256(data.as_bytes()))
    }
    
    /// Update statistics
    async fn update_stats(
        &self,
        setup_time_ms: u64,
        stake_amount: u64,
        commission_rate: u16,
        success: bool,
    ) {
        let mut stats = self.stats.write().await;
        
        stats.total_validators_setup += 1;
        if success {
            stats.successful_setups += 1;
        } else {
            stats.failed_setups += 1;
        }
        
        // Update averages
        let total = stats.total_validators_setup as f64;
        stats.average_setup_time_ms = 
            (stats.average_setup_time_ms * (total - 1.0) + setup_time_ms as f64) / total;
        stats.average_stake_amount = 
            (stats.average_stake_amount * (total - 1.0) + stake_amount as f64) / total;
        stats.average_commission_rate = 
            (stats.average_commission_rate * (total - 1.0) + commission_rate as f64) / total;
        
        // Update success rate
        stats.setup_success_rate = stats.successful_setups as f64 / total;
        
        // Update active/inactive counts
        let validators = self.validators.read().await;
        stats.active_validators = validators.values().filter(|v| v.is_active).count() as u64;
        stats.inactive_validators = validators.values().filter(|v| !v.is_active).count() as u64;
        
        // Update timestamps
        stats.last_setup = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
        stats.uptime_seconds = self.start_time.elapsed().as_secs();
    }
    
    /// Get validator setup statistics
    pub async fn get_stats(&self) -> Result<ValidatorSetupStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }
    
    /// Get validator count
    pub async fn get_validator_count(&self) -> Result<usize> {
        let validators = self.validators.read().await;
        Ok(validators.len())
    }
    
    /// Get active validator count
    pub async fn get_active_validator_count(&self) -> Result<usize> {
        let validators = self.validators.read().await;
        Ok(validators.values().filter(|v| v.is_active).count())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_validator_setup_config() {
        let config = ValidatorSetupConfig::default();
        assert!(config.enable_validator_setup);
        assert_eq!(config.min_validators_required, 4);
        assert_eq!(config.max_validators_allowed, 100);
        assert_eq!(config.min_stake_required, 10_000_000_000);
        assert_eq!(config.max_stake_allowed, 100_000_000_000);
        assert_eq!(config.default_commission_rate, 500);
        assert_eq!(config.max_commission_rate, 2000);
        assert!(config.enable_validator_rotation);
        assert_eq!(config.validator_rotation_interval_seconds, 24 * 3600);
        assert!(config.enable_validator_slashing);
        assert_eq!(config.slashing_penalty, 1000);
    }
    
    #[tokio::test]
    async fn test_validator_setup_request() {
        let request = ValidatorSetupRequest {
            validator_name: "Test Validator".to_string(),
            validator_description: "Test validator description".to_string(),
            validator_website: "https://test.com".to_string(),
            validator_contact: "contact@test.com".to_string(),
            stake_amount: 50_000_000_000,
            commission_rate: 750,
            validator_location: "US".to_string(),
            validator_operator: "Test Operator".to_string(),
            public_key: [1u8; 32],
            validator_address: "i1234567890abcdef".to_string(),
        };
        
        assert_eq!(request.validator_name, "Test Validator");
        assert_eq!(request.validator_description, "Test validator description");
        assert_eq!(request.validator_website, "https://test.com");
        assert_eq!(request.validator_contact, "contact@test.com");
        assert_eq!(request.stake_amount, 50_000_000_000);
        assert_eq!(request.commission_rate, 750);
        assert_eq!(request.validator_location, "US");
        assert_eq!(request.validator_operator, "Test Operator");
        assert_eq!(request.public_key, [1u8; 32]);
        assert_eq!(request.validator_address, "i1234567890abcdef");
    }
    
    #[tokio::test]
    async fn test_validator_setup_result() {
        let result = ValidatorSetupResult {
            validator_info: ValidatorInfo {
                validator_id: [1u8; 32],
                validator_address: "i1234567890abcdef".to_string(),
                public_key: [2u8; 32],
                stake_amount: 50_000_000_000,
                commission_rate: 750,
                is_active: true,
                validator_name: "Test Validator".to_string(),
                validator_description: "Test validator description".to_string(),
                validator_website: "https://test.com".to_string(),
                validator_contact: "contact@test.com".to_string(),
            },
            success: true,
            error_message: None,
            setup_time_ms: 100,
            validator_id: [1u8; 32],
            validator_address: "i1234567890abcdef".to_string(),
            stake_amount: 50_000_000_000,
            commission_rate: 750,
        };
        
        assert!(result.success);
        assert!(result.error_message.is_none());
        assert_eq!(result.setup_time_ms, 100);
        assert_eq!(result.validator_id, [1u8; 32]);
        assert_eq!(result.validator_address, "i1234567890abcdef");
        assert_eq!(result.stake_amount, 50_000_000_000);
        assert_eq!(result.commission_rate, 750);
    }
    
    #[tokio::test]
    async fn test_validator_setup_stats() {
        let stats = ValidatorSetupStats {
            total_validators_setup: 10,
            successful_setups: 9,
            failed_setups: 1,
            average_setup_time_ms: 50.0,
            average_stake_amount: 25_000_000_000.0,
            average_commission_rate: 600.0,
            setup_success_rate: 0.9,
            active_validators: 8,
            inactive_validators: 1,
            uptime_seconds: 3600,
            last_setup: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()),
        };
        
        assert_eq!(stats.total_validators_setup, 10);
        assert_eq!(stats.successful_setups, 9);
        assert_eq!(stats.failed_setups, 1);
        assert_eq!(stats.average_setup_time_ms, 50.0);
        assert_eq!(stats.average_stake_amount, 25_000_000_000.0);
        assert_eq!(stats.average_commission_rate, 600.0);
        assert_eq!(stats.setup_success_rate, 0.9);
        assert_eq!(stats.active_validators, 8);
        assert_eq!(stats.inactive_validators, 1);
        assert_eq!(stats.uptime_seconds, 3600);
    }
}
