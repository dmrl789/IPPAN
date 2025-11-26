//! Deterministic feature engineering for GBDT inference.
//!
//! This module operates exclusively on fixed-point arithmetic to guarantee
//! reproducible behaviour across architectures.

use crate::{
    fixed::{Fixed, SCALE},
    gbdt_legacy::{FeatureNormalization, GBDTError},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, instrument, warn};

/// Feature engineering configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureEngineeringConfig {
    pub enable_feature_engineering: bool,
    pub enable_normalization: bool,
    pub enable_scaling: bool,
    pub enable_feature_selection: bool,
    pub max_features: usize,
    pub feature_selection_threshold: Fixed,
    pub enable_outlier_detection: bool,
    pub outlier_threshold: Fixed,
    pub enable_feature_interactions: bool,
    pub max_interaction_depth: usize,
}

impl Default for FeatureEngineeringConfig {
    fn default() -> Self {
        Self {
            enable_feature_engineering: true,
            enable_normalization: true,
            enable_scaling: true,
            enable_feature_selection: true,
            max_features: 1000,
            feature_selection_threshold: Fixed::from_micro(10_000),
            enable_outlier_detection: true,
            outlier_threshold: Fixed::from_int(3),
            enable_feature_interactions: false,
            max_interaction_depth: 2,
        }
    }
}

/// Feature statistics used for normalization and scaling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureStatistics {
    pub means: Vec<Fixed>,
    pub std_devs: Vec<Fixed>,
    pub mins: Vec<Fixed>,
    pub maxs: Vec<Fixed>,
    pub medians: Vec<Fixed>,
    pub quartiles: Vec<[Fixed; 4]>,
    pub feature_counts: Vec<usize>,
    pub missing_counts: Vec<usize>,
}

/// Feature importance scores
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureImportance {
    pub feature_names: Vec<String>,
    pub importance_scores: Vec<Fixed>,
    pub selected_features: Vec<usize>,
    pub total_variance_explained: Fixed,
}

/// Raw deterministic feature data
#[derive(Debug, Clone)]
pub struct RawFeatureData {
    pub features: Vec<Vec<Fixed>>,
    pub feature_names: Vec<String>,
    pub sample_count: usize,
    pub feature_count: usize,
    pub metadata: HashMap<String, String>,
}

/// Processed deterministic integer features
#[derive(Debug, Clone)]
pub struct ProcessedFeatureData {
    pub features: Vec<Vec<i64>>,
    pub feature_names: Vec<String>,
    pub feature_importance: Option<FeatureImportance>,
    pub normalization: Option<FeatureNormalization>,
    pub statistics: FeatureStatistics,
    pub processing_time_ms: u64,
}

/// Main feature engineering pipeline
#[derive(Debug)]
pub struct FeatureEngineeringPipeline {
    config: FeatureEngineeringConfig,
    statistics: Option<FeatureStatistics>,
    feature_importance: Option<FeatureImportance>,
    normalization: Option<FeatureNormalization>,
}

impl FeatureEngineeringPipeline {
    pub fn new(config: FeatureEngineeringConfig) -> Self {
        Self {
            config,
            statistics: None,
            feature_importance: None,
            normalization: None,
        }
    }

    /// Fit pipeline on raw training data
    #[instrument(skip(self, data))]
    pub async fn fit(&mut self, data: &RawFeatureData) -> Result<(), GBDTError> {
        let start = std::time::Instant::now();
        info!(
            "Fitting feature pipeline on {} samples × {} features",
            data.sample_count, data.feature_count
        );

        let stats = self.calculate_statistics(data)?;

        if self.config.enable_outlier_detection {
            self.handle_outliers(&stats, data);
        }

        let importance = if self.config.enable_feature_selection {
            Some(self.calculate_feature_importance(&stats, data))
        } else {
            None
        };

        let normalization = if self.config.enable_normalization {
            Some(self.calculate_normalization(&stats))
        } else {
            None
        };

        self.statistics = Some(stats);
        self.feature_importance = importance;
        self.normalization = normalization;

        info!(
            "Feature pipeline fitted in {} ms",
            start.elapsed().as_millis()
        );
        Ok(())
    }

