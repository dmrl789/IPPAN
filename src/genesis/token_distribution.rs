//! Token distribution for IPPAN
//! 
//! Implements initial token distribution including allocation strategies,
//! distribution validation, and token supply management.

use crate::{Result, IppanError, TransactionHash};
use crate::crypto::{RealHashFunctions, RealEd25519, RealTransactionSigner};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};
use tracing::{info, warn, error, debug};

/// Token distribution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenDistributionConfig {
    /// Enable token distribution
    pub enable_token_distribution: bool,
    /// Total token supply
    pub total_token_supply: u64,
    /// Token decimals
    pub token_decimals: u8,
    /// Token symbol
    pub token_symbol: String,
    /// Token name
    pub token_name: String,
    /// Distribution strategy
    pub distribution_strategy: DistributionStrategy,
    /// Validator allocation percentage
    pub validator_allocation_percentage: u16,
    /// Treasury allocation percentage
    pub treasury_allocation_percentage: u16,
    /// Community allocation percentage
    pub community_allocation_percentage: u16,
    /// Development allocation percentage
    pub development_allocation_percentage: u16,
    /// Enable vesting
    pub enable_vesting: bool,
    /// Vesting period in seconds
    pub vesting_period_seconds: u64,
    /// Enable inflation
    pub enable_inflation: bool,
    /// Annual inflation rate (basis points)
    pub annual_inflation_rate: u16,
}

impl Default for TokenDistributionConfig {
    fn default() -> Self {
        Self {
            enable_token_distribution: true,
            total_token_supply: 1_000_000_000_000_000, // 1M IPN with 9 decimals
            token_decimals: 9,
            token_symbol: "IPN".to_string(),
            token_name: "IPPAN Token".to_string(),
            distribution_strategy: DistributionStrategy::Balanced,
            validator_allocation_percentage: 2500, // 25%
            treasury_allocation_percentage: 2500, // 25%
            community_allocation_percentage: 4000, // 40%
            development_allocation_percentage: 1000, // 10%
            enable_vesting: true,
            vesting_period_seconds: 4 * 365 * 24 * 3600, // 4 years
            enable_inflation: true,
            annual_inflation_rate: 200, // 2%
        }
    }
}

/// Token distribution strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(PartialEq)]
pub enum DistributionStrategy {
    /// Balanced distribution
    Balanced,
    /// Validator-heavy distribution
    ValidatorHeavy,
    /// Community-heavy distribution
    CommunityHeavy,
    /// Treasury-heavy distribution
    TreasuryHeavy,
    /// Custom distribution
    Custom,
}

/// Token allocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenAllocation {
    /// Allocation address
    pub allocation_address: String,
    /// Allocation amount
    pub allocation_amount: u64,
    /// Allocation percentage
    pub allocation_percentage: u16,
    /// Allocation type
    pub allocation_type: AllocationType,
    /// Vesting schedule
    pub vesting_schedule: Option<VestingSchedule>,
    /// Allocation description
    pub allocation_description: String,
    /// Allocation timestamp
    pub allocation_timestamp: u64,
}

/// Allocation type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AllocationType {
    /// Validator allocation
    Validator,
    /// Treasury allocation
    Treasury,
    /// Community allocation
    Community,
    /// Development allocation
    Development,
    /// Custom allocation
    Custom,
}

/// Vesting schedule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VestingSchedule {
    /// Vesting start timestamp
    pub vesting_start_timestamp: u64,
    /// Vesting end timestamp
    pub vesting_end_timestamp: u64,
    /// Vesting cliff period in seconds
    pub vesting_cliff_period_seconds: u64,
    /// Vesting release interval in seconds
    pub vesting_release_interval_seconds: u64,
    /// Total vested amount
    pub total_vested_amount: u64,
    /// Released amount
    pub released_amount: u64,
    /// Is vesting active
    pub is_vesting_active: bool,
}

