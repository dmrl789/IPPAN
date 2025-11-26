#![allow(deprecated)]

use anyhow::Result;
use ippan_ai_core::fairness::DeterministicFairnessModel;
use ippan_ai_core::gbdt::{Node as DNode, Tree as DTree, SCALE};
use ippan_ai_registry::d_gbdt::{compute_model_hash, DGBDTRegistry};
use ippan_consensus_dlc::{bond, dgbdt::FairnessModel, DlcConfig, DlcConsensus};
use ippan_consensus_dlc::{dgbdt::ValidatorMetrics, verifier::VerifierSet};
use ippan_types::Amount;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

const HIGH_A: &str = "validator-high-a";
const HIGH_B: &str = "validator-high-b";
const MEDIUM: &str = "validator-medium";
const LOW: &str = "validator-low";
const ADVERSARIAL: &str = "validator-adversarial";

#[test]
fn long_run_fairness_roles_remain_balanced() -> Result<()> {
    let temp = TempDir::new()?;
    let root = temp.path();
    let registry_path = root.join("registry");
    let config_path = root.join("dlc.toml");
    write_registry_and_config(root, &registry_path, &config_path)?;

    let _registry_guard = EnvGuard::new("IPPAN_DGBDT_REGISTRY_PATH", &registry_path);
    let _config_guard = EnvGuard::new("IPPAN_DLC_CONFIG_PATH", &config_path);

    let mut consensus = DlcConsensus::new(DlcConfig {
        validators_per_round: 5,
        min_validator_stake: bond::MIN_VALIDATOR_BOND,
        unstaking_lock_rounds: 8,
        min_reputation: 4_000,
        enable_slashing: false,
    });

    let profiles = validator_profiles();
    for profile in &profiles {
        consensus
            .register_validator(
                profile.id.clone(),
                profile.stake,
                profile.initial_metrics.clone(),
            )
            .expect("validator registration should succeed");
    }

    let mut tally = RoleTally::default();
    for round in 1..=240 {
        apply_round_updates(&mut consensus, round, &profiles);

        let seed = format!("fairness-seed-{round:04}");
        let verifier_set = consensus
            .validators
            .select_for_round(seed, round)
            .expect("verifier selection should succeed");

        tally.record(verifier_set);
        if verifier_set.primary == ADVERSARIAL {
            assert!(
                !verifier_set.shadows.is_empty(),
                "adversarial primaries must be shadowed for safety",
            );
        }
    }

    let fairness_scores = reload_scores(&registry_path, consensus.validators.all_validators())?;

    let score_high_a = *fairness_scores.get(HIGH_A).expect("score for high A");
    let score_high_b = *fairness_scores.get(HIGH_B).expect("score for high B");
    let score_medium = *fairness_scores.get(MEDIUM).expect("score for medium");
    let score_low = *fairness_scores.get(LOW).expect("score for low");
    let score_adversarial = *fairness_scores
        .get(ADVERSARIAL)
        .expect("score for adversarial");

    let high_a_primary = tally.primary(HIGH_A);
    let high_b_primary = tally.primary(HIGH_B);
    let medium_primary = tally.primary(MEDIUM);
    let low_primary = tally.primary(LOW);
    let adversarial_primary = tally.primary(ADVERSARIAL);

    let high_average = (high_a_primary + high_b_primary) / 2;
    let high_diff = high_a_primary.abs_diff(high_b_primary);
    eprintln!(
        "primary counts -> high_a: {high_a_primary}, high_b: {high_b_primary}, medium: {medium_primary}, low: {low_primary}, adversarial: {adversarial_primary}",
    );
    eprintln!(
        "shadow coverage -> high_a: {}, high_b: {}, medium: {}, low: {}",
        tally.shadow(HIGH_A),
        tally.shadow(HIGH_B),
        tally.shadow(MEDIUM),
        tally.shadow(LOW)
    );

    eprintln!(
        "fairness scores -> high_a: {score_high_a}, high_b: {score_high_b}, medium: {score_medium}, low: {score_low}, adversarial: {score_adversarial}"
    );

    assert!(
        high_diff <= high_average / 5 + 1,
        "high contributors should remain balanced: a={}, b={}, diff={}",
        high_a_primary,
        high_b_primary,
        high_diff
    );

    assert!(
        high_a_primary > medium_primary && high_b_primary > medium_primary,
        "high contributors should outrank medium contributors"
    );
    assert!(
        medium_primary > low_primary,
        "medium contributors should outrank low contributors"
    );
    assert!(
        adversarial_primary * 3 < high_average,
        "adversarial primaries must stay rare"
    );

    assert!(
        tally.shadow(HIGH_A) > 0
            && tally.shadow(HIGH_B) > 0
            && tally.shadow(MEDIUM) > 0
            && tally.shadow(LOW) > 0,
        "all honest validators should receive shadow coverage"
    );

    assert!(
        score_high_a >= score_high_b
            && score_high_b >= score_medium
            && score_medium > score_low
            && score_low >= score_adversarial,
        "final fairness scores should reflect contribution ordering"
    );

    Ok(())
}

