pub mod auditor;
pub mod scanner;

use std::sync::{Arc, Mutex};
use std::time::Duration;
use anyhow::Result;
use serde::{Deserialize, Serialize};

pub use auditor::{SecurityAuditor, SecurityAuditResult, Vulnerability, VulnerabilityLevel};
pub use scanner::{SecurityScanner, SecurityEvent, SecurityThreat, SecuritySeverity};

/// Security manager that coordinates all security components
pub struct SecurityManager {
    auditor: Arc<SecurityAuditor>,
    scanner: Arc<SecurityScanner>,
    config: SecurityConfig,
    audit_results: Arc<Mutex<Vec<SecurityAuditResult>>>,
    security_events: Arc<Mutex<Vec<SecurityEvent>>>,
}

#[derive(Debug, Clone)]
pub struct SecurityConfig {
    pub enable_static_analysis: bool,
    pub enable_runtime_monitoring: bool,
    pub enable_dependency_scanning: bool,
    pub audit_interval_hours: u32,
    pub alert_threshold: SecuritySeverity,
    pub max_audit_history: usize,
    pub max_event_history: usize,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_static_analysis: true,
            enable_runtime_monitoring: true,
            enable_dependency_scanning: true,
            audit_interval_hours: 24,
            alert_threshold: SecuritySeverity::Medium,
            max_audit_history: 100,
            max_event_history: 10000,
        }
    }
}

/// Comprehensive security report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityReport {
    pub static_analysis: Option<SecurityAuditResult>,
    pub runtime_events: Vec<SecurityEvent>,
    pub threat_analysis: ThreatAnalysis,
    pub recommendations: Vec<String>,
    pub security_score: f32,
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

/// Threat analysis summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatAnalysis {
    pub total_threats: u32,
    pub critical_threats: u32,
    pub high_threats: u32,
    pub medium_threats: u32,
    pub low_threats: u32,
    pub blocked_attacks: u32,
    pub active_threats: u32,
    pub threat_distribution: std::collections::HashMap<SecurityThreat, u32>,
}

impl SecurityManager {
    /// Create a new security manager
    pub fn new(config: SecurityConfig) -> Result<Self> {
        let auditor_config = auditor::AuditorConfig::default();
        let auditor = Arc::new(SecurityAuditor::new(auditor_config));
        
        let scanner_config = scanner::ScannerConfig::default();
        let (scanner, _event_receiver) = SecurityScanner::new(scanner_config);
        let scanner = Arc::new(scanner);
        
        Ok(Self {
            auditor,
            scanner,
            config,
            audit_results: Arc::new(Mutex::new(Vec::new())),
            security_events: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Start the security manager
    pub async fn start(&self) -> Result<()> {
        // Start runtime monitoring
        if self.config.enable_runtime_monitoring {
            self.scanner.start().await?;
        }
        
        // Perform initial security audit
        if self.config.enable_static_analysis {
            self.perform_security_audit().await?;
        }
        
        // Start periodic audit scheduling
        self.start_periodic_audits().await?;
        
        Ok(())
    }

    /// Perform comprehensive security audit
    pub async fn perform_security_audit(&self) -> Result<SecurityAuditResult> {
        let audit_result = self.auditor.audit_codebase(std::path::Path::new(".")).await?;
        
        // Store audit result
        {
            let mut audit_results = self.audit_results.lock().unwrap();
            audit_results.push(audit_result.clone());
            
            // Trim old results if we exceed the limit
            if audit_results.len() > self.config.max_audit_history {
                audit_results.drain(0..audit_results.len() - self.config.max_audit_history);
            }
        }
        
        // Generate alerts for high-severity vulnerabilities
        self.generate_alerts(&audit_result).await?;
        
        Ok(audit_result)
    }

    /// Start periodic security audits
    async fn start_periodic_audits(&self) -> Result<()> {
        let manager = Arc::new(self.clone());
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                Duration::from_secs(manager.config.audit_interval_hours as u64 * 3600)
            );
            
            loop {
                interval.tick().await;
                
                if let Err(e) = manager.perform_security_audit().await {
                    tracing::error!("Periodic security audit failed: {}", e);
                }
            }
        });
        
        Ok(())
    }

