//! Production example of AI Service usage
//!
//! This example demonstrates how to use the AI Service in a production environment
//! with proper configuration, monitoring, and error handling.

use ippan_ai_service::{
    AIService, AIServiceConfig, AnalyticsConfig, ContractAnalysisType, LLMConfig, LLMRequest,
    OptimizationConstraints, OptimizationGoal, SmartContractAnalysisRequest, TransactionData,
    TransactionOptimizationRequest,
};
use std::collections::HashMap;
use tracing::{error, info, warn};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt().with_env_filter("info").init();

    info!("Starting AI Service in production mode");

    // Create production configuration
    let config = create_production_config();

    // Initialize AI Service
    let mut service = AIService::new(config).map_err(|e| {
        error!("Failed to create AI Service: {}", e);
        e
    })?;

    // Start the service
    service.start().await.map_err(|e| {
        error!("Failed to start AI Service: {}", e);
        e
    })?;

    info!("AI Service started successfully");

    // Demonstrate LLM functionality
    if service.get_config().enable_llm {
        demonstrate_llm_functionality(&service).await?;
    }

    // Demonstrate smart contract analysis
    if service.get_config().enable_smart_contracts {
        demonstrate_smart_contract_analysis(&service).await?;
    }

    // Demonstrate transaction optimization
    demonstrate_transaction_optimization(&service).await?;

    // Demonstrate analytics and monitoring
    if service.get_config().enable_analytics {
        demonstrate_analytics(&mut service).await?;
    }

    if service.get_config().enable_monitoring {
        demonstrate_monitoring(&mut service).await?;
    }

    // Show service status
    let status = service.get_status();
    info!("Service Status: {:?}", status);

    // Graceful shutdown
    service.stop().await.map_err(|e| {
        error!("Failed to stop AI Service: {}", e);
        e
    })?;

    info!("AI Service stopped gracefully");
    Ok(())
}

fn create_production_config() -> AIServiceConfig {
    AIServiceConfig {
        enable_llm: true,
        enable_analytics: true,
        enable_smart_contracts: true,
        enable_monitoring: true,
        llm_config: LLMConfig {
            api_endpoint: std::env::var("LLM_API_ENDPOINT")
                .unwrap_or_else(|_| "https://api.openai.com/v1".to_string()),
            api_key: std::env::var("LLM_API_KEY")
                .unwrap_or_else(|_| "your-api-key-here".to_string()),
            model_name: std::env::var("LLM_MODEL").unwrap_or_else(|_| "gpt-4".to_string()),
            max_tokens: 4000,
            temperature: 0.7,
            timeout_seconds: 30,
        },
        analytics_config: AnalyticsConfig {
            enable_realtime: true,
            retention_days: 30,
            analysis_interval: 60,
            enable_predictive: true,
        },
    }
}

async fn demonstrate_llm_functionality(
    service: &AIService,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Demonstrating LLM functionality");

    let request = LLMRequest {
        prompt: "Explain the benefits of using AI in blockchain technology".to_string(),
        context: Some({
            let mut context = HashMap::new();
            context.insert(
                "domain".to_string(),
                serde_json::Value::String("blockchain".to_string()),
            );
            context.insert(
                "audience".to_string(),
                serde_json::Value::String("developers".to_string()),
            );
            context
        }),
        max_tokens: Some(500),
        temperature: Some(0.7),
        stream: false,
    };

    match service.generate_text(request).await {
        Ok(response) => {
            info!("LLM Response: {}", response.text);
            info!("Usage: {:?}", response.usage);
        }
        Err(e) => {
            warn!(
                "LLM request failed (this is expected without a real API key): {}",
                e
            );
        }
    }

    Ok(())
}

async fn demonstrate_smart_contract_analysis(
    service: &AIService,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Demonstrating smart contract analysis");

    let contract_code = r#"
pragma solidity ^0.8.0;

contract VulnerableContract {
    mapping(address => uint256) public balances;
    
    function withdraw(uint256 amount) public {
        require(balances[msg.sender] >= amount);
        balances[msg.sender] -= amount;
        payable(msg.sender).transfer(amount);
    }
}
"#;

    let request = SmartContractAnalysisRequest {
        code: contract_code.to_string(),
        language: "solidity".to_string(),
        analysis_type: ContractAnalysisType::Security,
        context: None,
    };

    match service.analyze_smart_contract(request).await {
        Ok(analysis) => {
            info!("Contract Analysis Results:");
            info!("  Security Score: {:.2}", analysis.security_score);
            info!(
                "  Gas Efficiency Score: {:.2}",
                analysis.gas_efficiency_score
            );
            info!("  Issues Found: {}", analysis.issues.len());
            for issue in &analysis.issues {
                info!("    - {}: {}", issue.issue_type, issue.description);
            }
            info!("  Recommendations: {}", analysis.recommendations.len());
            for rec in &analysis.recommendations {
                info!("    - {}", rec);
            }
        }
        Err(e) => {
            error!("Smart contract analysis failed: {}", e);
        }
    }

    Ok(())
}

