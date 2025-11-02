//! Production-grade feature engineering and data preprocessing for GBDT models
//!
//! Provides deterministic preprocessing for GBDT inference:
//! - Normalization, scaling, feature selection
//! - Outlier handling (IQR-based)
//! - Integer scaling for deterministic inference
//! - Supports reproducible transformation and normalization stats reuse

use crate::gbdt::{FeatureNormalization, GBDTError};
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
    pub feature_selection_threshold: f64,
    pub enable_outlier_detection: bool,
    pub outlier_threshold: f64,
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
            feature_selection_threshold: 0.01,
            enable_outlier_detection: true,
            outlier_threshold: 3.0,
            enable_feature_interactions: false,
            max_interaction_depth: 2,
        }
    }
}

/// Feature statistics used for normalization and scaling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureStatistics {
    pub means: Vec<f64>,
    pub std_devs: Vec<f64>,
    pub mins: Vec<f64>,
    pub maxs: Vec<f64>,
    pub medians: Vec<f64>,
    pub quartiles: Vec<[f64; 4]>,
    pub feature_counts: Vec<usize>,
    pub missing_counts: Vec<usize>,
}

/// Feature importance scores
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureImportance {
    pub feature_names: Vec<String>,
    pub importance_scores: Vec<f64>,
    pub selected_features: Vec<usize>,
    pub total_variance_explained: f64,
}

/// Raw numeric features (float-based)
#[derive(Debug, Clone)]
pub struct RawFeatureData {
    pub features: Vec<Vec<f64>>,
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

        self.statistics = Some(self.calculate_statistics(data)?);

        if self.config.enable_outlier_detection {
            self.handle_outliers(data)?;
        }

        if self.config.enable_feature_selection {
            self.feature_importance = Some(self.calculate_feature_importance(data)?);
        }

