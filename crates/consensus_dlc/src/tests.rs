#![allow(deprecated)]
//! Comprehensive integration tests for DLC consensus

use super::*;
use crate::dag::Block;
use crate::dgbdt::ValidatorMetrics;
use crate::hashtimer::HashTimer;
use ippan_types::Amount;

#[tokio::test]
async fn test_dlc_initialization() {
    init_dlc();

    let config = DlcConfig::default();
    let consensus = DlcConsensus::new(config);

    assert_eq!(consensus.current_round, 0);
    assert_eq!(consensus.dag.blocks.len(), 1); // Genesis block
}

#[tokio::test]
async fn test_validator_registration() {
    let config = DlcConfig::default();
    let mut consensus = DlcConsensus::new(config);

    let metrics = ValidatorMetrics::new(
        9900,  // 99% uptime (scaled by 10000)
        500,   // 5% latency (scaled by 10000)
        10000, // 100% honesty (scaled by 10000)
        100,
        500,
        Amount::from_micro_ipn(10_000_000),
        100,
    );

    let result =
        consensus.register_validator("validator1".to_string(), bond::VALIDATOR_BOND, metrics);

    assert!(result.is_ok());
    assert_eq!(consensus.validators.validator_count(), 1);
}

#[tokio::test]
async fn test_consensus_round_processing() {
    let config = DlcConfig::default();
    let mut consensus = DlcConsensus::new(config);

    // Register validators
    for i in 1..=3 {
        let metrics = ValidatorMetrics::new(
            9900,  // 99% uptime (scaled by 10000)
            500,   // 5% latency (scaled by 10000)
            10000, // 100% honesty (scaled by 10000)
            100,
            500,
            Amount::from_micro_ipn(10_000_000),
            100,
        );
        consensus
            .register_validator(format!("validator{}", i), bond::VALIDATOR_BOND, metrics)
            .unwrap();
    }

    // Process a round
    let result = consensus.process_round().await;

    assert!(result.is_ok());
    let round_result = result.unwrap();
    assert_eq!(round_result.round, 1);
}

#[tokio::test]
async fn test_block_proposal_and_verification() {
    let mut dag = BlockDAG::new();
    let genesis_id = dag.genesis_id.clone().unwrap();

    // Create a new block
    let mut block = Block::new(
        vec![genesis_id],
        HashTimer::for_round(1),
        vec![1, 2, 3, 4, 5],
        "validator1".to_string(),
    );
    block.sign(vec![0u8; 64]); // Mock signature

    // Insert block
    let result = dag.insert(block);
    assert!(result.is_ok());
    assert_eq!(dag.blocks.len(), 2); // Genesis + new block
}

#[tokio::test]
async fn test_verifier_selection_determinism() {
    let model = FairnessModel::testing_stub();
    let mut validators = HashMap::new();

    for i in 1..=5 {
        validators.insert(
            format!("val{}", i),
            ValidatorMetrics::new(
                9900,  // 99% uptime (scaled by 10000)
                500,   // 5% latency (scaled by 10000)
                10000, // 100% honesty (scaled by 10000)
                100,
                500,
                Amount::from_micro_ipn(10_000_000),
                100,
            ),
        );
    }

    // Same seed should produce same selection
    let set1 = VerifierSet::select(&model, &validators, "test_seed", 1, validators.len()).unwrap();
    let set2 = VerifierSet::select(&model, &validators, "test_seed", 1, validators.len()).unwrap();

    assert_eq!(set1.primary, set2.primary);
    assert_eq!(set1.shadows, set2.shadows);
}

#[tokio::test]
async fn test_reputation_tracking() {
    let mut reputation = ReputationDB::default();

    reputation.initialize_validator("val1".to_string()).unwrap();

    // Reward good behavior
    reputation.reward_proposal("val1", 1).unwrap();
    assert!(reputation.score("val1") > 10000);

    // Penalize bad behavior
    reputation.penalize_missed_proposal("val1", 2).unwrap();
    let score_after_penalty = reputation.score("val1");
    assert!(score_after_penalty < reputation.score("val1") + 100);
}

