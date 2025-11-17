use anyhow::Result;
use ippan_ai_trainer::{train_model_from_csv, TrainingParams};
use std::io::Write;
use tempfile::NamedTempFile;

fn write_dataset() -> Result<NamedTempFile> {
    let mut file = NamedTempFile::new()?;
    writeln!(
        file,
        "validator_id,timestamp,uptime_micros,latency_micros,votes_cast,votes_missed,stake_atomic,label"
    )?;
    writeln!(file, "1,100,90000000,120000,100,0,500000000000,820000")?;
    writeln!(file, "1,200,91000000,110000,120,2,500000000000,840000")?;
    writeln!(file, "2,100,87000000,150000,80,5,450000000000,780000")?;
    writeln!(file, "2,200,88000000,140000,90,3,450000000000,800000")?;
    file.flush()?;
    Ok(file)
}

#[test]
fn deterministic_training() -> Result<()> {
    let dataset = write_dataset()?;
    let params = TrainingParams {
        tree_count: 3,
        max_depth: 2,
        min_samples_leaf: 1,
        learning_rate_micro: 100_000,
        quantization_step: 1_000,
    };

    let model_a = train_model_from_csv(dataset.path(), params.clone())?;
    let model_b = train_model_from_csv(dataset.path(), params)?;
    assert_eq!(model_a, model_b);
    Ok(())
}
