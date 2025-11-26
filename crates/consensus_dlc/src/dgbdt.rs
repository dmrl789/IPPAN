//! Deterministic Gradient-Boosted Decision Tree (D-GBDT) fairness engine
//!
//! This module implements a deterministic machine learning model for validator
//! selection and reputation scoring using integer-only arithmetic.

use crate::error::{DlcError, Result};
use ippan_ai_core::fairness::{DeterministicFairnessModel, ValidatorFeatureVector};
use ippan_ai_core::gbdt::{Model as DgbdtModel, SCALE as DGBDT_SCALE};
use ippan_ai_registry::d_gbdt::DGBDTRegistry;
use ippan_types::currency::denominations;
use ippan_types::Amount;
use serde::{Deserialize, Serialize};
use sled;
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};

const REGISTRY_ENV_KEY: &str = "IPPAN_DGBDT_REGISTRY_PATH";
const DEFAULT_REGISTRY_PATH: &str = "data/dgbdt_registry";
const DLC_CONFIG_ENV_KEY: &str = "IPPAN_DLC_CONFIG_PATH";
const DEFAULT_DLC_CONFIG_PATH: &str = "config/dlc.toml";

/// Validator metrics used for fairness scoring (deterministic, scaled integers)
/// All percentage/ratio fields are scaled by 10000 (e.g., 10000 = 100%)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ValidatorMetrics {
    /// Uptime percentage scaled (0-10000 = 0%-100%)
    pub uptime: i64,
    /// Average latency scaled (scaled by 10000)
    pub latency: i64,
    /// Honesty score scaled (0-10000 = 0%-100%)
    pub honesty: i64,
    /// Number of blocks proposed
    pub blocks_proposed: u64,
    /// Number of blocks verified
    pub blocks_verified: u64,
    /// Stake amount
    pub stake: Amount,
    /// Time active in rounds
    pub rounds_active: u64,
}

impl Default for ValidatorMetrics {
    fn default() -> Self {
        Self {
            uptime: 10000, // 100%
            latency: 0,
            honesty: 10000, // 100%
            blocks_proposed: 0,
            blocks_verified: 0,
            stake: Amount::zero(),
            rounds_active: 0,
        }
    }
}

impl ValidatorMetrics {
    /// Create new validator metrics (scaled integers)
    pub fn new(
        uptime: i64,
        latency: i64,
        honesty: i64,
        blocks_proposed: u64,
        blocks_verified: u64,
        stake: Amount,
        rounds_active: u64,
    ) -> Self {
        Self {
            uptime,
            latency,
            honesty,
            blocks_proposed,
            blocks_verified,
            stake,
            rounds_active,
        }
    }

    // from_floats() removed - use new() with scaled i64 values directly

    /// Update metrics with new data (scaled integer inputs)
    pub fn update(&mut self, uptime_delta: i64, latency_sample: i64, proposed: u64, verified: u64) {
        // Exponential moving average for uptime (integer math)
        self.uptime = (9000 * self.uptime + 1000 * uptime_delta) / 10000;

        // Exponential moving average for latency (integer math)
        self.latency = (9000 * self.latency + 1000 * latency_sample) / 10000;

        self.blocks_proposed += proposed;
        self.blocks_verified += verified;
        self.rounds_active += 1;
    }

    /// Normalize metrics to 0-10000 range (integer arithmetic)
    pub fn to_normalized(&self) -> NormalizedMetrics {
        NormalizedMetrics {
            uptime: self.uptime,                                   // Already scaled
            latency_inv: (10000 - self.latency.min(10000)).max(0), // Invert latency
            honesty: self.honesty,                                 // Already scaled
            proposal_rate: if self.rounds_active > 0 {
                (self.blocks_proposed as i64 * 10000) / self.rounds_active as i64
            } else {
                0
            },
            verification_rate: if self.rounds_active > 0 {
                (self.blocks_verified as i64 * 10000) / self.rounds_active as i64
            } else {
                0
            },
            stake_weight: {
                let stake_micro = self.stake.atomic() / denominations::MICRO_IPN;
                (stake_micro / 1_000_000u128).min(10_000u128) as i64
            },
        }
    }
}

/// Normalized metrics for integer arithmetic
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NormalizedMetrics {
    pub uptime: i64,
    pub latency_inv: i64,
    pub honesty: i64,
    pub proposal_rate: i64,
    pub verification_rate: i64,
    pub stake_weight: i64,
}

