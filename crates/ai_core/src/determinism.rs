//! Deterministic execution utilities

use crate::{
    errors::{AiCoreError, Result},
    types::*,
};
use std::collections::HashMap;
use tracing::{info, warn, error};

/// Deterministic execution manager
pub struct DeterminismManager {
    /// Execution seeds
    seeds: HashMap<String, u64>,
    /// Deterministic state
    state: DeterministicState,
}

/// Deterministic execution state
#[derive(Debug, Clone)]
pub struct DeterministicState {
    /// Current seed
    pub seed: u64,
    /// Execution counter
    pub counter: u64,
    /// State hash
    pub state_hash: String,
}

/// Deterministic execution context
#[derive(Debug, Clone)]
pub struct DeterministicContext {
    /// Execution ID
    pub execution_id: String,
    /// Model ID
    pub model_id: ModelId,
    /// Input hash
    pub input_hash: String,
    /// Seed
    pub seed: u64,
    /// Parameters
    pub parameters: HashMap<String, String>,
}

impl DeterminismManager {
    /// Create a new determinism manager
    pub fn new() -> Self {
        Self {
            seeds: HashMap::new(),
            state: DeterministicState {
                seed: 0,
                counter: 0,
                state_hash: String::new(),
            },
        }
    }

    /// Set deterministic seed for execution
    pub fn set_seed(&mut self, execution_id: &str, seed: u64) {
        info!("Setting deterministic seed for execution {}: {}", execution_id, seed);
        self.seeds.insert(execution_id.to_string(), seed);
    }

    /// Get deterministic seed for execution
    pub fn get_seed(&self, execution_id: &str) -> Option<u64> {
        self.seeds.get(execution_id).copied()
    }

    /// Create deterministic context for execution
    pub fn create_context(
        &mut self,
        execution_id: &str,
        model_id: &ModelId,
        input: &ModelInput,
        parameters: HashMap<String, String>,
    ) -> Result<DeterministicContext> {
        info!("Creating deterministic context for execution: {}", execution_id);
        
        // Get or generate seed
        let seed = self.get_seed(execution_id)
            .unwrap_or_else(|| self.generate_deterministic_seed(execution_id, model_id, input));
        
        // Compute input hash
        let input_hash = self.compute_input_hash(input)?;
        
        // Create context
        let context = DeterministicContext {
            execution_id: execution_id.to_string(),
            model_id: model_id.clone(),
            input_hash,
            seed,
            parameters,
        };
        
        // Update state
        self.update_state(&context);
        
        Ok(context)
    }

    /// Verify deterministic execution
    pub fn verify_execution(
        &self,
        context: &DeterministicContext,
        output: &ModelOutput,
    ) -> Result<bool> {
        info!("Verifying deterministic execution: {}", context.execution_id);
        
        // Compute expected execution hash
        let expected_hash = self.compute_execution_hash(context, output)?;
        
        // Compare with actual execution hash
        let is_deterministic = expected_hash == *output.metadata.get("execution_hash").unwrap_or(&String::new());
        
        if !is_deterministic {
            warn!("Non-deterministic execution detected for: {}", context.execution_id);
        }
        
        Ok(is_deterministic)
    }

