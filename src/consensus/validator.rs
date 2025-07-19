use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Validator information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Validator {
    pub node_id: String,
    pub stake_amount: u64,           // Stake amount in IPPAN tokens
    pub public_key: String,          // Validator's public key
    pub address: String,             // Validator's address
    pub is_active: bool,             // Whether validator is currently active
    pub last_activity: u64,          // Last activity timestamp
    pub performance_score: f64,      // Performance score (0.0 to 1.0)
    pub uptime_percentage: f64,      // Uptime percentage
    pub total_blocks_produced: u64,  // Total blocks produced
    pub total_blocks_validated: u64, // Total blocks validated
    pub slashing_events: u32,        // Number of slashing events
    pub registration_time: u64,      // When validator was registered
    pub commission_rate: f64,        // Commission rate (0.0 to 1.0)
}

/// Validator selection parameters
#[derive(Debug, Clone)]
pub struct ValidatorSelectionParams {
    pub min_stake: u64,              // Minimum stake required
    pub max_validators: usize,        // Maximum number of validators
    pub selection_method: SelectionMethod,
    pub rotation_interval: u64,       // Rotation interval in rounds
    pub performance_threshold: f64,   // Minimum performance score
    pub uptime_threshold: f64,       // Minimum uptime percentage
}

/// Validator selection method
#[derive(Debug, Clone, PartialEq)]
pub enum SelectionMethod {
    StakeBased,      // Select based on stake amount
    Random,          // Random selection
    Performance,     // Select based on performance
    Hybrid,          // Combination of methods
}

/// Validator set for a specific round
#[derive(Debug, Clone)]
pub struct ValidatorSet {
    pub round: u64,
    pub validators: Vec<String>,      // Node IDs of selected validators
    pub primary_validator: String,    // Primary validator for this round
    pub backup_validators: Vec<String>, // Backup validators
    pub selection_timestamp: u64,     // When selection was made
    pub seed: u64,                   // Random seed for selection
}

/// Slashing event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashingEvent {
    pub validator_id: String,
    pub event_type: SlashingType,
    pub timestamp: u64,
    pub evidence: String,
    pub severity: SlashingSeverity,
    pub stake_penalty: u64,
}

/// Slashing event type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SlashingType {
    DoubleSigning,           // Double signing blocks
    Inactivity,             // Extended inactivity
    InvalidBlock,           // Producing invalid blocks
    NetworkAttack,          // Attempting network attacks
    StakeManipulation,      // Manipulating stake amounts
}

/// Slashing severity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SlashingSeverity {
    Minor,      // Minor penalty
    Major,      // Major penalty
    Critical,   // Critical penalty - removal from validator set
}

/// Validator Manager for IPPAN consensus
pub struct ValidatorManager {
    validators: Arc<RwLock<HashMap<String, Validator>>>,
    current_validator_set: Arc<RwLock<Option<ValidatorSet>>>,
    slashing_events: Arc<RwLock<Vec<SlashingEvent>>>,
    selection_params: ValidatorSelectionParams,
    rng: Arc<RwLock<StdRng>>,
}

impl Validator {
    /// Create a new validator
    pub fn new(
        node_id: &str,
        stake_amount: u64,
        public_key: &str,
        address: &str,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Validator {
            node_id: node_id.to_string(),
            stake_amount,
            public_key: public_key.to_string(),
            address: address.to_string(),
            is_active: true,
            last_activity: now,
            performance_score: 1.0,
            uptime_percentage: 100.0,
            total_blocks_produced: 0,
            total_blocks_validated: 0,
            slashing_events: 0,
            registration_time: now,
            commission_rate: 0.05, // 5% default commission
        }
    }