impl From<NormalizedMetrics> for ValidatorFeatureVector {
    fn from(metrics: NormalizedMetrics) -> Self {
        ValidatorFeatureVector::new(
            metrics.uptime,
            metrics.latency_inv,
            metrics.honesty,
            metrics.proposal_rate,
            metrics.verification_rate,
            metrics.stake_weight,
        )
        .clamped()
    }
}

/// Fairness model backed by deterministic GBDT inference.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FairnessModel {
    #[serde(rename = "model")]
    model: DeterministicFairnessModel,
    #[serde(skip)]
    active_hash: Option<String>,
}

impl FairnessModel {
    /// Load a D-GBDT model from a file path
    pub fn from_d_gbdt_file(path: &Path) -> Result<Self> {
        use ippan_ai_registry::d_gbdt::load_model_from_path;

        let (model, _hash) = load_model_from_path(path)
            .map_err(|e| DlcError::Model(format!("Failed to load D-GBDT model: {}", e)))?;
        Ok(Self::from_d_gbdt_model(model))
    }

    /// Create a FairnessModel from a loaded D-GBDT model
    pub fn from_d_gbdt_model(model: DgbdtModel) -> Self {
        Self {
            model: DeterministicFairnessModel::from_model(model),
            active_hash: None,
        }
    }

    fn with_hash(model: DgbdtModel, hash: Option<String>) -> Self {
        Self {
            model: DeterministicFairnessModel::from_model(model),
            active_hash: hash,
        }
    }

    /// Build a fairness model from the active registry entry.
    pub fn from_registry(registry: &mut DGBDTRegistry) -> Result<(Self, String)> {
        let (model, hash) = registry
            .get_active_model()
            .map_err(|e| DlcError::Model(format!("Failed to read active D-GBDT model: {e}")))?;
        let fairness = FairnessModel::with_hash(model, Some(hash.clone()));
        Ok((fairness, hash))
    }

    /// Build a fairness model, activating the configured model if none is stored yet.
    pub fn from_registry_with_config(
        registry: &mut DGBDTRegistry,
        config_path: &Path,
    ) -> Result<(Self, String)> {
        let (model, hash) = registry
            .ensure_active_model_from_config(config_path)
            .map_err(|e| DlcError::Model(format!("Failed to load D-GBDT model: {e}")))?;
        let fairness = FairnessModel::with_hash(model, Some(hash.clone()));
        Ok((fairness, hash))
    }

    /// Load a fairness model from a registry database located at `path`.
    pub fn from_registry_path<P: AsRef<Path>>(path: P) -> Result<(Self, String)> {
        let db_path = path.as_ref();
        let db = sled::open(db_path).map_err(|e| {
            DlcError::Model(format!(
                "Failed to open D-GBDT registry at {}: {}",
                db_path.display(),
                e
            ))
        })?;
        let mut registry = DGBDTRegistry::new(db);
        Self::from_registry(&mut registry)
    }

    /// Load a fairness model from a registry database, activating from config if needed.
    pub fn from_registry_path_with_config<P: AsRef<Path>, C: AsRef<Path>>(
        path: P,
        config_path: C,
    ) -> Result<(Self, String)> {
        let db_path = path.as_ref();
        let db = sled::open(db_path).map_err(|e| {
            DlcError::Model(format!(
                "Failed to open D-GBDT registry at {}: {}",
                db_path.display(),
                e
            ))
        })?;
        let mut registry = DGBDTRegistry::new(db);
        Self::from_registry_with_config(&mut registry, config_path.as_ref())
    }

    /// Load a fairness model using the configured registry environment variable.
    pub fn load_from_env_registry() -> Result<(Self, String)> {
        let registry_path =
            env::var(REGISTRY_ENV_KEY).unwrap_or_else(|_| DEFAULT_REGISTRY_PATH.to_string());
        let config_path =
            env::var(DLC_CONFIG_ENV_KEY).unwrap_or_else(|_| DEFAULT_DLC_CONFIG_PATH.to_string());
        Self::from_registry_path_with_config(
            PathBuf::from(registry_path),
            PathBuf::from(config_path),
        )
    }

    /// Deterministic integer-based scoring via D-GBDT model.
    pub fn score_deterministic(&self, metrics: &ValidatorMetrics) -> i64 {
        let normalized = ValidatorFeatureVector::from(metrics.to_normalized());
        self.model.score(&normalized)
    }

    /// Validate model integrity
    pub fn validate(&self) -> Result<()> {
        self.model
            .model()
            .validate()
            .map_err(|e| DlcError::Model(format!("Invalid D-GBDT model: {e}")))
    }

