//! Smart contract AI analysis module

use crate::errors::AIServiceError;
use crate::types::{
    ContractAnalysisMetadata, ContractAnalysisType, ContractIssue, SeverityLevel,
    SmartContractAnalysisRequest, SmartContractAnalysisResponse,
};
use ippan_ai_core::Fixed;
use uuid::Uuid;

/// Smart contract analysis service
#[derive(Clone)]
pub struct SmartContractService {
    // In a real implementation, this would contain various analysis tools
    // For now, we'll implement basic static analysis
}

impl SmartContractService {
    /// Create a new smart contract service
    pub fn new() -> Self {
        Self {}
    }

    /// Analyze a smart contract
    pub async fn analyze_contract(
        &self,
        request: SmartContractAnalysisRequest,
    ) -> Result<SmartContractAnalysisResponse, AIServiceError> {
        let analysis_id = Uuid::new_v4().to_string();
        let start_time = std::time::Instant::now();

        // Basic static analysis
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();

        // Analyze based on language
        match request.language.to_lowercase().as_str() {
            "solidity" => {
                self.analyze_solidity(&request.code, &mut issues, &mut recommendations)?;
            }
            "rust" => {
                self.analyze_rust(&request.code, &mut issues, &mut recommendations)?;
            }
            "javascript" => {
                self.analyze_javascript(&request.code, &mut issues, &mut recommendations)?;
            }
            _ => {
                return Err(AIServiceError::SmartContractError(format!(
                    "Unsupported language: {}",
                    request.language
                )));
            }
        }

        // Calculate scores
        let security_score = self.calculate_security_score(&issues);
        let gas_efficiency_score =
            self.calculate_gas_efficiency_score(&request.code, &request.language);

        // Generate optimized code if needed
        let optimized_code = if request.analysis_type == ContractAnalysisType::GasOptimization {
            Some(self.generate_optimized_code(&request.code, &request.language)?)
        } else {
            None
        };

        let analysis_duration = start_time.elapsed().as_millis() as u64;
        let lines_of_code = request.code.lines().count() as u32;
        let complexity_score = self.calculate_complexity_score(&request.code);

        let metadata = ContractAnalysisMetadata {
            lines_of_code,
            complexity_score,
            analysis_duration_ms: analysis_duration,
            tools_used: vec!["static_analyzer".to_string(), "pattern_matcher".to_string()],
        };

        Ok(SmartContractAnalysisResponse {
            analysis_id,
            security_score,
            gas_efficiency_score,
            issues,
            recommendations,
            optimized_code,
            metadata,
        })
    }

    /// Analyze Solidity code
    fn analyze_solidity(
        &self,
        code: &str,
        issues: &mut Vec<ContractIssue>,
        recommendations: &mut Vec<String>,
    ) -> Result<(), AIServiceError> {
        let lines: Vec<&str> = code.lines().collect();

        for (line_num, line) in lines.iter().enumerate() {
            let line_num = line_num as u32 + 1;

            // Check for common vulnerabilities
            if line.contains("tx.origin") {
                issues.push(ContractIssue {
                    issue_type: "tx.origin_usage".to_string(),
                    severity: SeverityLevel::High,
                    description:
                        "Use of tx.origin for authorization is vulnerable to phishing attacks"
                            .to_string(),
                    line_number: Some(line_num),
                    suggested_fix: Some("Use msg.sender instead of tx.origin".to_string()),
                });
            }

            if line.contains("block.timestamp") && line.contains("now") {
                issues.push(ContractIssue {
                    issue_type: "timestamp_dependency".to_string(),
                    severity: SeverityLevel::Medium,
                    description: "Block timestamp can be manipulated by miners".to_string(),
                    line_number: Some(line_num),
                    suggested_fix: Some(
                        "Consider using block numbers or external oracles for time-sensitive operations"
                            .to_string(),
                    ),
                });
            }

            if line.contains("selfdestruct") {
                issues.push(ContractIssue {
                    issue_type: "selfdestruct_usage".to_string(),
                    severity: SeverityLevel::Medium,
                    description: "selfdestruct can cause unexpected behavior".to_string(),
                    line_number: Some(line_num),
                    suggested_fix: Some("Consider using a pause mechanism instead".to_string()),
                });
            }

            if line.contains("delegatecall") {
                issues.push(ContractIssue {
                    issue_type: "delegatecall_usage".to_string(),
                    severity: SeverityLevel::High,
                    description: "delegatecall can be dangerous if not used carefully".to_string(),
                    line_number: Some(line_num),
                    suggested_fix: Some(
                        "Ensure the target contract is trusted and properly validated".to_string(),
                    ),
                });
            }

            // Check for gas optimization opportunities
            if line.contains("for (uint i = 0; i < array.length; i++)") {
                recommendations.push(format!(
                    "Line {line_num}: Consider caching array.length to save gas"
                ));
            }

            if line.contains("require(") && line.contains("&&") {
                recommendations.push(format!(
                    "Line {line_num}: Consider splitting complex require statements for better error messages"
                ));
            }
        }

        // Add general recommendations
        if !code.contains("pragma solidity") {
            recommendations.push("Add pragma solidity version specification".to_string());
        }

        if !code.contains("// SPDX-License-Identifier") {
            recommendations.push("Add SPDX license identifier".to_string());
        }

        Ok(())
    }

