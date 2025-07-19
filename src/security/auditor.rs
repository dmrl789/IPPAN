use std::path::Path;
use serde::{Deserialize, Serialize};
use tokio::fs;
use anyhow::Result;

/// Security vulnerability levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VulnerabilityLevel {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

/// Security vulnerability types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VulnerabilityType {
    MemorySafety,
    CryptographicWeakness,
    InputValidation,
    AccessControl,
    DataExposure,
    NetworkSecurity,
    ConfigurationIssue,
    DependencyVulnerability,
    CodeInjection,
    DenialOfService,
}

/// Security vulnerability details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vulnerability {
    pub id: String,
    pub title: String,
    pub description: String,
    pub vulnerability_type: VulnerabilityType,
    pub level: VulnerabilityLevel,
    pub file_path: Option<String>,
    pub line_number: Option<u32>,
    pub code_snippet: Option<String>,
    pub remediation: String,
    pub cwe_id: Option<String>,
    pub cvss_score: Option<f32>,
    pub discovered_at: chrono::DateTime<chrono::Utc>,
}

/// Security audit results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAuditResult {
    pub vulnerabilities: Vec<Vulnerability>,
    pub summary: AuditSummary,
    pub recommendations: Vec<String>,
    pub audit_timestamp: chrono::DateTime<chrono::Utc>,
}

/// Audit summary statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditSummary {
    pub total_vulnerabilities: usize,
    pub critical_count: usize,
    pub high_count: usize,
    pub medium_count: usize,
    pub low_count: usize,
    pub info_count: usize,
    pub files_scanned: usize,
    pub lines_of_code: usize,
    pub security_score: f32,
}

/// Security auditor for IPPAN codebase
pub struct SecurityAuditor {
    rules: Vec<SecurityRule>,
    // patterns: HashMap<String, Vec<String>>, // TODO: Use when implementing pattern caching
    config: AuditorConfig,
}

#[derive(Debug, Clone)]
pub struct AuditorConfig {
    pub scan_dependencies: bool,
    pub scan_config_files: bool,
    pub scan_documentation: bool,
    pub max_file_size: usize,
    pub excluded_paths: Vec<String>,
    pub severity_threshold: VulnerabilityLevel,
}

impl Default for AuditorConfig {
    fn default() -> Self {
        Self {
            scan_dependencies: true,
            scan_config_files: true,
            scan_documentation: true,
            max_file_size: 1024 * 1024, // 1MB
            excluded_paths: vec![
                "target/".to_string(),
                ".git/".to_string(),
                "node_modules/".to_string(),
            ],
            severity_threshold: VulnerabilityLevel::Medium,
        }
    }
}

/// Security rule for pattern matching
#[derive(Debug, Clone)]
pub struct SecurityRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub patterns: Vec<String>,
    pub vulnerability_type: VulnerabilityType,
    pub level: VulnerabilityLevel,
    pub cwe_id: Option<String>,
    pub remediation: String,
}

impl SecurityAuditor {
    /// Create a new security auditor
    pub fn new(config: AuditorConfig) -> Self {
        let mut auditor = Self {
            rules: Vec::new(),
            // patterns: HashMap::new(), // TODO: Use when implementing pattern caching
            config,
        };
        
        auditor.initialize_rules();
        auditor
    }

