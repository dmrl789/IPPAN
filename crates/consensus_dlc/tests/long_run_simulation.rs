#![allow(deprecated)]

use anyhow::Result;
use ippan_consensus_dlc::{
    bond,
    dag::{Block, BlockDAG},
    dgbdt::ValidatorMetrics,
    hashtimer::HashTimer,
    verifier::{VerifiedBlock, VerifierSet},
    DlcConfig, DlcConsensus,
};
use ippan_types::Amount;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::collections::{HashMap, HashSet};
use tokio::sync::mpsc;
use tokio::task::yield_now;

const TOTAL_ROUNDS: u64 = 1_024;
const INITIAL_VALIDATORS: usize = 24;

#[derive(Debug, Clone)]
enum SimulationEvent {
    JoinValidator,
    LeaveValidator,
    NetworkSplit {
        double_sign_once: bool,
        duration: u64,
    },
    HealNetwork,
}

#[derive(Debug, Clone)]
struct SplitState {
    rounds_remaining: u64,
    double_sign_once: bool,
    double_sign_consumed: bool,
}

impl SplitState {
    fn new(double_sign_once: bool, duration: u64) -> Self {
        Self {
            rounds_remaining: duration.max(1),
            double_sign_once,
            double_sign_consumed: false,
        }
    }
}

#[derive(Default)]
struct SimulationLogs {
    finality: Vec<String>,
    convergence: Vec<String>,
    slashing: Vec<String>,
}

#[derive(Debug)]
struct RoundSummary {
    round: u64,
}

#[derive(Clone)]
struct BlockPlan {
    parents: Vec<String>,
    proposer: String,
    payload: Vec<u8>,
}

impl BlockPlan {
    fn new(parents: Vec<String>, proposer: String, payload_tag: String) -> Self {
        Self {
            parents,
            proposer,
            payload: payload_tag.into_bytes(),
        }
    }
}