    /// Transform new data deterministically
    #[instrument(skip(self, data))]
    pub async fn transform(
        &self,
        data: &RawFeatureData,
    ) -> Result<ProcessedFeatureData, GBDTError> {
        let start = std::time::Instant::now();
        let stats = self
            .statistics
            .as_ref()
            .ok_or_else(|| GBDTError::ModelValidationFailed {
                reason: "Pipeline not fitted (call fit first)".into(),
            })?;

        let mut processed = Vec::with_capacity(data.sample_count);

        for sample in &data.features {
            let mut row = Vec::with_capacity(sample.len());
            for (i, &value) in sample.iter().enumerate() {
                if i >= stats.means.len() {
                    return Err(GBDTError::FeatureSizeMismatch {
                        expected: stats.means.len(),
                        actual: sample.len(),
                    });
                }

                let normalized = if let Some(ref norm) = self.normalization {
                    self.normalize_value(value, i, norm)?
                } else {
                    value
                };

                row.push(self.scale_to_integer(normalized));
            }
            processed.push(row);
        }

        let (final_features, final_names) = if let Some(ref imp) = self.feature_importance {
            self.apply_feature_selection(&processed, &data.feature_names, imp)?
        } else {
            (processed, data.feature_names.clone())
        };

        Ok(ProcessedFeatureData {
            features: final_features,
            feature_names: final_names,
            feature_importance: self.feature_importance.clone(),
            normalization: self.normalization.clone(),
            statistics: stats.clone(),
            processing_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    fn calculate_statistics(&self, data: &RawFeatureData) -> Result<FeatureStatistics, GBDTError> {
        let n = data.feature_count;
        let mut sums = vec![Fixed::ZERO; n];
        let mut mins = vec![Fixed::ZERO; n];
        let mut maxs = vec![Fixed::ZERO; n];
        let mut medians = vec![Fixed::ZERO; n];
        let mut quartiles = vec![[Fixed::ZERO; 4]; n];
        let mut counts = vec![0usize; n];
        let mut missing = vec![0usize; n];
        let mut initialized = vec![false; n];
        let mut values: Vec<Vec<Fixed>> = vec![Vec::with_capacity(data.sample_count); n];

        for row in &data.features {
            for (i, &v) in row.iter().enumerate() {
                sums[i] += v;
                if !initialized[i] {
                    mins[i] = v;
                    maxs[i] = v;
                    initialized[i] = true;
                } else {
                    if v < mins[i] {
                        mins[i] = v;
                    }
                    if v > maxs[i] {
                        maxs[i] = v;
                    }
                }
                counts[i] += 1;
                values[i].push(v);
            }

            if row.len() < n {
                for count in missing.iter_mut().skip(row.len()) {
                    *count += 1;
                }
            }
        }

        let mut means = vec![Fixed::ZERO; n];
        for i in 0..n {
            if counts[i] > 0 {
                means[i] = sums[i].div_int(counts[i] as i64);
            }
        }

        let mut variance_acc = vec![Fixed::ZERO; n];
        for row in &data.features {
            for (i, &v) in row.iter().enumerate() {
                let diff = v - means[i];
                variance_acc[i] += diff * diff;
            }
        }

        let mut stds = vec![Fixed::ZERO; n];
        for i in 0..n {
            if counts[i] > 1 {
                let variance = variance_acc[i].div_int((counts[i] - 1) as i64);
                let std = fixed_sqrt(variance);
                stds[i] = if std.is_zero() { Fixed::ONE } else { std };
            } else {
                stds[i] = Fixed::ONE;
            }
        }

        for i in 0..n {
            let vec = &mut values[i];
            if vec.is_empty() {
                continue;
            }
            vec.sort();
            let len = vec.len();
            medians[i] = vec[len / 2];
            quartiles[i][0] = vec[len / 4];
            quartiles[i][1] = medians[i];
            quartiles[i][2] = vec[(3 * len) / 4];
            quartiles[i][3] = *vec.last().unwrap();
        }

        Ok(FeatureStatistics {
            means,
            std_devs: stds,
            mins,
            maxs,
            medians,
            quartiles,
            feature_counts: counts,
            missing_counts: missing,
        })
    }

    fn calculate_feature_importance(
        &self,
        stats: &FeatureStatistics,
        data: &RawFeatureData,
    ) -> FeatureImportance {
        let mut variances = Vec::with_capacity(stats.std_devs.len());
        let mut total = Fixed::ZERO;

        for std in &stats.std_devs {
            let var = *std * *std;
            variances.push(var);
            total += var;
        }

        if total.is_zero() {
            total = Fixed::ONE;
        }

        for score in &mut variances {
            *score /= total;
        }

        let mut selected: Vec<usize> = variances
            .iter()
            .enumerate()
            .filter(|(_, &score)| score >= self.config.feature_selection_threshold)
            .map(|(i, _)| i)
            .collect();

        if selected.len() > self.config.max_features {
            selected.truncate(self.config.max_features);
        }

        let total_variance_explained = selected
            .iter()
            .fold(Fixed::ZERO, |acc, &idx| acc + variances[idx]);

        FeatureImportance {
            feature_names: data.feature_names.clone(),
            importance_scores: variances,
            selected_features: selected,
            total_variance_explained,
        }
    }

    fn calculate_normalization(&self, stats: &FeatureStatistics) -> FeatureNormalization {
        let mut means = Vec::with_capacity(stats.means.len());
        let mut stds = Vec::with_capacity(stats.std_devs.len());
        let mut mins = Vec::with_capacity(stats.mins.len());
        let mut maxs = Vec::with_capacity(stats.maxs.len());

        for i in 0..stats.means.len() {
            means.push(scale_fixed_to_i64(stats.means[i]));
            let std = if stats.std_devs[i].is_zero() {
                Fixed::ONE
            } else {
                stats.std_devs[i]
            };
            stds.push(scale_fixed_to_i64(std));
            mins.push(scale_fixed_to_i64(stats.mins[i]));
            maxs.push(scale_fixed_to_i64(stats.maxs[i]));
        }

        FeatureNormalization {
            means,
            std_devs: stds,
            mins,
            maxs,
        }
    }

    fn normalize_value(
        &self,
        value: Fixed,
        idx: usize,
        norm: &FeatureNormalization,
    ) -> Result<Fixed, GBDTError> {
        if idx >= norm.means.len() {
            return Err(GBDTError::FeatureSizeMismatch {
                expected: norm.means.len(),
                actual: idx + 1,
            });
        }

        let mean = Fixed::from_ratio(norm.means[idx], 1000);
        let std = Fixed::from_ratio(norm.std_devs[idx], 1000);
        let min_bound = Fixed::from_ratio(norm.mins[idx], 1000);
        let max_bound = Fixed::from_ratio(norm.maxs[idx], 1000);

        let standardized = if std.is_zero() {
            Fixed::ZERO
        } else {
            (value - mean) / std
        };

        Ok(standardized.clamp(min_bound, max_bound))
    }

    fn scale_to_integer(&self, value: Fixed) -> i64 {
        value.to_micro().clamp(-1_000_000_000, 1_000_000_000)
    }

    fn apply_feature_selection(
        &self,
        features: &[Vec<i64>],
        names: &[String],
        imp: &FeatureImportance,
    ) -> Result<(Vec<Vec<i64>>, Vec<String>), GBDTError> {
        if imp.selected_features.is_empty() {
            return Ok((features.to_vec(), names.to_vec()));
        }

        let mut selected_rows = Vec::with_capacity(features.len());
        for row in features {
            let mut selected = Vec::with_capacity(imp.selected_features.len());
            for &idx in &imp.selected_features {
                if idx >= row.len() {
                    return Err(GBDTError::FeatureSizeMismatch {
                        expected: row.len(),
                        actual: idx + 1,
                    });
                }
                selected.push(row[idx]);
            }
            selected_rows.push(selected);
        }

        let mut selected_names = Vec::new();
        for &idx in &imp.selected_features {
            if idx < names.len() {
                selected_names.push(names[idx].clone());
            }
        }

        Ok((selected_rows, selected_names))
    }

    fn handle_outliers(&self, stats: &FeatureStatistics, data: &RawFeatureData) {
        for i in 0..data.feature_count {
            let q1 = stats.quartiles[i][0];
            let q3 = stats.quartiles[i][2];
            let iqr = q3 - q1;
            let threshold = self.config.outlier_threshold * iqr;
            let lower = q1 - threshold;
            let upper = q3 + threshold;

            let mut outliers = 0usize;
            for row in &data.features {
                if i < row.len() {
                    let v = row[i];
                    if v < lower || v > upper {
                        outliers += 1;
                    }
                }
            }

            if outliers > 0 {
                warn!(
                    "Feature {} has {} outliers ({})",
                    i,
                    outliers,
                    Fixed::from_ratio(outliers as i64 * 100, data.sample_count as i64)
                );
            }
        }
    }

    pub fn get_feature_importance(&self) -> Option<&FeatureImportance> {
        self.feature_importance.as_ref()
    }
    pub fn get_normalization(&self) -> Option<&FeatureNormalization> {
        self.normalization.as_ref()
    }
    pub fn get_statistics(&self) -> Option<&FeatureStatistics> {
        self.statistics.as_ref()
    }
}

/// Utilities for data generation and conversion
pub mod utils {
    use super::*;

    pub fn create_raw_data(features: Vec<Vec<Fixed>>, names: Vec<String>) -> RawFeatureData {
        let samples = features.len();
        let count = if samples > 0 { features[0].len() } else { 0 };
        RawFeatureData {
            features,
            feature_names: names,
            sample_count: samples,
            feature_count: count,
            metadata: HashMap::new(),
        }
    }

    pub fn create_raw_data_with_metadata(
        features: Vec<Vec<Fixed>>,
        names: Vec<String>,
        metadata: HashMap<String, String>,
    ) -> RawFeatureData {
        let samples = features.len();
        let count = if samples > 0 { features[0].len() } else { 0 };
        RawFeatureData {
            features,
            feature_names: names,
            sample_count: samples,
            feature_count: count,
            metadata,
        }
    }

    pub fn generate_synthetic_data(
        samples: usize,
        features: usize,
        noise: Fixed,
    ) -> RawFeatureData {
        let mut names = Vec::with_capacity(features);
        for i in 0..features {
            names.push(format!("feature_{i}"));
        }

        let mut data = Vec::with_capacity(samples);
        for s in 0..samples {
            let mut row = Vec::with_capacity(features);
            for f in 0..features {
                let base = Fixed::from_int((f as i64) * 10);
                let modifier = ((s + f) % 5) as i64;
                let sign = if (s + f) % 2 == 0 {
                    Fixed::ONE
                } else {
                    Fixed::NEG_ONE
                };
                row.push(base + (noise.mul_int(modifier) * sign));
            }
            data.push(row);
        }

        create_raw_data(data, names)
    }
}

fn scale_fixed_to_i64(value: Fixed) -> i64 {
    (value.to_micro() / 1000).clamp(i64::MIN + 1, i64::MAX - 1)
}

fn fixed_sqrt(value: Fixed) -> Fixed {
    if value.is_negative() {
        return Fixed::ZERO;
    }
    let raw = value.to_micro() as i128;
    let scaled = raw * i128::from(SCALE);
    let sqrt = integer_sqrt_u128(scaled.max(0) as u128);
    Fixed::from_micro(sqrt as i64)
}

fn integer_sqrt_u128(n: u128) -> u128 {
    if n <= 1 {
        return n;
    }
    let mut x0 = n;
    let mut x1 = (x0 + 1) >> 1;
    while x1 < x0 {
        x0 = x1;
        x1 = (x0 + n / x0) >> 1;
    }
    x0
}

#[cfg(test)]
mod tests {
    use super::{utils, *};

    #[tokio::test]
    async fn test_feature_engineering_pipeline() {
        let mut pipeline = FeatureEngineeringPipeline::new(FeatureEngineeringConfig::default());
        let raw = utils::generate_synthetic_data(100, 5, Fixed::from_micro(100_000));
        pipeline.fit(&raw).await.unwrap();
        let processed = pipeline.transform(&raw).await.unwrap();
        assert_eq!(processed.features.len(), 100);
        assert_eq!(processed.feature_names.len(), 5);
    }

    #[tokio::test]
    async fn test_feature_selection() {
        let config = FeatureEngineeringConfig {
            enable_feature_selection: true,
            feature_selection_threshold: Fixed::from_micro(100_000),
            max_features: 3,
            ..Default::default()
        };
        let mut pipeline = FeatureEngineeringPipeline::new(config);
        let raw = utils::generate_synthetic_data(100, 10, Fixed::from_micro(100_000));
        pipeline.fit(&raw).await.unwrap();
        let processed = pipeline.transform(&raw).await.unwrap();
        assert!(processed.feature_names.len() <= 3);
    }

    #[tokio::test]
    async fn test_normalization() {
        let mut pipeline = FeatureEngineeringPipeline::new(FeatureEngineeringConfig {
            enable_normalization: true,
            ..Default::default()
        });
        let raw = utils::generate_synthetic_data(100, 5, Fixed::from_micro(100_000));
        pipeline.fit(&raw).await.unwrap();
        let processed = pipeline.transform(&raw).await.unwrap();
        assert!(processed.normalization.is_some());
        assert_eq!(processed.statistics.means.len(), 5);
    }

    #[test]
    fn test_utils_create_raw_data() {
        let raw = utils::create_raw_data(
            vec![
                vec![Fixed::from_int(1), Fixed::from_int(2)],
                vec![Fixed::from_int(3), Fixed::from_int(4)],
            ],
            vec!["a".into(), "b".into()],
        );
        assert_eq!(raw.sample_count, 2);
        assert_eq!(raw.feature_count, 2);
    }
}
