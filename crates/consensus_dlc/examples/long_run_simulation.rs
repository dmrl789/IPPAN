use ippan_consensus_dlc::{
    dag::Block, dgbdt::ValidatorMetrics, error::Result, hashtimer::HashTimer, init_dlc, DlcConfig,
    DlcConsensus,
};
use ippan_types::Amount;

/// Run a long-horizon DLC consensus simulation and print periodic metrics.
#[tokio::main]
async fn main() -> Result<()> {
    init_dlc();

    const TOTAL_ROUNDS: u64 = 1_024;
    const VALIDATOR_COUNT: usize = 48;

    let config = DlcConfig {
        validators_per_round: 17,
        unstaking_lock_rounds: 720,
        min_reputation: 2_500,
        ..DlcConfig::default()
    };

    let mut consensus = DlcConsensus::new(config);

    for i in 0..VALIDATOR_COUNT {
        // Use scaled integers (scaled by 10000): 9300 = 0.93, 150 = 0.015, etc.
        let uptime = 9300 + ((i % 11) as i64 * 40);
        let latency = 150 + ((i % 9) as i64 * 15);
        let honesty = 9000 + ((i % 13) as i64 * 50);

        let metrics = ValidatorMetrics::new(
            uptime.min(9990),
            latency,
            honesty.min(9990),
            75 + (i as u64 * 4),
            190 + (i as u64 * 7),
            Amount::from_micro_ipn(6_000_000 + (i as u64 * 200_000)),
            250 + i as u64,
        );

        let stake = Amount::from_ipn(12 + (i as u64 % 6));
        consensus.register_validator(format!("sim-val-{i:02}"), stake, metrics)?;
    }

    let mut total_blocks = 0u64;
    let mut total_emission = 0u128;

    for round in 1..=TOTAL_ROUNDS {
        // Synthesize a block proposal to keep the DAG active during the simulation
        let parent_ids = consensus
            .dag
            .get_tips()
            .into_iter()
            .map(|block| block.id.clone())
            .collect();
        let proposer = format!("sim-val-{:02}", (round as usize % VALIDATOR_COUNT));
        let mut block = Block::new(
            parent_ids,
            HashTimer::for_round(round),
            vec![round as u8],
            proposer,
        );
        block.sign(vec![1u8; 64]);
        consensus.dag.insert(block).ok();

        let result = consensus.process_round().await?;
        total_blocks += result.blocks_processed as u64;
        total_emission += (result.block_reward as u128) * (result.blocks_processed as u128);

        if round % 128 == 0 || round == TOTAL_ROUNDS {
            let stats = consensus.stats();
            println!(
                "[round {:>4}] blocks={:<3} reward={:<10}µ supply={:<12}µ pending={:<11}µ inflation={}bps",
                stats.current_round,
                result.blocks_processed,
                result.block_reward,
                stats.emission_stats.current_supply,
                stats.reward_stats.total_pending,
                stats.emission_stats.current_inflation_bps,
            );
        }
    }

    let stats = consensus.stats();
    println!("\n=== DLC Long-Run Simulation Summary ===");
    println!("Rounds processed: {}", stats.current_round);
    println!(
        "Validators registered: {}",
        stats.reputation_stats.total_validators
    );
    println!(
        "Active validators: {}",
        stats.reputation_stats.active_validators
    );
    println!("Blocks processed: {}", total_blocks);
    println!(
        "Emission produced (µIPN): {}",
        stats.emission_stats.emitted_supply
    );
    println!(
        "Pending rewards (µIPN): {}",
        stats.reward_stats.total_pending
    );
    println!(
        "Total distributed rewards (µIPN): {}",
        stats.reward_stats.total_distributed
    );
    println!(
        "Emission progress: {:.2}%",
        stats.emission_stats.emission_progress_bps as f64 / 100.0 // Convert BPS to percentage
    );
    println!(
        "Tracked reward entries: {}",
        stats.reward_stats.pending_validator_count
    );
    println!("Simulation emission tally (µIPN): {total_emission}");

    if total_blocks == 0 {
        println!("Warning: no blocks were produced during the simulation.");
    }

    Ok(())
}
