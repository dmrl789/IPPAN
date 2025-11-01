//! Integration tests for AI-enabled consensus
//!
//! Tests the full AI consensus path including:
//! - Validator selection with GBDT models
//! - Telemetry tracking
//! - Reputation scoring
//! - Multi-node consensus

#[cfg(feature = "ai_l1")]
mod ai_consensus_tests {
    use ippan_ai_core::gbdt::{GBDTModel, Node, Tree};
    use ippan_consensus::{
        L1AIConfig, L1AIConsensus, NetworkState, PoAConfig, PoAConsensus, Validator,
        ValidatorCandidate,
    };
    use ippan_mempool::Mempool;
    use ippan_storage::{SledStorage, Storage, ValidatorTelemetry};
    use std::sync::Arc;
    use tempfile::tempdir;

    fn create_test_gbdt_model() -> GBDTModel {
        // Simple test model that scores based on first feature (reputation)
        GBDTModel {
            trees: vec![Tree {
                nodes: vec![
                    Node::Internal {
                        feature: 0,
                        threshold: 5000,
                        left: 1,
                        right: 2,
                    },
                    Node::Leaf { value: 3000 }, // Low reputation
                    Node::Leaf { value: 8000 }, // High reputation
                ],
            }],
            bias: 0,
            scale: 10000,
        }
    }

    #[test]
    fn test_ai_consensus_validator_selection() {
        let dir = tempdir().unwrap();
        let storage =
            Arc::new(SledStorage::new(dir.path()).unwrap()) as Arc<dyn Storage + Send + Sync>;

        // Create validators
        let validators = vec![
            Validator {
                id: [1u8; 32],
                address: [1u8; 32],
                stake: 1000,
                is_active: true,
            },
            Validator {
                id: [2u8; 32],
                address: [2u8; 32],
                stake: 2000,
                is_active: true,
            },
            Validator {
                id: [3u8; 32],
                address: [3u8; 32],
                stake: 1500,
                is_active: true,
            },
        ];

        // Store telemetry for validators
        for v in &validators {
            let telemetry = ValidatorTelemetry {
                validator_id: v.id,
                blocks_proposed: 10,
                blocks_verified: 50,
                rounds_active: 100,
                avg_latency_us: 50_000 + (v.stake as u64),
                slash_count: 0,
                stake: v.stake,
                age_rounds: 1000,
                last_active_round: 100,
                uptime_percentage: 99.0 + (v.stake as f64 / 10000.0),
                recent_performance: 0.9 + (v.stake as f64 / 20000.0),
                network_contribution: 0.8,
            };
            storage
                .store_validator_telemetry(&v.id, &telemetry)
                .unwrap();
        }

        // Create consensus with AI enabled
        let mut config = PoAConfig {
            slot_duration_ms: 100,
            validators: validators.clone(),
            max_transactions_per_block: 1000,
            block_reward: 10,
            finalization_interval_ms: 200,
            enable_ai_reputation: true,
            enable_fee_caps: true,
            enable_dag_fair_emission: true,
        };

        let consensus = PoAConsensus::new(config.clone(), storage.clone(), [1u8; 32]);

        // Load GBDT model
        let model = create_test_gbdt_model();
        consensus
            .load_ai_models(Some(model), None, None, None)
            .unwrap();

        // Get consensus state - this should trigger AI-based validator selection
        let state = consensus.get_state();

        // Verify that a proposer was selected
        assert!(state.current_proposer.is_some());
        assert_eq!(state.validator_count, 3);

        // Verify telemetry was loaded
        let telemetry = consensus.telemetry_manager.get_telemetry(&[1u8; 32]);
        assert!(telemetry.is_some());
        assert_eq!(telemetry.unwrap().blocks_proposed, 10);
    }

