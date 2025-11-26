//! Health check and status endpoints for production monitoring

use crate::{AIService, AIServiceError};
use ippan_ai_core::Fixed;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: HealthStatus,
    pub timestamp: u64,
    pub version: String,
    pub uptime: Duration,
    pub checks: HashMap<String, CheckResult>,
}

/// Health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Individual check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub status: CheckStatus,
    pub message: Option<String>,
    pub duration_ms: u64,
}

/// Check status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CheckStatus {
    Pass,
    Fail,
    Warn,
}

impl AIService {
    /// Get health status
    pub async fn health_check(&self) -> Result<HealthResponse, AIServiceError> {
        let start_time = SystemTime::now();
        let mut checks = HashMap::new();

        // Check service status
        let service_status = self.get_status();
        checks.insert(
            "service_status".to_string(),
            CheckResult {
                status: if service_status.is_running {
                    CheckStatus::Pass
                } else {
                    CheckStatus::Fail
                },
                message: Some(format!(
                    "Service is {}",
                    if service_status.is_running {
                        "running"
                    } else {
                        "stopped"
                    }
                )),
                duration_ms: 0,
            },
        );

        // Check monitoring service
        if self.get_config().enable_monitoring {
            let monitor_check = self.check_monitoring_health().await;
            checks.insert("monitoring".to_string(), monitor_check);
        }

        // Check analytics service
        if self.get_config().enable_analytics {
            let analytics_check = self.check_analytics_health().await;
            checks.insert("analytics".to_string(), analytics_check);
        }

        // Check LLM service
        if self.get_config().enable_llm {
            let llm_check = self.check_llm_health().await;
            checks.insert("llm".to_string(), llm_check);
        }

        // Check smart contract service
        if self.get_config().enable_smart_contracts {
            let sc_check = self.check_smart_contract_health().await;
            checks.insert("smart_contracts".to_string(), sc_check);
        }

        // Determine overall status
        let status = determine_health_status(&checks);
        let _duration = start_time.elapsed().unwrap_or_default();

        Ok(HealthResponse {
            status,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime: self.get_uptime(),
            checks,
        })
    }

    /// Check monitoring service health
    async fn check_monitoring_health(&self) -> CheckResult {
        let start = SystemTime::now();
        #[cfg(feature = "analytics")]
        {
            let alerts = self.monitoring_alerts_snapshot();
            let duration = start.elapsed().unwrap_or_default();
            let has_warnings = !alerts.is_empty();
            CheckResult {
                status: if has_warnings {
                    CheckStatus::Warn
                } else {
                    CheckStatus::Pass
                },
                message: Some(if has_warnings {
                    "Monitoring alerts present".to_string()
                } else {
                    "Monitoring healthy".to_string()
                }),
                duration_ms: duration.as_millis() as u64,
            }
        }
        #[cfg(not(feature = "analytics"))]
        {
            let duration = start.elapsed().unwrap_or_default();
            return CheckResult {
                status: CheckStatus::Pass,
                message: Some("Monitoring feature not compiled".to_string()),
                duration_ms: duration.as_millis() as u64,
            };
        }
    }

    /// Check analytics service health
    async fn check_analytics_health(&self) -> CheckResult {
        let start = SystemTime::now();
        #[cfg(feature = "analytics")]
        {
            let insights = self.analytics_insights_snapshot();
            let has_high = insights.iter().any(|i| {
                matches!(
                    i.severity,
                    crate::types::SeverityLevel::High | crate::types::SeverityLevel::Critical
                )
            });
            let duration = start.elapsed().unwrap_or_default();
            CheckResult {
                status: if has_high {
                    CheckStatus::Warn
                } else {
                    CheckStatus::Pass
                },
                message: Some(if has_high {
                    "Analytics elevated severities present".to_string()
                } else {
                    "Analytics healthy".to_string()
                }),
                duration_ms: duration.as_millis() as u64,
            }
        }
        #[cfg(not(feature = "analytics"))]
        {
            let duration = start.elapsed().unwrap_or_default();
            return CheckResult {
                status: CheckStatus::Pass,
                message: Some("Analytics feature not compiled".to_string()),
                duration_ms: duration.as_millis() as u64,
            };
        }
    }

    /// Check LLM service health
    async fn check_llm_health(&self) -> CheckResult {
        let start = SystemTime::now();

        // Try a simple LLM request to check connectivity
        let request = crate::LLMRequest {
            prompt: "Health check".to_string(),
            context: None,
            max_tokens: Some(1),
            temperature: Some(Fixed::ZERO),
            stream: false,
        };

        match self.generate_text(request).await {
            Ok(_) => {
                let duration = start.elapsed().unwrap_or_default();
                CheckResult {
                    status: CheckStatus::Pass,
                    message: Some("LLM service: healthy".to_string()),
                    duration_ms: duration.as_millis() as u64,
                }
            }
            Err(e) => {
                let duration = start.elapsed().unwrap_or_default();
                CheckResult {
                    status: CheckStatus::Warn,
                    message: Some(format!("LLM service warning: {e}")),
                    duration_ms: duration.as_millis() as u64,
                }
            }
        }
    }

    /// Check smart contract service health
    async fn check_smart_contract_health(&self) -> CheckResult {
        let start = SystemTime::now();

        // Try a simple smart contract analysis
        let request = crate::SmartContractAnalysisRequest {
            code: "contract Test {}".to_string(),
            language: "solidity".to_string(),
            analysis_type: crate::ContractAnalysisType::Security,
            context: None,
        };

        match self.analyze_smart_contract(request).await {
            Ok(_) => {
                let duration = start.elapsed().unwrap_or_default();
                CheckResult {
                    status: CheckStatus::Pass,
                    message: Some("Smart contract service: healthy".to_string()),
                    duration_ms: duration.as_millis() as u64,
                }
            }
            Err(e) => {
                let duration = start.elapsed().unwrap_or_default();
                CheckResult {
                    status: CheckStatus::Warn,
                    message: Some(format!("Smart contract service warning: {e}")),
                    duration_ms: duration.as_millis() as u64,
                }
            }
        }
    }

    /// Get service uptime
    fn get_uptime(&self) -> Duration {
        // This would be tracked from service start time in a real implementation
        Duration::from_secs(0)
    }
}

// (no helper clones of AIService; health checks use live service state via getters)

/// Determine overall health status based on individual checks
fn determine_health_status(checks: &HashMap<String, CheckResult>) -> HealthStatus {
    let mut has_failures = false;
    let mut has_warnings = false;

    for check in checks.values() {
        match check.status {
            CheckStatus::Fail => has_failures = true,
            CheckStatus::Warn => has_warnings = true,
            CheckStatus::Pass => {}
        }
    }

    if has_failures {
        HealthStatus::Unhealthy
    } else if has_warnings {
        HealthStatus::Degraded
    } else {
        HealthStatus::Healthy
    }
}