fn reload_scores(
    registry_path: &Path,
    validators: &HashMap<String, ValidatorMetrics>,
) -> Result<HashMap<String, i64>> {
    let (model, _) = FairnessModel::from_registry_path(registry_path)?;
    let mut scores = HashMap::new();
    for (id, metrics) in validators {
        scores.insert(id.clone(), model.score_deterministic(metrics));
    }
    Ok(scores)
}

fn apply_round_updates(consensus: &mut DlcConsensus, round: u64, profiles: &[ValidatorProfile]) {
    for profile in profiles {
        let sample = profile.behavior.sample(round);
        if let Some(existing) = consensus.validators.get_validator(&profile.id).cloned() {
            let mut updated = existing.clone();
            updated.update(
                sample.uptime,
                sample.latency,
                sample.proposed,
                sample.verified,
            );
            consensus
                .validators
                .update_validator(&profile.id, updated)
                .expect("validator update should succeed");
        }
    }
}

#[derive(Clone)]
struct ContributionSample {
    uptime: i64,
    latency: i64,
    proposed: u64,
    verified: u64,
}

#[derive(Clone)]
struct ValidatorProfile {
    id: String,
    stake: Amount,
    initial_metrics: ValidatorMetrics,
    behavior: ContributionPattern,
}

#[derive(Clone)]
enum ContributionPattern {
    High,
    Medium,
    Low,
    Adversarial,
}

impl ContributionPattern {
    fn sample(&self, round: u64) -> ContributionSample {
        match self {
            ContributionPattern::High => ContributionSample {
                uptime: 9_850,
                latency: 450,
                proposed: 1,
                verified: 2,
            },
            ContributionPattern::Medium => ContributionSample {
                uptime: 9_500,
                latency: 650,
                proposed: if round.is_multiple_of(2) { 1 } else { 0 },
                verified: 1,
            },
            ContributionPattern::Low => ContributionSample {
                uptime: 9_050,
                latency: 900,
                proposed: if round.is_multiple_of(4) { 1 } else { 0 },
                verified: if round.is_multiple_of(3) { 1 } else { 0 },
            },
            ContributionPattern::Adversarial => ContributionSample {
                uptime: 8_200,
                latency: 1_800,
                proposed: if round.is_multiple_of(5) { 1 } else { 0 },
                verified: if round.is_multiple_of(6) { 1 } else { 0 },
            },
        }
    }
}

#[derive(Default)]
struct RoleTally {
    primary: HashMap<String, u64>,
    shadow: HashMap<String, u64>,
}

impl RoleTally {
    fn record(&mut self, set: &VerifierSet) {
        *self.primary.entry(set.primary.clone()).or_insert(0) += 1;
        for shadow in &set.shadows {
            *self.shadow.entry(shadow.clone()).or_insert(0) += 1;
        }
    }

    fn primary(&self, id: &str) -> u64 {
        *self.primary.get(id).unwrap_or(&0)
    }

    fn shadow(&self, id: &str) -> u64 {
        *self.shadow.get(id).unwrap_or(&0)
    }
}