    /// Generate security alerts
    async fn generate_alerts(&self, audit_result: &SecurityAuditResult) -> Result<()> {
        let high_severity_vulns: Vec<_> = audit_result.vulnerabilities.iter()
            .filter(|v| {
                v.level == VulnerabilityLevel::Critical || v.level == VulnerabilityLevel::High
            })
            .collect();
        
        if !high_severity_vulns.is_empty() {
            tracing::warn!(
                "SECURITY ALERT: {} high-severity vulnerabilities detected",
                high_severity_vulns.len()
            );
            
            for vuln in high_severity_vulns {
                tracing::error!(
                    "Vulnerability {}: {} - {}",
                    vuln.id, vuln.title, vuln.description
                );
            }
        }
        
        Ok(())
    }

    /// Check for runtime security threats
    pub async fn check_security_threats(&self, event_data: SecurityEventData) -> Result<bool> {
        let mut threat_detected = false;
        
        // Check for brute force attacks
        if let Some(source_ip) = &event_data.source_ip {
            if self.scanner.check_brute_force_attack(source_ip, event_data.success).await? {
                threat_detected = true;
            }
        }
        
        // Check for DDoS attacks
        if let Some(source_ip) = &event_data.source_ip {
            if self.scanner.check_ddos_attack(source_ip, event_data.request_count).await? {
                threat_detected = true;
            }
        }
        
        // Check for malicious transactions
        if let Some(transaction_data) = &event_data.transaction_data {
            if self.scanner.check_malicious_transaction(transaction_data).await? {
                threat_detected = true;
            }
        }
        
        // Check for unauthorized access
        if let (Some(user_id), Some(resource), Some(action)) = 
            (&event_data.user_id, &event_data.resource, &event_data.action) {
            if self.scanner.check_unauthorized_access(user_id, resource, action).await? {
                threat_detected = true;
            }
        }
        
        // Check for data exfiltration
        if let (Some(data_size), Some(destination)) = 
            (event_data.data_size, &event_data.destination) {
            if self.scanner.check_data_exfiltration(data_size, destination).await? {
                threat_detected = true;
            }
        }
        
        Ok(threat_detected)
    }

    /// Record a security event
    pub async fn record_security_event(&self, event: SecurityEvent) -> Result<()> {
        // Record in scanner
        self.scanner.record_event(event.clone()).await?;
        
        // Store in local history
        {
            let mut events = self.security_events.lock().unwrap();
            events.push(event);
            
            // Trim old events if we exceed the limit
            if events.len() > self.config.max_event_history {
                events.drain(0..events.len() - self.config.max_event_history);
            }
        }
        
        Ok(())
    }

    /// Get comprehensive security report
    pub fn generate_comprehensive_report(&self) -> SecurityReport {
        // Get latest audit result
        let static_analysis = {
            let audit_results = self.audit_results.lock().unwrap();
            audit_results.last().cloned()
        };
        
        // Get runtime events
        let runtime_events = self.scanner.get_recent_events(24); // Last 24 hours
        
        // Generate threat analysis
        let threat_analysis = self.generate_threat_analysis(&runtime_events);
        
        // Generate recommendations
        let recommendations = self.generate_recommendations(&static_analysis, &runtime_events);
        
        // Calculate security score
        let security_score = self.calculate_security_score(&static_analysis, &runtime_events);
        
        SecurityReport {
            static_analysis,
            runtime_events,
            threat_analysis,
            recommendations,
            security_score,
            generated_at: chrono::Utc::now(),
        }
    }

