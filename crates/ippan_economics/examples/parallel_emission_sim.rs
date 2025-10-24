//! Parallel DAG-Fair Emission Simulation for IPPAN
//!
//! Simulates 10 000 rounds of deterministic emission and validator participation
//! using Rayon. Produces `emission_data.csv` and (optionally) a PNG emission curve
//! if built with `--features plotters`.

use ippan_economics::*;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use rayon::prelude::*;
use std::{collections::HashMap, fs::File, io::Write, sync::Mutex};

const ROUNDS: u64 = 10_000;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting IPPAN parallel emission simulation over {ROUNDS} rounds");

    let mut params = EconomicsParams::default();
    params.halving_interval_rounds = 2_000; // halving every 2000 rounds for demo

    let balances = Mutex::new(HashMap::<ValidatorId, MicroIPN>::new());
    let mut total_issued: MicroIPN = 0;
    let mut total_burned: MicroIPN = 0;

    let validators: Vec<ValidatorId> = (0..10)
        .map(|i| ValidatorId(format!("@validator{}.ipn", i + 1)))
        .collect();

    let mut csv = File::create("emission_data.csv")?;
    writeln!(csv, "round,emission_micro,total_supply_micro,halving_index")?;

    // Simulate in parallel using Rayon
    let results: Vec<_> = (0..ROUNDS)
        .into_par_iter()
        .map_init(
            || StdRng::seed_from_u64(rand::random()),
            |rng, round| simulate_round(rng, round, &validators, &params),
        )
        .collect();

    for res in results {
        total_issued = total_issued.saturating_add(res.emission_paid);
        for (vid, amt) in res.payouts {
            *balances.lock().unwrap().entry(vid).or_default() += amt;
        }
        writeln!(
            csv,
            "{},{},{},{}",
            res.round, res.emission_paid, total_issued, res.halving_index
        )?;
        total_burned = total_burned.saturating_add(res.burned);
    }

    println!(
        "✅ Simulation complete: issued={} μIPN (≈ {:.6} IPN), burned={} μIPN",
        total_issued,
        total_issued as f64 / MICRO_PER_IPN as f64,
        total_burned
    );

    analyze_fairness(&balances.lock().unwrap());
    try_plot()?;
    Ok(())
}

#[derive(Debug)]
struct RoundResult {
    round: u64,
    emission_paid: MicroIPN,
    halving_index: u64,
    payouts: Payouts,
    burned: MicroIPN,
}

fn simulate_round(
    rng: &mut StdRng,
    round: u64,
    validators: &[ValidatorId],
    params: &EconomicsParams,
) -> RoundResult {
    // Compute deterministic emission
    let emission_micro = emission_for_round(round, params);
    let fees_micro = rng.gen_range(0..=10); // random small fee

    // Generate synthetic participation
    let mut parts = ParticipationSet::default();
    for vid in validators {
        let role = if rng.gen_bool(0.3) { Role::Proposer } else { Role::Verifier };
        let blocks = rng.gen_range(1..=3);
        parts.insert(vid.clone(), Participation { role, blocks });
    }

    // Distribute rewards
    let (payouts, emission_paid, _fees_paid) =
        distribute_round(emission_micro, fees_micro, &parts, params)
            .unwrap_or_default();

    let halving_index = round / params.halving_interval_rounds;
    let burned = emission_micro.saturating_sub(emission_paid);

    RoundResult {
        round,
        emission_paid,
        halving_index,
        payouts,
        burned,
    }
}

fn analyze_fairness(balances: &HashMap<ValidatorId, MicroIPN>) {
    if balances.is_empty() {
        println!("⚠️  No validator data");
        return;
    }
    let min = balances.values().min().copied().unwrap_or(0);
    let max = balances.values().max().copied().unwrap_or(0);
    let avg = balances.values().sum::<MicroIPN>() as f64 / balances.len() as f64;

    println!("⚖️  Validator reward distribution:");
    println!("   min={} μIPN, max={} μIPN, avg={avg:.1} μIPN", min, max);
    println!("   fairness ratio = {:.2}× (max/min)", (max as f64) / (min.max(1) as f64));
}

#[cfg(feature = "plotters")]
fn try_plot() -> Result<(), Box<dyn std::error::Error>> {
    use plotters::prelude::*;
    use plotters::style::RGBColor;

    if std::panic::catch_unwind(|| -> Result<(), Box<dyn std::error::Error>> {
        let path = "emission_curve.png";
        let mut buf = vec![0u8; (1200 * 600 * 3) as usize];
        {
            let root = BitMapBackend::with_buffer(&mut buf, (1200, 600)).into_drawing_area();
            root.fill(&WHITE)?;
            let mut reader = csv::Reader::from_path("emission_data.csv")?;
            let data: Vec<(f64, f64)> = reader
                .records()
                .filter_map(|r| r.ok())
                .filter_map(|r| {
                    let round = r[0].parse::<f64>().ok()?;
                    let emission = r[1].parse::<f64>().ok()?;
                    Some((round, emission))
                })
                .collect();

            let y_max = data.iter().map(|(_, y)| *y).fold(0.0, f64::max);
            let mut chart = ChartBuilder::on(&root)
                .caption("IPPAN DAG-Fair Emission Curve", ("sans-serif", 24))
                .margin(20)
                .x_label_area_size(40)
                .y_label_area_size(60)
                .build_cartesian_2d(0f64..ROUNDS as f64, 0f64..y_max)?;

            chart.configure_mesh().x_desc("Round").y_desc("Emission (μIPN)").draw()?;
            chart.draw_series(LineSeries::new(data, &RGBColor(0, 0, 255)))?;
            root.present()?;
        }
        image::save_buffer(path, &buf, 1200, 600, image::ColorType::Rgb8)?;
        Ok(())
    })?
    .is_ok()
    {
        println!("📊 Chart saved → emission_curve.png");
    } else {
        println!("⚠️ Chart generation failed (CSV available in emission_data.csv)");
    }
    Ok(())
}

#[cfg(not(feature = "plotters"))]
fn try_plot() -> Result<(), Box<dyn std::error::Error>> {
    println!("(plot skipped — enable 'plotters' feature to generate PNG chart)");
    println!("(CSV data available in emission_data.csv)");
    Ok(())
}