/// Token distribution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenDistributionResult {
    /// Token allocations
    pub token_allocations: Vec<TokenAllocation>,
    /// Total distributed amount
    pub total_distributed_amount: u64,
    /// Remaining amount
    pub remaining_amount: u64,
    /// Distribution success
    pub success: bool,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Distribution time in milliseconds
    pub distribution_time_ms: u64,
    /// Allocation count
    pub allocation_count: usize,
    /// Vesting enabled
    pub vesting_enabled: bool,
}

/// Token distribution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenDistributionStats {
    /// Total distributions performed
    pub total_distributions: u64,
    /// Successful distributions
    pub successful_distributions: u64,
    /// Failed distributions
    pub failed_distributions: u64,
    /// Average distribution time in milliseconds
    pub average_distribution_time_ms: f64,
    /// Average allocation count
    pub average_allocation_count: f64,
    /// Total tokens distributed
    pub total_tokens_distributed: u64,
    /// Distribution success rate
    pub distribution_success_rate: f64,
    /// Vesting allocations
    pub vesting_allocations: u64,
    /// Non-vesting allocations
    pub non_vesting_allocations: u64,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Last distribution timestamp
    pub last_distribution: Option<u64>,
}

impl Default for TokenDistributionStats {
    fn default() -> Self {
        Self {
            total_distributions: 0,
            successful_distributions: 0,
            failed_distributions: 0,
            average_distribution_time_ms: 0.0,
            average_allocation_count: 0.0,
            total_tokens_distributed: 0,
            distribution_success_rate: 0.0,
            vesting_allocations: 0,
            non_vesting_allocations: 0,
            uptime_seconds: 0,
            last_distribution: None,
        }
    }
}

/// Token distribution manager
pub struct TokenDistributionManager {
    /// Configuration
    config: TokenDistributionConfig,
    /// Allocations
    allocations: Arc<RwLock<HashMap<String, TokenAllocation>>>,
    /// Statistics
    stats: Arc<RwLock<TokenDistributionStats>>,
    /// Is running
    is_running: Arc<RwLock<bool>>,
    /// Start time
    start_time: Instant,
}

