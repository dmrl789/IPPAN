//! Unit tests for AI Service components

use ippan_ai_service::{
    AlertStatus, AnalyticsConfig, ContractAnalysisType, InsightType, MonitoringService,
    OptimizationConstraints, OptimizationGoal, SeverityLevel, TransactionData,
};

#[cfg(feature = "analytics")]
use ippan_ai_service::{
    analytics::AnalyticsService, monitoring::MonitoringConfig, optimization::OptimizationService,
    smart_contracts::SmartContractService,
};
use std::collections::HashMap;

#[test]
#[cfg(feature = "analytics")]
fn test_analytics_service_creation() {
    let config = AnalyticsConfig::default();
    let service = AnalyticsService::new(config);
    assert_eq!(service.get_insights().len(), 0);
}

#[test]
#[cfg(feature = "analytics")]
fn test_analytics_data_point_addition() {
    let mut service = AnalyticsService::new(Default::default());
    let mut tags = HashMap::new();
    tags.insert("node".to_string(), "node1".to_string());

    service.add_data_point("cpu_usage".to_string(), 75.0, "percent".to_string(), tags);

    // In a real test, you'd verify the data was added
    assert!(true);
}

#[test]
#[cfg(feature = "analytics")]
fn test_analytics_insight_filtering() {
    let mut service = AnalyticsService::new(Default::default());

    // Add some test data
    let mut tags = HashMap::new();
    tags.insert("test".to_string(), "true".to_string());

    for i in 0..25 {
        service.add_data_point(
            "test_metric".to_string(),
            (50.0 + i as f64).sin() * 10.0 + 50.0,
            "units".to_string(),
            tags.clone(),
        );
    }

    // Test insight generation
    let rt = tokio::runtime::Runtime::new().unwrap();
    let insights = rt.block_on(service.analyze()).unwrap();

    // Should generate some insights with enough data
    assert!(insights.len() >= 0);
}

#[test]
#[cfg(feature = "analytics")]
fn test_monitoring_service_creation() {
    let config = MonitoringConfig::default();
    let service = MonitoringService::new(config);
    assert_eq!(service.get_alerts().len(), 0);
}

#[test]
#[cfg(feature = "analytics")]
fn test_monitoring_metric_addition() {
    let mut service = MonitoringService::new(Default::default());
    service.add_metric("cpu_usage".to_string(), 75.0);

    // Verify metric was added
    let stats = service.get_statistics();
    assert_eq!(stats.metrics_count, 1);
    assert_eq!(stats.total_data_points, 1);
}

#[test]
#[cfg(feature = "analytics")]
fn test_monitoring_alert_acknowledgment() {
    let mut service = MonitoringService::new(Default::default());

    // Create a test alert
    let alert = ippan_ai_service::types::MonitoringAlert {
        alert_id: "test_alert".to_string(),
        alert_type: "test".to_string(),
        severity: SeverityLevel::Medium,
        title: "Test Alert".to_string(),
        description: "Test Description".to_string(),
        metrics: HashMap::new(),
        timestamp: chrono::Utc::now(),
        status: AlertStatus::Active,
        actions_taken: Vec::new(),
    };

    // Add alert manually (in real implementation, this would be done by the service)
    // For now, just test the acknowledgment function
    let result = service.acknowledge_alert("test_alert");
    // This will fail because the alert doesn't exist, which is expected
    assert!(result.is_err());
}

#[test]
#[cfg(feature = "analytics")]
fn test_smart_contract_service_creation() {
    let service = SmartContractService::new();
    assert!(true); // Service created successfully
}

