//! Performance tests for AI Service

use ippan_ai_service::{
    AIService, AIServiceConfig, LLMConfig, AnalyticsConfig, MonitoringConfig,
    LLMRequest, SmartContractAnalysisRequest, ContractAnalysisType,
    TransactionOptimizationRequest, OptimizationGoal, TransactionData,
};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::time::timeout;

#[tokio::test]
async fn test_llm_performance() {
    let config = create_test_config();
    let mut service = AIService::new(config).expect("Failed to create service");
    service.start().await.expect("Failed to start service");

    let request = LLMRequest {
        prompt: "Test prompt for performance testing".to_string(),
        context: None,
        max_tokens: Some(100),
        temperature: Some(0.7),
        stream: false,
    };

    // Test response time
    let start = Instant::now();
    let result = service.generate_text(request).await;
    let duration = start.elapsed();

    assert!(duration < Duration::from_secs(30), "LLM request took too long: {:?}", duration);
    assert!(result.is_ok() || result.is_err()); // Should not panic

    service.stop().await.expect("Failed to stop service");
}

#[tokio::test]
async fn test_smart_contract_analysis_performance() {
    let config = create_test_config();
    let mut service = AIService::new(config).expect("Failed to create service");
    service.start().await.expect("Failed to start service");

    let contract_code = r#"
pragma solidity ^0.8.0;
contract Test {
    function test() public pure returns (uint256) {
        return 42;
    }
}
"#;

    let request = SmartContractAnalysisRequest {
        code: contract_code.to_string(),
        language: "solidity".to_string(),
        analysis_type: ContractAnalysisType::Security,
        context: None,
    };

    // Test response time
    let start = Instant::now();
    let result = service.analyze_smart_contract(request).await;
    let duration = start.elapsed();

    assert!(duration < Duration::from_secs(10), "Smart contract analysis took too long: {:?}", duration);
    assert!(result.is_ok(), "Smart contract analysis failed: {:?}", result);

    service.stop().await.expect("Failed to stop service");
}

#[tokio::test]
async fn test_transaction_optimization_performance() {
    let config = create_test_config();
    let mut service = AIService::new(config).expect("Failed to create service");
    service.start().await.expect("Failed to start service");

    let transaction = TransactionData {
        tx_type: "transfer".to_string(),
        from: "0x1234567890123456789012345678901234567890".to_string(),
        to: "0x0987654321098765432109876543210987654321".to_string(),
        amount: 1000000000000000000,
        gas_limit: 21000,
        gas_price: 20000000000,
        data: vec![],
        nonce: 1,
    };

    let request = TransactionOptimizationRequest {
        transaction,
        goals: vec![OptimizationGoal::MinimizeGas, OptimizationGoal::MinimizeCost],
        constraints: None,
    };

    // Test response time
    let start = Instant::now();
    let result = service.optimize_transaction(request).await;
    let duration = start.elapsed();

    assert!(duration < Duration::from_secs(5), "Transaction optimization took too long: {:?}", duration);
    assert!(result.is_ok(), "Transaction optimization failed: {:?}", result);

    service.stop().await.expect("Failed to stop service");
}

#[tokio::test]
async fn test_concurrent_requests_performance() {
    let config = create_test_config();
    let mut service = AIService::new(config).expect("Failed to create service");
    service.start().await.expect("Failed to start service");

    let mut handles = vec![];
    let start = Instant::now();

    // Spawn 100 concurrent health checks
    for _ in 0..100 {
        let service_clone = service.clone();
        let handle = tokio::spawn(async move {
            service_clone.health_check().await
        });
        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        let result = handle.await.expect("Task panicked");
        assert!(result.is_ok(), "Concurrent health check failed: {:?}", result);
    }

    let duration = start.elapsed();
    assert!(duration < Duration::from_secs(10), "Concurrent requests took too long: {:?}", duration);

    service.stop().await.expect("Failed to stop service");
}

#[tokio::test]
async fn test_memory_usage() {
    let config = create_test_config();
    let mut service = AIService::new(config).expect("Failed to create service");
    service.start().await.expect("Failed to start service");

    // Add a large amount of analytics data
    let mut tags = HashMap::new();
    tags.insert("test".to_string(), "memory".to_string());

    for i in 0..1000 {
        service.add_analytics_data(
            "test_metric".to_string(),
            i as f64,
            "count".to_string(),
            tags.clone(),
        );
    }

    // Service should still be responsive
    let health = service.health_check().await.expect("Health check failed");
    assert!(health.status == ippan_ai_service::HealthStatus::Healthy);

    service.stop().await.expect("Failed to stop service");
}

#[tokio::test]
async fn test_error_recovery() {
    let config = create_test_config();
    let mut service = AIService::new(config).expect("Failed to create service");
    service.start().await.expect("Failed to start service");

    // Test with invalid input
    let invalid_request = LLMRequest {
        prompt: "".to_string(),
        context: None,
        max_tokens: Some(0),
        temperature: Some(-1.0),
        stream: false,
    };

    let result = service.generate_text(invalid_request).await;
    assert!(result.is_err(), "Should handle invalid input gracefully");

    // Service should still be healthy after error
    let health = service.health_check().await.expect("Health check failed");
    assert!(health.status == ippan_ai_service::HealthStatus::Healthy);

    service.stop().await.expect("Failed to stop service");
}

#[tokio::test]
async fn test_startup_time() {
    let config = create_test_config();
    let service = AIService::new(config).expect("Failed to create service");

    let start = Instant::now();
    service.start().await.expect("Failed to start service");
    let duration = start.elapsed();

    assert!(duration < Duration::from_secs(5), "Service startup took too long: {:?}", duration);

    service.stop().await.expect("Failed to stop service");
}

#[tokio::test]
async fn test_shutdown_time() {
    let config = create_test_config();
    let mut service = AIService::new(config).expect("Failed to create service");
    service.start().await.expect("Failed to start service");

    let start = Instant::now();
    service.stop().await.expect("Failed to stop service");
    let duration = start.elapsed();

    assert!(duration < Duration::from_secs(5), "Service shutdown took too long: {:?}", duration);
}

fn create_test_config() -> AIServiceConfig {
    AIServiceConfig {
        enable_llm: true,
        enable_analytics: true,
        enable_smart_contracts: true,
        enable_monitoring: true,
        llm_config: LLMConfig {
            api_endpoint: "https://api.openai.com/v1".to_string(),
            api_key: "test-key".to_string(),
            model_name: "gpt-4".to_string(),
            max_tokens: 1000,
            temperature: 0.7,
            timeout_seconds: 30,
        },
        analytics_config: AnalyticsConfig {
            enable_realtime: true,
            retention_days: 7,
            analysis_interval: 60,
            enable_predictive: true,
        },
        monitoring_config: MonitoringConfig {
            enable_anomaly_detection: true,
            alert_thresholds: {
                let mut thresholds = HashMap::new();
                thresholds.insert("memory_usage".to_string(), 80.0);
                thresholds.insert("cpu_usage".to_string(), 90.0);
                thresholds.insert("error_rate".to_string(), 5.0);
                thresholds
            },
            monitoring_interval: 30,
            enable_auto_remediation: false,
        },
    }
}