        if self.config.enable_normalization {
            self.normalization = Some(self.calculate_normalization(data)?);
        }

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
            for (i, &v) in sample.iter().enumerate() {
                if i >= stats.means.len() {
                    return Err(GBDTError::FeatureSizeMismatch {
                        expected: stats.means.len(),
                        actual: sample.len(),
                    });
                }

                let clean = if v.is_nan() || v.is_infinite() {
                    stats.medians[i]
                } else {
                    v
                };

                let norm = if let Some(ref n) = self.normalization {
                    self.normalize_value(clean, i, n)?
                } else {
                    clean
                };

                row.push(self.scale_to_integer(norm)?);
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

    /// Compute statistics (mean, std, quartiles, etc.)
    fn calculate_statistics(&self, data: &RawFeatureData) -> Result<FeatureStatistics, GBDTError> {
        let n = data.feature_count;
        let mut means = vec![0.0; n];
        let mut stds = vec![0.0; n];
        let mut mins = vec![f64::INFINITY; n];
        let mut maxs = vec![f64::NEG_INFINITY; n];
        let mut medians = vec![0.0; n];
        let mut quartiles = vec![[0.0; 4]; n];
        let mut counts = vec![0; n];
        let mut missing = vec![0; n];

        for row in &data.features {
            for (i, &v) in row.iter().enumerate() {
                if v.is_nan() || v.is_infinite() {
                    missing[i] += 1;
                    continue;
                }
                means[i] += v;
                mins[i] = mins[i].min(v);
                maxs[i] = maxs[i].max(v);
                counts[i] += 1;
            }
        }

        for i in 0..n {
            if counts[i] > 0 {
                means[i] /= counts[i] as f64;
            }
        }

        for row in &data.features {
            for (i, &v) in row.iter().enumerate() {
                if !v.is_nan() && !v.is_infinite() {
                    let diff = v - means[i];
                    stds[i] += diff * diff;
                }
            }
        }

        for i in 0..n {
            if counts[i] > 1 {
                stds[i] = (stds[i] / (counts[i] - 1) as f64).sqrt();
            }
        }

        for i in 0..n {
            let mut values: Vec<f64> = data
                .features
                .iter()
                .filter_map(|r| {
                    if i < r.len() {
                        let v = r[i];
                        if v.is_finite() {
                            Some(v)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect();
            values.sort_by(|a, b| a.partial_cmp(b).unwrap());
            if !values.is_empty() {
                let len = values.len();
                medians[i] = if len % 2 == 0 {
                    (values[len / 2 - 1] + values[len / 2]) / 2.0
                } else {
                    values[len / 2]
                };
                quartiles[i] = [
                    values[len / 4],
                    medians[i],
                    values[3 * len / 4],
                    values[len - 1],
                ];
            }
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

    /// Compute variance-based feature importance
    fn calculate_feature_importance(
        &self,
        data: &RawFeatureData,
    ) -> Result<FeatureImportance, GBDTError> {
        let stats = self.statistics.as_ref().unwrap();
        let mut scores = Vec::with_capacity(data.feature_count);
        let mut total = 0.0;

        for i in 0..data.feature_count {
            let var = stats.std_devs[i].powi(2);
            scores.push(var);
            total += var;
        }

        for s in &mut scores {
            *s /= total.max(1e-12);
        }

        let mut selected: Vec<usize> = scores
            .iter()
            .enumerate()
            .filter(|(_, &s)| s >= self.config.feature_selection_threshold)
            .map(|(i, _)| i)
            .collect();

        if selected.len() > self.config.max_features {
            let mut sorted: Vec<(usize, f64)> = selected.iter().map(|&i| (i, scores[i])).collect();
            sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            selected = sorted
                .iter()
                .take(self.config.max_features)
                .map(|(i, _)| *i)
                .collect();
        }

        let variance_explained: f64 = selected.iter().map(|&i| scores[i]).sum();

        Ok(FeatureImportance {
            feature_names: data.feature_names.clone(),
            importance_scores: scores,
            selected_features: selected,
            total_variance_explained: variance_explained,
        })
    }

    /// Calculate normalization (scaled to int for deterministic GBDT)
    fn calculate_normalization(
        &self,
        data: &RawFeatureData,
    ) -> Result<FeatureNormalization, GBDTError> {
        let stats = self.statistics.as_ref().unwrap();
        let mut means = Vec::new();
        let mut stds = Vec::new();
        let mut mins = Vec::new();
        let mut maxs = Vec::new();

        for i in 0..data.feature_count {
            means.push((stats.means[i] * 1000.0) as i64);
            stds.push((stats.std_devs[i] * 1000.0).max(1.0) as i64);
            mins.push((stats.mins[i] * 1000.0) as i64);
            maxs.push((stats.maxs[i] * 1000.0) as i64);
        }

        Ok(FeatureNormalization {
            means,
            std_devs: stds,
            mins,
            maxs,
        })
    }

    fn normalize_value(
        &self,
        value: f64,
        idx: usize,
        norm: &FeatureNormalization,
    ) -> Result<f64, GBDTError> {
        if idx >= norm.means.len() {
            return Err(GBDTError::FeatureSizeMismatch {
                expected: norm.means.len(),
                actual: idx + 1,
            });
        }
        let normalized =
            (value - (norm.means[idx] as f64 / 1000.0)) / (norm.std_devs[idx] as f64 / 1000.0);
        let minb = norm.mins[idx] as f64 / 1000.0;
        let maxb = norm.maxs[idx] as f64 / 1000.0;
        Ok(normalized.clamp(minb, maxb))
    }

    fn scale_to_integer(&self, value: f64) -> Result<i64, GBDTError> {
        Ok((value * 1_000_000.0).round() as i64).map(|v| v.clamp(-1_000_000_000, 1_000_000_000))
    }

    fn apply_feature_selection(
        &self,
        features: &[Vec<i64>],
        names: &[String],
        imp: &FeatureImportance,
    ) -> Result<(Vec<Vec<i64>>, Vec<String>), GBDTError> {
        let mut selected_rows = Vec::with_capacity(features.len());
        let mut selected_names = Vec::new();

        for row in features {
            let mut sel_row = Vec::with_capacity(imp.selected_features.len());
            for &idx in &imp.selected_features {
                if idx >= row.len() {
                    return Err(GBDTError::FeatureSizeMismatch {
                        expected: row.len(),
                        actual: idx + 1,
                    });
                }
                sel_row.push(row[idx]);
            }
            selected_rows.push(sel_row);
        }

        for &idx in &imp.selected_features {
            if idx < names.len() {
                selected_names.push(names[idx].clone());
            }
        }
        Ok((selected_rows, selected_names))
    }

    fn handle_outliers(&self, data: &RawFeatureData) -> Result<(), GBDTError> {
        let stats = self.statistics.as_ref().unwrap();
        for i in 0..data.feature_count {
            let q1 = stats.quartiles[i][0];
            let q3 = stats.quartiles[i][2];
            let iqr = q3 - q1;
            let lower = q1 - self.config.outlier_threshold * iqr;
            let upper = q3 + self.config.outlier_threshold * iqr;

            let mut outliers = 0;
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
                    "Feature {} has {} outliers ({:.2}%)",
                    i,
                    outliers,
                    (outliers as f64 / data.sample_count as f64) * 100.0
                );
            }
        }
        Ok(())
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

    pub fn create_raw_data(features: Vec<Vec<f64>>, names: Vec<String>) -> RawFeatureData {
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
        features: Vec<Vec<f64>>,
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

    pub fn generate_synthetic_data(samples: usize, features: usize, noise: f64) -> RawFeatureData {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let mut names = Vec::with_capacity(features);
        for i in 0..features {
            names.push(format!("feature_{}", i));
        }

        let mut data = Vec::with_capacity(samples);
        for _ in 0..samples {
            let mut row = Vec::with_capacity(features);
            for i in 0..features {
                let base = (i as f64) * 10.0 + rng.gen_range(0.0..10.0);
                let eps = rng.gen_range(-noise..noise);
                row.push(base + eps);
            }
            data.push(row);
        }

        create_raw_data(data, names)
    }
}

#[cfg(all(test, feature = "enable-tests"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_feature_engineering_pipeline() {
        let mut pipeline = FeatureEngineeringPipeline::new(FeatureEngineeringConfig::default());
        let raw = utils::generate_synthetic_data(100, 5, 0.1);
        pipeline.fit(&raw).await.unwrap();
        let processed = pipeline.transform(&raw).await.unwrap();
        assert_eq!(processed.features.len(), 100);
        assert_eq!(processed.feature_names.len(), 5);
    }

    #[tokio::test]
    async fn test_feature_selection() {
        let config = FeatureEngineeringConfig {
            enable_feature_selection: true,
            feature_selection_threshold: 0.1,
            max_features: 3,
            ..Default::default()
        };
        let mut pipeline = FeatureEngineeringPipeline::new(config);
        let raw = utils::generate_synthetic_data(100, 10, 0.1);
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
        let raw = utils::generate_synthetic_data(100, 5, 0.1);
        pipeline.fit(&raw).await.unwrap();
        let processed = pipeline.transform(&raw).await.unwrap();
        assert!(processed.normalization.is_some());
        assert_eq!(processed.statistics.means.len(), 5);
    }

    #[test]
    fn test_utils_create_raw_data() {
        let raw = utils::create_raw_data(
            vec![vec![1.0, 2.0], vec![3.0, 4.0]],
            vec!["a".into(), "b".into()],
        );
        assert_eq!(raw.sample_count, 2);
        assert_eq!(raw.feature_count, 2);
    }
}