    /// Initialize security rules
    fn initialize_rules(&mut self) {
        // Memory safety rules
        self.rules.push(SecurityRule {
            id: "MEM001".to_string(),
            name: "Unsafe Memory Access".to_string(),
            description: "Detect unsafe memory operations".to_string(),
            patterns: vec![
                r"unsafe\s*\{".to_string(),
                r"std::ptr::".to_string(),
                r"std::mem::".to_string(),
            ],
            vulnerability_type: VulnerabilityType::MemorySafety,
            level: VulnerabilityLevel::High,
            cwe_id: Some("CWE-119".to_string()),
            remediation: "Review unsafe blocks and ensure proper bounds checking".to_string(),
        });

        // Cryptographic weakness rules
        self.rules.push(SecurityRule {
            id: "CRY001".to_string(),
            name: "Weak Cryptographic Algorithm".to_string(),
            description: "Detect use of weak cryptographic algorithms".to_string(),
            patterns: vec![
                r"MD5".to_string(),
                r"SHA1".to_string(),
                r"DES".to_string(),
                r"RC4".to_string(),
            ],
            vulnerability_type: VulnerabilityType::CryptographicWeakness,
            level: VulnerabilityLevel::Critical,
            cwe_id: Some("CWE-327".to_string()),
            remediation: "Use strong cryptographic algorithms (SHA-256, AES-256)".to_string(),
        });

        // Input validation rules
        self.rules.push(SecurityRule {
            id: "INP001".to_string(),
            name: "Missing Input Validation".to_string(),
            description: "Detect missing input validation".to_string(),
            patterns: vec![
                r"unwrap\(\)".to_string(),
                r"expect\(".to_string(),
                r"panic!".to_string(),
            ],
            vulnerability_type: VulnerabilityType::InputValidation,
            level: VulnerabilityLevel::Medium,
            cwe_id: Some("CWE-20".to_string()),
            remediation: "Add proper input validation and error handling".to_string(),
        });

        // Access control rules
        self.rules.push(SecurityRule {
            id: "ACC001".to_string(),
            name: "Insufficient Access Control".to_string(),
            description: "Detect missing access control checks".to_string(),
            patterns: vec![
                r"pub\s+fn".to_string(),
                r"pub\s+struct".to_string(),
            ],
            vulnerability_type: VulnerabilityType::AccessControl,
            level: VulnerabilityLevel::Medium,
            cwe_id: Some("CWE-284".to_string()),
            remediation: "Review public APIs and ensure proper access control".to_string(),
        });

        // Data exposure rules
        self.rules.push(SecurityRule {
            id: "EXP001".to_string(),
            name: "Sensitive Data Exposure".to_string(),
            description: "Detect potential sensitive data exposure".to_string(),
            patterns: vec![
                r"password".to_string(),
                r"secret".to_string(),
                r"private_key".to_string(),
                r"api_key".to_string(),
            ],
            vulnerability_type: VulnerabilityType::DataExposure,
            level: VulnerabilityLevel::High,
            cwe_id: Some("CWE-200".to_string()),
            remediation: "Ensure sensitive data is properly encrypted and not logged".to_string(),
        });

        // Network security rules
        self.rules.push(SecurityRule {
            id: "NET001".to_string(),
            name: "Insecure Network Communication".to_string(),
            description: "Detect insecure network communication".to_string(),
            patterns: vec![
                r"http://".to_string(),
                r"ws://".to_string(),
            ],
            vulnerability_type: VulnerabilityType::NetworkSecurity,
            level: VulnerabilityLevel::High,
            cwe_id: Some("CWE-319".to_string()),
            remediation: "Use HTTPS/WSS for secure communication".to_string(),
        });

        // Configuration issues
        self.rules.push(SecurityRule {
            id: "CFG001".to_string(),
            name: "Insecure Configuration".to_string(),
            description: "Detect insecure configuration patterns".to_string(),
            patterns: vec![
                r"debug\s*=\s*true".to_string(),
                r"production\s*=\s*false".to_string(),
            ],
            vulnerability_type: VulnerabilityType::ConfigurationIssue,
            level: VulnerabilityLevel::Medium,
            cwe_id: Some("CWE-16".to_string()),
            remediation: "Use secure configuration for production environments".to_string(),
        });

        // Code injection rules
        self.rules.push(SecurityRule {
            id: "INJ001".to_string(),
            name: "Potential Code Injection".to_string(),
            description: "Detect potential code injection vulnerabilities".to_string(),
            patterns: vec![
                r"eval\(".to_string(),
                r"exec\(".to_string(),
                r"system\(".to_string(),
            ],
            vulnerability_type: VulnerabilityType::CodeInjection,
            level: VulnerabilityLevel::Critical,
            cwe_id: Some("CWE-94".to_string()),
            remediation: "Avoid dynamic code execution, use safe alternatives".to_string(),
        });

        // Denial of service rules
        self.rules.push(SecurityRule {
            id: "DOS001".to_string(),
            name: "Potential Denial of Service".to_string(),
            description: "Detect potential DoS vulnerabilities".to_string(),
            patterns: vec![
                r"loop\s*\{".to_string(),
                r"while\s+true".to_string(),
                r"recursive".to_string(),
            ],
            vulnerability_type: VulnerabilityType::DenialOfService,
            level: VulnerabilityLevel::Medium,
            cwe_id: Some("CWE-400".to_string()),
            remediation: "Add proper termination conditions and resource limits".to_string(),
        });
    }