async fn demonstrate_transaction_optimization(
    service: &AIService,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Demonstrating transaction optimization");

    let transaction = TransactionData {
        tx_type: "transfer".to_string(),
        from: "0x1234567890123456789012345678901234567890".to_string(),
        to: "0x0987654321098765432109876543210987654321".to_string(),
        amount: 1000000000000000000, // 1 ETH in wei
        gas_limit: 21000,
        gas_price: 20000000000, // 20 gwei
        data: vec![],
        nonce: 1,
    };

    let request = TransactionOptimizationRequest {
        transaction,
        goals: vec![
            OptimizationGoal::MinimizeGas,
            OptimizationGoal::MinimizeCost,
        ],
        constraints: Some(OptimizationConstraints {
            max_gas_limit: Some(50000),
            max_gas_price: Some(50000000000), // 50 gwei
            max_latency_ms: Some(30000),      // 30 seconds
            security_requirements: vec!["multisig".to_string()],
        }),
    };

    match service.optimize_transaction(request).await {
        Ok(optimization) => {
            info!("Transaction Optimization Results:");
            info!("  Confidence: {:.2}", optimization.confidence);
            info!("  Suggestions: {}", optimization.suggestions.len());
            for suggestion in &optimization.suggestions {
                info!(
                    "    - {}: {}",
                    suggestion.suggestion_type, suggestion.description
                );
            }
            info!("  Expected Improvements:");
            for (metric, improvement) in &optimization.expected_improvements {
                info!("    - {}: {:.2}%", metric, improvement * 100.0);
            }
        }
        Err(e) => {
            error!("Transaction optimization failed: {}", e);
        }
    }

    Ok(())
}

async fn demonstrate_analytics(service: &mut AIService) -> Result<(), Box<dyn std::error::Error>> {
    info!("Demonstrating analytics functionality");

    // Add some sample data
    let mut tags = HashMap::new();
    tags.insert("node".to_string(), "node1".to_string());
    tags.insert("region".to_string(), "us-west-2".to_string());

    for i in 0..10 {
        service.add_analytics_data(
            "cpu_usage".to_string(),
            50.0 + (i as f64 * 5.0),
            "percent".to_string(),
            tags.clone(),
        );

        service.add_analytics_data(
            "memory_usage".to_string(),
            60.0 + (i as f64 * 2.0),
            "percent".to_string(),
            tags.clone(),
        );

        service.add_analytics_data(
            "transaction_throughput".to_string(),
            100.0 + (i as f64 * 10.0),
            "tps".to_string(),
            tags.clone(),
        );
    }

    // Get analytics insights
    match service.get_analytics_insights().await {
        Ok(insights) => {
            info!("Analytics Insights Generated: {}", insights.len());
            for insight in &insights {
                info!(
                    "  - {}: {} (confidence: {:.2})",
                    insight.title, insight.description, insight.confidence
                );
            }
        }
        Err(e) => {
            warn!("Analytics insights failed: {}", e);
        }
    }

    Ok(())
}

async fn demonstrate_monitoring(service: &mut AIService) -> Result<(), Box<dyn std::error::Error>> {
    info!("Demonstrating monitoring functionality");

    // Add some metrics that should trigger alerts
    service.add_monitoring_metric("memory_usage".to_string(), 85.0);
    service.add_monitoring_metric("cpu_usage".to_string(), 95.0);
    service.add_monitoring_metric("error_rate".to_string(), 8.0);

    // Check for alerts
    match service.check_monitoring_alerts().await {
        Ok(alerts) => {
            info!("Monitoring Alerts Generated: {}", alerts.len());
            for alert in &alerts {
                info!(
                    "  - {}: {} (severity: {:?})",
                    alert.title, alert.description, alert.severity
                );
            }
        }
        Err(e) => {
            warn!("Monitoring alerts failed: {}", e);
        }
    }

    // Get monitoring statistics
    let stats = service.get_monitoring_alerts();
    info!("Total alerts in system: {}", stats.len());

    Ok(())
}