    /// Update validator performance
    pub fn update_performance(&mut self, blocks_produced: u64, blocks_validated: u64) {
        self.total_blocks_produced += blocks_produced;
        self.total_blocks_validated += blocks_validated;
        
        // Calculate performance score based on block production and validation
        let total_expected = self.total_blocks_produced + self.total_blocks_validated;
        if total_expected > 0 {
            self.performance_score = (self.total_blocks_produced + self.total_blocks_validated) as f64 
                / total_expected as f64;
        }
        
        self.last_activity = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    /// Update uptime percentage
    pub fn update_uptime(&mut self, uptime_percentage: f64) {
        self.uptime_percentage = uptime_percentage.max(0.0).min(100.0);
    }

    /// Check if validator meets minimum requirements
    pub fn meets_requirements(&self, min_stake: u64, min_performance: f64, min_uptime: f64) -> bool {
        self.stake_amount >= min_stake &&
        self.performance_score >= min_performance &&
        self.uptime_percentage >= min_uptime &&
        self.is_active &&
        self.slashing_events < 3 // Max 3 slashing events
    }

    /// Get validator score for selection
    pub fn get_selection_score(&self, method: &SelectionMethod) -> f64 {
        match method {
            SelectionMethod::StakeBased => self.stake_amount as f64,
            SelectionMethod::Performance => self.performance_score,
            SelectionMethod::Random => 1.0, // Equal weight for random
            SelectionMethod::Hybrid => {
                // Combine stake and performance
                let stake_score = (self.stake_amount as f64).log10();
                let performance_score = self.performance_score;
                (stake_score * 0.7) + (performance_score * 0.3)
            }
        }
    }
}

impl ValidatorManager {
    /// Create new validator manager
    pub fn new(params: ValidatorSelectionParams) -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        
        ValidatorManager {
            validators: Arc::new(RwLock::new(HashMap::new())),
            current_validator_set: Arc::new(RwLock::new(None)),
            slashing_events: Arc::new(RwLock::new(Vec::new())),
            selection_params: params,
            rng: Arc::new(RwLock::new(StdRng::seed_from_u64(seed))),
        }
    }

    /// Register a new validator
    pub async fn register_validator(
        &self,
        node_id: &str,
        stake_amount: u64,
        public_key: &str,
        address: &str,
    ) -> Result<(), String> {
        if stake_amount < self.selection_params.min_stake {
            return Err(format!(
                "Stake amount {} is below minimum required {}",
                stake_amount, self.selection_params.min_stake
            ));
        }

        let mut validators = self.validators.write().await;
        
        if validators.contains_key(node_id) {
            return Err("Validator already registered".to_string());
        }

        let validator = Validator::new(node_id, stake_amount, public_key, address);
        validators.insert(node_id.to_string(), validator);
        
        Ok(())
    }

    /// Unregister a validator
    pub async fn unregister_validator(&self, node_id: &str) -> Result<(), String> {
        let mut validators = self.validators.write().await;
        
        if let Some(validator) = validators.get_mut(node_id) {
            validator.is_active = false;
            Ok(())
        } else {
            Err("Validator not found".to_string())
        }
    }

    /// Update validator stake
    pub async fn update_stake(&self, node_id: &str, new_stake: u64) -> Result<(), String> {
        let mut validators = self.validators.write().await;
        
        if let Some(validator) = validators.get_mut(node_id) {
            if new_stake < self.selection_params.min_stake {
                return Err(format!(
                    "New stake amount {} is below minimum required {}",
                    new_stake, self.selection_params.min_stake
                ));
            }
            validator.stake_amount = new_stake;
            Ok(())
        } else {
            Err("Validator not found".to_string())
        }
    }

    /// Update validator performance
    pub async fn update_validator_performance(
        &self,
        node_id: &str,
        blocks_produced: u64,
        blocks_validated: u64,
    ) -> Result<(), String> {
        let mut validators = self.validators.write().await;
        
        if let Some(validator) = validators.get_mut(node_id) {
            validator.update_performance(blocks_produced, blocks_validated);
            Ok(())
        } else {
            Err("Validator not found".to_string())
        }
    }

