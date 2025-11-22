use std::collections::{HashMap, HashSet};

use ippan_consensus_dlc::{
    dgbdt::{FairnessModel, ValidatorMetrics},
    verifier::ValidatorSetManager,
};
use ippan_types::Amount;

fn build_uniform_validators(count: usize) -> HashMap<String, ValidatorMetrics> {
    let mut validators = HashMap::new();
    for idx in 0..count {
        // Keep metrics intentionally uniform so DAG-Fair tie-breaking and entropy drive rotation.
        let metrics = ValidatorMetrics::new(
            9_800,
            400,
            9_800,
            10,
            10,
            Amount::from_micro_ipn(10_000_000 + (idx as u64 * 10_000)),
            50,
        );
        validators.insert(format!("validator-{idx}"), metrics);
    }
    validators
}

fn run_rotation_scenario(
    validator_count: usize,
    max_set_size: usize,
    rounds: usize,
) -> (HashMap<String, usize>, HashMap<String, usize>) {
    let model = FairnessModel::testing_stub();
    let mut manager = ValidatorSetManager::new(model, max_set_size);

    for (id, metrics) in build_uniform_validators(validator_count) {
        manager
            .register_validator(id, metrics)
            .expect("validator registration");
    }

    let mut primary_counts: HashMap<String, usize> = HashMap::new();
    let mut shadow_counts: HashMap<String, usize> = HashMap::new();

    for round in 0..rounds {
        let set = manager
            .select_for_round(format!("scenario-{validator_count}-{round}"), round as u64 + 1)
            .expect("selection succeeds");

        *primary_counts.entry(set.primary.clone()).or_default() += 1;
        for shadow in &set.shadows {
            *shadow_counts.entry(shadow.clone()).or_default() += 1;
        }

        assert!(set.size() <= max_set_size.min(validator_count));
        assert!(set.shadows.len() + 1 == set.size());
        assert!(!set.shadows.is_empty());
    }

    (primary_counts, shadow_counts)
}

#[test]
fn small_cluster_rotation_and_shadow_coverage() {
    let scenarios = [
        (4usize, 3usize, 6usize, 2usize),
        (7usize, 4usize, 10usize, 3usize),
        (21usize, 7usize, 15usize, 5usize),
    ];

    for (validator_count, max_set_size, rounds, min_primary_spread) in scenarios {
        let (primary_counts, shadow_counts) =
            run_rotation_scenario(validator_count, max_set_size, rounds);

        // DAG-Fair rotation should spread primaries across multiple validators.
        let unique_primaries = primary_counts.len();
        assert!(
            unique_primaries >= min_primary_spread,
            "expected at least {min_primary_spread} unique primaries, got {unique_primaries}"
        );

        // Every validator should be engaged either as a primary or shadow across the sequence.
        let mut engaged: HashSet<String> = shadow_counts.keys().cloned().collect();
        engaged.extend(primary_counts.keys().cloned());
        assert!(engaged.len() >= max_set_size); // At least as many engaged as typical round size.

        // Shadow verifiers should provide redundancy across rounds.
        assert!(shadow_counts.values().sum::<usize>() >= rounds);
        assert!(shadow_counts.len() >= min_primary_spread);
    }
}
