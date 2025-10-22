use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Telemetry data collected from validators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorTelemetry {
    /// Validator ID
    pub validator_id: [u8; 32],
    /// Block production rate (blocks per hour)
    pub block_production_rate: f64,
    /// Average block size (bytes)
    pub avg_block_size: f64,
    /// Uptime percentage (0.0 to 1.0)
    pub uptime: f64,
    /// Network latency (milliseconds)
    pub network_latency: f64,
    /// Transaction validation accuracy (0.0 to 1.0)
    pub validation_accuracy: f64,
    /// Stake amount
    pub stake: u64,
    /// Number of slashing events
    pub slashing_events: u32,
    /// Time since last activity (seconds)
    pub last_activity: u64,
    /// Additional custom metrics
    #[serde(default)]
    pub custom_metrics: HashMap<String, f64>,
}

/// Feature extraction and normalization
pub struct FeatureExtractor {
    /// Feature scaling parameters
    scaling_params: HashMap<String, (f64, f64)>, // (mean, std) for normalization
}

impl FeatureExtractor {
    /// Create a new feature extractor with default scaling parameters
    pub fn new() -> Self {
        let mut scaling_params = HashMap::new();
        
        // Default scaling parameters (can be updated based on historical data)
        scaling_params.insert("block_production_rate".to_string(), (10.0, 5.0));
        scaling_params.insert("avg_block_size".to_string(), (1000.0, 500.0));
        scaling_params.insert("uptime".to_string(), (0.95, 0.1));
        scaling_params.insert("network_latency".to_string(), (100.0, 50.0));
        scaling_params.insert("validation_accuracy".to_string(), (0.98, 0.05));
        scaling_params.insert("stake".to_string(), (1000000.0, 500000.0));
        scaling_params.insert("slashing_events".to_string(), (0.0, 1.0));
        scaling_params.insert("last_activity".to_string(), (3600.0, 1800.0));
        
        Self { scaling_params }
    }

    /// Extract and normalize features from telemetry data
    pub fn extract_features(&self, telemetry: &ValidatorTelemetry) -> Result<Vec<i64>> {
        let mut features = Vec::new();
        
        // Block production rate (normalized)
        let bp_rate = self.normalize_feature("block_production_rate", telemetry.block_production_rate);
        features.push(self.quantize_feature(bp_rate));
        
        // Average block size (normalized)
        let avg_size = self.normalize_feature("avg_block_size", telemetry.avg_block_size);
        features.push(self.quantize_feature(avg_size));
        
        // Uptime (normalized)
        let uptime = self.normalize_feature("uptime", telemetry.uptime);
        features.push(self.quantize_feature(uptime));
        
        // Network latency (inverted and normalized - lower is better)
        let latency = self.normalize_feature("network_latency", telemetry.network_latency);
        features.push(self.quantize_feature(-latency)); // Invert so lower latency = higher score
        
        // Validation accuracy (normalized)
        let accuracy = self.normalize_feature("validation_accuracy", telemetry.validation_accuracy);
        features.push(self.quantize_feature(accuracy));
        
        // Stake (normalized)
        let stake = self.normalize_feature("stake", telemetry.stake as f64);
        features.push(self.quantize_feature(stake));
        
        // Slashing events (inverted and normalized - fewer is better)
        let slashing = self.normalize_feature("slashing_events", telemetry.slashing_events as f64);
        features.push(self.quantize_feature(-slashing)); // Invert so fewer slashing = higher score
        
        // Last activity (inverted and normalized - more recent is better)
        let activity = self.normalize_feature("last_activity", telemetry.last_activity as f64);
        features.push(self.quantize_feature(-activity)); // Invert so more recent = higher score
        
        Ok(features)
    }

    /// Normalize a feature using z-score normalization
    fn normalize_feature(&self, name: &str, value: f64) -> f64 {
        if let Some((mean, std)) = self.scaling_params.get(name) {
            if *std > 0.0 {
                (value - mean) / std
            } else {
                0.0
            }
        } else {
            value // No scaling if parameter not found
        }
    }

    /// Quantize a normalized feature to integer range
    fn quantize_feature(&self, normalized_value: f64) -> i64 {
        // Clamp to reasonable range and scale to integer
        let clamped = normalized_value.clamp(-3.0, 3.0); // 3-sigma range
        (clamped * 1000.0).round() as i64 // Scale to [-3000, 3000] range
    }

    /// Update scaling parameters based on historical data
    pub fn update_scaling_params(&mut self, name: &str, mean: f64, std: f64) {
        self.scaling_params.insert(name.to_string(), (mean, std));
    }
}

impl Default for FeatureExtractor {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to extract features from telemetry
pub fn from_telemetry(telemetry: &ValidatorTelemetry) -> Result<Vec<i64>> {
    let extractor = FeatureExtractor::new();
    extractor.extract_features(telemetry)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_telemetry() -> ValidatorTelemetry {
        ValidatorTelemetry {
            validator_id: [1u8; 32],
            block_production_rate: 12.5,
            avg_block_size: 1200.0,
            uptime: 0.98,
            network_latency: 80.0,
            validation_accuracy: 0.99,
            stake: 1500000,
            slashing_events: 0,
            last_activity: 300,
            custom_metrics: HashMap::new(),
        }
    }

    #[test]
    fn test_feature_extraction() {
        let telemetry = create_test_telemetry();
        let features = from_telemetry(&telemetry).unwrap();
        
        // Should have 8 features
        assert_eq!(features.len(), 8);
        
        // All features should be in reasonable range
        for feature in &features {
            assert!(*feature >= -3000 && *feature <= 3000);
        }
    }

    #[test]
    fn test_feature_consistency() {
        let telemetry = create_test_telemetry();
        let features1 = from_telemetry(&telemetry).unwrap();
        let features2 = from_telemetry(&telemetry).unwrap();
        
        // Same input should produce same features
        assert_eq!(features1, features2);
    }

    #[test]
    fn test_feature_extractor_custom_params() {
        let mut extractor = FeatureExtractor::new();
        extractor.update_scaling_params("block_production_rate", 15.0, 3.0);
        
        let telemetry = create_test_telemetry();
        let features = extractor.extract_features(&telemetry).unwrap();
        
        // Should still produce valid features
        assert_eq!(features.len(), 8);
        for feature in &features {
            assert!(*feature >= -3000 && *feature <= 3000);
        }
    }
}