use ippan_consensus_dlc::{
    bond,
    dag::Block,
    dgbdt::{FairnessModel, ValidatorMetrics},
    emission,
    hashtimer::HashTimer,
    verifier::ValidatorSetManager,
    DlcConfig, DlcConsensus,
};
use ippan_types::Amount;
use proptest::prelude::*;
use proptest::test_runner::TestCaseResult;
use std::collections::HashSet;
use tokio::runtime::Runtime;

struct EnvVarGuard {
    key: &'static str,
    previous: Option<String>,
}

impl EnvVarGuard {
    fn new(key: &'static str, value: &str) -> Self {
        let previous = std::env::var(key).ok();
        std::env::set_var(key, value);
        Self { key, previous }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        if let Some(prev) = &self.previous {
            std::env::set_var(self.key, prev);
        } else {
            std::env::remove_var(self.key);
        }
    }
}

proptest! {
    #[test]
    fn fairness_scoring_is_deterministic_and_bounded(
        metrics in prop::collection::vec(adversarial_validator_metrics(), 1..20)
    ) {
        let model = FairnessModel::testing_stub();

        for metrics in metrics {
            let score_a = model.score_deterministic(&metrics);
            let score_b = model.score_deterministic(&metrics);

            prop_assert_eq!(score_a, score_b);
            prop_assert!(score_a >= 0);
            prop_assert!(score_a <= 10_000);
        }
    }
}

proptest! {
    #[test]
    fn fairness_scores_remain_bounded(metrics in prop::collection::vec(
        (
            0i64..=10_000, // uptime
            0i64..=15_000, // latency can exceed 100% to test clamping
            0i64..=10_000, // honesty
            0u64..=500,    // blocks proposed
            0u64..=500,    // blocks verified
            0u64..=2_000,  // rounds active
            1_000_000u64..=50_000_000u64, // stake (micro-IPN)
        ),
        1..25,
    )) {
        let model = FairnessModel::testing_stub();

        for (
            uptime,
            latency,
            honesty,
            proposed,
            verified,
            rounds_active,
            stake,
        ) in metrics {
            let metrics = ValidatorMetrics::new(
                uptime,
                latency,
                honesty,
                proposed,
                verified,
                Amount::from_micro_ipn(stake),
                rounds_active,
            );

            let score = model.score_deterministic(&metrics);
            prop_assert!(score >= 0);
            prop_assert!(score <= 10_000);
        }
    }
}

proptest! {
    #[test]
    fn verifier_selection_stays_within_active_set(
        validators in prop::collection::vec(
            (
                0i64..=10_000,
                0i64..=12_000,
                0i64..=10_000,
                0u64..=200,
                0u64..=200,
                0u64..=64,
            ),
            3..10,
        )
    ) {
        let mut manager = ValidatorSetManager::new(FairnessModel::testing_stub(), 6);

        for (idx, (uptime, latency, honesty, proposed, verified, rounds_active)) in validators.into_iter().enumerate() {
            let metrics = ValidatorMetrics::new(
                uptime,
                latency,
                honesty,
                proposed,
                verified,
                Amount::from_micro_ipn(10_000_000 + (idx as u64 * 1_000_000)),
                rounds_active,
            );
            manager
                .register_validator(format!("validator-{idx}"), metrics)
                .expect("registration succeeds");
        }

        let known: HashSet<String> = manager.all_validators().keys().cloned().collect();
        prop_assert!(!known.is_empty());

        for round in 1..=3 {
            let selection = manager
                .select_for_round(format!("seed-{round}"), round as u64)
                .expect("selection succeeds");

            prop_assert!(known.contains(&selection.primary));

            let mut seen = HashSet::new();
            seen.insert(selection.primary.clone());
            for shadow in &selection.shadows {
                prop_assert!(known.contains(shadow));
                prop_assert!(seen.insert(shadow.clone()));
            }

            prop_assert!(selection.size() <= known.len());
        }
    }
}

proptest! {
    #[test]
    fn shadow_verifier_selected_when_capacity_and_activity_allow(
        validator_count in 3usize..15,
        max_set_size in 2usize..8,
    ) {
        let mut manager = ValidatorSetManager::new(FairnessModel::testing_stub(), max_set_size);

        for idx in 0..validator_count {
            let metrics = ValidatorMetrics::new(
                9_000 - (idx as i64 * 25),
                400 + (idx as i64 * 10),
                9_500,
                idx as u64,
                idx as u64,
                Amount::from_micro_ipn(5_000_000 + (idx as u64 * 100_000)),
                10 + idx as u64,
            );
            manager
                .register_validator(format!("validator-{idx}"), metrics)
                .expect("registration succeeds");
        }

        let effective_set_size = max_set_size.min(validator_count).max(2);
        let set = manager
            .select_for_round("shadow-check".to_string(), 1)
            .expect("selection succeeds");

        prop_assert!(set.size() <= effective_set_size);
        prop_assert!(!set.shadows.is_empty());
        prop_assert!(set.shadows.iter().all(|id| id != &set.primary));
    }
}