    #[test]
    fn test_telemetry_tracking() {
        let dir = tempdir().unwrap();
        let storage =
            Arc::new(SledStorage::new(dir.path()).unwrap()) as Arc<dyn Storage + Send + Sync>;

        let validator_id = [1u8; 32];

        let validators = vec![Validator {
            id: validator_id,
            address: [1u8; 32],
            stake: 1000,
            is_active: true,
        }];

        let config = PoAConfig {
            slot_duration_ms: 100,
            validators,
            max_transactions_per_block: 1000,
            block_reward: 10,
            finalization_interval_ms: 200,
            enable_ai_reputation: true,
            enable_fee_caps: true,
            enable_dag_fair_emission: true,
        };

        let consensus = PoAConsensus::new(config, storage.clone(), validator_id);

        // Record block proposal
        consensus
            .telemetry_manager
            .record_block_proposal(&validator_id)
            .unwrap();

        // Check telemetry
        let telemetry = consensus
            .telemetry_manager
            .get_telemetry(&validator_id)
            .unwrap();
        assert_eq!(telemetry.blocks_proposed, 1);
        assert_eq!(telemetry.blocks_verified, 0);

        // Record verification
        consensus
            .telemetry_manager
            .record_block_verification(&validator_id)
            .unwrap();

        let telemetry = consensus
            .telemetry_manager
            .get_telemetry(&validator_id)
            .unwrap();
        assert_eq!(telemetry.blocks_verified, 1);

        // Verify persistence
        let stored_telemetry = storage
            .get_validator_telemetry(&validator_id)
            .unwrap()
            .unwrap();
        assert_eq!(stored_telemetry.blocks_proposed, 1);
        assert_eq!(stored_telemetry.blocks_verified, 1);
    }

    #[test]
    fn test_l1_ai_validator_selection_with_model() {
        let mut l1_ai = L1AIConsensus::new(L1AIConfig::default());

        // Load test model
        let model = create_test_gbdt_model();
        l1_ai.load_models(Some(model), None, None, None).unwrap();

        let candidates = vec![
            ValidatorCandidate {
                id: [1u8; 32],
                stake: 1000,
                reputation_score: 8000, // High reputation
                uptime_percentage: 99.0,
                recent_performance: 0.9,
                network_contribution: 0.8,
            },
            ValidatorCandidate {
                id: [2u8; 32],
                stake: 2000,
                reputation_score: 4000, // Low reputation
                uptime_percentage: 95.0,
                recent_performance: 0.7,
                network_contribution: 0.6,
            },
        ];

        let network_state = NetworkState {
            congestion_level: 0.3,
            avg_block_time_ms: 200.0,
            active_validators: 2,
            total_stake: 3000,
            current_round: 100,
            recent_tx_volume: 1000,
        };

        let result = l1_ai.select_validator(&candidates, &network_state).unwrap();

        // Should select validator 1 with higher reputation
        assert_eq!(result.selected_validator, [1u8; 32]);
        assert!(result.confidence_score > 0.0);
        assert!(!result.ai_features_used.is_empty());
    }

    #[test]
    fn test_reputation_scoring_from_telemetry() {
        let dir = tempdir().unwrap();
        let storage =
            Arc::new(SledStorage::new(dir.path()).unwrap()) as Arc<dyn Storage + Send + Sync>;

        let telemetry_good = ValidatorTelemetry {
            validator_id: [1u8; 32],
            blocks_proposed: 100,
            blocks_verified: 500,
            rounds_active: 1000,
            avg_latency_us: 50_000, // 50ms
            slash_count: 0,
            stake: 1_000_000,
            age_rounds: 10_000,
            last_active_round: 1000,
            uptime_percentage: 99.9,
            recent_performance: 0.95,
            network_contribution: 0.9,
        };

        let telemetry_poor = ValidatorTelemetry {
            validator_id: [2u8; 32],
            blocks_proposed: 10,
            blocks_verified: 20,
            rounds_active: 1000,
            avg_latency_us: 180_000, // 180ms
            slash_count: 5,
            stake: 100_000,
            age_rounds: 1_000,
            last_active_round: 500,
            uptime_percentage: 85.0,
            recent_performance: 0.5,
            network_contribution: 0.3,
        };

        // Calculate reputation scores
        use ippan_consensus::PoAConsensus;
        let score_good = PoAConsensus::calculate_reputation_from_telemetry(&telemetry_good);
        let score_poor = PoAConsensus::calculate_reputation_from_telemetry(&telemetry_poor);

        // Good validator should have higher score
        assert!(score_good > score_poor);
        assert!(score_good >= 7000); // Should be high
        assert!(score_poor <= 5000); // Should be lower
        assert!(score_good <= 10000); // Within bounds
        assert!(score_poor >= 0); // Within bounds
    }