    /// Select validators for a round
    pub async fn select_validators(&self, round: u64) -> ValidatorSet {
        let validators = self.validators.read().await;
        let mut rng = self.rng.write().await;
        
        // Generate seed for this round
        let seed = Self::generate_round_seed(round);
        *rng = StdRng::seed_from_u64(seed);
        
        // Filter eligible validators
        let eligible_validators: Vec<&Validator> = validators.values()
            .filter(|v| v.meets_requirements(
                self.selection_params.min_stake,
                self.selection_params.performance_threshold,
                self.selection_params.uptime_threshold
            ))
            .collect();

        let mut selected_validators = Vec::new();
        
        match self.selection_params.selection_method {
            SelectionMethod::StakeBased => {
                selected_validators = self.select_stake_based(&eligible_validators, &mut rng);
            }
            SelectionMethod::Random => {
                selected_validators = self.select_random(&eligible_validators, &mut rng);
            }
            SelectionMethod::Performance => {
                selected_validators = self.select_performance_based(&eligible_validators, &mut rng);
            }
            SelectionMethod::Hybrid => {
                selected_validators = self.select_hybrid(&eligible_validators, &mut rng);
            }
        }

        // Select primary and backup validators
        let primary_validator = if !selected_validators.is_empty() {
            selected_validators[0].node_id.clone()
        } else {
            String::new()
        };

        let backup_validators = selected_validators.iter()
            .skip(1)
            .map(|v| v.node_id.clone())
            .collect();

        let validator_set = ValidatorSet {
            round,
            validators: selected_validators.iter().map(|v| v.node_id.clone()).collect(),
            primary_validator,
            backup_validators,
            selection_timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            seed,
        };

        // Update current validator set
        let mut current_set = self.current_validator_set.write().await;
        *current_set = Some(validator_set.clone());

        validator_set
    }

    /// Stake-based selection
    fn select_stake_based<'a>(
        &self,
        eligible: &[&'a Validator],
        rng: &mut StdRng,
    ) -> Vec<&'a Validator> {
        let mut validators = eligible.to_vec();
        validators.sort_by(|a, b| b.stake_amount.cmp(&a.stake_amount));
        
