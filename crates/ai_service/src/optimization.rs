//! Transaction and performance optimization module
//!
//! Provides AI-driven heuristics for optimizing transaction cost, latency,
//! throughput, gas usage, and security parameters before submission.

use crate::errors::AIServiceError;
use crate::types::{
    DifficultyLevel, OptimizationConstraints, OptimizationGoal, OptimizationSuggestion,
    TransactionData, TransactionOptimizationRequest, TransactionOptimizationResponse,
};
use std::collections::HashMap;

/// Optimization service
pub struct OptimizationService;

impl OptimizationService {
    /// Create a new optimization service
    pub fn new() -> Self {
        Self
    }

    /// Optimize a transaction according to provided goals and constraints
    pub async fn optimize_transaction(
        &self,
        request: TransactionOptimizationRequest,
    ) -> Result<TransactionOptimizationResponse, AIServiceError> {
        let mut optimized_transaction = request.transaction.clone();
        let mut suggestions = Vec::new();
        let mut expected_improvements = HashMap::new();
        let mut confidence: f32 = 0.8;

        for goal in &request.goals {
            match goal {
                OptimizationGoal::MinimizeGas => {
                    let (t, s, i) =
                        self.optimize_gas_usage(&optimized_transaction, &request.constraints)?;
                    optimized_transaction = t;
                    suggestions.extend(s);
                    expected_improvements.insert("gas_usage".into(), i);
                }
                OptimizationGoal::MaximizeThroughput => {
                    let (t, s, i) = self.optimize_throughput(&optimized_transaction)?;
                    optimized_transaction = t;
                    suggestions.extend(s);
                    expected_improvements.insert("throughput".into(), i);
                }
                OptimizationGoal::MinimizeLatency => {
                    let (t, s, i) = self.optimize_latency(&optimized_transaction)?;
                    optimized_transaction = t;
                    suggestions.extend(s);
                    expected_improvements.insert("latency".into(), i);
                }
                OptimizationGoal::MaximizeSecurity => {
                    let (t, s, i) = self.optimize_security(&optimized_transaction)?;
                    optimized_transaction = t;
                    suggestions.extend(s);
                    expected_improvements.insert("security".into(), i);
                }
                OptimizationGoal::MinimizeCost => {
                    let (t, s, i) = self.optimize_cost(&optimized_transaction)?;
                    optimized_transaction = t;
                    suggestions.extend(s);
                    expected_improvements.insert("cost".into(), i);
                }
            }
        }

        confidence = match suggestions.len() {
            n if n > 5 => 0.9,
            n if n > 2 => 0.8,
            _ => 0.7,
        };

        Ok(TransactionOptimizationResponse {
            optimized_transaction,
            suggestions,
            expected_improvements,
            confidence: confidence as f64,
        })
    }

    /// Optimize gas usage
    fn optimize_gas_usage(
        &self,
        transaction: &TransactionData,
        constraints: &Option<OptimizationConstraints>,
    ) -> Result<(TransactionData, Vec<OptimizationSuggestion>, f64), AIServiceError> {
        let mut optimized = transaction.clone();
        let mut suggestions = Vec::new();
        let mut improvement: f64 = 0.0;

        if let Some(c) = constraints {
            if let Some(max_gas) = c.max_gas_limit {
                if optimized.gas_limit > max_gas {
                    optimized.gas_limit = max_gas;
                    suggestions.push(OptimizationSuggestion {
                        suggestion_type: "gas_limit".into(),
                        description: format!(
                            "Reduced gas limit from {} to {}",
                            transaction.gas_limit, max_gas
                        ),
                        expected_improvement: 0.15,
                        difficulty: DifficultyLevel::Easy,
                    });
                    improvement += 0.15;
                }
            }

            if let Some(max_price) = c.max_gas_price {
                if optimized.gas_price > max_price {
                    optimized.gas_price = max_price;
                    suggestions.push(OptimizationSuggestion {
                        suggestion_type: "gas_price".into(),
                        description: format!(
                            "Reduced gas price from {} to {}",
                            transaction.gas_price, max_price
                        ),
                        expected_improvement: 0.20,
                        difficulty: DifficultyLevel::Easy,
                    });
                    improvement += 0.20;
                }
            }
        }

        if !optimized.data.is_empty() {
            suggestions.push(OptimizationSuggestion {
                suggestion_type: "data_optimization".into(),
                description: "Consider compressing or optimizing transaction data".into(),
                expected_improvement: 0.10,
                difficulty: DifficultyLevel::Medium,
            });
            improvement += 0.10;
        }

        if optimized.tx_type == "transfer" {
            suggestions.push(OptimizationSuggestion {
                suggestion_type: "batch_operations".into(),
                description: "Consider batching multiple transfers into a single transaction"
                    .into(),
                expected_improvement: 0.25,
                difficulty: DifficultyLevel::Hard,
            });
            improvement += 0.25;
        }

        Ok((optimized, suggestions, improvement.min(1.0)))
    }