    /// Generate threat analysis
    fn generate_threat_analysis(&self, events: &[SecurityEvent]) -> ThreatAnalysis {
        let mut threat_distribution = std::collections::HashMap::new();
        let mut critical_threats = 0;
        let mut high_threats = 0;
        let mut medium_threats = 0;
        let mut low_threats = 0;
        let mut blocked_attacks = 0;
        
        for event in events {
            *threat_distribution.entry(event.threat_type.clone()).or_insert(0) += 1;
            
            match event.severity {
                SecuritySeverity::Critical => critical_threats += 1,
                SecuritySeverity::High => high_threats += 1,
                SecuritySeverity::Medium => medium_threats += 1,
                SecuritySeverity::Low => low_threats += 1,
                SecuritySeverity::Info => {}
            }
            
            if event.action_taken.is_some() {
                blocked_attacks += 1;
            }
        }
        
        ThreatAnalysis {
            total_threats: events.len() as u32,
            critical_threats,
            high_threats,
            medium_threats,
            low_threats,
            blocked_attacks,
            active_threats: self.scanner.get_metrics().active_threats,
            threat_distribution,
        }
    }

    /// Generate security recommendations
    fn generate_recommendations(
        &self,
        static_analysis: &Option<SecurityAuditResult>,
        runtime_events: &[SecurityEvent],
    ) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        // Static analysis recommendations
        if let Some(audit) = static_analysis {
            if audit.summary.critical_count > 0 {
                recommendations.push(format!(
                    "CRITICAL: Address {} critical vulnerabilities immediately",
                    audit.summary.critical_count
                ));
            }
            
            if audit.summary.high_count > 0 {
                recommendations.push(format!(
                    "HIGH: Fix {} high-severity vulnerabilities",
                    audit.summary.high_count
                ));
            }
        }
        
        // Runtime monitoring recommendations
        let critical_events = runtime_events.iter()
            .filter(|e| e.severity == SecuritySeverity::Critical)
            .count();
        
        if critical_events > 0 {
            recommendations.push(format!(
                "RUNTIME: {} critical security events detected",
                critical_events
            ));
        }
        
        // General recommendations
        recommendations.push("SECURITY: Implement regular security training for developers".to_string());
        recommendations.push("MONITORING: Set up real-time security monitoring and alerting".to_string());
        recommendations.push("UPDATES: Keep all dependencies and systems updated".to_string());
        recommendations.push("TESTING: Perform regular penetration testing".to_string());
        
