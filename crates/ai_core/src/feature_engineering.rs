//! Production-grade feature engineering and data preprocessing for GBDT models
//!
//! This module provides comprehensive feature engineering capabilities including:
//! - Feature extraction from various data sources
//! - Data normalization and scaling
//! - Feature selection and dimensionality reduction
//! - Data validation and quality checking
//! - Feature importance analysis
//! - Real-time feature pipeline processing

use crate::gbdt::{FeatureNormalization, GBDTError, GBDTModel};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info, instrument, warn};

/// Feature engineering configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureEngineeringConfig {
    /// Enable feature engineering
    pub enable_feature_engineering: bool,
    /// Enable feature normalization
    pub enable_normalization: bool,
    /// Enable feature scaling
    pub enable_scaling: bool,
    /// Enable feature selection
    pub enable_feature_selection: bool,
    /// Maximum number of features
    pub max_features: usize,
    /// Feature selection threshold (0.0 to 1.0)
    pub feature_selection_threshold: f64,
    /// Enable outlier detection
    pub enable_outlier_detection: bool,
    /// Outlier detection threshold (standard deviations)
    pub outlier_threshold: f64,
    /// Enable feature interaction generation
    pub enable_feature_interactions: bool,
    /// Maximum interaction depth
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

/// Feature statistics for normalization and scaling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureStatistics {
    pub means: Vec<f64>,
    pub std_devs: Vec<f64>,
    pub mins: Vec<f64>,
    pub maxs: Vec<f64>,
    pub medians: Vec<f64>,
    pub quartiles: Vec<[f64; 4]>, // Q1, Q2, Q3, Q4
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

/// Raw feature data from various sources
#[derive(Debug, Clone)]
pub struct RawFeatureData {
    pub features: Vec<Vec<f64>>,
    pub feature_names: Vec<String>,
    pub sample_count: usize,
    pub feature_count: usize,
    pub metadata: HashMap<String, String>,
}

/// Processed feature data ready for GBDT
#[derive(Debug, Clone)]
pub struct ProcessedFeatureData {
    pub features: Vec<Vec<i64>>, // Scaled to integers for GBDT
    pub feature_names: Vec<String>,
    pub feature_importance: Option<FeatureImportance>,
    pub normalization: Option<FeatureNormalization>,
    pub statistics: FeatureStatistics,
    pub processing_time_ms: u64,
}

/// Feature engineering pipeline
#[derive(Debug)]
pub struct FeatureEngineeringPipeline {
    config: FeatureEngineeringConfig,
    statistics: Option<FeatureStatistics>,
    feature_importance: Option<FeatureImportance>,
    normalization: Option<FeatureNormalization>,
}

impl FeatureEngineeringPipeline {
    /// Create a new feature engineering pipeline
    pub fn new(config: FeatureEngineeringConfig) -> Self {
        Self {
            config,
            statistics: None,
            feature_importance: None,
            normalization: None,
        }
    }

    /// Fit the pipeline on training data
    #[instrument(skip(self, data))]
    pub async fn fit(&mut self, data: &RawFeatureData) -> Result<(), GBDTError> {
        let start_time = std::time::Instant::now();

        info!(
            "Fitting feature engineering pipeline on {} samples with {} features",
            data.sample_count, data.feature_count
        );

        // Calculate feature statistics
        self.statistics = Some(self.calculate_statistics(data)?);

        // Detect and handle outliers
        if self.config.enable_outlier_detection {
            self.handle_outliers(data)?;
        }

        // Calculate feature importance
        if self.config.enable_feature_selection {
            self.feature_importance = Some(self.calculate_feature_importance(data)?);
        }

        // Calculate normalization parameters
        if self.config.enable_normalization {
            self.normalization = Some(self.calculate_normalization(data)?);
        }

        let processing_time = start_time.elapsed();
        info!(
            "Feature engineering pipeline fitted in {}ms",
            processing_time.as_millis()
        );

        Ok(())
    }