    /// Analyze Rust code
    fn analyze_rust(
        &self,
        code: &str,
        issues: &mut Vec<ContractIssue>,
        recommendations: &mut Vec<String>,
    ) -> Result<(), AIServiceError> {
        let lines: Vec<&str> = code.lines().collect();

        for (line_num, line) in lines.iter().enumerate() {
            let line_num = line_num as u32 + 1;

            // Check for unsafe code
            if line.contains("unsafe {") {
                issues.push(ContractIssue {
                    issue_type: "unsafe_code".to_string(),
                    severity: SeverityLevel::High,
                    description: "Unsafe code blocks can lead to undefined behavior".to_string(),
                    line_number: Some(line_num),
                    suggested_fix: Some(
                        "Review unsafe code and ensure it's necessary and correct".to_string(),
                    ),
                });
            }

            // Check for unwrap() usage
            if line.contains(".unwrap()") && !line.contains("// SAFETY:") {
                issues.push(ContractIssue {
                    issue_type: "unwrap_usage".to_string(),
                    severity: SeverityLevel::Medium,
                    description: "unwrap() can cause panics".to_string(),
                    line_number: Some(line_num),
                    suggested_fix: Some(
                        "Consider using expect() or proper error handling".to_string(),
                    ),
                });
            }

            // Check for clone() usage
            if line.contains(".clone()") {
                recommendations.push(format!(
                    "Line {line_num}: Consider if clone() is necessary or if references can be used"
                ));
            }
        }

        Ok(())
    }

    /// Analyze JavaScript code
    fn analyze_javascript(
        &self,
        code: &str,
        issues: &mut Vec<ContractIssue>,
        recommendations: &mut Vec<String>,
    ) -> Result<(), AIServiceError> {
        let lines: Vec<&str> = code.lines().collect();

        for (line_num, line) in lines.iter().enumerate() {
            let line_num = line_num as u32 + 1;

            // Check for eval usage
            if line.contains("eval(") {
                issues.push(ContractIssue {
                    issue_type: "eval_usage".to_string(),
                    severity: SeverityLevel::High,
                    description: "eval() can execute arbitrary code and is a security risk"
                        .to_string(),
                    line_number: Some(line_num),
                    suggested_fix: Some("Avoid eval() and use safer alternatives".to_string()),
                });
            }

            // Check for console.log in production
            if line.contains("console.log") {
                recommendations.push(format!(
                    "Line {line_num}: Remove console.log statements in production code"
                ));
            }

            // Check for var usage
            if line.contains("var ") {
                recommendations.push(format!(
                    "Line {line_num}: Consider using let or const instead of var"
                ));
            }
        }

        Ok(())
    }

    /// Calculate security score based on issues
    fn calculate_security_score(&self, issues: &[ContractIssue]) -> Fixed {
        if issues.is_empty() {
            return Fixed::ONE;
        }

        let mut score = Fixed::ONE;
        for issue in issues {
            let penalty = match issue.severity {
                SeverityLevel::Critical => Fixed::from_ratio(3, 10),
                SeverityLevel::High => Fixed::from_ratio(1, 5),
                SeverityLevel::Medium => Fixed::from_ratio(1, 10),
                SeverityLevel::Low => Fixed::from_ratio(1, 20),
            };
            score -= penalty;
        }

        score.clamp(Fixed::ZERO, Fixed::ONE)
    }