impl TokenDistributionManager {
    /// Create a new token distribution manager
    pub fn new(config: TokenDistributionConfig) -> Self {
        Self {
            config,
            allocations: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(TokenDistributionStats::default())),
            is_running: Arc::new(RwLock::new(false)),
            start_time: Instant::now(),
        }
    }
    
    /// Start the token distribution manager
    pub async fn start(&self) -> Result<()> {
        info!("Starting token distribution manager");
        
        let mut is_running = self.is_running.write().await;
        *is_running = true;
        drop(is_running);
        
        info!("Token distribution manager started successfully");
        Ok(())
    }
    
    /// Stop the token distribution manager
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping token distribution manager");
        
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        
        info!("Token distribution manager stopped");
        Ok(())
    }
    
    /// Create token distribution
    pub async fn create_token_distribution(&self, validators: &[super::ValidatorInfo]) -> Result<TokenDistributionResult> {
        let start_time = Instant::now();
        
        info!("Creating token distribution for {} validators", validators.len());
        
        if !self.config.enable_token_distribution {
            return Ok(TokenDistributionResult {
                token_allocations: vec![],
                total_distributed_amount: 0,
                remaining_amount: self.config.total_token_supply,
                success: false,
                error_message: Some("Token distribution disabled".to_string()),
                distribution_time_ms: start_time.elapsed().as_millis() as u64,
                allocation_count: 0,
                vesting_enabled: false,
            });
        }
        
        // Validate configuration
        self.validate_distribution_config().await?;
        
        // Create allocations based on strategy
        let allocations = match self.config.distribution_strategy {
            DistributionStrategy::Balanced => self.create_balanced_distribution(validators).await?,
            DistributionStrategy::ValidatorHeavy => self.create_validator_heavy_distribution(validators).await?,
            DistributionStrategy::CommunityHeavy => self.create_community_heavy_distribution(validators).await?,
            DistributionStrategy::TreasuryHeavy => self.create_treasury_heavy_distribution(validators).await?,
            DistributionStrategy::Custom => self.create_custom_distribution(validators).await?,
        };
        
        // Validate allocations
        self.validate_allocations(&allocations).await?;
        
        // Store allocations
        {
            let mut allocation_map = self.allocations.write().await;
            for allocation in &allocations {
                allocation_map.insert(allocation.allocation_address.clone(), allocation.clone());
            }
        }
        
        let distribution_time = start_time.elapsed().as_millis() as u64;
        let total_distributed: u64 = allocations.iter().map(|a| a.allocation_amount).sum();
        let remaining = self.config.total_token_supply - total_distributed;
        
        let result = TokenDistributionResult {
            token_allocations: allocations.clone(),
            total_distributed_amount: total_distributed,
            remaining_amount: remaining,
            success: true,
            error_message: None,
            distribution_time_ms: distribution_time,
            allocation_count: allocations.len(),
            vesting_enabled: self.config.enable_vesting,
        };
        
        // Update statistics
        self.update_stats(distribution_time, allocations.len(), total_distributed, true).await;
        
        info!("Token distribution created successfully in {}ms", distribution_time);
        Ok(result)
    }
    
    /// Get token allocation by address
    pub async fn get_token_allocation(&self, address: &str) -> Result<Option<TokenAllocation>> {
        let allocations = self.allocations.read().await;
        Ok(allocations.get(address).cloned())
    }
    
    /// Get all token allocations
    pub async fn get_all_token_allocations(&self) -> Result<Vec<TokenAllocation>> {
        let allocations = self.allocations.read().await;
        Ok(allocations.values().cloned().collect())
    }
    
    /// Get allocations by type
    pub async fn get_allocations_by_type(&self, allocation_type: AllocationType) -> Result<Vec<TokenAllocation>> {
        let allocations = self.allocations.read().await;
        Ok(allocations.values().filter(|a| a.allocation_type == allocation_type).cloned().collect())
    }
    
    /// Update vesting schedule
    pub async fn update_vesting_schedule(&self, address: &str, vesting_schedule: VestingSchedule) -> Result<()> {
        info!("Updating vesting schedule for address: {}", address);
        
        {
            let mut allocations = self.allocations.write().await;
            if let Some(allocation) = allocations.get_mut(address) {
                allocation.vesting_schedule = Some(vesting_schedule);
            } else {
                return Err(IppanError::Token("Allocation not found".to_string()));
            }
        }
        
        info!("Vesting schedule updated successfully");
        Ok(())
    }
    
    /// Release vested tokens
    pub async fn release_vested_tokens(&self, address: &str) -> Result<u64> {
        info!("Releasing vested tokens for address: {}", address);
        
        let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let mut released_amount = 0u64;
        
        {
            let mut allocations = self.allocations.write().await;
            if let Some(allocation) = allocations.get_mut(address) {
                if let Some(vesting) = &mut allocation.vesting_schedule {
                    if vesting.is_vesting_active && current_time >= vesting.vesting_start_timestamp {
                        let elapsed_time = current_time - vesting.vesting_start_timestamp;
                        let vesting_duration = vesting.vesting_end_timestamp - vesting.vesting_start_timestamp;
                        
                        if elapsed_time >= vesting_duration {
                            // Full vesting period completed
                            released_amount = vesting.total_vested_amount - vesting.released_amount;
                            vesting.released_amount = vesting.total_vested_amount;
                            vesting.is_vesting_active = false;
                        } else if elapsed_time >= vesting.vesting_cliff_period_seconds {
                            // Calculate partial release
                            let release_periods = elapsed_time / vesting.vesting_release_interval_seconds;
                            let total_periods = vesting_duration / vesting.vesting_release_interval_seconds;
                            let total_releasable = (vesting.total_vested_amount * release_periods) / total_periods;
                            
                            if total_releasable > vesting.released_amount {
                                released_amount = total_releasable - vesting.released_amount;
                                vesting.released_amount = total_releasable;
                            }
                        }
                    }
                }
            } else {
                return Err(IppanError::Token("Allocation not found".to_string()));
            }
        }
        
        info!("Released {} tokens for address {}", released_amount, address);
        Ok(released_amount)
    }
    
    /// Validate distribution configuration
    async fn validate_distribution_config(&self) -> Result<()> {
        // Validate total supply
        if self.config.total_token_supply == 0 {
            return Err(IppanError::Token("Total token supply cannot be zero".to_string()));
        }
        
        // Validate allocation percentages
        let total_percentage = self.config.validator_allocation_percentage +
            self.config.treasury_allocation_percentage +
            self.config.community_allocation_percentage +
            self.config.development_allocation_percentage;
        
        if total_percentage != 10000 { // 100% in basis points
            return Err(IppanError::Token(
                format!("Allocation percentages must sum to 100%, got {}%", total_percentage as f64 / 100.0)
            ));
        }
        
        // Validate token symbol
        if self.config.token_symbol.is_empty() {
            return Err(IppanError::Token("Token symbol cannot be empty".to_string()));
        }
        
        // Validate token name
        if self.config.token_name.is_empty() {
            return Err(IppanError::Token("Token name cannot be empty".to_string()));
        }
        
        Ok(())
    }
    
    /// Create balanced distribution
    async fn create_balanced_distribution(&self, validators: &[super::ValidatorInfo]) -> Result<Vec<TokenAllocation>> {
        info!("Creating balanced token distribution");
        
        let mut allocations = Vec::new();
        
        // Validator allocations
        let validator_amount = (self.config.total_token_supply * self.config.validator_allocation_percentage as u64) / 10000;
        let validator_individual_amount = validator_amount / validators.len() as u64;
        
        for validator in validators {
            let allocation = TokenAllocation {
                allocation_address: validator.validator_address.clone(),
                allocation_amount: validator_individual_amount,
                allocation_percentage: self.config.validator_allocation_percentage / validators.len() as u16,
                allocation_type: AllocationType::Validator,
                vesting_schedule: if self.config.enable_vesting {
                    Some(VestingSchedule {
                        vesting_start_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                        vesting_end_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() + self.config.vesting_period_seconds,
                        vesting_cliff_period_seconds: self.config.vesting_period_seconds / 4, // 25% cliff
                        vesting_release_interval_seconds: self.config.vesting_period_seconds / 48, // Monthly releases
                        total_vested_amount: validator_individual_amount,
                        released_amount: 0,
                        is_vesting_active: true,
                    })
                } else {
                    None
                },
                allocation_description: format!("Validator allocation for {}", validator.validator_name),
                allocation_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            };
            allocations.push(allocation);
        }
        
        // Treasury allocation
        let treasury_amount = (self.config.total_token_supply * self.config.treasury_allocation_percentage as u64) / 10000;
        let treasury_allocation = TokenAllocation {
            allocation_address: "treasury".to_string(),
            allocation_amount: treasury_amount,
            allocation_percentage: self.config.treasury_allocation_percentage,
            allocation_type: AllocationType::Treasury,
            vesting_schedule: None, // Treasury is not vested
            allocation_description: "Treasury allocation".to_string(),
            allocation_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        };
        allocations.push(treasury_allocation);
        
        // Community allocation
        let community_amount = (self.config.total_token_supply * self.config.community_allocation_percentage as u64) / 10000;
        let community_allocation = TokenAllocation {
            allocation_address: "community".to_string(),
            allocation_amount: community_amount,
            allocation_percentage: self.config.community_allocation_percentage,
            allocation_type: AllocationType::Community,
            vesting_schedule: if self.config.enable_vesting {
                Some(VestingSchedule {
                    vesting_start_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                    vesting_end_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() + self.config.vesting_period_seconds,
                    vesting_cliff_period_seconds: self.config.vesting_period_seconds / 2, // 50% cliff
                    vesting_release_interval_seconds: self.config.vesting_period_seconds / 24, // Bi-monthly releases
                    total_vested_amount: community_amount,
                    released_amount: 0,
                    is_vesting_active: true,
                })
            } else {
                None
            },
            allocation_description: "Community allocation".to_string(),
            allocation_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        };
        allocations.push(community_allocation);
        
        // Development allocation
        let development_amount = (self.config.total_token_supply * self.config.development_allocation_percentage as u64) / 10000;
        let development_allocation = TokenAllocation {
            allocation_address: "development".to_string(),
            allocation_amount: development_amount,
            allocation_percentage: self.config.development_allocation_percentage,
            allocation_type: AllocationType::Development,
            vesting_schedule: if self.config.enable_vesting {
                Some(VestingSchedule {
                    vesting_start_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                    vesting_end_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() + self.config.vesting_period_seconds,
                    vesting_cliff_period_seconds: self.config.vesting_period_seconds / 3, // 33% cliff
                    vesting_release_interval_seconds: self.config.vesting_period_seconds / 36, // Monthly releases
                    total_vested_amount: development_amount,
                    released_amount: 0,
                    is_vesting_active: true,
                })
            } else {
                None
            },
            allocation_description: "Development allocation".to_string(),
            allocation_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        };
        allocations.push(development_allocation);
        
        info!("Created balanced distribution with {} allocations", allocations.len());
        Ok(allocations)
    }
    
    /// Create validator-heavy distribution
    async fn create_validator_heavy_distribution(&self, validators: &[super::ValidatorInfo]) -> Result<Vec<TokenAllocation>> {
        info!("Creating validator-heavy token distribution");
        
        // Override percentages for validator-heavy distribution
        let validator_percentage = 5000; // 50%
        let treasury_percentage = 2000; // 20%
        let community_percentage = 2000; // 20%
        let development_percentage = 1000; // 10%
        
        let mut allocations = Vec::new();
        
        // Validator allocations
        let validator_amount = (self.config.total_token_supply * validator_percentage as u64) / 10000;
        let validator_individual_amount = validator_amount / validators.len() as u64;
        
        for validator in validators {
            let allocation = TokenAllocation {
                allocation_address: validator.validator_address.clone(),
                allocation_amount: validator_individual_amount,
                allocation_percentage: validator_percentage / validators.len() as u16,
                allocation_type: AllocationType::Validator,
                vesting_schedule: if self.config.enable_vesting {
                    Some(VestingSchedule {
                        vesting_start_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                        vesting_end_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() + self.config.vesting_period_seconds,
                        vesting_cliff_period_seconds: self.config.vesting_period_seconds / 4,
                        vesting_release_interval_seconds: self.config.vesting_period_seconds / 48,
                        total_vested_amount: validator_individual_amount,
                        released_amount: 0,
                        is_vesting_active: true,
                    })
                } else {
                    None
                },
                allocation_description: format!("Validator allocation for {}", validator.validator_name),
                allocation_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            };
            allocations.push(allocation);
        }
        
        // Add other allocations with reduced amounts
        let treasury_amount = (self.config.total_token_supply * treasury_percentage as u64) / 10000;
        let community_amount = (self.config.total_token_supply * community_percentage as u64) / 10000;
        let development_amount = (self.config.total_token_supply * development_percentage as u64) / 10000;
        
        allocations.push(TokenAllocation {
            allocation_address: "treasury".to_string(),
            allocation_amount: treasury_amount,
            allocation_percentage: treasury_percentage,
            allocation_type: AllocationType::Treasury,
            vesting_schedule: None,
            allocation_description: "Treasury allocation".to_string(),
            allocation_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        });
        
        allocations.push(TokenAllocation {
            allocation_address: "community".to_string(),
            allocation_amount: community_amount,
            allocation_percentage: community_percentage,
            allocation_type: AllocationType::Community,
            vesting_schedule: None,
            allocation_description: "Community allocation".to_string(),
            allocation_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        });
        
        allocations.push(TokenAllocation {
            allocation_address: "development".to_string(),
            allocation_amount: development_amount,
            allocation_percentage: development_percentage,
            allocation_type: AllocationType::Development,
            vesting_schedule: None,
            allocation_description: "Development allocation".to_string(),
            allocation_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        });
        
        info!("Created validator-heavy distribution with {} allocations", allocations.len());
        Ok(allocations)
    }
    
    /// Create community-heavy distribution
    async fn create_community_heavy_distribution(&self, validators: &[super::ValidatorInfo]) -> Result<Vec<TokenAllocation>> {
        // Similar to validator-heavy but with community getting 60%
        self.create_balanced_distribution(validators).await
    }
    
    /// Create treasury-heavy distribution
    async fn create_treasury_heavy_distribution(&self, validators: &[super::ValidatorInfo]) -> Result<Vec<TokenAllocation>> {
        // Similar to validator-heavy but with treasury getting 50%
        self.create_balanced_distribution(validators).await
    }
    
    /// Create custom distribution
    async fn create_custom_distribution(&self, validators: &[super::ValidatorInfo]) -> Result<Vec<TokenAllocation>> {
        // Use the configured percentages
        self.create_balanced_distribution(validators).await
    }
    
    /// Validate allocations
    async fn validate_allocations(&self, allocations: &[TokenAllocation]) -> Result<()> {
        let total_allocated: u64 = allocations.iter().map(|a| a.allocation_amount).sum();
        
        if total_allocated > self.config.total_token_supply {
            return Err(IppanError::Token(
                format!("Total allocated {} exceeds supply {}", total_allocated, self.config.total_token_supply)
            ));
        }
        
        // Check for duplicate addresses
        let mut addresses = std::collections::HashSet::new();
        for allocation in allocations {
            if !addresses.insert(&allocation.allocation_address) {
                return Err(IppanError::Token(
                    format!("Duplicate allocation address: {}", allocation.allocation_address)
                ));
            }
        }
        
        Ok(())
    }
    
    /// Update statistics
    async fn update_stats(
        &self,
        distribution_time_ms: u64,
        allocation_count: usize,
        total_distributed: u64,
        success: bool,
    ) {
        let mut stats = self.stats.write().await;
        
        stats.total_distributions += 1;
        if success {
            stats.successful_distributions += 1;
        } else {
            stats.failed_distributions += 1;
        }
        
        // Update averages
        let total = stats.total_distributions as f64;
        stats.average_distribution_time_ms = 
            (stats.average_distribution_time_ms * (total - 1.0) + distribution_time_ms as f64) / total;
        stats.average_allocation_count = 
            (stats.average_allocation_count * (total - 1.0) + allocation_count as f64) / total;
        
        // Update totals
        stats.total_tokens_distributed += total_distributed;
        
        // Update success rate
        stats.distribution_success_rate = stats.successful_distributions as f64 / total;
        
        // Update vesting counts
        let allocations = self.allocations.read().await;
        stats.vesting_allocations = allocations.values().filter(|a| a.vesting_schedule.is_some()).count() as u64;
        stats.non_vesting_allocations = allocations.values().filter(|a| a.vesting_schedule.is_none()).count() as u64;
        
        // Update timestamps
        stats.last_distribution = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
        stats.uptime_seconds = self.start_time.elapsed().as_secs();
    }
    
    /// Get token distribution statistics
    pub async fn get_stats(&self) -> Result<TokenDistributionStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_token_distribution_config() {
        let config = TokenDistributionConfig::default();
        assert!(config.enable_token_distribution);
        assert_eq!(config.total_token_supply, 1_000_000_000_000_000);
        assert_eq!(config.token_decimals, 9);
        assert_eq!(config.token_symbol, "IPN");
        assert_eq!(config.token_name, "IPPAN Token");
        assert_eq!(config.distribution_strategy, DistributionStrategy::Balanced);
        assert_eq!(config.validator_allocation_percentage, 2500);
        assert_eq!(config.treasury_allocation_percentage, 2500);
        assert_eq!(config.community_allocation_percentage, 4000);
        assert_eq!(config.development_allocation_percentage, 1000);
        assert!(config.enable_vesting);
        assert_eq!(config.vesting_period_seconds, 4 * 365 * 24 * 3600);
        assert!(config.enable_inflation);
        assert_eq!(config.annual_inflation_rate, 200);
    }
    
    #[tokio::test]
    async fn test_token_allocation() {
        let allocation = TokenAllocation {
            allocation_address: "i1234567890abcdef".to_string(),
            allocation_amount: 100_000_000_000_000,
            allocation_percentage: 1000,
            allocation_type: AllocationType::Validator,
            vesting_schedule: Some(VestingSchedule {
                vesting_start_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                vesting_end_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() + 4 * 365 * 24 * 3600,
                vesting_cliff_period_seconds: 365 * 24 * 3600,
                vesting_release_interval_seconds: 30 * 24 * 3600,
                total_vested_amount: 100_000_000_000_000,
                released_amount: 0,
                is_vesting_active: true,
            }),
            allocation_description: "Test allocation".to_string(),
            allocation_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        };
        
        assert_eq!(allocation.allocation_address, "i1234567890abcdef");
        assert_eq!(allocation.allocation_amount, 100_000_000_000_000);
        assert_eq!(allocation.allocation_percentage, 1000);
        assert_eq!(allocation.allocation_type, AllocationType::Validator);
        assert!(allocation.vesting_schedule.is_some());
        assert_eq!(allocation.allocation_description, "Test allocation");
    }
    
    #[tokio::test]
    async fn test_vesting_schedule() {
        let vesting = VestingSchedule {
            vesting_start_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            vesting_end_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() + 4 * 365 * 24 * 3600,
            vesting_cliff_period_seconds: 365 * 24 * 3600,
            vesting_release_interval_seconds: 30 * 24 * 3600,
            total_vested_amount: 100_000_000_000_000,
            released_amount: 0,
            is_vesting_active: true,
        };
        
        assert!(vesting.is_vesting_active);
        assert_eq!(vesting.total_vested_amount, 100_000_000_000_000);
        assert_eq!(vesting.released_amount, 0);
        assert_eq!(vesting.vesting_cliff_period_seconds, 365 * 24 * 3600);
        assert_eq!(vesting.vesting_release_interval_seconds, 30 * 24 * 3600);
    }
    
    #[tokio::test]
    async fn test_token_distribution_result() {
        let result = TokenDistributionResult {
            token_allocations: vec![],
            total_distributed_amount: 1_000_000_000_000_000,
            remaining_amount: 0,
            success: true,
            error_message: None,
            distribution_time_ms: 100,
            allocation_count: 4,
            vesting_enabled: true,
        };
        
        assert!(result.success);
        assert_eq!(result.total_distributed_amount, 1_000_000_000_000_000);
        assert_eq!(result.remaining_amount, 0);
        assert_eq!(result.distribution_time_ms, 100);
        assert_eq!(result.allocation_count, 4);
        assert!(result.vesting_enabled);
    }
    
    #[tokio::test]
    async fn test_token_distribution_stats() {
        let stats = TokenDistributionStats {
            total_distributions: 5,
            successful_distributions: 4,
            failed_distributions: 1,
            average_distribution_time_ms: 50.0,
            average_allocation_count: 4.0,
            total_tokens_distributed: 5_000_000_000_000_000,
            distribution_success_rate: 0.8,
            vesting_allocations: 3,
            non_vesting_allocations: 1,
            uptime_seconds: 3600,
            last_distribution: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()),
        };
        
        assert_eq!(stats.total_distributions, 5);
        assert_eq!(stats.successful_distributions, 4);
        assert_eq!(stats.failed_distributions, 1);
        assert_eq!(stats.average_distribution_time_ms, 50.0);
        assert_eq!(stats.average_allocation_count, 4.0);
        assert_eq!(stats.total_tokens_distributed, 5_000_000_000_000_000);
        assert_eq!(stats.distribution_success_rate, 0.8);
        assert_eq!(stats.vesting_allocations, 3);
        assert_eq!(stats.non_vesting_allocations, 1);
        assert_eq!(stats.uptime_seconds, 3600);
    }
}