    #[test]
    fn test_multi_round_telemetry_updates() {
        let dir = tempdir().unwrap();
        let storage =
            Arc::new(SledStorage::new(dir.path()).unwrap()) as Arc<dyn Storage + Send + Sync>;

        let validator_id = [1u8; 32];
        let validators = vec![Validator {
            id: validator_id,
            address: [1u8; 32],
            stake: 1000,
            is_active: true,
        }];

        let config = PoAConfig {
            slot_duration_ms: 100,
            validators,
            max_transactions_per_block: 1000,
            block_reward: 10,
            finalization_interval_ms: 200,
            enable_ai_reputation: true,
            enable_fee_caps: true,
            enable_dag_fair_emission: true,
        };

        let consensus = PoAConsensus::new(config, storage.clone(), validator_id);

        // Simulate multiple rounds
        for _ in 0..5 {
            consensus
                .telemetry_manager
                .record_block_proposal(&validator_id)
                .unwrap();
            consensus.telemetry_manager.advance_round().unwrap();
        }

        // Check final telemetry
        let telemetry = consensus
            .telemetry_manager
            .get_telemetry(&validator_id)
            .unwrap();
        assert_eq!(telemetry.blocks_proposed, 5);
        assert_eq!(telemetry.age_rounds, 6); // Started at 1, advanced 5 times
    }

    #[test]
    fn test_ai_consensus_fallback() {
        let dir = tempdir().unwrap();
        let storage =
            Arc::new(SledStorage::new(dir.path()).unwrap()) as Arc<dyn Storage + Send + Sync>;

        let validators = vec![
            Validator {
                id: [1u8; 32],
                address: [1u8; 32],
                stake: 1000,
                is_active: true,
            },
            Validator {
                id: [2u8; 32],
                address: [2u8; 32],
                stake: 2000,
                is_active: true,
            },
        ];

        let config = PoAConfig {
            slot_duration_ms: 100,
            validators,
            max_transactions_per_block: 1000,
            block_reward: 10,
            finalization_interval_ms: 200,
            enable_ai_reputation: true, // Enabled but no model loaded
            enable_fee_caps: true,
            enable_dag_fair_emission: true,
        };

        let consensus = PoAConsensus::new(config, storage, [1u8; 32]);

        // Without model, should still work (fallback to stake-weighted)
        let state = consensus.get_state();
        assert!(state.current_proposer.is_some());

        // Should select validator with higher stake (fallback)
        assert_eq!(state.current_proposer.unwrap(), [2u8; 32]);
    }

    #[test]
    fn test_slash_penalty_in_reputation() {
        let dir = tempdir().unwrap();
        let storage =
            Arc::new(SledStorage::new(dir.path()).unwrap()) as Arc<dyn Storage + Send + Sync>;

        let validator_id = [1u8; 32];
        let validators = vec![Validator {
            id: validator_id,
            address: [1u8; 32],
            stake: 1000,
            is_active: true,
        }];

        let config = PoAConfig {
            slot_duration_ms: 100,
            validators,
            max_transactions_per_block: 1000,
            block_reward: 10,
            finalization_interval_ms: 200,
            enable_ai_reputation: true,
            enable_fee_caps: true,
            enable_dag_fair_emission: true,
        };

        let consensus = PoAConsensus::new(config, storage, validator_id);

        // Get initial score
        consensus
            .telemetry_manager
            .record_block_proposal(&validator_id)
            .unwrap();
        let telemetry_before = consensus
            .telemetry_manager
            .get_telemetry(&validator_id)
            .unwrap();
        let score_before = PoAConsensus::calculate_reputation_from_telemetry(&telemetry_before);

        // Record slash
        consensus
            .telemetry_manager
            .record_slash(&validator_id)
            .unwrap();
        let telemetry_after = consensus
            .telemetry_manager
            .get_telemetry(&validator_id)
            .unwrap();
        let score_after = PoAConsensus::calculate_reputation_from_telemetry(&telemetry_after);

        // Score should decrease after slash
        assert!(score_after < score_before);
        assert_eq!(telemetry_after.slash_count, 1);
        assert!(telemetry_after.recent_performance < telemetry_before.recent_performance);
    }
}