struct SplitRoundContext {
    double_sign_round: bool,
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn long_run_dlc_handles_churn_and_network_partitions() -> Result<()> {
    let mut consensus = DlcConsensus::new(DlcConfig {
        validators_per_round: 11,
        min_validator_stake: bond::MIN_VALIDATOR_BOND,
        unstaking_lock_rounds: 32,
        min_reputation: 4_500,
        enable_slashing: true,
    });

    let mut rng = StdRng::seed_from_u64(0xD1CE_F00D);
    let mut next_validator_id = 0u64;
    let mut active_validators = HashSet::new();
    let mut logs = SimulationLogs::default();

    for _ in 0..INITIAL_VALIDATORS {
        if let Some(id) = register_validator(
            &mut consensus,
            &mut rng,
            &mut next_validator_id,
            &mut active_validators,
            &mut logs,
        ) {
            logs.convergence.push(format!("bootstrapped {}", id));
        }
    }

    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    spawn_validator_churn(event_tx.clone(), 0xABCD_EF01);
    spawn_network_splits(event_tx.clone(), 0xFACE_CAFE);
    let _ = event_tx.send(SimulationEvent::NetworkSplit {
        double_sign_once: true,
        duration: 12,
    });

    let mut split_state: Option<SplitState> = None;

    for _ in 0..TOTAL_ROUNDS {
        while let Ok(event) = event_rx.try_recv() {
            match event {
                SimulationEvent::JoinValidator => {
                    register_validator(
                        &mut consensus,
                        &mut rng,
                        &mut next_validator_id,
                        &mut active_validators,
                        &mut logs,
                    );
                }
                SimulationEvent::LeaveValidator => {
                    remove_random_validator(
                        &mut consensus,
                        &mut rng,
                        &mut active_validators,
                        &mut logs,
                    );
                }
                SimulationEvent::NetworkSplit {
                    double_sign_once,
                    duration,
                } => {
                    if split_state.is_none() {
                        split_state = Some(SplitState::new(double_sign_once, duration));
                        logs.convergence.push(format!(
                            "round {} split activated (double_sign={})",
                            consensus.current_round + 1,
                            double_sign_once
                        ));
                    }
                }
                SimulationEvent::HealNetwork => {
                    if split_state.take().is_some() {
                        logs.convergence.push(format!(
                            "round {} split healed by event",
                            consensus.current_round + 1
                        ));
                    }
                }
            }
        }

        let summary =
            process_round_with_behaviour(&mut consensus, split_state.as_mut(), &mut logs, &mut rng)
                .await?;

        if let Some(state) = split_state.as_mut() {
            if state.rounds_remaining > 0 {
                state.rounds_remaining -= 1;
            }
            if state.rounds_remaining == 0 {
                split_state = None;
                logs.convergence
                    .push(format!("round {} split expired", summary.round));
            }
        }

        if summary.round % 16 == 0 {
            drift_validator_metrics(&mut consensus, &mut rng);
        }
    }

    let stats = consensus.stats();
    let dag_stats = stats.dag_stats;
    assert!(
        dag_stats.finalized_blocks > 0,
        "expected at least one finalized block after long-run simulation"
    );
    assert!(
        dag_stats.tips_count <= 2,
        "expected at most 2 tips after long run, got {}",
        dag_stats.tips_count
    );
    assert!(
        stats.bond_stats.total_slashed > Amount::zero(),
        "expected at least one slashing event"
    );
    assert!(
        logs.finality
            .iter()
            .any(|entry| entry.contains("finalized")),
        "finality log should record round finalization events"
    );
    assert!(
        logs.slashing
            .iter()
            .any(|entry| entry.contains("double-sign")),
        "slashing log should include double-sign detection"
    );

    Ok(())
}

fn spawn_validator_churn(tx: mpsc::UnboundedSender<SimulationEvent>, seed: u64) {
    tokio::spawn(async move {
        let mut rng = StdRng::seed_from_u64(seed);
        for _ in 0..512 {
            if rng.gen_bool(0.35) {
                let _ = tx.send(SimulationEvent::JoinValidator);
            }
            if rng.gen_bool(0.30) {
                let _ = tx.send(SimulationEvent::LeaveValidator);
            }
            yield_now().await;
        }
    });
}

fn spawn_network_splits(tx: mpsc::UnboundedSender<SimulationEvent>, seed: u64) {
    tokio::spawn(async move {
        let mut rng = StdRng::seed_from_u64(seed);
        for _ in 0..128 {
            if rng.gen_bool(0.40) {
                let duration = rng.gen_range(6..=20);
                let double_sign = rng.gen_bool(0.55);
                let _ = tx.send(SimulationEvent::NetworkSplit {
                    double_sign_once: double_sign,
                    duration,
                });
            }
            if rng.gen_bool(0.50) {
                let _ = tx.send(SimulationEvent::HealNetwork);
            }
            yield_now().await;
        }
    });
}

async fn process_round_with_behaviour(
    consensus: &mut DlcConsensus,
    split_state: Option<&mut SplitState>,
    logs: &mut SimulationLogs,
    rng: &mut StdRng,
) -> Result<RoundSummary> {
    let round = consensus.current_round + 1;
    consensus.current_round = round;

    let round_time = HashTimer::for_round(round);
    let seed = round_time.hash.clone();
    let verifier_set = consensus.validators.select_for_round(seed, round)?;

    let split_context = split_state.map(|state| {
        let double_sign_round = state.double_sign_once && !state.double_sign_consumed;
        if double_sign_round {
            state.double_sign_consumed = true;
        }
        SplitRoundContext { double_sign_round }
    });

    let plans = assemble_block_plans(
        &consensus.dag,
        &verifier_set,
        split_context.as_ref(),
        rng,
        round,
    );
    let blocks = produce_blocks_async(plans, round).await?;

    let mut verified_blocks = Vec::new();
    let mut proposer_blocks: HashMap<String, Vec<String>> = HashMap::new();
    let mut slashed_this_round = HashSet::new();

    for block in blocks {
        match verifier_set.validate(&block) {
            Ok(()) => {
                consensus.dag.insert(block.clone())?;
                proposer_blocks
                    .entry(block.proposer.clone())
                    .or_default()
                    .push(block.id.clone());

                let verified = VerifiedBlock::new(block.clone(), verifier_set.all_verifiers());
                verified_blocks.push(verified.clone());

                if !block.is_genesis() {
                    let _ = consensus.reputation.reward_proposal(&block.proposer, round);
                    for validator in &verified.verified_by {
                        let _ = consensus.reputation.reward_verification(validator, round);
                    }
                }

                if proposer_blocks
                    .get(&block.proposer)
                    .map(|ids| ids.len() > 1)
                    .unwrap_or(false)
                    && slashed_this_round.insert(block.proposer.clone())
                {
                    if let Ok(amount) = consensus.bonds.slash_validator(
                        &block.proposer,
                        "double-sign detected".to_string(),
                        bond::DOUBLE_SIGN_SLASH_BPS,
                        round,
                    ) {
                        logs.slashing.push(format!(
                            "round {} slashed {} for double-sign: {}",
                            round, block.proposer, amount
                        ));
                        let _ = consensus
                            .reputation
                            .penalize_invalid_proposal(&block.proposer, round);
                    }
                }
            }
            Err(err) => {
                logs.convergence.push(format!(
                    "round {} rejected block from {}: {}",
                    round, block.proposer, err
                ));
                if consensus.config.enable_slashing {
                    let _ = consensus
                        .reputation
                        .penalize_invalid_proposal(&block.proposer, round);
                    let _ = consensus.bonds.slash_validator(
                        &block.proposer,
                        "invalid block".to_string(),
                        bond::INVALID_BLOCK_SLASH_BPS,
                        round,
                    );
                }
            }
        }
    }

    let finalized_ids = consensus.dag.finalize_round(round_time.clone());
    if !finalized_ids.is_empty() && (round % 8 == 0 || finalized_ids.len() > 1) {
        logs.finality.push(format!(
            "round {} finalized {} blocks",
            round,
            finalized_ids.len()
        ));
    }

    let block_reward = consensus.emission.calculate_block_reward(round);
    if !verified_blocks.is_empty() && block_reward > 0 {
        for verified in &verified_blocks {
            if let Ok(distribution) = consensus.rewards.distribute_block_reward(
                block_reward,
                &verified.block.proposer,
                &verified.verified_by,
            ) {
                if round % 64 == 0 {
                    logs.finality.push(format!(
                        "round {} rewards {} total {}",
                        round, verified.block.proposer, distribution.total_distributed
                    ));
                }
            }
        }
    }

    consensus
        .emission
        .update(round, verified_blocks.len() as u64)?;

    let dag_stats = consensus.dag.stats();
    if round % 32 == 0 {
        logs.finality.push(format!(
            "round {} finality snapshot finalized={}, tips={}, pending={}",
            round, dag_stats.finalized_blocks, dag_stats.tips_count, dag_stats.pending_blocks
        ));
    }
    if round % 16 == 0 {
        logs.convergence.push(format!(
            "round {} tips={}, finalized={}, pending={}",
            round, dag_stats.tips_count, dag_stats.finalized_blocks, dag_stats.pending_blocks
        ));
    }

    Ok(RoundSummary { round })
}

fn assemble_block_plans(
    dag: &BlockDAG,
    verifier_set: &VerifierSet,
    split_context: Option<&SplitRoundContext>,
    rng: &mut StdRng,
    round: u64,
) -> Vec<BlockPlan> {
    let mut plans = Vec::new();
    let primary_parent = select_heaviest_tip(dag);
    plans.push(BlockPlan::new(
        vec![primary_parent.clone()],
        verifier_set.primary.clone(),
        format!("primary-{}-{}", round, rng.gen::<u64>()),
    ));

    if let Some(ctx) = split_context {
        if ctx.double_sign_round {
            plans.push(BlockPlan::new(
                vec![primary_parent.clone()],
                verifier_set.primary.clone(),
                format!("double-sign-{}-{}", round, rng.gen::<u64>()),
            ));
        } else if let Some(shadow) = verifier_set.shadows.first() {
            plans.push(BlockPlan::new(
                vec![primary_parent.clone()],
                shadow.clone(),
                format!("shadow-branch-{}-{}", round, rng.gen::<u64>()),
            ));
        }
    } else {
        let tips = dag.get_tips();
        if tips.len() > 1 {
            let parents = tips.iter().map(|b| b.id.clone()).collect::<Vec<_>>();
            let proposer = verifier_set
                .shadows
                .first()
                .cloned()
                .unwrap_or_else(|| verifier_set.primary.clone());
            plans.push(BlockPlan::new(
                parents,
                proposer,
                format!("merge-{}-{}", round, rng.gen::<u64>()),
            ));
        }
    }

    plans
}

async fn produce_blocks_async(plans: Vec<BlockPlan>, round: u64) -> Result<Vec<Block>> {
    let mut handles = Vec::with_capacity(plans.len());
    for plan in plans {
        handles.push(tokio::spawn(async move {
            yield_now().await;
            let mut block = Block::new(
                plan.parents,
                HashTimer::for_round(round),
                plan.payload,
                plan.proposer,
            );
            block.sign(vec![0u8; 64]);
            block
        }));
    }

    let mut blocks = Vec::with_capacity(handles.len());
    for handle in handles {
        blocks.push(handle.await.expect("block task panicked"));
    }
    Ok(blocks)
}

fn select_heaviest_tip(dag: &BlockDAG) -> String {
    dag.get_tips()
        .into_iter()
        .max_by_key(|block| block.height)
        .map(|block| block.id.clone())
        .or_else(|| dag.genesis_id.clone())
        .expect("DAG must have at least genesis")
}

fn register_validator(
    consensus: &mut DlcConsensus,
    rng: &mut StdRng,
    next_validator_id: &mut u64,
    active_validators: &mut HashSet<String>,
    logs: &mut SimulationLogs,
) -> Option<String> {
    let id = format!("val-{}", next_validator_id);
    *next_validator_id += 1;

    let stake_micro = rng.gen_range(10_000_000u64..=60_000_000u64);
    let stake = Amount::from_micro_ipn(stake_micro);
    let metrics = random_metrics(rng, stake_micro);

    match consensus.register_validator(id.clone(), stake, metrics) {
        Ok(()) => {
            active_validators.insert(id.clone());
            logs.convergence.push(format!("registered {}", id));
            Some(id)
        }
        Err(err) => {
            logs.convergence
                .push(format!("failed to register {}: {}", id, err));
            None
        }
    }
}

fn remove_random_validator(
    consensus: &mut DlcConsensus,
    rng: &mut StdRng,
    active_validators: &mut HashSet<String>,
    logs: &mut SimulationLogs,
) {
    if active_validators.len() <= consensus.config.validators_per_round {
        return;
    }

    if let Some(id) = pick_random_validator(active_validators, rng) {
        if consensus.validators.remove_validator(&id).is_ok() {
            let _ = consensus
                .bonds
                .initiate_unstaking(&id, consensus.current_round);
            let _ = consensus.bonds.complete_unstaking(
                &id,
                consensus.current_round + consensus.config.unstaking_lock_rounds,
            );
            consensus.reputation.scores.remove(&id);
            active_validators.remove(&id);
            logs.convergence.push(format!("removed {}", id));
        }
    }
}

fn pick_random_validator(validators: &HashSet<String>, rng: &mut StdRng) -> Option<String> {
    if validators.is_empty() {
        return None;
    }
    let idx = rng.gen_range(0..validators.len());
    validators.iter().nth(idx).cloned()
}

fn random_metrics(rng: &mut StdRng, stake_micro: u64) -> ValidatorMetrics {
    let uptime = rng.gen_range(0.92..=1.0);
    let latency = rng.gen_range(0.01..=0.18);
    let honesty = rng.gen_range(0.88..=1.0);
    let blocks_proposed = rng.gen_range(50..=400);
    let blocks_verified = rng.gen_range(blocks_proposed..=(blocks_proposed + 200));
    let rounds_active = rng.gen_range(32..=1_024);

    ValidatorMetrics::from_floats(
        uptime,
        latency,
        honesty,
        blocks_proposed,
        blocks_verified,
        Amount::from_micro_ipn(stake_micro),
        rounds_active,
    )
}

fn drift_validator_metrics(consensus: &mut DlcConsensus, rng: &mut StdRng) {
    let validator_ids: Vec<String> = consensus
        .validators
        .all_validators()
        .keys()
        .cloned()
        .collect();
    if validator_ids.is_empty() {
        return;
    }

    for id in validator_ids.iter().take(8) {
        if let Some(metrics) = consensus.validators.get_validator(id).cloned() {
            let mut updated = metrics.clone();
            let uptime_delta = rng.gen_range(0.85..=1.0);
            let latency_sample = rng.gen_range(0.01..=0.25);
            let proposed_delta = rng.gen_range(0..=2);
            let verified_delta = rng.gen_range(0..=3);
            // Convert to scaled integers for update
            let uptime_delta_scaled = (uptime_delta * 10000.0) as i64;
            let latency_sample_scaled = (latency_sample * 10000.0) as i64;
            updated.update(uptime_delta_scaled, latency_sample_scaled, proposed_delta, verified_delta);
            let _ = consensus.validators.update_validator(id, updated);
        }
    }
}