        validators.truncate(self.selection_params.max_validators);
        validators
    }

    /// Random selection
    fn select_random<'a>(
        &self,
        eligible: &[&'a Validator],
        rng: &mut StdRng,
    ) -> Vec<&'a Validator> {
        let mut validators = eligible.to_vec();
        let count = validators.len().min(self.selection_params.max_validators);
        
        // Fisher-Yates shuffle
        for i in (1..validators.len()).rev() {
            let j = rng.gen_range(0..=i);
            validators.swap(i, j);
        }
        
        validators.truncate(count);
        validators
    }

    /// Performance-based selection
    fn select_performance_based<'a>(
        &self,
        eligible: &[&'a Validator],
        rng: &mut StdRng,
    ) -> Vec<&'a Validator> {
        let mut validators = eligible.to_vec();
        validators.sort_by(|a, b| b.performance_score.partial_cmp(&a.performance_score).unwrap());
        
        validators.truncate(self.selection_params.max_validators);
        validators
    }

    /// Hybrid selection (stake + performance)
    fn select_hybrid<'a>(
        &self,
        eligible: &[&'a Validator],
        rng: &mut StdRng,
    ) -> Vec<&'a Validator> {
        let mut validators = eligible.to_vec();
        validators.sort_by(|a, b| {
            let score_a = a.get_selection_score(&SelectionMethod::Hybrid);
            let score_b = b.get_selection_score(&SelectionMethod::Hybrid);
            score_b.partial_cmp(&score_a).unwrap()
        });
        
        validators.truncate(self.selection_params.max_validators);
        validators
    }

    /// Generate round seed
    fn generate_round_seed(round: u64) -> u64 {
        let mut hasher = Sha256::new();
        hasher.update(format!("round_{}", round).as_bytes());
        let result = hasher.finalize();
        
        // Convert first 8 bytes to u64
        let mut seed_bytes = [0u8; 8];
        seed_bytes.copy_from_slice(&result[..8]);
        u64::from_le_bytes(seed_bytes)
    }

    /// Get current validator set
    pub async fn get_current_validator_set(&self) -> Option<ValidatorSet> {
        let current_set = self.current_validator_set.read().await;
        current_set.clone()
    }

    /// Check if node is validator for current round
    pub async fn is_validator(&self, node_id: &str) -> bool {
        let current_set = self.current_validator_set.read().await;
        if let Some(set) = &*current_set {
            set.validators.contains(&node_id.to_string())
        } else {
            false
        }
    }

    /// Check if node is primary validator
    pub async fn is_primary_validator(&self, node_id: &str) -> bool {
        let current_set = self.current_validator_set.read().await;
        if let Some(set) = &*current_set {
            set.primary_validator == node_id
        } else {
            false
        }
    }

    /// Record slashing event
    pub async fn record_slashing_event(
        &self,
        validator_id: &str,
        event_type: SlashingType,
        evidence: &str,
        severity: SlashingSeverity,
    ) -> Result<(), String> {
        let mut validators = self.validators.write().await;
        
        if let Some(validator) = validators.get_mut(validator_id) {
            validator.slashing_events += 1;
            
            // Apply slashing based on severity
            let penalty = match severity {
                SlashingSeverity::Minor => validator.stake_amount / 100, // 1% penalty
                SlashingSeverity::Major => validator.stake_amount / 10,  // 10% penalty
                SlashingSeverity::Critical => validator.stake_amount / 2, // 50% penalty
            };
            
            validator.stake_amount = validator.stake_amount.saturating_sub(penalty);
            
            // Deactivate validator if critical or too many events
            if severity == SlashingSeverity::Critical || validator.slashing_events >= 3 {
                validator.is_active = false;
            }
        }

        // Record slashing event
        let event = SlashingEvent {
            validator_id: validator_id.to_string(),
            event_type,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            evidence: evidence.to_string(),
            severity,
            stake_penalty: 0, // Will be calculated based on severity
        };

        let mut slashing_events = self.slashing_events.write().await;
        slashing_events.push(event);

        Ok(())
    }

    /// Get validator statistics
    pub async fn get_validator_stats(&self) -> ValidatorStats {
        let validators = self.validators.read().await;
        let slashing_events = self.slashing_events.read().await;
        
        let total_validators = validators.len();
        let active_validators = validators.values().filter(|v| v.is_active).count();
        let total_stake: u64 = validators.values().map(|v| v.stake_amount).sum();
        let avg_performance: f64 = if total_validators > 0 {
            validators.values().map(|v| v.performance_score).sum::<f64>() / total_validators as f64
        } else {
            0.0
        };
        let avg_uptime: f64 = if total_validators > 0 {
            validators.values().map(|v| v.uptime_percentage).sum::<f64>() / total_validators as f64
        } else {
            0.0
        };

        ValidatorStats {
            total_validators,
            active_validators,
            total_stake,
            avg_performance,
            avg_uptime,
            total_slashing_events: slashing_events.len(),
        }
    }

    /// Get validator by ID
    pub async fn get_validator(&self, node_id: &str) -> Option<Validator> {
        let validators = self.validators.read().await;
        validators.get(node_id).cloned()
    }

    /// Get all validators
    pub async fn get_all_validators(&self) -> Vec<Validator> {
        let validators = self.validators.read().await;
        validators.values().cloned().collect()
    }

    /// Rotate validators (called periodically)
    pub async fn rotate_validators(&self, round: u64) -> ValidatorSet {
        // Check if rotation is needed
        let current_set = self.current_validator_set.read().await;
        let should_rotate = if let Some(set) = &*current_set {
            round % self.selection_params.rotation_interval == 0
        } else {
            true
        };

        if should_rotate {
            self.select_validators(round).await
        } else {
            current_set.clone().unwrap_or_else(|| {
                // Fallback if no current set
                tokio::spawn(async move {
                    self.select_validators(round).await
                });
                ValidatorSet {
                    round,
                    validators: Vec::new(),
                    primary_validator: String::new(),
                    backup_validators: Vec::new(),
                    selection_timestamp: 0,
                    seed: 0,
                }
            })
        }
    }
}

/// Validator statistics
#[derive(Debug, Clone)]
pub struct ValidatorStats {
    pub total_validators: usize,
    pub active_validators: usize,
    pub total_stake: u64,
    pub avg_performance: f64,
    pub avg_uptime: f64,
    pub total_slashing_events: usize,
}

impl Default for Validator {
    fn default() -> Self {
        Validator::new("default_node", 1000, "default_key", "default_address")
    }
} 