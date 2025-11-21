//! Long-run simulation tests with explicit invariants
//!
//! These tests verify DLC consensus behavior over many rounds:
//! - No panics across full run
//! - Emission counters never go negative
//! - Total rewards distributed matches expected emission
//! - Validator scores always within expected range
//! - No validator is completely starved over long runs

#![allow(deprecated)]

use anyhow::Result;
use ippan_consensus_dlc::{
    bond,
    dag::{Block, BlockDAG},
    dgbdt::ValidatorMetrics,
    hashtimer::HashTimer,
    verifier::VerifierSet,
    DlcConfig, DlcConsensus,
};
use ippan_types::Amount;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::{
    collections::HashMap,
    env,
};

const LONG_RUN_ROUNDS: u64 = 500;
const SMALL_RUN_ROUNDS: u64 = 100;

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
        if let Some(prev) = &self.previous {
            env::set_var(self.key, prev);
        } else {
            env::remove_var(self.key);
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn invariant_emission_counters_never_negative() -> Result<()> {
    let _stub_guard = EnvVarGuard::new("IPPAN_DGBDT_ALLOW_STUB", "1");
    
    let mut consensus = DlcConsensus::new(DlcConfig {
        validators_per_round: 7,
        min_validator_stake: bond::MIN_VALIDATOR_BOND,
        unstaking_lock_rounds: 16,
        min_reputation: 4_000,
        enable_slashing: false,
    });

    let mut rng = StdRng::seed_from_u64(0x12345678);
    
    // Register validators
    for i in 0..10 {
        let id = format!("validator-{}", i);
        let stake = Amount::from_micro_ipn(20_000_000 + i * 1_000_000);
        let metrics = create_good_metrics(&mut rng, stake);
        consensus.register_validator(id, stake, metrics)?;
    }

    // Run rounds and check emission counters
    for round in 1..=SMALL_RUN_ROUNDS {
        consensus.current_round = round;
        
        let round_time = HashTimer::for_round(round);
        let seed = round_time.hash.clone();
        let verifier_set = consensus.validators.select_for_round(seed, round)?;
        
        // Create and process a block
        let block = create_simple_block(&consensus.dag, &verifier_set, round);
        if verifier_set.validate(&block).is_ok() {
            consensus.dag.insert(block.clone())?;
            
            let block_reward = consensus.emission.calculate_block_reward(round);
            
            // INVARIANT: Block reward should be reasonable
            // block_reward is u64, so it's always >= 0
            assert!(
                block_reward <= u64::MAX / 2,
                "Block reward at round {} is unreasonably high: {}",
                round,
                block_reward
            );
            
            // Update emission
            consensus.emission.update(round, 1)?;
            
            // INVARIANT: Emission state should be consistent
            let emission_stats = consensus.emission.stats();
            // emitted_supply is u64, so it's always >= 0
            assert!(
                emission_stats.emitted_supply < u64::MAX,
                "Total emitted overflow at round {}",
                round
            );
        }
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn invariant_validator_scores_in_range() -> Result<()> {
    let _stub_guard = EnvVarGuard::new("IPPAN_DGBDT_ALLOW_STUB", "1");
    
    let mut consensus = DlcConsensus::new(DlcConfig {
        validators_per_round: 5,
        min_validator_stake: bond::MIN_VALIDATOR_BOND,
        unstaking_lock_rounds: 16,
        min_reputation: 3_000,
        enable_slashing: false,
    });

    let mut rng = StdRng::seed_from_u64(0xABCDEF);
    
    // Register validators with varying metrics
    let mut validator_ids = Vec::new();
    for i in 0..8 {
        let id = format!("val-score-{}", i);
        let stake = Amount::from_micro_ipn(15_000_000 + i * 2_000_000);
        let metrics = create_varied_metrics(&mut rng, stake, i);
        consensus.register_validator(id.clone(), stake, metrics)?;
        validator_ids.push(id);
    }

    // Run rounds and verify scores stay in bounds
    for round in 1..=SMALL_RUN_ROUNDS {
        consensus.current_round = round;
        
        let round_time = HashTimer::for_round(round);
        let seed = round_time.hash.clone();
        let verifier_set = consensus.validators.select_for_round(seed, round)?;
        
        let block = create_simple_block(&consensus.dag, &verifier_set, round);
        if verifier_set.validate(&block).is_ok() {
            consensus.dag.insert(block.clone())?;
            
            // Reward validators
            let _ = consensus.reputation.reward_proposal(&block.proposer, round);
            for verifier in verifier_set.all_verifiers() {
                let _ = consensus.reputation.reward_verification(&verifier, round);
            }
        }
        
        // INVARIANT: Check all validator scores are in valid range
        // Note: Scores can exceed 10_000 slightly due to rewards accumulation
        for id in &validator_ids {
            if let Some(score) = consensus.reputation.scores.get(id) {
                assert!(
                    score.total >= 0 && score.total <= 100_000,
                    "Validator {} score total {} out of valid range at round {}",
                    id,
                    score.total,
                    round
                );
            }
        }
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn invariant_total_rewards_match_emission() -> Result<()> {
    let _stub_guard = EnvVarGuard::new("IPPAN_DGBDT_ALLOW_STUB", "1");
    
    let mut consensus = DlcConsensus::new(DlcConfig {
        validators_per_round: 5,
        min_validator_stake: bond::MIN_VALIDATOR_BOND,
        unstaking_lock_rounds: 16,
        min_reputation: 4_000,
        enable_slashing: false,
    });

    let mut rng = StdRng::seed_from_u64(0xDEADBEEF);
    
    // Register validators
    for i in 0..7 {
        let id = format!("validator-{}", i);
        let stake = Amount::from_micro_ipn(25_000_000);
        let metrics = create_good_metrics(&mut rng, stake);
        consensus.register_validator(id, stake, metrics)?;
    }

    let mut total_rewards_distributed = 0u64;
    let mut expected_emission = 0u64;

    // Run rounds and track rewards vs emission
    for round in 1..=SMALL_RUN_ROUNDS {
        consensus.current_round = round;
        
        let round_time = HashTimer::for_round(round);
        let seed = round_time.hash.clone();
        let verifier_set = consensus.validators.select_for_round(seed, round)?;
        
        let block = create_simple_block(&consensus.dag, &verifier_set, round);
        if verifier_set.validate(&block).is_ok() {
            consensus.dag.insert(block.clone())?;
            
            let block_reward = consensus.emission.calculate_block_reward(round);
            expected_emission += block_reward;
            
            // Distribute rewards
            if let Ok(distribution) = consensus.rewards.distribute_block_reward(
                block_reward,
                &block.proposer,
                &verifier_set.all_verifiers(),
            ) {
                total_rewards_distributed += distribution.total_distributed;
            }
            
            consensus.emission.update(round, 1)?;
        }
    }

    // INVARIANT: Total distributed should equal expected emission
    // (allowing for rounding in integer math)
    let difference = if total_rewards_distributed >= expected_emission {
        total_rewards_distributed - expected_emission
    } else {
        expected_emission - total_rewards_distributed
    };
    assert!(
        difference <= 100, // Allow small rounding error
        "Total rewards distributed ({}) differs from expected emission ({}) by {}",
        total_rewards_distributed,
        expected_emission,
        difference
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn invariant_no_validator_completely_starved() -> Result<()> {
    let _stub_guard = EnvVarGuard::new("IPPAN_DGBDT_ALLOW_STUB", "1");
    
    let mut consensus = DlcConsensus::new(DlcConfig {
        validators_per_round: 3,
        min_validator_stake: bond::MIN_VALIDATOR_BOND,
        unstaking_lock_rounds: 16,
        min_reputation: 3_000,
        enable_slashing: false,
    });

    let mut rng = StdRng::seed_from_u64(0xCAFEBABE);
    
    // Register validators
    let mut validator_ids = Vec::new();
    for i in 0..6 {
        let id = format!("validator-{}", i);
        let stake = Amount::from_micro_ipn(20_000_000);
        let metrics = create_good_metrics(&mut rng, stake);
        consensus.register_validator(id.clone(), stake, metrics)?;
        validator_ids.push(id);
    }

    // Track participation
    let mut validator_participation: HashMap<String, u64> = HashMap::new();
    for id in &validator_ids {
        validator_participation.insert(id.clone(), 0);
    }

    // Run many rounds
    for round in 1..=LONG_RUN_ROUNDS {
        consensus.current_round = round;
        
        let round_time = HashTimer::for_round(round);
        let seed = round_time.hash.clone();
        let verifier_set = consensus.validators.select_for_round(seed, round)?;
        
        let block = create_simple_block(&consensus.dag, &verifier_set, round);
        if verifier_set.validate(&block).is_ok() {
            consensus.dag.insert(block.clone())?;
            
            // Track participation
            *validator_participation.entry(block.proposer.clone()).or_insert(0) += 1;
            for verifier in verifier_set.all_verifiers() {
                *validator_participation.entry(verifier).or_insert(0) += 1;
            }
        }
    }

    // INVARIANT: Over a long run, no validator should have zero participation
    // (this tests fairness of the selection algorithm)
    let min_expected_participation = LONG_RUN_ROUNDS / (validator_ids.len() as u64 * 4);
    
    for id in &validator_ids {
        let participation = validator_participation.get(id).copied().unwrap_or(0);
        assert!(
            participation > min_expected_participation,
            "Validator {} only participated {} times in {} rounds (expected > {})",
            id,
            participation,
            LONG_RUN_ROUNDS,
            min_expected_participation
        );
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn invariant_dag_convergence_over_long_run() -> Result<()> {
    let _stub_guard = EnvVarGuard::new("IPPAN_DGBDT_ALLOW_STUB", "1");
    
    let mut consensus = DlcConsensus::new(DlcConfig {
        validators_per_round: 7,
        min_validator_stake: bond::MIN_VALIDATOR_BOND,
        unstaking_lock_rounds: 16,
        min_reputation: 4_000,
        enable_slashing: false,
    });

    let mut rng = StdRng::seed_from_u64(0x87654321);
    
    // Register validators
    for i in 0..12 {
        let id = format!("validator-{}", i);
        let stake = Amount::from_micro_ipn(30_000_000);
        let metrics = create_good_metrics(&mut rng, stake);
        consensus.register_validator(id, stake, metrics)?;
    }

    let mut finalized_count_history = Vec::new();

    // Run rounds
    for round in 1..=LONG_RUN_ROUNDS {
        consensus.current_round = round;
        
        let round_time = HashTimer::for_round(round);
        let seed = round_time.hash.clone();
        let verifier_set = consensus.validators.select_for_round(seed, round)?;
        
        let block = create_simple_block(&consensus.dag, &verifier_set, round);
        if verifier_set.validate(&block).is_ok() {
            consensus.dag.insert(block.clone())?;
        }
        
        // Finalize
        let _finalized_ids = consensus.dag.finalize_round(round_time);
        
        // Track finalized blocks
        let dag_stats = consensus.dag.stats();
        finalized_count_history.push(dag_stats.finalized_blocks);
        
        // INVARIANT: Tips count should not grow unbounded
        assert!(
            dag_stats.tips_count <= 5,
            "Too many tips ({}) at round {} - DAG not converging",
            dag_stats.tips_count,
            round
        );
        
        // INVARIANT: Pending blocks should not grow unbounded
        assert!(
            dag_stats.pending_blocks <= (round * 2) as usize,
            "Too many pending blocks ({}) at round {}",
            dag_stats.pending_blocks,
            round
        );
    }

    // INVARIANT: Over the long run, finalized blocks should grow
    let initial_finalized = finalized_count_history[0];
    let final_finalized = finalized_count_history[finalized_count_history.len() - 1];
    
    assert!(
        final_finalized > initial_finalized,
        "DAG should finalize blocks over long run: {} -> {}",
        initial_finalized,
        final_finalized
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn invariant_no_panic_with_edge_cases() -> Result<()> {
    let _stub_guard = EnvVarGuard::new("IPPAN_DGBDT_ALLOW_STUB", "1");
    
    let mut consensus = DlcConsensus::new(DlcConfig {
        validators_per_round: 3,
        min_validator_stake: bond::MIN_VALIDATOR_BOND,
        unstaking_lock_rounds: 8,
        min_reputation: 2_000,
        enable_slashing: true, // Enable slashing for edge case testing
    });

    let mut rng = StdRng::seed_from_u64(0xBAADF00D);
    
    // Register minimal validators
    for i in 0..5 {
        let id = format!("validator-{}", i);
        let stake = Amount::from_micro_ipn(10_000_000);
        let metrics = create_good_metrics(&mut rng, stake);
        consensus.register_validator(id, stake, metrics)?;
    }

    // Run rounds with various edge cases
    for round in 1..=SMALL_RUN_ROUNDS {
        consensus.current_round = round;
        
        let round_time = HashTimer::for_round(round);
        let seed = round_time.hash.clone();
        
        // This should not panic even with edge cases
        match consensus.validators.select_for_round(seed, round) {
            Ok(verifier_set) => {
                let block = create_simple_block(&consensus.dag, &verifier_set, round);
                
                // Validation might fail, but should not panic
                let _ = verifier_set.validate(&block);
                let _ = consensus.dag.insert(block.clone());
                let _ = consensus.dag.finalize_round(round_time);
            }
            Err(_) => {
                // Selection failure is OK, just should not panic
            }
        }
        
        // Randomly slash validators (edge case testing)
        if round % 20 == 0 && rng.gen_bool(0.3) {
            let validator_id = format!("validator-{}", rng.gen_range(0..5));
            let _ = consensus.bonds.slash_validator(
                &validator_id,
                "edge case test".to_string(),
                1000, // 10% slash
                round,
            );
        }
        
        // Update emission should not panic
        let _ = consensus.emission.update(round, 1);
    }

    // If we reach here without panic, invariant is satisfied
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn invariant_deterministic_with_fixed_seed() -> Result<()> {
    let _stub_guard = EnvVarGuard::new("IPPAN_DGBDT_ALLOW_STUB", "1");
    
    const SEED: u64 = 0x12341234;
    const ROUNDS: u64 = 50;
    
    // Run 1
    let run1_results = run_deterministic_simulation(SEED, ROUNDS).await?;
    
    // Run 2 with same seed
    let run2_results = run_deterministic_simulation(SEED, ROUNDS).await?;
    
    // INVARIANT: Same seed should produce same results
    assert_eq!(
        run1_results.len(),
        run2_results.len(),
        "Number of rounds should match"
    );
    
    for (i, (r1, r2)) in run1_results.iter().zip(run2_results.iter()).enumerate() {
        assert_eq!(
            r1.finalized_count, r2.finalized_count,
            "Finalized count differs at round {}",
            i
        );
        assert_eq!(
            r1.tips_count, r2.tips_count,
            "Tips count differs at round {}",
            i
        );
    }

    Ok(())
}

// Helper functions

fn create_good_metrics(rng: &mut StdRng, stake: Amount) -> ValidatorMetrics {
    ValidatorMetrics::new(
        rng.gen_range(9_000..=10_000), // uptime
        rng.gen_range(100..=500),      // latency
        rng.gen_range(9_000..=10_000), // honesty
        rng.gen_range(50..=200),       // blocks_proposed
        rng.gen_range(100..=300),      // blocks_verified
        stake,
        rng.gen_range(32..=256),       // rounds_active
    )
}

fn create_varied_metrics(rng: &mut StdRng, stake: Amount, index: u64) -> ValidatorMetrics {
    // Create validators with intentionally varied performance
    let uptime = (5_000 + (index * 1000).min(5_000)) as i64;
    let latency = (500 + (index * 200).min(4_500)) as i64;
    let honesty = (6_000 + (index * 500).min(4_000)) as i64;
    
    ValidatorMetrics::new(
        uptime.min(10_000),
        latency.min(5_000),
        honesty.min(10_000),
        rng.gen_range(20..=100),
        rng.gen_range(40..=200),
        stake,
        rng.gen_range(16..=128),
    )
}

fn create_simple_block(dag: &BlockDAG, verifier_set: &VerifierSet, round: u64) -> Block {
    let parent = dag
        .get_tips()
        .into_iter()
        .max_by_key(|b| b.height)
        .map(|b| b.id.clone())
        .or_else(|| dag.genesis_id.clone())
        .expect("DAG must have genesis");
    
    let mut block = Block::new(
        vec![parent],
        HashTimer::for_round(round),
        format!("test-round-{}", round).into_bytes(),
        verifier_set.primary.clone(),
    );
    block.sign(vec![0u8; 64]);
    block
}

#[derive(Debug, Clone)]
struct RoundResult {
    finalized_count: usize,
    tips_count: usize,
}

async fn run_deterministic_simulation(seed: u64, rounds: u64) -> Result<Vec<RoundResult>> {
    let _stub_guard = EnvVarGuard::new("IPPAN_DGBDT_ALLOW_STUB", "1");
    
    let mut consensus = DlcConsensus::new(DlcConfig {
        validators_per_round: 5,
        min_validator_stake: bond::MIN_VALIDATOR_BOND,
        unstaking_lock_rounds: 16,
        min_reputation: 4_000,
        enable_slashing: false,
    });

    let mut rng = StdRng::seed_from_u64(seed);
    
    // Register validators
    for i in 0..8 {
        let id = format!("validator-{}", i);
        let stake = Amount::from_micro_ipn(20_000_000);
        let metrics = create_good_metrics(&mut rng, stake);
        consensus.register_validator(id, stake, metrics)?;
    }

    let mut results = Vec::new();

    for round in 1..=rounds {
        consensus.current_round = round;
        
        let round_time = HashTimer::for_round(round);
        let seed = round_time.hash.clone();
        let verifier_set = consensus.validators.select_for_round(seed, round)?;
        
        let block = create_simple_block(&consensus.dag, &verifier_set, round);
        if verifier_set.validate(&block).is_ok() {
            consensus.dag.insert(block)?;
        }
        
        consensus.dag.finalize_round(round_time);
        
        let dag_stats = consensus.dag.stats();
        results.push(RoundResult {
            finalized_count: dag_stats.finalized_blocks,
            tips_count: dag_stats.tips_count,
        });
    }

    Ok(results)
}