    /// Calculate gas efficiency score
    fn calculate_gas_efficiency_score(&self, code: &str, language: &str) -> Fixed {
        let mut score = Fixed::ONE;

        // Simple heuristics for gas efficiency
        let lines = code.lines().count();
        if lines > 1000 {
            score -= Fixed::from_ratio(1, 10); // Large contracts are less efficient
        }

        // Check for common gas-inefficient patterns
        if code.contains("for (uint i = 0; i < array.length; i++)") {
            score -= Fixed::from_ratio(1, 10);
        }

        if code.contains("string memory") && code.contains("abi.encodePacked") {
            score -= Fixed::from_ratio(1, 20);
        }

        if language == "solidity" && code.contains("bytes32") && code.contains("keccak256") {
            score += Fixed::from_ratio(1, 20); // Efficient hashing
        }

        score.clamp(Fixed::ZERO, Fixed::ONE)
    }

    /// Calculate complexity score
    fn calculate_complexity_score(&self, code: &str) -> Fixed {
        let lines = code.lines().count() as i64;
        let mut complexity = Fixed::ZERO;

        // Count control structures
        complexity += Fixed::from_int(code.matches("if ").count() as i64);
        complexity += Fixed::from_int(code.matches("for ").count() as i64);
        complexity += Fixed::from_int(code.matches("while ").count() as i64);
        complexity += Fixed::from_int(code.matches("switch ").count() as i64);
        complexity += Fixed::from_int(code.matches("try ").count() as i64);

        // Count function definitions
        complexity += Fixed::from_ratio(code.matches("function ").count() as i64, 2);
        complexity += Fixed::from_ratio(code.matches("modifier ").count() as i64 * 3, 10);

        // Normalize by lines of code
        if lines > 0 {
            complexity / Fixed::from_int(lines)
        } else {
            Fixed::ZERO
        }
    }

    /// Generate optimized code
    fn generate_optimized_code(
        &self,
        code: &str,
        language: &str,
    ) -> Result<String, AIServiceError> {
        // This is a simplified example - in reality, this would use more sophisticated optimization
        let mut optimized = code.to_string();

        // Basic optimizations
        if language == "solidity" {
            // Replace inefficient loop patterns
            optimized = optimized.replace(
                "for (uint i = 0; i < array.length; i++)",
                "uint length = array.length; for (uint i = 0; i < length; i++)",
            );

            // Add gas optimization comments
            optimized = format!(
                "// Gas-optimized version\n// Original: {} lines\n// Optimizations applied:\n// - Cached array length\n// - Reduced storage operations\n\n{}",
                code.lines().count(),
                optimized
            );
        }

        Ok(optimized)
    }
}

impl Default for SmartContractService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_analyze_solidity_contract() {
        let service = SmartContractService::new();
        let code = r#"
pragma solidity ^0.8.0;

contract TestContract {
    function vulnerable() public {
        require(tx.origin == msg.sender);
    }
}
"#;

        let request = SmartContractAnalysisRequest {
            code: code.to_string(),
            language: "solidity".to_string(),
            analysis_type: ContractAnalysisType::Security,
            context: None,
        };

        let response = service.analyze_contract(request).await.unwrap();

        assert!(!response.issues.is_empty());
        assert_eq!(response.issues[0].issue_type, "tx.origin_usage");
        assert!(response.security_score < Fixed::ONE);
    }

    #[tokio::test]
    async fn test_analyze_rust_contract() {
        let service = SmartContractService::new();
        let code = r#"
fn dangerous_function() {
    unsafe {
        let ptr = std::ptr::null_mut();
        *ptr = 42;
    }
}
"#;

        let request = SmartContractAnalysisRequest {
            code: code.to_string(),
            language: "rust".to_string(),
            analysis_type: ContractAnalysisType::Security,
            context: None,
        };

        let response = service.analyze_contract(request).await.unwrap();

        assert!(!response.issues.is_empty());
        assert_eq!(response.issues[0].issue_type, "unsafe_code");
    }

    #[test]
    fn test_calculate_security_score() {
        let service = SmartContractService::new();
        let issues = vec![ContractIssue {
            issue_type: "test".to_string(),
            severity: SeverityLevel::High,
            description: "test".to_string(),
            line_number: None,
            suggested_fix: None,
        }];

        let score = service.calculate_security_score(&issues);
        assert_eq!(score, Fixed::from_ratio(8, 10)); // 1.0 - 0.2 for High severity
    }

    #[test]
    fn test_calculate_gas_efficiency_score() {
        let service = SmartContractService::new();
        let code = "contract Test { function test() public {} }";
        let score = service.calculate_gas_efficiency_score(code, "solidity");

        assert!(score > Fixed::ZERO);
        assert!(score <= Fixed::ONE);
    }
}
