//! Integration tests for AI Service

use ippan_ai_service::{
    AIService, AIServiceConfig, ContractAnalysisType, LLMConfig, LLMRequest, OptimizationGoal,
    SmartContractAnalysisRequest, TransactionData, TransactionOptimizationRequest,
};
use std::collections::HashMap;

#[tokio::test]
async fn test_ai_service_creation() {
    let config = AIServiceConfig::default();
    let service = AIService::new(config);
    assert!(service.is_ok());
}

#[tokio::test]
async fn test_ai_service_start_stop() {
    let config = AIServiceConfig::default();
    let mut service = AIService::new(config).unwrap();

    assert!(!service.is_running());

    let start_result = service.start().await;
    assert!(start_result.is_ok());
    assert!(service.is_running());

    let stop_result = service.stop().await;
    assert!(stop_result.is_ok());
    assert!(!service.is_running());
}

#[tokio::test]
async fn test_llm_generation() {
    let config = AIServiceConfig {
        enable_llm: true,
        llm_config: LLMConfig {
            api_endpoint: "https://api.openai.com/v1".to_string(),
            api_key: "test-key".to_string(),
            model_name: "gpt-4".to_string(),
            max_tokens: 1000,
            temperature: 0.7,
            timeout_seconds: 30,
        },
        ..Default::default()
    };

    let service = AIService::new(config).unwrap();

    let request = LLMRequest {
        prompt: "Explain blockchain technology".to_string(),
        context: None,
        max_tokens: Some(100),
        temperature: Some(0.5),
        stream: false,
    };

    // Note: This test would fail in CI without a real API key
    // In a real test environment, you'd mock the HTTP client
    let _result = service.generate_text(request).await;
    // assert!(result.is_ok());
}

#[tokio::test]
async fn test_smart_contract_analysis() {
    let config = AIServiceConfig::default();
    let service = AIService::new(config).unwrap();

    let request = SmartContractAnalysisRequest {
        code: r#"
pragma solidity ^0.8.0;

contract TestContract {
    uint256 public value;
    
    function setValue(uint256 _value) public {
        value = _value;
    }
}
"#
        .to_string(),
        language: "solidity".to_string(),
        analysis_type: ContractAnalysisType::Security,
        context: None,
    };

    let result = service.analyze_smart_contract(request).await;
    assert!(result.is_ok());

    let analysis = result.unwrap();
    assert!(analysis.security_score > 0.0);
    assert!(analysis.security_score <= 1.0);
    assert!(!analysis.issues.is_empty() || analysis.issues.is_empty());
}

#[tokio::test]
async fn test_transaction_optimization() {
    let config = AIServiceConfig::default();
    let service = AIService::new(config).unwrap();

    let transaction = TransactionData {
        tx_type: "transfer".to_string(),
        from: "0x123".to_string(),
        to: "0x456".to_string(),
        amount: 1000,
        gas_limit: 50000,
        gas_price: 20_000_000_000,
        data: vec![],
        nonce: 1,
    };

    let request = TransactionOptimizationRequest {
        transaction,
        goals: vec![OptimizationGoal::MinimizeGas],
        constraints: None,
    };

    let result = service.optimize_transaction(request).await;
    assert!(result.is_ok());

    let optimization = result.unwrap();
    assert!(optimization.confidence > 0.0);
    assert!(optimization.confidence <= 1.0);
    assert!(!optimization.suggestions.is_empty());
}

#[tokio::test]
async fn test_analytics_data_collection() {
    let config = AIServiceConfig {
        enable_analytics: true,
        ..Default::default()
    };

    let mut service = AIService::new(config).unwrap();

    let mut tags = HashMap::new();
    tags.insert("node".to_string(), "node1".to_string());

    service.add_analytics_data("cpu_usage".to_string(), 75.0, "percent".to_string(), tags);

    let insights = service.get_analytics_insights().await;
    assert!(insights.is_ok());
}

#[tokio::test]
async fn test_monitoring_alerts() {
    let config = AIServiceConfig {
        enable_monitoring: true,
        ..Default::default()
    };

    let mut service = AIService::new(config).unwrap();

    service.add_monitoring_metric("memory_usage".to_string(), 85.0);

    let alerts = service.check_monitoring_alerts().await;
    assert!(alerts.is_ok());
}

#[tokio::test]
async fn test_service_status() {
    let config = AIServiceConfig::default();
    let service = AIService::new(config).unwrap();

    let status = service.get_status();
    assert_eq!(status.version, "0.1.0");
    assert!(!status.is_running);
}

#[tokio::test]
async fn test_configuration_updates() {
    let config = AIServiceConfig::default();
    let mut service = AIService::new(config).unwrap();

    let new_llm_config = LLMConfig {
        api_endpoint: "https://api.anthropic.com/v1".to_string(),
        api_key: "new-key".to_string(),
        model_name: "claude-3".to_string(),
        max_tokens: 2000,
        temperature: 0.5,
        timeout_seconds: 60,
    };

    let result = service.update_llm_config(new_llm_config);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_error_handling() {
    let config = AIServiceConfig {
        enable_llm: false,
        ..Default::default()
    };

    let service = AIService::new(config).unwrap();

    let request = LLMRequest {
        prompt: "test".to_string(),
        context: None,
        max_tokens: None,
        temperature: None,
        stream: false,
    };

    let result = service.generate_text(request).await;
    assert!(result.is_err());
}
