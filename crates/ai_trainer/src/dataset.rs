//! CSV dataset loading and preprocessing
//!
//! Reads validator telemetry exported as deterministic CSV. All values must be
//! integers that are already scaled to the runtime SCALE (1_000_000 for micros)
//! and each row must be sorted by `(validator_id, timestamp)`.

use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::path::Path;

use crate::deterministic::xxhash64_i64;

/// Dataset columns required for training.
pub static DATASET_COLUMNS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        "validator_id",
        "timestamp",
        "uptime_micros",
        "latency_micros",
        "votes_cast",
        "votes_missed",
        "stake_atomic",
        "label",
    ]
});

/// Feature columns used as model inputs (in this specific order).
pub static FEATURE_COLUMNS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        "uptime_micros",
        "latency_micros",
        "votes_cast",
        "votes_missed",
        "stake_atomic",
    ]
});

/// Training dataset with integer features and targets
#[derive(Clone, Debug)]
pub struct Dataset {
    pub features: Vec<Vec<i64>>,
    pub targets: Vec<i64>,
    pub feature_count: usize,
    pub feature_names: Vec<String>,
}

/// Per-feature statistics used for diagnostics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeatureStats {
    pub name: String,
    pub min: i64,
    pub max: i64,
}

impl Dataset {
    /// Load dataset from CSV file with a deterministic header.
    ///
    /// Expected header columns:
    /// `validator_id,timestamp,uptime_micros,latency_micros,votes_cast,votes_missed,stake_atomic,label`
    pub fn from_csv<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref()).context("Failed to read CSV file")?;

        let mut lines = content.lines().enumerate();
        let mut header: Option<Vec<String>> = None;

        // Find the first non-empty, non-comment line as header.
        while let Some((_, raw_line)) = lines.next() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            header = Some(
                line.split(',')
                    .map(|part| part.trim().to_string())
                    .collect(),
            );
            break;
        }

        let header = header.ok_or_else(|| anyhow::anyhow!("Dataset is missing a header row"))?;
        let mut header_map = HashMap::new();
        for (idx, name) in header.iter().enumerate() {
            header_map.insert(name.as_str(), idx);
        }

        for col in DATASET_COLUMNS.iter() {
            if !header_map.contains_key(col) {
                anyhow::bail!("Missing required column '{col}' in dataset header");
            }
        }

        let feature_indices: Vec<usize> = FEATURE_COLUMNS
            .iter()
            .map(|name| header_map[name])
            .collect();
        let label_index = header_map["label"];
        let validator_index = header_map["validator_id"];
        let timestamp_index = header_map["timestamp"];

        let mut features = Vec::new();
        let mut targets = Vec::new();
        let mut prev_key: Option<(i64, i64)> = None;

        for (line_idx, raw_line) in lines {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            if parts.len() != header.len() {
                anyhow::bail!(
                    "Line {}: expected {} columns, found {}",
                    line_idx + 1,
                    header.len(),
                    parts.len()
                );
            }

            let parse = |idx: usize| -> Result<i64> {
                parts[idx].parse::<i64>().with_context(|| {
                    format!(
                        "Line {} column {} ('{}') is not an integer",
                        line_idx + 1,
                        idx + 1,
                        parts[idx]
                    )
                })
            };

            let validator_id = parse(validator_index)?;
            let timestamp = parse(timestamp_index)?;

            if let Some(prev) = prev_key {
                if (validator_id, timestamp) < prev {
                    anyhow::bail!(
                        "Line {}: dataset must be sorted by validator_id,timestamp",
                        line_idx + 1
                    );
                }
            }
            prev_key = Some((validator_id, timestamp));

            let mut row_features = Vec::with_capacity(feature_indices.len());
            for &idx in &feature_indices {
                row_features.push(parse(idx)?);
            }

            let label = parse(label_index)?;
            targets.push(label);
            features.push(row_features);
        }

        if features.is_empty() {
            anyhow::bail!("Dataset is empty");
        }

        Ok(Self {
            feature_count: feature_indices.len(),
            feature_names: FEATURE_COLUMNS.iter().map(|s| s.to_string()).collect(),
            features,
            targets,
        })
    }

    /// Deterministically shuffle the dataset using seed
    pub fn shuffle(&mut self, seed: i64) {
        let n = self.features.len();

        // Create index array with deterministic hash-based ordering
        let mut indices: Vec<(i64, usize)> = (0..n)
            .map(|i| {
                let hash = xxhash64_i64(&self.features[i], seed);
                (hash, i)
            })
            .collect();

        // Sort by hash for deterministic ordering
        indices.sort_by_key(|(hash, _)| *hash);

        // Reorder features and targets
        let mut new_features = Vec::with_capacity(n);
        let mut new_targets = Vec::with_capacity(n);

        for (_, idx) in indices {
            new_features.push(self.features[idx].clone());
            new_targets.push(self.targets[idx]);
        }

        self.features = new_features;
        self.targets = new_targets;
    }

    /// Get number of samples
    pub fn len(&self) -> usize {
        self.features.len()
    }

    /// Check if dataset is empty
    pub fn is_empty(&self) -> bool {
        self.features.is_empty()
    }

    /// Get feature statistics for validation
    pub fn feature_stats(&self) -> Vec<FeatureStats> {
        let mut mins = vec![i64::MAX; self.feature_count];
        let mut maxs = vec![i64::MIN; self.feature_count];

        for row in &self.features {
            for (idx, &val) in row.iter().enumerate() {
                mins[idx] = mins[idx].min(val);
                maxs[idx] = maxs[idx].max(val);
            }
        }

        self.feature_names
            .iter()
            .enumerate()
            .map(|(idx, name)| FeatureStats {
                name: name.clone(),
                min: mins[idx],
                max: maxs[idx],
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_csv() -> Result<NamedTempFile> {
        let mut file = NamedTempFile::new()?;
        writeln!(
            file,
            "validator_id,timestamp,uptime_micros,latency_micros,votes_cast,votes_missed,stake_atomic,label"
        )?;
        writeln!(file, "1,100,100,20,10,0,500,1000")?;
        writeln!(file, "1,200,150,25,12,1,500,1100")?;
        writeln!(file, "2,100,200,30,15,0,600,1200")?;
        file.flush()?;
        Ok(file)
    }

    #[test]
    fn test_load_csv() -> Result<()> {
        let file = create_test_csv()?;
        let dataset = Dataset::from_csv(file.path())?;

        assert_eq!(dataset.len(), 3);
        assert_eq!(dataset.feature_count, FEATURE_COLUMNS.len());
        assert_eq!(dataset.features[0], vec![100, 20, 10, 0, 500]);
        assert_eq!(dataset.targets[0], 1000);

        Ok(())
    }

    #[test]
    fn test_shuffle_determinism() -> Result<()> {
        let file = create_test_csv()?;
        let mut ds1 = Dataset::from_csv(file.path())?;
        let mut ds2 = ds1.clone();

        ds1.shuffle(42);
        ds2.shuffle(42);

        assert_eq!(ds1.features, ds2.features);
        assert_eq!(ds1.targets, ds2.targets);

        Ok(())
    }

    #[test]
    fn test_feature_stats() -> Result<()> {
        let file = create_test_csv()?;
        let dataset = Dataset::from_csv(file.path())?;

        let stats = dataset.feature_stats();
        assert_eq!(stats.len(), FEATURE_COLUMNS.len());
        assert_eq!(
            stats[0],
            FeatureStats {
                name: "uptime_micros".into(),
                min: 100,
                max: 200
            }
        );

        Ok(())
    }
}