fn validator_profiles() -> Vec<ValidatorProfile> {
    vec![
        ValidatorProfile {
            id: HIGH_A.to_string(),
            stake: Amount::from_micro_ipn(40_000_000),
            initial_metrics: ValidatorMetrics::new(
                9_900,
                400,
                9_600,
                2,
                3,
                Amount::from_micro_ipn(40_000_000),
                1,
            ),
            behavior: ContributionPattern::High,
        },
        ValidatorProfile {
            id: HIGH_B.to_string(),
            stake: Amount::from_micro_ipn(40_000_000),
            initial_metrics: ValidatorMetrics::new(
                9_900,
                400,
                9_600,
                2,
                3,
                Amount::from_micro_ipn(40_000_000),
                1,
            ),
            behavior: ContributionPattern::High,
        },
        ValidatorProfile {
            id: MEDIUM.to_string(),
            stake: Amount::from_micro_ipn(32_000_000),
            initial_metrics: ValidatorMetrics::new(
                9_600,
                600,
                9_300,
                1,
                2,
                Amount::from_micro_ipn(32_000_000),
                1,
            ),
            behavior: ContributionPattern::Medium,
        },
        ValidatorProfile {
            id: LOW.to_string(),
            stake: Amount::from_micro_ipn(25_000_000),
            initial_metrics: ValidatorMetrics::new(
                9_100,
                850,
                7_800,
                1,
                1,
                Amount::from_micro_ipn(25_000_000),
                1,
            ),
            behavior: ContributionPattern::Low,
        },
        ValidatorProfile {
            id: ADVERSARIAL.to_string(),
            stake: Amount::from_micro_ipn(21_000_000),
            initial_metrics: ValidatorMetrics::new(
                8_600,
                1_200,
                6_200,
                0,
                0,
                Amount::from_micro_ipn(21_000_000),
                1,
            ),
            behavior: ContributionPattern::Adversarial,
        },
    ]
}

fn write_registry_and_config(root: &Path, registry_path: &Path, config_path: &Path) -> Result<()> {
    let db = sled::open(registry_path)?;
    let mut registry = DGBDTRegistry::new(db);

    let model = build_fairness_model();
    let hash = compute_model_hash(&model)?;

    let model_path = root.join("dlc_fairness_model.json");
    fs::write(&model_path, serde_json::to_string(&model)?)?;

    let config_contents = format!(
        r#"[dgbdt]
[dgbdt.model]
path = "{}"
expected_hash = "{}"
"#,
        model_path.to_string_lossy().replace('\\', "/"),
        hash
    );
    fs::write(config_path, config_contents)?;

    registry.store_active_model(model, hash.clone())?;
    Ok(())
}

fn build_fairness_model() -> ippan_ai_core::gbdt::Model {
    let feature_scale = DeterministicFairnessModel::FEATURE_SCALE;

    // Weight proposal productivity most heavily to keep primary roles balanced
    let proposal_tree = DTree::new(
        vec![
            // Feature 3 = proposal rate (scaled 0-10000)
            // High-rate contributors get a higher leaf score
            DNode::internal(0, 3, 7_500 * feature_scale, 1, 2),
            DNode::leaf(1, 140_000),
            DNode::leaf(2, 360_000),
        ],
        SCALE,
    );

    // Verification rate ensures consistently validating nodes outrank sporadic ones
    let verification_tree = DTree::new(
        vec![
            // Feature 4 = verification rate (scaled 0-10000)
            DNode::internal(0, 4, 8_000 * feature_scale, 1, 2),
            DNode::leaf(1, 110_000),
            DNode::leaf(2, 300_000),
        ],
        SCALE,
    );

    // Uptime stabilizer keeps high-availability validators ahead of degraded nodes
    let uptime_tree = DTree::new(
        vec![
            // Feature 0 = uptime (scaled 0-10000)
            DNode::internal(0, 0, 9_700 * feature_scale, 1, 2),
            DNode::leaf(1, 80_000),
            DNode::leaf(2, 320_000),
        ],
        SCALE,
    );

    ippan_ai_core::gbdt::Model::new(vec![proposal_tree, verification_tree, uptime_tree], 0)
}

struct EnvGuard {
    key: &'static str,
    previous: Option<String>,
}

impl EnvGuard {
    fn new(key: &'static str, value: &Path) -> Self {
        let previous = std::env::var(key).ok();
        std::env::set_var(key, value);
        Self { key, previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        if let Some(prev) = &self.previous {
            std::env::set_var(self.key, prev);
        } else {
            std::env::remove_var(self.key);
        }
    }
}