proptest! {
    #[test]
    fn consensus_rounds_survive_randomized_events(
        validator_count in 3usize..8,
        events in prop::collection::vec((prop::bool::ANY, 0usize..16usize), 1..20),
    ) {
        let _guard = EnvVarGuard::new("IPPAN_DGBDT_ALLOW_STUB", "1");
        let rt = Runtime::new().expect("runtime");

        let outcome: TestCaseResult = rt.block_on(async move {
            let mut consensus = DlcConsensus::new(DlcConfig {
                validators_per_round: validator_count,
                ..DlcConfig::default()
            });

            let mut validator_ids = Vec::new();
            for idx in 0..validator_count {
                let metrics = ValidatorMetrics::new(
                    9_000 - (idx as i64 * 50),
                    (idx as i64 * 150).min(12_000),
                    9_500,
                    idx as u64,
                    idx as u64,
                    bond::VALIDATOR_BOND,
                    1 + idx as u64,
                );
                let id = format!("validator-{idx}");
                consensus
                    .register_validator(id.clone(), bond::VALIDATOR_BOND, metrics)
                    .expect("validator registered");
                validator_ids.push(id);
            }

            let baseline_supply = consensus.emission.stats().current_supply;

            for (idx, (add_block, proposer_hint)) in events.into_iter().enumerate() {
                if add_block {
                    let parents: Vec<String> = consensus
                        .dag
                        .get_tips()
                        .into_iter()
                        .map(|block| block.id.clone())
                        .collect();
                    let proposer = validator_ids[proposer_hint % validator_ids.len()].clone();

                    let mut block = Block::new(
                        parents,
                        HashTimer::for_round(consensus.current_round + 1),
                        vec![idx as u8],
                        proposer,
                    );
                    block.sign(vec![0u8; 64]);
                    let _ = consensus.dag.insert(block);
                }

                let result = consensus.process_round().await.expect("round result");
                prop_assert!(result.round >= 1);
                let current_supply = consensus.emission.stats().current_supply;
                prop_assert!(current_supply >= baseline_supply);
                prop_assert!(current_supply <= emission::SUPPLY_CAP);

                for vid in &validator_ids {
                    prop_assert!(consensus.reputation.score(vid) >= 0);
                }

                let bonded = consensus.bonds.total_bonded_amount();
                prop_assert!(bonded.atomic() > 0);
            }
            Ok(())
        });

        outcome?;
    }
}

fn adversarial_validator_metrics() -> impl Strategy<Value = ValidatorMetrics> {
    prop_oneof![
        Just(ValidatorMetrics::default()),
        (
            0i64..=10_000,
            0i64..=15_000,
            0i64..=10_000,
            0u64..=1_000,
            0u64..=1_000,
            1_000_000u64..=50_000_000u64,
            0u64..=1_000,
        )
            .prop_map(
                |(uptime, latency, honesty, proposed, verified, stake, rounds_active)| {
                    ValidatorMetrics::new(
                        uptime,
                        latency,
                        honesty,
                        proposed,
                        verified,
                        Amount::from_micro_ipn(stake),
                        rounds_active,
                    )
                }
            ),
        // Highly skewed stake and activity with identical uptime to stress ordering
        (
            prop_oneof![Just(0i64), Just(10_000i64)],
            prop_oneof![Just(0i64), Just(15_000i64)],
            prop_oneof![Just(0i64), Just(10_000i64)],
            prop_oneof![Just(0u64), Just(2_000u64)],
            prop_oneof![Just(0u64), Just(2_000u64)],
            prop_oneof![Just(1_000_000u64), Just(100_000_000u64)],
            prop_oneof![Just(0u64), Just(2_000u64)],
        )
            .prop_map(
                |(uptime, latency, honesty, proposed, verified, stake, rounds_active)| {
                    ValidatorMetrics::new(
                        uptime,
                        latency,
                        honesty,
                        proposed,
                        verified,
                        Amount::from_micro_ipn(stake),
                        rounds_active,
                    )
                }
            )
    ]
}