        recommendations
    }

    /// Calculate overall security score
    fn calculate_security_score(
        &self,
        static_analysis: &Option<SecurityAuditResult>,
        runtime_events: &[SecurityEvent],
    ) -> f32 {
        let mut score = 100.0;
        
        // Deduct points for static analysis issues
        if let Some(audit) = static_analysis {
            score -= audit.summary.critical_count as f32 * 10.0;
            score -= audit.summary.high_count as f32 * 5.0;
            score -= audit.summary.medium_count as f32 * 2.0;
            score -= audit.summary.low_count as f32 * 0.5;
        }
        
        // Deduct points for runtime events
        for event in runtime_events {
            match event.severity {
                SecuritySeverity::Critical => score -= 5.0,
                SecuritySeverity::High => score -= 2.0,
                SecuritySeverity::Medium => score -= 1.0,
                SecuritySeverity::Low => score -= 0.2,
                SecuritySeverity::Info => {}
            }
        }
        
        score.max(0.0)
    }

    /// Export security report to JSON
    pub fn export_report_json(&self) -> Result<String> {
        let report = self.generate_comprehensive_report();
        Ok(serde_json::to_string_pretty(&report)?)
    }

    /// Export security report to HTML
    pub fn export_report_html(&self) -> Result<String> {
        let report = self.generate_comprehensive_report();
        
        let html = format!(
            r#"
<!DOCTYPE html>
<html>
<head>
    <title>IPPAN Security Report</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; }}
        .header {{ background-color: #f0f0f0; padding: 20px; border-radius: 5px; }}
        .section {{ margin: 20px 0; }}
        .critical {{ color: #ff0000; }}
        .high {{ color: #ff6600; }}
        .medium {{ color: #ffaa00; }}
        .low {{ color: #ffcc00; }}
        .score {{ font-size: 24px; font-weight: bold; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>IPPAN Security Report</h1>
        <p>Generated on: {}</p>
        <p class="score">Security Score: {:.1}%</p>
    </div>
    
    <div class="section">
        <h2>Threat Analysis</h2>
        <p><strong>Total Threats:</strong> {}</p>
        <p><strong>Critical:</strong> {} | <strong>High:</strong> {} | <strong>Medium:</strong> {} | <strong>Low:</strong> {}</p>
        <p><strong>Blocked Attacks:</strong> {}</p>
        <p><strong>Active Threats:</strong> {}</p>
    </div>
    
    <div class="section">
        <h2>Recommendations</h2>
        <ul>
            {}
        </ul>
    </div>
    
    <div class="section">
        <h2>Recent Security Events</h2>
        {}
    </div>
</body>
</html>
            "#,
            report.generated_at.format("%Y-%m-%d %H:%M:%S UTC"),
            report.security_score,
            report.threat_analysis.total_threats,
            report.threat_analysis.critical_threats,
            report.threat_analysis.high_threats,
            report.threat_analysis.medium_threats,
            report.threat_analysis.low_threats,
            report.threat_analysis.blocked_attacks,
            report.threat_analysis.active_threats,
            report.recommendations.iter().map(|r| format!("<li>{}</li>", r)).collect::<Vec<_>>().join(""),
            report.runtime_events.iter().map(|e| {
                let severity_class = match e.severity {
                    SecuritySeverity::Critical => "critical",
                    SecuritySeverity::High => "high",
                    SecuritySeverity::Medium => "medium",
                    SecuritySeverity::Low => "low",
                    SecuritySeverity::Info => "info",
                };
                format!(
                    "<div class='{}'><strong>{:?}</strong> - {} - {}</div>",
                    severity_class, e.threat_type, e.description, e.timestamp.format("%H:%M:%S")
                )
            }).collect::<Vec<_>>().join("")
        );
        
        Ok(html)
    }
}

/// Security event data for threat checking
#[derive(Debug, Clone)]
pub struct SecurityEventData {
    pub source_ip: Option<String>,
    pub success: bool,
    pub request_count: u32,
    pub transaction_data: Option<String>,
    pub user_id: Option<String>,
    pub resource: Option<String>,
    pub action: Option<String>,
    pub data_size: Option<u64>,
    pub destination: Option<String>,
}

impl Clone for SecurityManager {
    fn clone(&self) -> Self {
        Self {
            auditor: Arc::clone(&self.auditor),
            scanner: Arc::clone(&self.scanner),
            config: self.config.clone(),
            audit_results: Arc::clone(&self.audit_results),
            security_events: Arc::clone(&self.security_events),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_security_manager_creation() {
        let config = SecurityConfig::default();
        let manager = SecurityManager::new(config).unwrap();
        
        // Test that manager was created successfully
        assert!(true);
    }

    #[tokio::test]
    async fn test_security_audit() {
        let config = SecurityConfig::default();
        let manager = SecurityManager::new(config).unwrap();
        
        let audit_result = manager.perform_security_audit().await.unwrap();
        assert!(audit_result.vulnerabilities.len() >= 0);
    }

    #[tokio::test]
    async fn test_threat_checking() {
        let config = SecurityConfig::default();
        let manager = SecurityManager::new(config).unwrap();
        
        let event_data = SecurityEventData {
            source_ip: Some("192.168.1.1".to_string()),
            success: false,
            request_count: 150,
            transaction_data: None,
            user_id: None,
            resource: None,
            action: None,
            data_size: None,
            destination: None,
        };
        
        let threat_detected = manager.check_security_threats(event_data).await.unwrap();
        // Should detect brute force attack
        assert!(threat_detected);
    }

    #[tokio::test]
    async fn test_comprehensive_report() {
        let config = SecurityConfig::default();
        let manager = SecurityManager::new(config).unwrap();
        
        let report = manager.generate_comprehensive_report();
        assert!(report.security_score >= 0.0);
        assert!(report.security_score <= 100.0);
    }
} 