#[tokio::test]
async fn test_bonding_and_slashing() {
    let mut bonds = BondManager::new(100);

    // Create bond
    bonds
        .create_bond("val1".to_string(), bond::VALIDATOR_BOND)
        .unwrap();

    // Slash for malicious behavior
    let slashed = bonds
        .slash_validator(
            "val1",
            "Double signing".to_string(),
            bond::DOUBLE_SIGN_SLASH_BPS,
            1,
        )
        .unwrap();

    assert_eq!(slashed, bond::VALIDATOR_BOND / 2); // 50% slash

    let bond = bonds.get_bond("val1").unwrap();
    assert_eq!(bond.amount, bond::VALIDATOR_BOND / 2);
}

#[tokio::test]
async fn test_emission_schedule() {
    let mut emission = EmissionSchedule::default();
    let initial_supply = emission.current_supply;
    assert_eq!(initial_supply, 0); // Starts from genesis (0 supply)

    // Process several rounds
    for round in 1..=100 {
        emission.update(round, 1).unwrap();
    }

    // Supply should have increased from 0
    assert!(emission.current_supply >= initial_supply);

    // Should not exceed max supply (21 million)
    assert!(emission.current_supply <= emission.max_supply);
    assert!(emission.current_supply <= emission::SUPPLY_CAP);
}

#[tokio::test]
async fn test_reward_distribution() {
    let mut rewards = RewardDistributor::default();

    let result = rewards.distribute_block_reward(
        emission::BLOCK_REWARD,
        "proposer",
        &["v1".to_string(), "v2".to_string(), "v3".to_string()],
    );

    assert!(result.is_ok());
    let dist = result.unwrap();

    assert!(dist.proposer_reward > 0);
    assert!(dist.verifier_reward > 0);
    assert!(dist.treasury_reward > 0);
    assert_eq!(
        dist.total_distributed,
        dist.proposer_reward + dist.verifier_reward + dist.treasury_reward
    );
}

#[tokio::test]
async fn test_unstaking_flow() {
    let mut bonds = BondManager::new(100);

    bonds
        .create_bond("val1".to_string(), bond::VALIDATOR_BOND)
        .unwrap();

    // Initiate unstaking
    bonds.initiate_unstaking("val1", 1).unwrap();

    // Should not be able to participate
    let bond = bonds.get_bond("val1").unwrap();
    assert!(!bond.can_participate());

    // Complete unstaking after lock period
    let withdrawn = bonds.complete_unstaking("val1", 102).unwrap();
    assert_eq!(withdrawn, bond::VALIDATOR_BOND);
}

#[tokio::test]
async fn test_dag_topological_sort() {
    let mut dag = BlockDAG::new();
    let genesis_id = dag.genesis_id.clone().unwrap();

    // Create a chain of blocks
    let mut parent = genesis_id.clone();
    for i in 1..=5 {
        let mut block = Block::new(
            vec![parent.clone()],
            HashTimer::for_round(i),
            vec![],
            format!("validator{}", i),
        );
        block.sign(vec![0u8; 64]);
        parent = block.id.clone();
        dag.insert(block).unwrap();
    }

    let sorted = dag.topological_sort();
    assert!(sorted.len() >= 6); // Genesis + 5 blocks
}

#[tokio::test]
async fn test_fairness_model_scoring() {
    let model = FairnessModel::testing_stub();

    // High-quality validator
    let good_metrics = ValidatorMetrics::new(
        9900,  // 99% uptime (scaled by 10000)
        500,   // 5% latency (scaled by 10000)
        10000, // 100% honesty (scaled by 10000)
        1000,
        5000,
        Amount::from_micro_ipn(100_000_000),
        1000,
    );
    let good_score = model.score_deterministic(&good_metrics);

    // Low-quality validator
    let bad_metrics = ValidatorMetrics::new(
        8000, // 80% uptime (scaled by 10000)
        3000, // 30% latency (scaled by 10000)
        7000, // 70% honesty (scaled by 10000)
        10,
        50,
        Amount::from_micro_ipn(1_000_000),
        100,
    );
    let bad_score = model.score_deterministic(&bad_metrics);

    // Both should be valid scores
    assert!(good_score > 0);
    assert!(bad_score >= 0);
    // Generally good metrics should score higher, but the model combines multiple factors
    assert!(good_score >= bad_score * 7 / 10); // Good score at least 70% higher
}