    /// Transform raw data using the fitted pipeline
    #[instrument(skip(self, data))]
    pub async fn transform(
        &self,
        data: &RawFeatureData,
    ) -> Result<ProcessedFeatureData, GBDTError> {
        let start_time = std::time::Instant::now();

        if self.statistics.is_none() {
            return Err(GBDTError::ModelValidationFailed {
                reason: "Pipeline not fitted. Call fit() first.".to_string(),
            });
        }

        let statistics = self.statistics.as_ref().unwrap();
        let mut processed_features = Vec::with_capacity(data.sample_count);

        for sample in &data.features {
            let mut processed_sample = Vec::with_capacity(sample.len());

            for (i, &value) in sample.iter().enumerate() {
                if i >= statistics.means.len() {
                    return Err(GBDTError::FeatureSizeMismatch {
                        expected: statistics.means.len(),
                        actual: sample.len(),
                    });
                }

                // Handle missing values
                let processed_value = if value.is_nan() || value.is_infinite() {
                    statistics.medians[i] // Use median for missing values
                } else {
                    value
                };

                // Apply normalization if enabled
                let normalized_value = if let Some(ref norm) = self.normalization {
                    self.normalize_value(processed_value, i, norm)?
                } else {
                    processed_value
                };

                // Scale to integer for GBDT
                let scaled_value = self.scale_to_integer(normalized_value)?;
                processed_sample.push(scaled_value);
            }

            processed_features.push(processed_sample);
        }

        // Apply feature selection if enabled
        let (final_features, final_feature_names) =
            if let Some(ref importance) = self.feature_importance {
                self.apply_feature_selection(&processed_features, &data.feature_names, importance)?
            } else {
                (processed_features, data.feature_names.clone())
            };

        let processing_time = start_time.elapsed();

        Ok(ProcessedFeatureData {
            features: final_features,
            feature_names: final_feature_names,
            feature_importance: self.feature_importance.clone(),
            normalization: self.normalization.clone(),
            statistics: statistics.clone(),
            processing_time_ms: processing_time.as_millis() as u64,
        })
    }