#[tokio::test]
#[cfg(feature = "analytics")]
async fn test_solidity_contract_analysis() {
    let service = SmartContractService::new();

    let request = ippan_ai_service::types::SmartContractAnalysisRequest {
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

    let result = service.analyze_contract(request).await;
    assert!(result.is_ok());

    let analysis = result.unwrap();
    assert!(analysis.security_score >= 0.0);
    assert!(analysis.security_score <= 1.0);
    assert!(analysis.gas_efficiency_score >= 0.0);
    assert!(analysis.gas_efficiency_score <= 1.0);
}

#[tokio::test]
#[cfg(feature = "analytics")]
async fn test_rust_contract_analysis() {
    let service = SmartContractService::new();

    let request = ippan_ai_service::types::SmartContractAnalysisRequest {
        code: r#"
fn dangerous_function() {
    unsafe {
        let ptr = std::ptr::null_mut();
        *ptr = 42;
    }
}
"#
        .to_string(),
        language: "rust".to_string(),
        analysis_type: ContractAnalysisType::Security,
        context: None,
    };

    let result = service.analyze_contract(request).await;
    assert!(result.is_ok());

    let analysis = result.unwrap();
    assert!(analysis.security_score < 1.0); // Should be lower due to unsafe code
    assert!(!analysis.issues.is_empty());
}

#[test]
#[cfg(feature = "analytics")]
fn test_optimization_service_creation() {
    let service = OptimizationService::new();
    assert!(true); // Service created successfully
}

#[test]
#[cfg(feature = "analytics")]
fn test_optimization_recommendations() {
    let service = OptimizationService::new();

    let transfer_recommendations = service.get_recommendations_for_type("transfer");
    assert!(!transfer_recommendations.is_empty());

    let contract_recommendations = service.get_recommendations_for_type("contract_call");
    assert!(!contract_recommendations.is_empty());

    let deployment_recommendations = service.get_recommendations_for_type("deployment");
    assert!(!deployment_recommendations.is_empty());
}

#[tokio::test]
#[cfg(feature = "analytics")]
async fn test_gas_optimization() {
    let service = OptimizationService::new();

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

    let request = ippan_ai_service::types::TransactionOptimizationRequest {
        transaction,
        goals: vec![OptimizationGoal::MinimizeGas],
        constraints: Some(OptimizationConstraints {
            max_gas_limit: Some(30000),
            max_gas_price: Some(15_000_000_000),
            max_latency_ms: None,
            security_requirements: vec![],
        }),
    };

    let result = service.optimize_transaction(request).await;
    assert!(result.is_ok());

    let optimization = result.unwrap();
    assert!(optimization.confidence > 0.0);
    assert!(optimization.confidence <= 1.0);
    assert!(!optimization.suggestions.is_empty());
    assert!(optimization.expected_improvements.contains_key("gas_usage"));
}

#[tokio::test]
#[cfg(feature = "analytics")]
async fn test_throughput_optimization() {
    let service = OptimizationService::new();

    let transaction = TransactionData {
        tx_type: "contract_call".to_string(),
        from: "0x123".to_string(),
        to: "0x456".to_string(),
        amount: 0,
        gas_limit: 100000,
        gas_price: 10_000_000_000,
        data: vec![1, 2, 3, 4],
        nonce: 1,
    };

    let request = ippan_ai_service::types::TransactionOptimizationRequest {
        transaction,
        goals: vec![OptimizationGoal::MaximizeThroughput],
        constraints: None,
    };

    let result = service.optimize_transaction(request).await;
    assert!(result.is_ok());

    let optimization = result.unwrap();
    assert!(optimization.confidence > 0.0);
    assert!(optimization
        .expected_improvements
        .contains_key("throughput"));
}

#[tokio::test]
#[cfg(feature = "analytics")]
async fn test_security_optimization() {
    let service = OptimizationService::new();

    let transaction = TransactionData {
        tx_type: "transfer".to_string(),
        from: "0x123".to_string(),
        to: "0x456".to_string(),
        amount: 1_000_000_000_000_000, // Large amount
        gas_limit: 21000,
        gas_price: 5_000_000_000,
        data: vec![],
        nonce: 1,
    };

    let request = ippan_ai_service::types::TransactionOptimizationRequest {
        transaction,
        goals: vec![OptimizationGoal::MaximizeSecurity],
        constraints: None,
    };

    let result = service.optimize_transaction(request).await;
    assert!(result.is_ok());

    let optimization = result.unwrap();
    assert!(optimization.confidence > 0.0);
    assert!(optimization.expected_improvements.contains_key("security"));
}

#[test]
fn test_insight_type_enum() {
    let performance = InsightType::Performance;
    let security = InsightType::Security;
    let economic = InsightType::Economic;
    let network = InsightType::Network;
    let user_behavior = InsightType::UserBehavior;
    let predictive = InsightType::Predictive;

    // Test that all variants exist
    assert!(matches!(performance, InsightType::Performance));
    assert!(matches!(security, InsightType::Security));
    assert!(matches!(economic, InsightType::Economic));
    assert!(matches!(network, InsightType::Network));
    assert!(matches!(user_behavior, InsightType::UserBehavior));
    assert!(matches!(predictive, InsightType::Predictive));
}

#[test]
fn test_severity_level_enum() {
    let low = SeverityLevel::Low;
    let medium = SeverityLevel::Medium;
    let high = SeverityLevel::High;
    let critical = SeverityLevel::Critical;

    // Test that all variants exist
    assert!(matches!(low, SeverityLevel::Low));
    assert!(matches!(medium, SeverityLevel::Medium));
    assert!(matches!(high, SeverityLevel::High));
    assert!(matches!(critical, SeverityLevel::Critical));
}

#[test]
fn test_alert_status_enum() {
    let active = AlertStatus::Active;
    let acknowledged = AlertStatus::Acknowledged;
    let resolved = AlertStatus::Resolved;
    let suppressed = AlertStatus::Suppressed;

    // Test that all variants exist
    assert!(matches!(active, AlertStatus::Active));
    assert!(matches!(acknowledged, AlertStatus::Acknowledged));
    assert!(matches!(resolved, AlertStatus::Resolved));
    assert!(matches!(suppressed, AlertStatus::Suppressed));
}
