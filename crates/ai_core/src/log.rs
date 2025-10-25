use crate::gbdt::GBDTModel as Model;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// HashTimer proof for AI model evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiEvaluationProof {
    /// The model used for evaluation
    pub model_id: String,
    /// Model version
    pub model_version: u32,
    /// Input features used
    pub features: Vec<i64>,
    /// Computed reputation score
    pub score: i32,
    /// HashTimer proof of evaluation
    pub hashtimer_proof: String,
    /// Timestamp of evaluation
    pub timestamp: u64,
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// AI evaluation logger for consensus integration
pub struct AiLogger {
    /// Current active model
    active_model: Option<Model>,
    /// Evaluation history
    evaluation_history: Vec<AiEvaluationProof>,
}

impl AiLogger {
    /// Create a new AI logger
    pub fn new() -> Self {
        Self {
            active_model: None,
            evaluation_history: Vec::new(),
        }
    }

    /// Set the active model for evaluation
    pub fn set_active_model(&mut self, model: Model) -> Result<()> {
        // Validate model structure
        if model.trees.is_empty() {
            return Err(anyhow::anyhow!("Model must have at least one tree"));
        }
        self.active_model = Some(model);
        Ok(())
    }

    /// Log an AI evaluation with HashTimer proof
    pub fn log_evaluation(
        &mut self,
        model_id: String,
        features: Vec<i64>,
        score: i32,
        hashtimer_proof: String,
    ) -> Result<()> {
        // Note: GBDTModel doesn't have a version field
        // We'll use 0 as a placeholder
        let model_version = 0;

        let proof = AiEvaluationProof {
            model_id,
            model_version,
            features,
            score,
            hashtimer_proof,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            metadata: HashMap::new(),
        };

        self.evaluation_history.push(proof);
        Ok(())
    }

    /// Get the evaluation history
    pub fn get_evaluation_history(&self) -> &[AiEvaluationProof] {
        &self.evaluation_history
    }

    /// Clear the evaluation history
    pub fn clear_history(&mut self) {
        self.evaluation_history.clear();
    }

    /// Get the current active model
    pub fn get_active_model(&self) -> Option<&Model> {
        self.active_model.as_ref()
    }
}

impl Default for AiLogger {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to create a HashTimer proof for AI evaluation
pub fn create_hashtimer_proof(
    model_id: &str,
    features: &[i64],
    score: i32,
    timestamp: u64,
) -> String {
    // Create a deterministic proof of the evaluation
    let mut proof_data = Vec::new();
    proof_data.extend_from_slice(model_id.as_bytes());
    proof_data.extend_from_slice(&score.to_be_bytes());
    proof_data.extend_from_slice(&timestamp.to_be_bytes());
    
    for feature in features {
        proof_data.extend_from_slice(&feature.to_be_bytes());
    }
    
    // Create a simple hash-based proof for now
    use blake3::Hasher;
    let mut hasher = Hasher::new();
    hasher.update(b"ai_evaluation");
    hasher.update(&proof_data);
    hasher.update(&score.to_be_bytes());
    hasher.update(&timestamp.to_be_bytes());
    
    let hash = hasher.finalize();
    format!("ai_proof_{}", hex::encode(hash.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gbdt::{GBDTModel as Model, Tree, Node};

    fn create_test_model() -> Model {
        Model::new(
            1,
            3,
            100,
            10000,
            vec![Tree {
                nodes: vec![
                    Node {
                        feature_index: 0,
                        threshold: 50,
                        left: 1,
                        right: 2,
                        value: None,
                    },
                    Node {
                        feature_index: 0,
                        threshold: 0,
                        left: 0,
                        right: 0,
                        value: Some(1000),
                    },
                    Node {
                        feature_index: 0,
                        threshold: 0,
                        left: 0,
                        right: 0,
                        value: Some(500),
                    },
                ],
            }],
        )
    }

    #[test]
    fn test_ai_logger() {
        let mut logger = AiLogger::new();
        let model = create_test_model();
        
        // Set active model
        assert!(logger.set_active_model(model).is_ok());
        assert!(logger.get_active_model().is_some());
        
        // Log an evaluation
        let features = vec![30, 20, 10];
        let score = 1100;
        let proof = create_hashtimer_proof("test_model", &features, score, 1234567890);
        
        assert!(logger.log_evaluation(
            "test_model".to_string(),
            features.clone(),
            score,
            proof
        ).is_ok());
        
        // Check history
        let history = logger.get_evaluation_history();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].score, score);
        assert_eq!(history[0].features, features);
    }

    #[test]
    fn test_hashtimer_proof_creation() {
        let features = vec![30, 20, 10];
        let score = 1100;
        let timestamp = 1234567890;
        
        let proof = create_hashtimer_proof("test_model", &features, score, timestamp);
        
        // Proof should be a valid proof string
        assert!(!proof.is_empty());
        assert!(proof.starts_with("ai_proof_"));
    }

    #[test]
    fn test_hashtimer_proof_determinism() {
        let features = vec![30, 20, 10];
        let score = 1100;
        let timestamp = 1234567890;
        
        let proof1 = create_hashtimer_proof("test_model", &features, score, timestamp);
        let proof2 = create_hashtimer_proof("test_model", &features, score, timestamp);
        
        // Same inputs should produce same proof
        assert_eq!(proof1, proof2);
    }
}