    /// Get model metadata
    pub fn metadata(&self) -> ModelMetadata {
        let model = self.model.model();
        ModelMetadata {
            num_trees: model.trees.len(),
            num_features: 6,
            scale: model.scale,
            bias: model.bias,
        }
    }

    /// Access the underlying deterministic model (for direct scoring with custom features)
    pub fn raw_model(&self) -> &DgbdtModel {
        self.model.model()
    }

    /// Access the underlying model hash when loaded from a registry.
    pub fn active_hash(&self) -> Option<&str> {
        self.active_hash.as_deref()
    }

    /// Optional human-readable model version string.
    pub fn model_version(&self) -> Option<String> {
        Some(self.model.model().version.to_string())
    }

    pub fn testing_stub() -> Self {
        use ippan_ai_core::gbdt::{Node as TestNode, Tree as TestTree};

        let tree = TestTree::new(
            vec![
                TestNode::internal(0, 0, 50 * DGBDT_SCALE, 1, 2),
                TestNode::leaf(1, 100 * DGBDT_SCALE),
                TestNode::leaf(2, 200 * DGBDT_SCALE),
            ],
            DGBDT_SCALE,
        );
        Self::from_d_gbdt_model(DgbdtModel::new(vec![tree], 0))
    }
}

impl Default for FairnessModel {
    fn default() -> Self {
        Self::testing_stub()
    }
}

/// Model metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    pub num_trees: usize,
    pub num_features: usize,
    pub scale: i64,
    pub bias: i64,
}

/// Validator ranking result (deterministic integer scoring)
#[derive(Debug, Clone)]
pub struct ValidatorRanking {
    pub validator_id: String,
    pub score: i64, // Scaled integer score
    pub rank: usize,
}