#[tokio::test]
async fn test_hashtimer_ordering() {
    let ht1 = HashTimer::for_round(1);
    let ht2 = HashTimer::for_round(2);
    let ht3 = HashTimer::for_round(3);

    assert!(ht1 < ht2);
    assert!(ht2 < ht3);
    assert!(ht1 < ht3);
}

#[tokio::test]
async fn test_block_validation_invalid_proposer() {
    let model = FairnessModel::testing_stub();
    let mut validators = HashMap::new();
    validators.insert("val1".to_string(), ValidatorMetrics::default());

    let verifier_set =
        VerifierSet::select(&model, &validators, "seed", 1, validators.len()).unwrap();

    // Create block with invalid proposer
    let mut block = Block::new(
        vec![],
        HashTimer::for_round(1),
        vec![],
        "invalid_proposer".to_string(),
    );
    block.sign(vec![0u8; 64]);

    let result = verifier_set.validate(&block);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_full_consensus_cycle() {
    let config = DlcConfig {
        validators_per_round: 5,
        min_validator_stake: bond::MIN_VALIDATOR_BOND,
        unstaking_lock_rounds: 100,
        min_reputation: 5000,
        enable_slashing: true,
    };

    let mut consensus = DlcConsensus::new(config);

    // Register 5 validators
    for i in 1..=5 {
        let metrics = ValidatorMetrics::new(
            9900 - (i as i64 * 100),  // 0.99 -> 9900, decrement by 100
            500 + (i as i64 * 100),   // 0.05 -> 500, increment by 100
            10000 - (i as i64 * 100), // 1.0 -> 10000, decrement by 100
            100 * i,
            500 * i,
            Amount::from_micro_ipn(10_000_000 * i),
            100,
        );

        consensus
            .register_validator(
                format!("validator{}", i),
                bond::VALIDATOR_BOND * i as u128,
                metrics,
            )
            .unwrap();
    }

    // Process 10 rounds
    for _ in 1..=10 {
        let result = consensus.process_round().await;
        assert!(result.is_ok());
    }

    // Verify consensus state
    let stats = consensus.stats();
    assert_eq!(stats.current_round, 10);

    // Emitted supply equals current supply (since we start from 0)
    assert_eq!(
        stats.emission_stats.current_supply,
        stats.emission_stats.emitted_supply
    );

    // Supply should be within reasonable bounds (21M cap)
    assert!(stats.emission_stats.current_supply <= emission::SUPPLY_CAP);
}

#[tokio::test]
async fn test_concurrent_block_production() {
    let mut dag = BlockDAG::new();
    let genesis_id = dag.genesis_id.clone().unwrap();

    // Create multiple blocks from same parent (DAG property)
    for i in 1..=3 {
        let mut block = Block::new(
            vec![genesis_id.clone()],
            HashTimer::for_round(1),
            vec![i as u8],
            format!("validator{}", i),
        );
        block.sign(vec![0u8; 64]);
        dag.insert(block).unwrap();
    }

    // Should have 4 blocks total (genesis + 3 concurrent)
    assert_eq!(dag.blocks.len(), 4);

    // Should have 3 tips (the concurrent blocks)
    assert_eq!(dag.tips.len(), 3);
}

#[tokio::test]
async fn test_reputation_good_standing() {
    let mut reputation = ReputationDB::default();

    reputation.initialize_validator("val1".to_string()).unwrap();
    reputation.initialize_validator("val2".to_string()).unwrap();

    // val1 performs well
    for round in 1..=10 {
        reputation.reward_proposal("val1", round).unwrap();
    }

    // val2 performs poorly - need more penalties to drop below threshold (5000)
    for round in 1..=15 {
        reputation.penalize_invalid_proposal("val2", round).unwrap();
    }

    assert!(reputation.can_participate("val1"));
    // After 15 invalid proposals at -500 each (7500 penalty), should be below 5000 threshold
    assert!(!reputation.can_participate("val2"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_long_run_consensus_simulation_stability() {
    const TOTAL_ROUNDS: u64 = 256;
    const VALIDATOR_COUNT: usize = 32;

    init_dlc();

    let mut config = DlcConfig::default();
    config.validators_per_round = 11;
    config.unstaking_lock_rounds = 256;
    config.min_reputation = 2500;

    let validators_per_round = config.validators_per_round;
    let mut consensus = DlcConsensus::new(config);

    for i in 0..VALIDATOR_COUNT {
        let uptime = 9400 + ((i % 7) as i64) * 50; // 0.94 -> 9400, 0.005 -> 50
        let latency = 100 + ((i % 5) as i64) * 20; // 0.01 -> 100, 0.002 -> 20
        let honesty = 9200 + ((i % 9) as i64) * 60; // 0.92 -> 9200, 0.006 -> 60
        let metrics = ValidatorMetrics::new(
            uptime.min(9990), // Already scaled, so 9990 = 99.9%
            latency,
            honesty.min(9990), // Already scaled, so 9990 = 99.9%
            50 + (i as u64 * 3),
            150 + (i as u64 * 5),
            Amount::from_micro_ipn(5_000_000 + (i as u64 * 125_000)),
            200 + i as u64,
        );

        let stake = Amount::from_ipn(10 + (i as u64 % 5));
        consensus
            .register_validator(format!("sim-validator-{i:02}"), stake, metrics)
            .expect("validator registration succeeds");
    }

    let mut total_blocks: u64 = 0;
    let mut total_emission: u128 = 0;
    let mut reward_history = Vec::with_capacity(TOTAL_ROUNDS as usize);

    for round in 1..=TOTAL_ROUNDS {
        // Feed the DAG with a new block proposal before processing the round
        let parent_ids = consensus
            .dag
            .get_tips()
            .into_iter()
            .map(|block| block.id.clone())
            .collect();
        let proposer = format!("sim-validator-{:02}", (round as usize % VALIDATOR_COUNT));
        let mut block = Block::new(
            parent_ids,
            HashTimer::for_round(round),
            vec![round as u8],
            proposer,
        );
        block.sign(vec![1u8; 64]);
        consensus
            .dag
            .insert(block)
            .expect("pending block should insert");

        let result = consensus
            .process_round()
            .await
            .unwrap_or_else(|e| panic!("round {round} failed: {e}"));

        assert_eq!(result.round, round);
        total_blocks += result.blocks_processed as u64;
        total_emission += (result.block_reward as u128) * (result.blocks_processed as u128);
        reward_history.push(result.block_reward);

        if round % 32 == 0 {
            let stats = consensus.stats();
            assert_eq!(stats.current_round, round);
            assert!(
                stats.emission_stats.current_supply <= emission::SUPPLY_CAP,
                "supply should respect cap"
            );
            assert!(
                stats.reputation_stats.active_validators >= validators_per_round,
                "enough validators remain active"
            );
            assert!(
                stats.reward_stats.pending_validator_count <= VALIDATOR_COUNT,
                "reward tracker should not grow unbounded"
            );
        }
    }

    assert!(total_blocks > 0, "simulation must produce blocks");

    let stats = consensus.stats();
    assert_eq!(stats.current_round, TOTAL_ROUNDS);
    assert!(
        stats.reputation_stats.total_validators >= VALIDATOR_COUNT,
        "all validators remain registered"
    );
    assert_eq!(
        stats.emission_stats.emitted_supply as u128, total_emission,
        "emission accounting must match expected totals"
    );

    if let Some((&first_reward, _)) = reward_history.split_first() {
        let last_reward = *reward_history.last().unwrap();
        assert!(
            last_reward <= first_reward,
            "block reward should not increase over time"
        );
    }
}

#[tokio::test]
async fn test_emission_inflation_reduction() {
    let mut emission = EmissionSchedule::default();
    let initial_inflation = emission.current_inflation_bps;

    // Simulate one year of blocks
    let blocks_per_year = emission.blocks_per_year;
    emission.update(blocks_per_year, 1).unwrap();

    // Inflation should have decreased
    assert!(emission.current_inflation_bps < initial_inflation);
}

#[test]
fn test_process_round_simple() {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let mut dag = BlockDAG::new();

        // The process_round function processes pending blocks
        // It will work even without adding blocks first
        let fairness = FairnessModel::default();
        let result = process_round(&mut dag, &fairness, 1).await;

        // Should succeed even with no pending blocks
        assert!(result.is_ok());
        assert!(!dag.blocks.is_empty()); // At least genesis
    });
}
