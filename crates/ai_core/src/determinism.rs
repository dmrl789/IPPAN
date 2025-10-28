//! Deterministic execution utilities
//!
//! Ensures reproducible, verifiable GBDT inference across nodes by
//! enforcing seed control, input hashing, and deterministic context state.

use crate::{errors::Result, types::*};
use std::collections::HashMap;
use tracing::{info, warn};

/// Deterministic execution manager
pub struct DeterminismManager {
    /// Execution seeds per run
    seeds: HashMap<String, u64>,
    /// Deterministic state tracker
    state: DeterministicState,
}

/// Deterministic execution state
#[derive(Debug, Clone)]
pub struct DeterministicState {
    /// Current seed used
    pub seed: u64,
    /// Number of deterministic runs performed
    pub counter: u64,
    /// Running state hash (to detect drift)
    pub state_hash: String,
}

/// Deterministic execution context
#[derive(Debug, Clone)]
pub struct DeterministicContext {
    /// Unique execution identifier
    pub execution_id: String,
    /// Model reference
    pub model_id: ModelId,
    /// Hash of input data and shape
    pub input_hash: String,
    /// Deterministic seed
    pub seed: u64,
    /// Context parameters (model-specific)
    pub parameters: HashMap<String, String>,
}

impl DeterminismManager {
    /// Create new deterministic manager
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

    /// Manually set deterministic seed
    pub fn set_seed(&mut self, execution_id: &str, seed: u64) {
        info!("Setting deterministic seed for execution {}: {}", execution_id, seed);
        self.seeds.insert(execution_id.to_string(), seed);
    }

    /// Retrieve deterministic seed if available
    pub fn get_seed(&self, execution_id: &str) -> Option<u64> {
        self.seeds.get(execution_id).copied()
    }

    /// Create deterministic context from execution parameters
    pub fn create_context(
        &mut self,
        execution_id: &str,
        model_id: &ModelId,
        input: &ModelInput,
        parameters: HashMap<String, String>,
    ) -> Result<DeterministicContext> {
        info!("Creating deterministic context for execution: {}", execution_id);

        let seed = self
            .get_seed(execution_id)
            .unwrap_or_else(|| self.generate_deterministic_seed(execution_id, model_id, input));

        let input_hash = self.compute_input_hash(input)?;

        let context = DeterministicContext {
            execution_id: execution_id.to_string(),
            model_id: model_id.clone(),
            input_hash,
            seed,
            parameters,
        };

        self.update_state(&context);
        Ok(context)
    }

    /// Verify deterministic execution consistency
    pub fn verify_execution(&self, context: &DeterministicContext, output: &ModelOutput) -> Result<bool> {
        info!("Verifying deterministic execution: {}", context.execution_id);

        let expected_hash = self.compute_execution_hash(context, output)?;
        let actual_hash = output
            .metadata
            .metadata
            .get("execution_hash")
            .cloned()
            .unwrap_or_default();

        let is_deterministic = expected_hash == actual_hash;

        if !is_deterministic {
            warn!("⚠️ Non-deterministic execution detected for {}", context.execution_id);
        }

        Ok(is_deterministic)
    }

    /// Generate deterministic seed from execution identifiers
    fn generate_deterministic_seed(&self, execution_id: &str, model_id: &ModelId, input: &ModelInput) -> u64 {
        let mut hasher = blake3::Hasher::new();
        hasher.update(execution_id.as_bytes());
        hasher.update(model_id.name.as_bytes());
        hasher.update(model_id.version.as_bytes());
        hasher.update(model_id.hash.as_bytes());
        hasher.update(&input.data);
        hasher.update(
            &input
                .shape
                .iter()
                .flat_map(|x| x.to_le_bytes())
                .collect::<Vec<_>>(),
        );

        let hash = hasher.finalize();
        let bytes = hash.as_bytes();
        u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ])
    }

    /// Compute stable input hash for reproducibility
    fn compute_input_hash(&self, input: &ModelInput) -> Result<String> {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&input.data);
        hasher.update(
            &input
                .shape
                .iter()
                .flat_map(|x| x.to_le_bytes())
                .collect::<Vec<_>>(),
        );
        hasher.update(&(input.dtype as u8).to_le_bytes());
        Ok(hasher.finalize().to_hex().to_string())
    }

    /// Compute deterministic execution hash for verification
    fn compute_execution_hash(&self, context: &DeterministicContext, output: &ModelOutput) -> Result<String> {
        let mut hasher = blake3::Hasher::new();

        hasher.update(context.execution_id.as_bytes());
        hasher.update(context.input_hash.as_bytes());
        hasher.update(&context.seed.to_le_bytes());
        hasher.update(context.model_id.name.as_bytes());
        hasher.update(context.model_id.version.as_bytes());
        hasher.update(context.model_id.hash.as_bytes());

        for (k, v) in &context.parameters {
            hasher.update(k.as_bytes());
            hasher.update(v.as_bytes());
        }

        hasher.update(&output.data);
        hasher.update(
            &output
                .shape
                .iter()
                .flat_map(|x| x.to_le_bytes())
                .collect::<Vec<_>>(),
        );

        if let Some(ver) = output.metadata.metadata.get("model_version") {
            hasher.update(ver.as_bytes());
        }

        Ok(hasher.finalize().to_hex().to_string())
    }

    /// Update internal deterministic state hash
    fn update_state(&mut self, context: &DeterministicContext) {
        self.state.counter += 1;
        self.state.seed = context.seed;

        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.state.seed.to_le_bytes());
        hasher.update(&self.state.counter.to_le_bytes());
        hasher.update(context.execution_id.as_bytes());
        self.state.state_hash = hasher.finalize().to_hex().to_string();
    }

    pub fn get_state(&self) -> &DeterministicState {
        &self.state
    }

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
    state: u64,
    seed: u64,
}

impl DeterministicRng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed, seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(1103515245).wrapping_add(12345);
        self.state
    }

    pub fn next_range(&mut self, max: u64) -> u64 {
        if max == 0 {
            return 0;
        }
        self.next_u64() % max
    }

    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() as f64) / (u64::MAX as f64)
    }

    pub fn reset(&mut self) {
        self.state = self.seed;
    }

    pub fn get_state(&self) -> u64 {
        self.state
    }
}

/// Deterministic execution environment utilities
pub mod utils {
    use super::*;

    pub fn setup_deterministic_environment() -> Result<()> {
        if let Ok(seed) = std::env::var("DETERMINISTIC_SEED") {
            if let Ok(seed_val) = seed.parse::<u64>() {
                std::env::set_var("RUST_RAND_SEED", seed_val.to_string());
            }
        }
        std::env::set_var("RAYON_NUM_THREADS", "1");
        std::env::set_var("RUST_FLOAT_DETERMINISTIC", "1");
        Ok(())
    }

    pub fn is_deterministic_execution() -> bool {
        std::env::var("DETERMINISTIC_EXECUTION").unwrap_or_default() == "1"
    }

    pub fn get_deterministic_seed() -> Option<u64> {
        std::env::var("DETERMINISTIC_SEED")
            .ok()
            .and_then(|s| s.parse().ok())
    }
}