    /// Optimize throughput
    fn optimize_throughput(
        &self,
        transaction: &TransactionData,
    ) -> Result<(TransactionData, Vec<OptimizationSuggestion>, f64), AIServiceError> {
        let mut optimized = transaction.clone();
        let mut suggestions = Vec::new();
        let mut improvement: f64 = 0.0;

        if transaction.tx_type == "contract_call" {
            suggestions.push(OptimizationSuggestion {
                suggestion_type: "parallel_execution".into(),
                description: "Structure contract calls for parallel execution".into(),
                expected_improvement: 0.30,
                difficulty: DifficultyLevel::Hard,
            });
            improvement += 0.30;
        }

        if !transaction.data.is_empty() {
            suggestions.push(OptimizationSuggestion {
                suggestion_type: "data_compression".into(),
                description: "Compress transaction data to reduce network overhead".into(),
                expected_improvement: 0.15,
                difficulty: DifficultyLevel::Medium,
            });
            improvement += 0.15;
        }

        suggestions.push(OptimizationSuggestion {
            suggestion_type: "async_operations".into(),
            description: "Use asynchronous operations where possible".into(),
            expected_improvement: 0.20,
            difficulty: DifficultyLevel::Medium,
        });
        improvement += 0.20;

        Ok((optimized, suggestions, improvement.min(1.0)))
    }

    /// Optimize latency
    fn optimize_latency(
        &self,
        transaction: &TransactionData,
    ) -> Result<(TransactionData, Vec<OptimizationSuggestion>, f64), AIServiceError> {
        let mut optimized = transaction.clone();
        let mut suggestions = Vec::new();
        let mut improvement: f64 = 0.0;

        if optimized.gas_price < 20_000_000_000 {
            optimized.gas_price = 20_000_000_000;
            suggestions.push(OptimizationSuggestion {
                suggestion_type: "gas_price_optimization".into(),
                description: "Increased gas price for faster transaction inclusion".into(),
                expected_improvement: 0.25,
                difficulty: DifficultyLevel::Easy,
            });
            improvement += 0.25;
        }

        if transaction.tx_type == "contract_call" {
            suggestions.push(OptimizationSuggestion {
                suggestion_type: "local_execution".into(),
                description: "Consider executing contract calls locally when possible".into(),
                expected_improvement: 0.40,
                difficulty: DifficultyLevel::Hard,
            });
            improvement += 0.40;
        }

        if !transaction.data.is_empty() {
            suggestions.push(OptimizationSuggestion {
                suggestion_type: "data_optimization".into(),
                description: "Reduce transaction data size for faster processing".into(),
                expected_improvement: 0.10,
                difficulty: DifficultyLevel::Medium,
            });
            improvement += 0.10;
        }

        Ok((optimized, suggestions, improvement.min(1.0)))
    }

    /// Optimize security
    fn optimize_security(
        &self,
        transaction: &TransactionData,
    ) -> Result<(TransactionData, Vec<OptimizationSuggestion>, f64), AIServiceError> {
        let mut optimized = transaction.clone();
        let mut suggestions = Vec::new();
        let mut improvement: f64 = 0.0;

        suggestions.push(OptimizationSuggestion {
            suggestion_type: "input_validation".into(),
            description: "Add comprehensive input validation".into(),
            expected_improvement: 0.30,
            difficulty: DifficultyLevel::Medium,
        });
        improvement += 0.30;

        if transaction.amount > 1_000_000_000_000_000 {
            suggestions.push(OptimizationSuggestion {
                suggestion_type: "multisig".into(),
                description: "Use multi-signature for large transactions".into(),
                expected_improvement: 0.25,
                difficulty: DifficultyLevel::Hard,
            });
            improvement += 0.25;
        }

        suggestions.push(OptimizationSuggestion {
            suggestion_type: "rate_limiting".into(),
            description: "Implement rate limiting for transaction frequency".into(),
            expected_improvement: 0.20,
            difficulty: DifficultyLevel::Medium,
        });
        improvement += 0.20;

        if !transaction.data.is_empty() {
            suggestions.push(OptimizationSuggestion {
                suggestion_type: "data_encryption".into(),
                description: "Encrypt sensitive transaction data".into(),
                expected_improvement: 0.15,
                difficulty: DifficultyLevel::Hard,
            });
            improvement += 0.15;
        }

        Ok((optimized, suggestions, improvement.min(1.0)))
    }