    /// Perform comprehensive security audit
    pub async fn audit_codebase(&self, root_path: &Path) -> Result<SecurityAuditResult> {
        let mut vulnerabilities = Vec::new();
        let mut files_scanned = 0;
        let mut lines_of_code = 0;

        // Scan source files
        let source_files = self.collect_source_files(root_path).await?;
        
        for file_path in source_files {
            if let Ok(file_content) = fs::read_to_string(&file_path).await {
                files_scanned += 1;
                lines_of_code += file_content.lines().count();

                let file_vulnerabilities = self.scan_file(&file_path, &file_content);
                vulnerabilities.extend(file_vulnerabilities);
            }
        }

        // Scan dependencies if enabled
        if self.config.scan_dependencies {
            let dep_vulnerabilities = self.scan_dependencies().await?;
            vulnerabilities.extend(dep_vulnerabilities);
        }

        // Generate summary
        let summary = self.generate_summary(&vulnerabilities, files_scanned, lines_of_code);
        
        // Generate recommendations
        let recommendations = self.generate_recommendations(&vulnerabilities);

        Ok(SecurityAuditResult {
            vulnerabilities,
            summary,
            recommendations,
            audit_timestamp: chrono::Utc::now(),
        })
    }

    /// Collect source files for scanning
    async fn collect_source_files(&self, root_path: &Path) -> Result<Vec<std::path::PathBuf>> {
        let mut files = Vec::new();
        let mut dirs_to_scan = vec![root_path.to_path_buf()];
        
        while let Some(current_dir) = dirs_to_scan.pop() {
            let mut entries = fs::read_dir(&current_dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                
                if path.is_file() {
                    if let Some(extension) = path.extension() {
                        if extension == "rs" || extension == "toml" || extension == "md" {
                            if !self.is_excluded(&path) {
                                files.push(path);
                            }
                        }
                    }
                } else if path.is_dir() {
                    if !self.is_excluded(&path) {
                        dirs_to_scan.push(path);
                    }
                }
            }
        }
        
        Ok(files)
    }

    /// Check if path should be excluded
    fn is_excluded(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        self.config.excluded_paths.iter().any(|excluded| {
            path_str.contains(excluded)
        })
    }

    /// Scan a single file for vulnerabilities
    fn scan_file(&self, file_path: &Path, content: &str) -> Vec<Vulnerability> {
        let mut vulnerabilities = Vec::new();
        
        for (line_number, line) in content.lines().enumerate() {
            for rule in &self.rules {
                for pattern in &rule.patterns {
                    if line.contains(pattern) {
                        let vulnerability = Vulnerability {
                            id: rule.id.clone(),
                            title: rule.name.clone(),
                            description: rule.description.clone(),
                            vulnerability_type: rule.vulnerability_type.clone(),
                            level: rule.level.clone(),
                            file_path: Some(file_path.to_string_lossy().to_string()),
                            line_number: Some(line_number as u32 + 1),
                            code_snippet: Some(line.trim().to_string()),
                            remediation: rule.remediation.clone(),
                            cwe_id: rule.cwe_id.clone(),
                            cvss_score: self.calculate_cvss_score(&rule.level),
                            discovered_at: chrono::Utc::now(),
                        };
                        
                        vulnerabilities.push(vulnerability);
                    }
                }
            }
        }
        
        vulnerabilities
    }

    /// Scan dependencies for known vulnerabilities
    async fn scan_dependencies(&self) -> Result<Vec<Vulnerability>> {
        let mut vulnerabilities = Vec::new();
        
        // Check Cargo.lock for known vulnerabilities
        if let Ok(cargo_lock) = fs::read_to_string("Cargo.lock").await {
            // This is a simplified check - in a real implementation,
            // you would query vulnerability databases
            if cargo_lock.contains("vulnerability") || cargo_lock.contains("CVE") {
                vulnerabilities.push(Vulnerability {
                    id: "DEP001".to_string(),
                    title: "Known Dependency Vulnerability".to_string(),
                    description: "Dependency with known security vulnerability detected".to_string(),
                    vulnerability_type: VulnerabilityType::DependencyVulnerability,
                    level: VulnerabilityLevel::High,
                    file_path: Some("Cargo.lock".to_string()),
                    line_number: None,
                    code_snippet: None,
                    remediation: "Update dependencies to latest secure versions".to_string(),
                    cwe_id: Some("CWE-400".to_string()),
                    cvss_score: Some(7.5),
                    discovered_at: chrono::Utc::now(),
                });
            }
        }
        
        Ok(vulnerabilities)
    }