    /// Generate deterministic seed
    fn generate_deterministic_seed(
        &self,
        execution_id: &str,
        model_id: &ModelId,
        input: &ModelInput,
    ) -> u64 {
        let mut hasher = blake3::Hasher::new();
        
        // Hash execution ID
        hasher.update(execution_id.as_bytes());
        
        // Hash model ID
        hasher.update(model_id.name.as_bytes());
        hasher.update(model_id.version.as_bytes());
        hasher.update(model_id.hash.as_bytes());
        
        // Hash input data
        hasher.update(&input.data);
        hasher.update(&input.shape.iter().map(|x| x.to_le_bytes()).flatten().collect::<Vec<_>>());
        
        // Convert hash to u64 seed
        let hash = hasher.finalize();
        let bytes = hash.as_bytes();
        u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
            bytes[4], bytes[5], bytes[6], bytes[7],
        ])
    }

    /// Compute input hash
    fn compute_input_hash(&self, input: &ModelInput) -> Result<String> {
        let mut hasher = blake3::Hasher::new();
        
        // Hash input data
        hasher.update(&input.data);
        
        // Hash input shape
        hasher.update(&input.shape.iter().map(|x| x.to_le_bytes()).flatten().collect::<Vec<_>>());
        
        // Hash data type
        hasher.update(&(input.dtype as u8).to_le_bytes());
        
        Ok(hasher.finalize().to_hex().to_string())
    }

    /// Compute execution hash
    fn compute_execution_hash(
        &self,
        context: &DeterministicContext,
        output: &ModelOutput,
    ) -> Result<String> {
        let mut hasher = blake3::Hasher::new();
        
        // Hash execution context
        hasher.update(context.execution_id.as_bytes());
        hasher.update(context.input_hash.as_bytes());
        hasher.update(&context.seed.to_le_bytes());
        
        // Hash model ID
        hasher.update(context.model_id.name.as_bytes());
        hasher.update(context.model_id.version.as_bytes());
        hasher.update(context.model_id.hash.as_bytes());
        
        // Hash parameters
        for (key, value) in &context.parameters {
            hasher.update(key.as_bytes());
            hasher.update(value.as_bytes());
        }
        
        // Hash output data
        hasher.update(&output.data);
        hasher.update(&output.shape.iter().map(|x| x.to_le_bytes()).flatten().collect::<Vec<_>>());
        
        // Hash execution metadata
        hasher.update(context.execution_id.as_bytes());
        hasher.update(output.metadata.get("model_version").unwrap_or(&String::new()).as_bytes());
        
        Ok(hasher.finalize().to_hex().to_string())
    }

    /// Update deterministic state
    fn update_state(&mut self, context: &DeterministicContext) {
        self.state.counter += 1;
        self.state.seed = context.seed;
        
        // Update state hash
        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.state.seed.to_le_bytes());
        hasher.update(&self.state.counter.to_le_bytes());
        hasher.update(context.execution_id.as_bytes());
        self.state.state_hash = hasher.finalize().to_hex().to_string();
    }

    /// Get current state
    pub fn get_state(&self) -> &DeterministicState {
        &self.state
    }

    /// Reset state
    pub fn reset_state(&mut self) {
        self.seeds.clear();
        self.state = DeterministicState {
            seed: 0,
            counter: 0,
            state_hash: String::new(),
        };
    }
}

/// Deterministic random number generator
pub struct DeterministicRng {
    /// Current state
    state: u64,
    /// Seed
    seed: u64,
}

impl DeterministicRng {
    /// Create a new deterministic RNG
    pub fn new(seed: u64) -> Self {
        Self {
            state: seed,
            seed,
        }
    }

    /// Generate next random number
    pub fn next_u64(&mut self) -> u64 {
        // Linear congruential generator for deterministic behavior
        self.state = self.state.wrapping_mul(1103515245).wrapping_add(12345);
        self.state
    }

    /// Generate next random number in range [0, max)
    pub fn next_range(&mut self, max: u64) -> u64 {
        if max == 0 {
            return 0;
        }
        self.next_u64() % max
    }

    /// Generate next random f64 in range [0.0, 1.0)
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() as f64) / (u64::MAX as f64)
    }

    /// Reset RNG to initial seed
    pub fn reset(&mut self) {
        self.state = self.seed;
    }

    /// Get current state
    pub fn get_state(&self) -> u64 {
        self.state
    }
}

/// Deterministic execution utilities
pub mod utils {
    use super::*;
    
    /// Ensure deterministic execution environment
    pub fn setup_deterministic_environment() -> Result<()> {
        // Set deterministic random seed
        if let Ok(seed) = std::env::var("DETERMINISTIC_SEED") {
            if let Ok(seed_val) = seed.parse::<u64>() {
                std::env::set_var("RUST_RAND_SEED", seed_val.to_string());
            }
        }
        
        // Disable parallel execution for deterministic behavior
        std::env::set_var("RAYON_NUM_THREADS", "1");
        
        // Set deterministic floating point behavior
        std::env::set_var("RUST_FLOAT_DETERMINISTIC", "1");
        
        Ok(())
    }
    
    /// Check if execution is deterministic
    pub fn is_deterministic_execution() -> bool {
        std::env::var("DETERMINISTIC_EXECUTION").unwrap_or_default() == "1"
    }
    
    /// Get deterministic seed from environment
    pub fn get_deterministic_seed() -> Option<u64> {
        std::env::var("DETERMINISTIC_SEED")
            .ok()
            .and_then(|s| s.parse().ok())
    }
}