    /// Optimize cost
    fn optimize_cost(
        &self,
        transaction: &TransactionData,
    ) -> Result<(TransactionData, Vec<OptimizationSuggestion>, f64), AIServiceError> {
        let mut optimized = transaction.clone();
        let mut suggestions = Vec::new();
        let mut improvement: f64 = 0.0;

        if optimized.gas_price > 10_000_000_000 {
            optimized.gas_price = 10_000_000_000;
            suggestions.push(OptimizationSuggestion {
                suggestion_type: "gas_price_optimization".into(),
                description: "Reduced gas price for cost optimization".into(),
                expected_improvement: 0.30,
                difficulty: DifficultyLevel::Easy,
            });
            improvement += 0.30;
        }

        if optimized.gas_limit > 21000 {
            suggestions.push(OptimizationSuggestion {
                suggestion_type: "gas_limit_optimization".into(),
                description: "Review and optimize gas limit usage".into(),
                expected_improvement: 0.20,
                difficulty: DifficultyLevel::Medium,
            });
            improvement += 0.20;
        }

        if transaction.tx_type == "transfer" {
            suggestions.push(OptimizationSuggestion {
                suggestion_type: "batch_transactions".into(),
                description: "Batch multiple transfers to reduce overall cost".into(),
                expected_improvement: 0.25,
                difficulty: DifficultyLevel::Medium,
            });
            improvement += 0.25;
        }

        suggestions.push(OptimizationSuggestion {
            suggestion_type: "off_chain_solution".into(),
            description: "Consider off-chain solutions for cost reduction".into(),
            expected_improvement: 0.40,
            difficulty: DifficultyLevel::Hard,
        });
        improvement += 0.40;

        Ok((optimized, suggestions, improvement.min(1.0)))
    }

    /// Retrieve static optimization recommendations per transaction type
    pub fn get_recommendations_for_type(&self, tx_type: &str) -> Vec<OptimizationSuggestion> {
        match tx_type {
            "transfer" => vec![
                OptimizationSuggestion {
                    suggestion_type: "batch_transfers".into(),
                    description: "Batch multiple transfers into a single transaction".into(),
                    expected_improvement: 0.30,
                    difficulty: DifficultyLevel::Medium,
                },
                OptimizationSuggestion {
                    suggestion_type: "gas_optimization".into(),
                    description: "Use optimal gas price for current network conditions".into(),
                    expected_improvement: 0.20,
                    difficulty: DifficultyLevel::Easy,
                },
            ],
            "contract_call" => vec![
                OptimizationSuggestion {
                    suggestion_type: "function_optimization".into(),
                    description: "Optimize contract function calls for gas efficiency".into(),
                    expected_improvement: 0.25,
                    difficulty: DifficultyLevel::Hard,
                },
                OptimizationSuggestion {
                    suggestion_type: "data_compression".into(),
                    description: "Compress function parameters and return values".into(),
                    expected_improvement: 0.15,
                    difficulty: DifficultyLevel::Medium,
                },
            ],
            "deployment" => vec![
                OptimizationSuggestion {
                    suggestion_type: "bytecode_optimization".into(),
                    description: "Optimize contract bytecode for deployment cost".into(),
                    expected_improvement: 0.35,
                    difficulty: DifficultyLevel::Hard,
                },
                OptimizationSuggestion {
                    suggestion_type: "constructor_optimization".into(),
                    description: "Optimize constructor parameters and initialization".into(),
                    expected_improvement: 0.20,
                    difficulty: DifficultyLevel::Medium,
                },
            ],
            _ => vec![OptimizationSuggestion {
                suggestion_type: "general_optimization".into(),
                description: "Apply general transaction optimization techniques".into(),
                expected_improvement: 0.10,
                difficulty: DifficultyLevel::Easy,
            }],
        }
    }
}

impl Default for OptimizationService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_optimize_transaction_gas() {
        let service = OptimizationService::new();
        let transaction = TransactionData {
            tx_type: "transfer".into(),
            from: "0x123".into(),
            to: "0x456".into(),
            amount: 1000,
            gas_limit: 50000,
            gas_price: 20_000_000_000,
            data: vec![],
            nonce: 1,
        };

        let request = TransactionOptimizationRequest {
            transaction,
            goals: vec![OptimizationGoal::MinimizeGas],
            constraints: Some(OptimizationConstraints {
                max_gas_limit: Some(30000),
                max_gas_price: Some(15_000_000_000),
                max_latency_ms: None,
                security_requirements: vec![],
            }),
        };

        let response = service.optimize_transaction(request).await.unwrap();
        assert!(response.optimized_transaction.gas_limit <= 30000);
        assert!(response.optimized_transaction.gas_price <= 15_000_000_000);
        assert!(!response.suggestions.is_empty());
        assert!(response.confidence > 0.0);
    }

    #[tokio::test]
    async fn test_optimize_transaction_throughput() {
        let service = OptimizationService::new();
        let transaction = TransactionData {
            tx_type: "contract_call".into(),
            from: "0x123".into(),
            to: "0x456".into(),
            amount: 0,
            gas_limit: 100000,
            gas_price: 10_000_000_000,
            data: vec![1, 2, 3, 4],
            nonce: 1,
        };

        let request = TransactionOptimizationRequest {
            transaction,
            goals: vec![OptimizationGoal::MaximizeThroughput],
            constraints: None,
        };

        let response = service.optimize_transaction(request).await.unwrap();
        assert!(!response.suggestions.is_empty());
        assert!(response.expected_improvements.contains_key("throughput"));
    }

    #[test]
    fn test_get_recommendations_for_type() {
        let service = OptimizationService::new();
        let recs = service.get_recommendations_for_type("transfer");
        assert!(!recs.is_empty());
        assert_eq!(recs[0].suggestion_type, "batch_transfers");
    }
}