    /// Calculate CVSS score based on vulnerability level
    fn calculate_cvss_score(&self, level: &VulnerabilityLevel) -> Option<f32> {
        match level {
            VulnerabilityLevel::Critical => Some(9.0),
            VulnerabilityLevel::High => Some(7.5),
            VulnerabilityLevel::Medium => Some(5.0),
            VulnerabilityLevel::Low => Some(2.0),
            VulnerabilityLevel::Info => Some(0.0),
        }
    }

    /// Generate audit summary
    fn generate_summary(
        &self,
        vulnerabilities: &[Vulnerability],
        files_scanned: usize,
        lines_of_code: usize,
    ) -> AuditSummary {
        let mut critical_count = 0;
        let mut high_count = 0;
        let mut medium_count = 0;
        let mut low_count = 0;
        let mut info_count = 0;

        for vuln in vulnerabilities {
            match vuln.level {
                VulnerabilityLevel::Critical => critical_count += 1,
                VulnerabilityLevel::High => high_count += 1,
                VulnerabilityLevel::Medium => medium_count += 1,
                VulnerabilityLevel::Low => low_count += 1,
                VulnerabilityLevel::Info => info_count += 1,
            }
        }

        let total_vulnerabilities = vulnerabilities.len();
        let security_score = if total_vulnerabilities > 0 {
            let weighted_score = (critical_count * 10 + high_count * 7 + medium_count * 4 + low_count * 1) as f32;
            (100.0 - (weighted_score / total_vulnerabilities as f32)).max(0.0)
        } else {
            100.0
        };

        AuditSummary {
            total_vulnerabilities,
            critical_count,
            high_count,
            medium_count,
            low_count,
            info_count,
            files_scanned,
            lines_of_code,
            security_score,
        }
    }

    /// Generate security recommendations
    fn generate_recommendations(&self, vulnerabilities: &[Vulnerability]) -> Vec<String> {
        let mut recommendations = Vec::new();

        // Critical vulnerabilities
        let critical_count = vulnerabilities.iter()
            .filter(|v| v.level == VulnerabilityLevel::Critical)
            .count();
        
        if critical_count > 0 {
            recommendations.push(format!(
                "CRITICAL: Address {} critical vulnerabilities immediately",
                critical_count
            ));
        }

        // High vulnerabilities
        let high_count = vulnerabilities.iter()
            .filter(|v| v.level == VulnerabilityLevel::High)
            .count();
        
        if high_count > 0 {
            recommendations.push(format!(
                "HIGH: Fix {} high-severity vulnerabilities as soon as possible",
                high_count
            ));
        }

        // Cryptographic issues
        let crypto_issues = vulnerabilities.iter()
            .filter(|v| matches!(v.vulnerability_type, VulnerabilityType::CryptographicWeakness))
            .count();
        
        if crypto_issues > 0 {
            recommendations.push("CRYPTO: Review and update cryptographic implementations".to_string());
        }

        // Memory safety issues
        let memory_issues = vulnerabilities.iter()
            .filter(|v| matches!(v.vulnerability_type, VulnerabilityType::MemorySafety))
            .count();
        
        if memory_issues > 0 {
            recommendations.push("MEMORY: Review unsafe code blocks and memory operations".to_string());
        }

        // Input validation
        let input_issues = vulnerabilities.iter()
            .filter(|v| matches!(v.vulnerability_type, VulnerabilityType::InputValidation))
            .count();
        
        if input_issues > 0 {
            recommendations.push("INPUT: Implement proper input validation and error handling".to_string());
        }

        // General recommendations
        recommendations.push("SECURITY: Implement regular security audits and penetration testing".to_string());
        recommendations.push("DEPENDENCIES: Keep dependencies updated and monitor for vulnerabilities".to_string());
        recommendations.push("CONFIGURATION: Use secure configuration for production environments".to_string());

        recommendations
    }

    /// Export audit results to JSON
    pub fn export_results_json(&self, results: &SecurityAuditResult) -> Result<String> {
        Ok(serde_json::to_string_pretty(results)?)
    }

