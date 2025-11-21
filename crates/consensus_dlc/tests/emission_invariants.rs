#![allow(deprecated)]

use anyhow::Result;
use ippan_consensus_dlc::{
    dag::Block, dgbdt::ValidatorMetrics, hashtimer::HashTimer, DlcConfig, DlcConsensus,
};
use ippan_types::Amount;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::env;

const ROUNDS: u64 = 256;
const VALIDATOR_COUNT: usize = 12;

struct EnvVarGuard {
    key: &'static str,
    previous: Option<String>,
}

impl EnvVarGuard {
    fn new(key: &'static str, value: &str) -> Self {
        let previous = env::var(key).ok();
        env::set_var(key, value);
        Self { key, previous }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        if let Some(value) = &self.previous {
            env::set_var(self.key, value);
        } else {
            env::remove_var(self.key);
        }
    }
}

fn deterministic_metrics(index: usize) -> ValidatorMetrics {
    let uptime = 9_500 + (index as i64 * 10);
    let latency = 200 + (index as i64 * 5);
    let honesty = 9_400 + (index as i64 * 7);
    ValidatorMetrics::new(
        uptime.min(9_999),
        latency,
        honesty.min(9_999),
        50 + index as u64,
        90 + index as u64,
        Amount::from_micro_ipn(5_000_000 + (index as u64 * 100_000)),
        150 + index as u64,
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn long_run_emission_and_fairness_invariants() -> Result<()> {
    let _guard = EnvVarGuard::new("IPPAN_DGBDT_ALLOW_STUB", "1");
    let mut config = DlcConfig::default();
    config.validators_per_round = VALIDATOR_COUNT;
    config.min_reputation = 2_500;

    let mut consensus = DlcConsensus::new(config);

    for idx in 0..VALIDATOR_COUNT {
        let stake = Amount::from_ipn(10 + (idx as u64 % 3));
        consensus.register_validator(
            format!("fair-val-{idx:02}"),
            stake,
            deterministic_metrics(idx),
        )?;
    }

    let mut expected_emission: u128 = 0;
    let mut rng = StdRng::seed_from_u64(0xEC0D_1E50);

    for _ in 0..ROUNDS {
        let next_round = consensus.current_round + 1;
        let round_time = HashTimer::for_round(next_round);
        let verifier_set = consensus
            .validators
            .select_for_round(round_time.hash.clone(), next_round)?
            .clone();

        let parent_ids = consensus
            .dag
            .get_tips()
            .into_iter()
            .map(|block| block.id.clone())
            .collect();

        let proposer = verifier_set.primary.clone();
        let mut block = Block::new(
            parent_ids,
            round_time.clone(),
            vec![next_round as u8, rng.gen()],
            proposer,
        );
        block.sign(vec![1u8; 64]);
        consensus.dag.insert(block).ok();

        let result = consensus.process_round().await?;
        expected_emission += (result.block_reward as u128) * (result.blocks_processed as u128);

        let stats = consensus.stats();
        assert!(
            stats.reward_stats.total_distributed as u128 <= expected_emission,
            "distributed rewards cannot exceed emitted supply"
        );
        assert!(
            stats.reward_stats.total_pending as u128 <= expected_emission,
            "pending rewards cannot exceed emitted supply"
        );
        assert!(
            stats.emission_stats.current_supply as u128
                >= stats.reward_stats.total_distributed as u128,
            "current supply should track distributed rewards"
        );
    }

    let stats = consensus.stats();
    assert_eq!(
        stats.emission_stats.emitted_supply as u128, expected_emission,
        "Emission supply should match cumulative block rewards"
    );
    let accounted_rewards =
        stats.reward_stats.total_distributed as u128 + stats.reward_stats.total_pending as u128;
    assert!(
        accounted_rewards <= expected_emission,
        "Accounted rewards should not exceed emitted supply"
    );
    assert!(
        accounted_rewards > 0,
        "Rewards should accumulate across the long-run simulation"
    );
    assert!(
        stats.reward_stats.pending_validator_count <= VALIDATOR_COUNT,
        "Pending rewards should not exceed validator count"
    );
    assert!(
        stats.reputation_stats.min_reputation >= 0
            && stats.reputation_stats.max_reputation <= 100_000,
        "Reputation scores must stay within normalized bounds"
    );

    let pending = consensus.rewards.all_pending();
    assert_eq!(
        pending.len(),
        VALIDATOR_COUNT,
        "All validators should have received rewards over long run"
    );

    Ok(())
}
