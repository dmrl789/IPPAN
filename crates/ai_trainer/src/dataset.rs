//! CSV dataset loading and preprocessing
//!
//! Reads integer-only datasets (already scaled to SCALE = 1_000_000)
//! and provides deterministic shuffling and splitting.

use anyhow::{Context, Result};
use std::path::Path;

use crate::deterministic::xxhash64_i64;

/// Training dataset with integer features and targets
#[derive(Clone, Debug)]
pub struct Dataset {
    pub features: Vec<Vec<i64>>,
    pub targets: Vec<i64>,
    pub feature_count: usize,
}

impl Dataset {
    /// Load dataset from CSV file
    /// Expected format: feature1,feature2,...,target
    /// All values must be integers (pre-scaled)
    pub fn from_csv<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .context("Failed to read CSV file")?;

        let mut features = Vec::new();
        let mut targets = Vec::new();
        let mut feature_count = 0;

        for (line_idx, line) in content.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            if parts.len() < 2 {
                anyhow::bail!("Line {}: expected at least 2 columns", line_idx + 1);
            }

            if feature_count == 0 {
                feature_count = parts.len() - 1;
            } else if parts.len() - 1 != feature_count {
                anyhow::bail!(
                    "Line {}: expected {} features, got {}",
                    line_idx + 1,
                    feature_count,
                    parts.len() - 1
                );
            }

            let mut row_features = Vec::with_capacity(feature_count);
            for (i, part) in parts.iter().take(feature_count).enumerate() {
                let val = part.parse::<i64>()
                    .with_context(|| format!("Line {}, column {}: invalid integer", line_idx + 1, i + 1))?;
                row_features.push(val);
            }

            let target = parts[feature_count].parse::<i64>()
                .with_context(|| format!("Line {}: invalid target", line_idx + 1))?;

            features.push(row_features);
            targets.push(target);
        }

        if features.is_empty() {
            anyhow::bail!("Dataset is empty");
        }

        Ok(Self {
            features,
            targets,
            feature_count,
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
    pub fn feature_stats(&self) -> Vec<(i64, i64)> {
        let mut stats = vec![(i64::MAX, i64::MIN); self.feature_count];

        for row in &self.features {
            for (i, &val) in row.iter().enumerate() {
                stats[i].0 = stats[i].0.min(val);
                stats[i].1 = stats[i].1.max(val);
            }
        }

        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_csv() -> Result<NamedTempFile> {
        let mut file = NamedTempFile::new()?;
        writeln!(file, "100,200,300,1")?;
        writeln!(file, "150,250,350,2")?;
        writeln!(file, "200,300,400,3")?;
        file.flush()?;
        Ok(file)
    }

    #[test]
    fn test_load_csv() -> Result<()> {
        let file = create_test_csv()?;
        let dataset = Dataset::from_csv(file.path())?;

        assert_eq!(dataset.len(), 3);
        assert_eq!(dataset.feature_count, 3);
        assert_eq!(dataset.features[0], vec![100, 200, 300]);
        assert_eq!(dataset.targets[0], 1);

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
        assert_eq!(stats.len(), 3);
        assert_eq!(stats[0], (100, 200)); // min, max for feature 0
        assert_eq!(stats[1], (200, 300));
        assert_eq!(stats[2], (300, 400));

        Ok(())
    }
}