/// Rank multiple validators using the fairness model (deterministic integer scoring)
pub fn rank_validators(
    model: &FairnessModel,
    validators: HashMap<String, ValidatorMetrics>,
) -> Vec<ValidatorRanking> {
    let mut rankings: Vec<(String, i64)> = validators
        .into_iter()
        .map(|(id, metrics)| (id, model.score_deterministic(&metrics)))
        .collect();

    // Sort by score (descending), then by ID for deterministic tie-breaking
    rankings.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    rankings
        .into_iter()
        .enumerate()
        .map(|(rank, (validator_id, score))| ValidatorRanking {
            validator_id,
            score,
            rank: rank + 1,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ippan_ai_registry::d_gbdt::compute_model_hash;
    use ippan_types::Amount;
    use tempfile::TempDir;

    #[test]
    fn test_validator_metrics() {
        let metrics = ValidatorMetrics::default();
        assert_eq!(metrics.uptime, 10000); // 100% scaled
        assert_eq!(metrics.honesty, 10000); // 100% scaled
    }

    #[test]
    fn test_metrics_normalization() {
        let metrics = ValidatorMetrics::new(
            9500,  // 0.95 * 10000
            1000,  // 0.1 * 10000
            10000, // 1.0 * 10000
            100,
            500,
            Amount::from_micro_ipn(10_000_000),
            1000,
        );
        let normalized = metrics.to_normalized();

        assert_eq!(normalized.uptime, 9500);
        assert!(normalized.latency_inv > 8000);
    }

    #[test]
    fn test_fairness_model_scoring() {
        let model = FairnessModel::testing_stub();
        let metrics = ValidatorMetrics::default();

        let score = model.score_deterministic(&metrics);
        assert!((0..=10000).contains(&score)); // Score is scaled 0-10000
    }

    #[test]
    fn test_validator_ranking() {
        let model = FairnessModel::testing_stub();
        let mut validators = HashMap::new();

        validators.insert(
            "val1".to_string(),
            ValidatorMetrics::new(
                9900,  // 0.99 * 10000
                500,   // 0.05 * 10000
                10000, // 1.0 * 10000
                100,
                500,
                Amount::from_micro_ipn(10_000_000),
                100,
            ),
        );
        validators.insert(
            "val2".to_string(),
            ValidatorMetrics::new(
                9500, // 0.95 * 10000
                1500, // 0.15 * 10000
                9800, // 0.98 * 10000
                80,
                400,
                Amount::from_micro_ipn(5_000_000),
                100,
            ),
        );

        let rankings = rank_validators(&model, validators);
        assert_eq!(rankings.len(), 2);
        assert_eq!(rankings[0].rank, 1);
    }

    #[test]
    fn test_deterministic_scoring() {
        let model = FairnessModel::testing_stub();
        let metrics = ValidatorMetrics::new(
            9900,  // 0.99 * 10000
            1000,  // 0.1 * 10000
            10000, // 1.0 * 10000
            100,
            500,
            Amount::from_micro_ipn(10_000_000),
            100,
        );

        // Score should be deterministic
        let score1 = model.score_deterministic(&metrics);
        let score2 = model.score_deterministic(&metrics);

        assert_eq!(score1, score2);
    }

    #[test]
    fn test_load_from_registry_path() {
        use ippan_ai_core::gbdt::{Node as DNode, Tree as DTree, SCALE};

        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("registry");
        let db = sled::open(&db_path).unwrap();
        let mut registry = DGBDTRegistry::new(db);

        let tree = DTree::new(
            vec![
                DNode::internal(0, 0, 50 * SCALE, 1, 2),
                DNode::leaf(1, 100 * SCALE),
                DNode::leaf(2, 200 * SCALE),
            ],
            SCALE,
        );
        let model = ippan_ai_core::gbdt::Model::new(vec![tree], 0);
        let hash = compute_model_hash(&model).unwrap();
        registry.store_active_model(model, hash.clone()).unwrap();
        drop(registry);

        let (fairness, loaded_hash) = FairnessModel::from_registry_path(&db_path).unwrap();
        assert_eq!(loaded_hash, hash);
        assert_eq!(fairness.active_hash(), Some(loaded_hash.as_str()));
    }

    #[test]
    fn test_fairness_model_from_registry_scores_consistently() {
        use ippan_ai_core::gbdt::{Node as DNode, Tree as DTree, SCALE};

        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("registry");
        let db = sled::open(&db_path).unwrap();
        let mut registry = DGBDTRegistry::new(db);

        let tree = DTree::new(
            vec![
                DNode::internal(0, 0, 50 * SCALE, 1, 2),
                DNode::leaf(1, 100 * SCALE),
                DNode::leaf(2, 200 * SCALE),
            ],
            SCALE,
        );
        let model = ippan_ai_core::gbdt::Model::new(vec![tree], 0);
        let hash = compute_model_hash(&model).unwrap();
        registry.store_active_model(model, hash.clone()).unwrap();
        drop(registry);

        let (fairness, loaded_hash) = FairnessModel::from_registry_path(&db_path).unwrap();
        assert_eq!(loaded_hash, hash);
        let metrics = ValidatorMetrics::new(
            9800,
            1500,
            9700,
            50,
            75,
            Amount::from_micro_ipn(5_000_000),
            500,
        );
        let score_a = fairness.score_deterministic(&metrics);
        let score_b = fairness.score_deterministic(&metrics);
        assert_eq!(score_a, score_b);
        assert!((0..=10_000).contains(&score_a));
    }

    #[test]
    fn test_registry_activation_from_config_and_scoring() {
        use ippan_ai_core::gbdt::{Node as DNode, Tree as DTree, SCALE};

        let temp = TempDir::new().unwrap();
        let root = temp.path();

        let config_dir = root.join("config");
        let models_dir = root.join("models");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::create_dir_all(&models_dir).unwrap();

        let tree = DTree::new(
            vec![
                DNode::internal(0, 0, 50 * SCALE, 1, 2),
                DNode::leaf(1, 500_000),
                DNode::leaf(2, 900_000),
            ],
            SCALE,
        );
        let model = ippan_ai_core::gbdt::Model::new(vec![tree], 0);
        let model_path = models_dir.join("model.json");
        std::fs::write(&model_path, serde_json::to_string(&model).unwrap()).unwrap();
        let hash = compute_model_hash(&model).unwrap();

        let config_path = config_dir.join("dlc.toml");
        let config_contents = format!(
            r#"
[dgbdt]
  [dgbdt.model]
  path = "models/model.json"
  expected_hash = "{hash}"
"#
        );
        std::fs::write(&config_path, config_contents).unwrap();

        let db_path = root.join("registry");
        let db = sled::open(&db_path).unwrap();
        let mut registry = DGBDTRegistry::new(db);

        let (fairness, loaded_hash) =
            FairnessModel::from_registry_with_config(&mut registry, &config_path).unwrap();
        assert_eq!(loaded_hash, hash);

        let metrics = ValidatorMetrics::default();
        let score = fairness.score_deterministic(&metrics);
        assert_eq!(score, 5_000);
    }
}