    /// Calculate comprehensive feature statistics
    fn calculate_statistics(&self, data: &RawFeatureData) -> Result<FeatureStatistics, GBDTError> {
        let feature_count = data.feature_count;
        let mut means = vec![0.0; feature_count];
        let mut std_devs = vec![0.0; feature_count];
        let mut mins = vec![f64::INFINITY; feature_count];
        let mut maxs = vec![f64::NEG_INFINITY; feature_count];
        let mut medians = vec![0.0; feature_count];
        let mut quartiles = vec![[0.0; 4]; feature_count];
        let mut feature_counts = vec![0; feature_count];
        let mut missing_counts = vec![0; feature_count];

        // Calculate basic statistics
        for sample in &data.features {
            for (i, &value) in sample.iter().enumerate() {
                if i >= feature_count {
                    continue;
                }

                if value.is_nan() || value.is_infinite() {
                    missing_counts[i] += 1;
                } else {
                    means[i] += value;
                    mins[i] = mins[i].min(value);
                    maxs[i] = maxs[i].max(value);
                    feature_counts[i] += 1;
                }
            }
        }

        // Calculate means
        for i in 0..feature_count {
            if feature_counts[i] > 0 {
                means[i] /= feature_counts[i] as f64;
            }
        }

        // Calculate standard deviations
        for sample in &data.features {
            for (i, &value) in sample.iter().enumerate() {
                if i < feature_count && !value.is_nan() && !value.is_infinite() {
                    let diff = value - means[i];
                    std_devs[i] += diff * diff;
                }
            }
        }

        for i in 0..feature_count {
            if feature_counts[i] > 1 {
                std_devs[i] = (std_devs[i] / (feature_counts[i] - 1) as f64).sqrt();
            }
        }

        // Calculate medians and quartiles
        for i in 0..feature_count {
            let mut values: Vec<f64> = data
                .features
                .iter()
                .filter_map(|sample| {
                    if i < sample.len() {
                        let value = sample[i];
                        if !value.is_nan() && !value.is_infinite() {
                            Some(value)
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

                // Calculate quartiles
                quartiles[i] = [
                    values[len / 4],     // Q1
                    medians[i],          // Q2
                    values[3 * len / 4], // Q3
                    values[len - 1],     // Q4 (max)
                ];
            }
        }

        Ok(FeatureStatistics {
            means,
            std_devs,
            mins,
            maxs,
            medians,
            quartiles,
            feature_counts,
            missing_counts,
        })
    }

    /// Calculate feature importance using variance analysis
    fn calculate_feature_importance(
        &self,
        data: &RawFeatureData,
    ) -> Result<FeatureImportance, GBDTError> {
        let statistics = self.statistics.as_ref().unwrap();
        let mut importance_scores = Vec::with_capacity(data.feature_count);
        let mut total_variance = 0.0;

        // Calculate variance for each feature
        for i in 0..data.feature_count {
            let variance = statistics.std_devs[i] * statistics.std_devs[i];
            importance_scores.push(variance);
            total_variance += variance;
        }

        // Normalize importance scores
        for score in &mut importance_scores {
            *score /= total_variance;
        }

        // Select features above threshold
        let mut selected_features = Vec::new();
        for (i, &score) in importance_scores.iter().enumerate() {
            if score >= self.config.feature_selection_threshold {
                selected_features.push(i);
            }
        }

        // Limit to max features
        if selected_features.len() > self.config.max_features {
            let mut indexed_scores: Vec<(usize, f64)> = selected_features
                .iter()
                .map(|&i| (i, importance_scores[i]))
                .collect();
            indexed_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            selected_features = indexed_scores
                .iter()
                .take(self.config.max_features)
                .map(|(i, _)| *i)
                .collect();
        }

        let total_variance_explained: f64 = selected_features
            .iter()
            .map(|&i| importance_scores[i])
            .sum();

        Ok(FeatureImportance {
            feature_names: data.feature_names.clone(),
            importance_scores,
            selected_features,
            total_variance_explained,
        })
    }

    /// Calculate normalization parameters
    fn calculate_normalization(
        &self,
        data: &RawFeatureData,
    ) -> Result<FeatureNormalization, GBDTError> {
        let statistics = self.statistics.as_ref().unwrap();
        let mut means = Vec::with_capacity(data.feature_count);
        let mut std_devs = Vec::with_capacity(data.feature_count);
        let mut mins = Vec::with_capacity(data.feature_count);
        let mut maxs = Vec::with_capacity(data.feature_count);

        for i in 0..data.feature_count {
            // Scale to integers for GBDT compatibility
            means.push((statistics.means[i] * 1000.0) as i64);
            std_devs.push((statistics.std_devs[i] * 1000.0).max(1.0) as i64);
            mins.push((statistics.mins[i] * 1000.0) as i64);
            maxs.push((statistics.maxs[i] * 1000.0) as i64);
        }

        Ok(FeatureNormalization {
            means,
            std_devs,
            mins,
            maxs,
        })
    }

    /// Normalize a single value
    fn normalize_value(
        &self,
        value: f64,
        feature_idx: usize,
        norm: &FeatureNormalization,
    ) -> Result<f64, GBDTError> {
        if feature_idx >= norm.means.len() {
            return Err(GBDTError::FeatureSizeMismatch {
                expected: norm.means.len(),
                actual: feature_idx + 1,
            });
        }

        // Z-score normalization: (x - mean) / std
        let normalized = (value - (norm.means[feature_idx] as f64 / 1000.0))
            / (norm.std_devs[feature_idx] as f64 / 1000.0);

        // Clip to min/max bounds
        let min_bound = norm.mins[feature_idx] as f64 / 1000.0;
        let max_bound = norm.maxs[feature_idx] as f64 / 1000.0;
        Ok(normalized.max(min_bound).min(max_bound))
    }

    /// Scale value to integer for GBDT
    fn scale_to_integer(&self, value: f64) -> Result<i64, GBDTError> {
        // Scale to 6 decimal places and convert to integer
        let scaled = (value * 1_000_000.0).round() as i64;

        // Clamp to reasonable bounds
        Ok(scaled.clamp(-1_000_000_000, 1_000_000_000))
    }

    /// Apply feature selection
    fn apply_feature_selection(
        &self,
        features: &[Vec<i64>],
        feature_names: &[String],
        importance: &FeatureImportance,
    ) -> Result<(Vec<Vec<i64>>, Vec<String>), GBDTError> {
        let mut selected_features = Vec::with_capacity(features.len());
        let mut selected_names = Vec::with_capacity(importance.selected_features.len());

        for sample in features {
            let mut selected_sample = Vec::with_capacity(importance.selected_features.len());
            for &feature_idx in &importance.selected_features {
                if feature_idx < sample.len() {
                    selected_sample.push(sample[feature_idx]);
                } else {
                    return Err(GBDTError::FeatureSizeMismatch {
                        expected: sample.len(),
                        actual: feature_idx + 1,
                    });
                }
            }
            selected_features.push(selected_sample);
        }

        for &feature_idx in &importance.selected_features {
            if feature_idx < feature_names.len() {
                selected_names.push(feature_names[feature_idx].clone());
            }
        }

        Ok((selected_features, selected_names))
    }

    /// Handle outliers using IQR method
    fn handle_outliers(&self, data: &RawFeatureData) -> Result<(), GBDTError> {
        let statistics = self.statistics.as_ref().unwrap();

        for i in 0..data.feature_count {
            let q1 = statistics.quartiles[i][0];
            let q3 = statistics.quartiles[i][2];
            let iqr = q3 - q1;
            let lower_bound = q1 - self.config.outlier_threshold * iqr;
            let upper_bound = q3 + self.config.outlier_threshold * iqr;

            let mut outlier_count = 0;
            for sample in &data.features {
                if i < sample.len() {
                    let value = sample[i];
                    if value < lower_bound || value > upper_bound {
                        outlier_count += 1;
                    }
                }
            }

            if outlier_count > 0 {
                warn!(
                    "Feature {} has {} outliers ({}% of data)",
                    i,
                    outlier_count,
                    (outlier_count as f64 / data.sample_count as f64) * 100.0
                );
            }
        }

        Ok(())
    }

    /// Get feature importance information
    pub fn get_feature_importance(&self) -> Option<&FeatureImportance> {
        self.feature_importance.as_ref()
    }

    /// Get normalization parameters
    pub fn get_normalization(&self) -> Option<&FeatureNormalization> {
        self.normalization.as_ref()
    }

    /// Get feature statistics
    pub fn get_statistics(&self) -> Option<&FeatureStatistics> {
        self.statistics.as_ref()
    }
}

/// Utility functions for feature engineering
pub mod utils {
    use super::*;

    /// Create raw feature data from a simple matrix
    pub fn create_raw_data(features: Vec<Vec<f64>>, feature_names: Vec<String>) -> RawFeatureData {
        let sample_count = features.len();
        let feature_count = if sample_count > 0 {
            features[0].len()
        } else {
            0
        };

        RawFeatureData {
            features,
            feature_names,
            sample_count,
            feature_count,
            metadata: HashMap::new(),
        }
    }

    /// Create raw feature data with metadata
    pub fn create_raw_data_with_metadata(
        features: Vec<Vec<f64>>,
        feature_names: Vec<String>,
        metadata: HashMap<String, String>,
    ) -> RawFeatureData {
        let sample_count = features.len();
        let feature_count = if sample_count > 0 {
            features[0].len()
        } else {
            0
        };

        RawFeatureData {
            features,
            feature_names,
            sample_count,
            feature_count,
            metadata,
        }
    }

    /// Generate synthetic training data for testing
    pub fn generate_synthetic_data(
        samples: usize,
        features: usize,
        noise_level: f64,
    ) -> RawFeatureData {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        let mut feature_data = Vec::with_capacity(samples);
        let mut feature_names = Vec::with_capacity(features);

        for i in 0..features {
            feature_names.push(format!("feature_{}", i));
        }

        for _ in 0..samples {
            let mut sample = Vec::with_capacity(features);
            for i in 0..features {
                // Generate correlated features
                let base_value = (i as f64) * 10.0 + rng.gen_range(0.0..10.0);
                let noise = rng.gen_range(-noise_level..noise_level);
                sample.push(base_value + noise);
            }
            feature_data.push(sample);
        }

        create_raw_data(feature_data, feature_names)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_feature_engineering_pipeline() {
        let config = FeatureEngineeringConfig::default();
        let mut pipeline = FeatureEngineeringPipeline::new(config);

        // Create test data
        let raw_data = utils::generate_synthetic_data(100, 5, 0.1);

        // Fit pipeline
        pipeline.fit(&raw_data).await.unwrap();

        // Transform data
        let processed_data = pipeline.transform(&raw_data).await.unwrap();

        assert_eq!(processed_data.features.len(), 100);
        assert_eq!(processed_data.feature_names.len(), 5);
        assert!(processed_data.processing_time_ms > 0);
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
        let raw_data = utils::generate_synthetic_data(100, 10, 0.1);

        pipeline.fit(&raw_data).await.unwrap();
        let processed_data = pipeline.transform(&raw_data).await.unwrap();

        // Should have selected at most 3 features
        assert!(processed_data.feature_names.len() <= 3);
    }

    #[tokio::test]
    async fn test_normalization() {
        let config = FeatureEngineeringConfig {
            enable_normalization: true,
            ..Default::default()
        };

        let mut pipeline = FeatureEngineeringPipeline::new(config);
        let raw_data = utils::generate_synthetic_data(100, 5, 0.1);

        pipeline.fit(&raw_data).await.unwrap();
        let processed_data = pipeline.transform(&raw_data).await.unwrap();

        assert!(processed_data.normalization.is_some());
        assert!(processed_data.statistics.means.len() == 5);
    }

    #[test]
    fn test_utils_create_raw_data() {
        let features = vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]];
        let feature_names = vec!["a".to_string(), "b".to_string(), "c".to_string()];

        let raw_data = utils::create_raw_data(features, feature_names);
        assert_eq!(raw_data.sample_count, 2);
        assert_eq!(raw_data.feature_count, 3);
    }
}