    /// Export audit results to HTML report
    pub fn export_results_html(&self, results: &SecurityAuditResult) -> Result<String> {
        let html = format!(
            r#"
<!DOCTYPE html>
<html>
<head>
    <title>IPPAN Security Audit Report</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; }}
        .header {{ background-color: #f0f0f0; padding: 20px; border-radius: 5px; }}
        .summary {{ margin: 20px 0; }}
        .vulnerability {{ margin: 10px 0; padding: 10px; border-left: 4px solid #ff4444; }}
        .critical {{ border-left-color: #ff0000; }}
        .high {{ border-left-color: #ff6600; }}
        .medium {{ border-left-color: #ffaa00; }}
        .low {{ border-left-color: #ffcc00; }}
        .info {{ border-left-color: #00aa00; }}
        .recommendations {{ background-color: #e8f4f8; padding: 15px; border-radius: 5px; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>IPPAN Security Audit Report</h1>
        <p>Generated on: {}</p>
    </div>
    
    <div class="summary">
        <h2>Audit Summary</h2>
        <p><strong>Total Vulnerabilities:</strong> {}</p>
        <p><strong>Critical:</strong> {} | <strong>High:</strong> {} | <strong>Medium:</strong> {} | <strong>Low:</strong> {} | <strong>Info:</strong> {}</p>
        <p><strong>Files Scanned:</strong> {}</p>
        <p><strong>Lines of Code:</strong> {}</p>
        <p><strong>Security Score:</strong> {:.1}%</p>
    </div>
    
    <div class="recommendations">
        <h2>Recommendations</h2>
        <ul>
            {}
        </ul>
    </div>
    
    <h2>Vulnerabilities</h2>
    {}
</body>
</html>
            "#,
            results.audit_timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            results.summary.total_vulnerabilities,
            results.summary.critical_count,
            results.summary.high_count,
            results.summary.medium_count,
            results.summary.low_count,
            results.summary.info_count,
            results.summary.files_scanned,
            results.summary.lines_of_code,
            results.summary.security_score,
            results.recommendations.iter().map(|r| format!("<li>{}</li>", r)).collect::<Vec<_>>().join(""),
            results.vulnerabilities.iter().map(|v| {
                let level_class = match v.level {
                    VulnerabilityLevel::Critical => "critical",
                    VulnerabilityLevel::High => "high",
                    VulnerabilityLevel::Medium => "medium",
                    VulnerabilityLevel::Low => "low",
                    VulnerabilityLevel::Info => "info",
                };
                format!(
                    r#"
                    <div class="vulnerability {}">
                        <h3>{} - {}</h3>
                        <p><strong>Level:</strong> {:?}</p>
                        <p><strong>Type:</strong> {:?}</p>
                        <p><strong>Description:</strong> {}</p>
                        {}
                        {}
                        <p><strong>Remediation:</strong> {}</p>
                    </div>
                    "#,
                    level_class,
                    v.id,
                    v.title,
                    v.level,
                    v.vulnerability_type,
                    v.description,
                    if let Some(ref file_path) = v.file_path {
                        format!("<p><strong>File:</strong> {}</p>", file_path)
                    } else {
                        "".to_string()
                    },
                    if let Some(line) = v.line_number {
                        format!("<p><strong>Line:</strong> {}</p>", line)
                    } else {
                        "".to_string()
                    },
                    v.remediation
                )
            }).collect::<Vec<_>>().join("")
        );
        
        Ok(html)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_security_auditor_creation() {
        let config = AuditorConfig::default();
        let auditor = SecurityAuditor::new(config);
        assert!(!auditor.rules.is_empty());
    }

    #[test]
    fn test_vulnerability_scanning() {
        let config = AuditorConfig::default();
        let auditor = SecurityAuditor::new(config);
        
        let test_code = r#"
            unsafe {
                let ptr = std::ptr::null_mut();
            }
            let password = "secret123";
            let hash = MD5::new();
        "#;
        
        let vulnerabilities = auditor.scan_file(&PathBuf::from("test.rs"), test_code);
        assert!(!vulnerabilities.is_empty());
    }

    #[test]
    fn test_summary_generation() {
        let config = AuditorConfig::default();
        let auditor = SecurityAuditor::new(config);
        
        let vulnerabilities = vec![
            Vulnerability {
                id: "TEST001".to_string(),
                title: "Test Vulnerability".to_string(),
                description: "Test description".to_string(),
                vulnerability_type: VulnerabilityType::MemorySafety,
                level: VulnerabilityLevel::High,
                file_path: Some("test.rs".to_string()),
                line_number: Some(10),
                code_snippet: Some("unsafe { }".to_string()),
                remediation: "Fix unsafe block".to_string(),
                cwe_id: Some("CWE-119".to_string()),
                cvss_score: Some(7.5),
                discovered_at: chrono::Utc::now(),
            }
        ];
        
        let summary = auditor.generate_summary(&vulnerabilities, 1, 100);
        assert_eq!(summary.total_vulnerabilities, 1);
        assert_eq!(summary.high_count, 1);
    }
} 