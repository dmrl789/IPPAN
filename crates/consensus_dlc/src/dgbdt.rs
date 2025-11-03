//! Deterministic Gradient-Boosted Decision Tree (D-GBDT) fairness engine.

use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FairnessModel {
    pub weights: Vec<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidatorMetrics {
    pub uptime: f64,
    pub latency: f64,
    pub honesty: f64,
}

impl FairnessModel {
    pub fn default() -> Self {
        Self { weights: vec![0.5, 0.3, 0.2] }
    }

    pub fn score(&self, m: &ValidatorMetrics) -> f64 {
        self.weights[0]*m.uptime + self.weights[1]*(1.0 - m.latency) + self.weights[2]*m.honesty
    }
}
