//! Deterministic GBDT (Gradient Boosted Decision Tree) Inference Engine
//!
//! This module provides a production-grade, deterministic GBDT inference engine
//! with the following properties:
//!
//! - **Zero floating-point operations**: All computations use fixed-point integers
//! - **Deterministic across platforms**: Same input always produces same output
//! - **Canonical serialization**: Models use sorted JSON keys for reproducible hashing
//! - **Blake3 hashing**: Fast, deterministic model verification
//! - **Integer-only model format**: All values are scaled integers (typically 1e6 scale)
//!
//! # Model Format
//!
//! Models are serialized as canonical JSON with the following structure:
//!
//! ```json
//! {
//!   "version": 1,
//!   "scale": 1000000,
//!   "trees": [
//!     {
//!       "nodes": [
//!         {"id":0,"left":1,"right":2,"feature":3,"threshold":12345,"leaf":null},
//!         {"id":1,"left":-1,"right":-1,"feature":-1,"threshold":0,"leaf":-234},
//!         {"id":2,"left":-1,"right":-1,"feature":-1,"threshold":0,"leaf":456}
//!       ],
//!       "weight": 1000000
//!     }
//!   ],
//!   "bias": 0,
//!   "post_scale": 1000000
//! }
//! ```
//!
//! # Usage
//!
//! ```rust,no_run
//! use ippan_ai_core::gbdt::{Model, Tree, tree::Node, model::SCALE};
//!
//! // Create a simple model
//! let tree = Tree::new(
//!     vec![
//!         Node::internal(0, 0, 50 * SCALE, 1, 2),
//!         Node::leaf(1, 100 * SCALE),
//!         Node::leaf(2, 200 * SCALE),
//!     ],
//!     SCALE,
//! );
//!
//! let model = Model::new(vec![tree], 0);
//!
//! // Perform inference
//! let features = vec![30 * SCALE, 40 * SCALE];
//! let score = model.score(&features);
//!
//! // Compute model hash
//! let hash = model.hash_hex().unwrap();
//! ```
//!
//! # Determinism Guarantees
//!
//! - All arithmetic is integer-only (no IEEE 754 floating point)
//! - Tree traversal uses deterministic comparison (<=)
//! - Model serialization uses canonical JSON (sorted keys)
//! - Hashing uses blake3 (pure Rust, deterministic)
//! - No dependency on system locale, timezone, or random state

pub mod model;
pub mod tree;

// Re-export main types for convenience
pub use model::{Model, ModelError, model_hash, model_hash_hex, SCALE};
pub use tree::{Node, Tree};

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::gbdt::tree::Node;
    
    #[test]
    fn test_two_tree_model_inference() {
        // Create a 2-tree model with known outputs
        let tree1 = Tree::new(
            vec![
                Node::internal(0, 0, 50_000_000, 1, 2),
                Node::leaf(1, 100_000_000),  // 100 at scale
                Node::leaf(2, 200_000_000),  // 200 at scale
            ],
            1_000_000,
        );
        
        let tree2 = Tree::new(
            vec![
                Node::internal(0, 1, 30_000_000, 1, 2),
                Node::leaf(1, -50_000_000),  // -50 at scale
                Node::leaf(2, 50_000_000),   // 50 at scale
            ],
            1_000_000,
        );
        
        let model = Model::new(vec![tree1, tree2], 0);
        
        // Test case 1: features = [30M, 20M]
        // Tree1: 30M <= 50M -> left -> 100M
        // Tree2: 20M <= 30M -> left -> -50M
        // Score: (100M * 1M / 1M) + (-50M * 1M / 1M) + 0 = 50M
        let score1 = model.score(&[30_000_000, 20_000_000]);
        assert_eq!(score1, 50_000_000);
        
        // Test case 2: features = [60M, 40M]
        // Tree1: 60M > 50M -> right -> 200M
        // Tree2: 40M > 30M -> right -> 50M
        // Score: (200M * 1M / 1M) + (50M * 1M / 1M) + 0 = 250M
        let score2 = model.score(&[60_000_000, 40_000_000]);
        assert_eq!(score2, 250_000_000);
    }
    
    #[test]
    fn test_model_hash_stability() {
        // Create two identical models
        let tree = Tree::new(
            vec![
                Node::internal(0, 0, 50_000_000, 1, 2),
                Node::leaf(1, 100_000_000),
                Node::leaf(2, 200_000_000),
            ],
            1_000_000,
        );
        
        let model1 = Model::new(vec![tree.clone()], 0);
        let model2 = Model::new(vec![tree], 0);
        
        let hash1 = model1.hash_hex().unwrap();
        let hash2 = model2.hash_hex().unwrap();
        
        // Identical models should have identical hashes
        assert_eq!(hash1, hash2);
        
        // Hash should be deterministic across multiple calls
        let hash3 = model1.hash_hex().unwrap();
        assert_eq!(hash1, hash3);
    }
    
    #[test]
    fn test_canonical_json_roundtrip() {
        let tree = Tree::new(
            vec![
                Node::internal(0, 0, 50_000_000, 1, 2),
                Node::leaf(1, 100_000_000),
                Node::leaf(2, 200_000_000),
            ],
            1_000_000,
        );
        
        let original = Model::new(vec![tree], 12345);
        
        // Serialize to canonical JSON
        let json = original.to_canonical_json().unwrap();
        
        // Deserialize back
        let restored: Model = serde_json::from_str(&json).unwrap();
        
        // Should be identical
        assert_eq!(original, restored);
        
        // Hashes should match
        assert_eq!(
            original.hash_hex().unwrap(),
            restored.hash_hex().unwrap()
        );
        
        // Inference should produce same results
        let features = vec![30_000_000];
        assert_eq!(original.score(&features), restored.score(&features));
    }
    
    #[test]
    fn test_deterministic_inference_repeated() {
        let tree = Tree::new(
            vec![
                Node::internal(0, 0, 50_000_000, 1, 2),
                Node::leaf(1, 100_000_000),
                Node::leaf(2, 200_000_000),
            ],
            1_000_000,
        );
        
        let model = Model::new(vec![tree], 0);
        let features = vec![30_000_000, 40_000_000, 50_000_000];
        
        // Run inference 100 times - should always get same result
        let mut scores = Vec::new();
        for _ in 0..100 {
            scores.push(model.score(&features));
        }
        
        // All scores should be identical
        let first = scores[0];
        assert!(scores.iter().all(|&s| s == first));
    }
    
    #[test]
    fn test_model_with_bias() {
        let tree = Tree::new(
            vec![Node::leaf(0, 100_000_000)],
            1_000_000,
        );
        
        let bias = 50_000_000;
        let model = Model::new(vec![tree], bias);
        
        let score = model.score(&[]);
        
        // Should be leaf value + bias = 100M + 50M = 150M
        assert_eq!(score, 150_000_000);
    }
    
    #[test]
    fn test_empty_features() {
        let tree = Tree::new(
            vec![Node::leaf(0, 123_456_789)],
            1_000_000,
        );
        
        let model = Model::new(vec![tree], 0);
        let score = model.score(&[]);
        
        // Should evaluate to leaf value
        assert_eq!(score, 123_456_789);
    }
    
    #[test]
    fn test_model_hash_hex_convenience() {
        let tree = Tree::new(
            vec![Node::leaf(0, 100_000_000)],
            1_000_000,
        );
        
        let model = Model::new(vec![tree], 0);
        
        let hash1 = model_hash_hex(&model);
        let hash2 = model.hash_hex().unwrap();
        
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // 32 bytes as hex
    }
}
