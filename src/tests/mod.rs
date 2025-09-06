//! Test suite for IPPAN
//! 
//! Comprehensive testing for all subsystems

pub mod address_tests;
pub mod block_parents_tests;
pub mod integration;
pub mod performance_integration;
pub mod security_integration;
pub mod end_to_end_integration;
pub mod unit;

use crate::config::Config;
use std::time::Instant;

/// Test runner for IPPAN
pub struct TestRunner {
    config: Config,
}

impl TestRunner {
    /// Create a new test runner
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Run all tests
    pub async fn run_all_tests(&self) -> Result<TestResults, Box<dyn std::error::Error>> {
        let start_time = Instant::now();
        log::info!("🚀 Starting IPPAN test suite...");

        let mut results = TestResults::new();

        // Run unit tests
        log::info!("🧪 Running unit tests...");
        let unit_start = Instant::now();
        match unit::run_unit_tests().await {
            Ok(_) => {
                results.unit_tests_passed = true;
                results.unit_test_duration = unit_start.elapsed();
                log::info!("✅ Unit tests passed in {:?}", results.unit_test_duration);
            }
            Err(e) => {
                results.unit_tests_passed = false;
                results.unit_test_error = Some(e.to_string());
                log::error!("❌ Unit tests failed: {}", e);
            }
        }

        // Run integration tests
        log::info!("🔗 Running integration tests...");
        let integration_start = Instant::now();
        match integration::run_integration_tests().await {
            Ok(_) => {
                results.integration_tests_passed = true;
                results.integration_test_duration = integration_start.elapsed();
                log::info!("✅ Integration tests passed in {:?}", results.integration_test_duration);
            }
            Err(e) => {
                results.integration_tests_passed = false;
                results.integration_test_error = Some(e.to_string());
                log::error!("❌ Integration tests failed: {}", e);
            }
        }

        // Run performance integration tests
        log::info!("🚀 Running performance integration tests...");
        let performance_start = Instant::now();
        match performance_integration::run_performance_integration_tests().await {
            Ok(_) => {
                results.performance_tests_passed = true;
                results.performance_test_duration = performance_start.elapsed();
                log::info!("✅ Performance integration tests passed in {:?}", results.performance_test_duration);
            }
            Err(e) => {
                results.performance_tests_passed = false;
                results.performance_test_error = Some(e.to_string());
                log::error!("❌ Performance integration tests failed: {}", e);
            }
        }

        // Run security integration tests
        log::info!("🔒 Running security integration tests...");
        let security_start = Instant::now();
        match security_integration::run_security_integration_tests().await {
            Ok(_) => {
                results.security_tests_passed = true;
                results.security_test_duration = security_start.elapsed();
                log::info!("✅ Security integration tests passed in {:?}", results.security_test_duration);
            }
            Err(e) => {
                results.security_tests_passed = false;
                results.security_test_error = Some(e.to_string());
                log::error!("❌ Security integration tests failed: {}", e);
            }
        }

        // Run end-to-end integration tests
        log::info!("🎯 Running end-to-end integration tests...");
        let e2e_start = Instant::now();
        match end_to_end_integration::run_end_to_end_integration_tests().await {
            Ok(_) => {
                results.e2e_tests_passed = true;
                results.e2e_test_duration = e2e_start.elapsed();
                log::info!("✅ End-to-end integration tests passed in {:?}", results.e2e_test_duration);
            }
            Err(e) => {
                results.e2e_tests_passed = false;
                results.e2e_test_error = Some(e.to_string());
                log::error!("❌ End-to-end integration tests failed: {}", e);
            }
        }

        results.total_duration = start_time.elapsed();
        
        if results.all_tests_passed() {
            log::info!("🎉 All tests passed in {:?}!", results.total_duration);
        } else {
            log::error!("💥 Some tests failed. Check results for details.");
        }

        Ok(results)
    }

    /// Run only unit tests
    pub async fn run_unit_tests(&self) -> Result<TestResults, Box<dyn std::error::Error>> {
        let start_time = Instant::now();
        log::info!("🧪 Running unit tests only...");

        let mut results = TestResults::new();
        
        match unit::run_unit_tests().await {
            Ok(_) => {
                results.unit_tests_passed = true;
                results.unit_test_duration = start_time.elapsed();
                log::info!("✅ Unit tests passed in {:?}", results.unit_test_duration);
            }
            Err(e) => {
                results.unit_tests_passed = false;
                results.unit_test_error = Some(e.to_string());
                log::error!("❌ Unit tests failed: {}", e);
            }
        }

        results.total_duration = start_time.elapsed();
        Ok(results)
    }

    /// Run only integration tests
    pub async fn run_integration_tests(&self) -> Result<TestResults, Box<dyn std::error::Error>> {
        let start_time = Instant::now();
        log::info!("🔗 Running integration tests only...");

        let mut results = TestResults::new();
        
        match integration::run_integration_tests().await {
            Ok(_) => {
                results.integration_tests_passed = true;
                results.integration_test_duration = start_time.elapsed();
                log::info!("✅ Integration tests passed in {:?}", results.integration_test_duration);
            }
            Err(e) => {
                results.integration_tests_passed = false;
                results.integration_test_error = Some(e.to_string());
                log::error!("❌ Integration tests failed: {}", e);
            }
        }

        results.total_duration = start_time.elapsed();
        Ok(results)
    }
}

/// Test results summary
#[derive(Debug)]
pub struct TestResults {
    pub unit_tests_passed: bool,
    pub unit_test_duration: std::time::Duration,
    pub unit_test_error: Option<String>,
    
    pub integration_tests_passed: bool,
    pub integration_test_duration: std::time::Duration,
    pub integration_test_error: Option<String>,
    
    pub performance_tests_passed: bool,
    pub performance_test_duration: std::time::Duration,
    pub performance_test_error: Option<String>,
    
    pub security_tests_passed: bool,
    pub security_test_duration: std::time::Duration,
    pub security_test_error: Option<String>,
    
    pub e2e_tests_passed: bool,
    pub e2e_test_duration: std::time::Duration,
    pub e2e_test_error: Option<String>,
    
    pub total_duration: std::time::Duration,
}

impl TestResults {
    /// Create new test results
    pub fn new() -> Self {
        Self {
            unit_tests_passed: false,
            unit_test_duration: std::time::Duration::ZERO,
            unit_test_error: None,
            integration_tests_passed: false,
            integration_test_duration: std::time::Duration::ZERO,
            integration_test_error: None,
            performance_tests_passed: false,
            performance_test_duration: std::time::Duration::ZERO,
            performance_test_error: None,
            security_tests_passed: false,
            security_test_duration: std::time::Duration::ZERO,
            security_test_error: None,
            e2e_tests_passed: false,
            e2e_test_duration: std::time::Duration::ZERO,
            e2e_test_error: None,
            total_duration: std::time::Duration::ZERO,
        }
    }

    /// Check if all tests passed
    pub fn all_tests_passed(&self) -> bool {
        self.unit_tests_passed && 
        self.integration_tests_passed && 
        self.performance_tests_passed && 
        self.security_tests_passed && 
        self.e2e_tests_passed
    }

    /// Get summary string
    pub fn summary(&self) -> String {
        let mut summary = String::new();
        
        summary.push_str("📊 Test Results Summary:\n");
        
        // Unit Tests
        summary.push_str(&format!("  Unit Tests: {}\n", 
            if self.unit_tests_passed { "✅ PASSED" } else { "❌ FAILED" }));
        if let Some(ref error) = self.unit_test_error {
            summary.push_str(&format!("    Error: {}\n", error));
        }
        summary.push_str(&format!("    Duration: {:?}\n", self.unit_test_duration));
        
        // Integration Tests
        summary.push_str(&format!("  Integration Tests: {}\n", 
            if self.integration_tests_passed { "✅ PASSED" } else { "❌ FAILED" }));
        if let Some(ref error) = self.integration_test_error {
            summary.push_str(&format!("    Error: {}\n", error));
        }
        summary.push_str(&format!("    Duration: {:?}\n", self.integration_test_duration));
        
        // Performance Tests
        summary.push_str(&format!("  Performance Tests: {}\n", 
            if self.performance_tests_passed { "✅ PASSED" } else { "❌ FAILED" }));
        if let Some(ref error) = self.performance_test_error {
            summary.push_str(&format!("    Error: {}\n", error));
        }
        summary.push_str(&format!("    Duration: {:?}\n", self.performance_test_duration));
        
        // Security Tests
        summary.push_str(&format!("  Security Tests: {}\n", 
            if self.security_tests_passed { "✅ PASSED" } else { "❌ FAILED" }));
        if let Some(ref error) = self.security_test_error {
            summary.push_str(&format!("    Error: {}\n", error));
        }
        summary.push_str(&format!("    Duration: {:?}\n", self.security_test_duration));
        
        // End-to-End Tests
        summary.push_str(&format!("  End-to-End Tests: {}\n", 
            if self.e2e_tests_passed { "✅ PASSED" } else { "❌ FAILED" }));
        if let Some(ref error) = self.e2e_test_error {
            summary.push_str(&format!("    Error: {}\n", error));
        }
        summary.push_str(&format!("    Duration: {:?}\n", self.e2e_test_duration));
        
        summary.push_str(&format!("  Total Duration: {:?}\n", self.total_duration));
        
        if self.all_tests_passed() {
            summary.push_str("🎉 All tests passed!\n");
        } else {
            summary.push_str("💥 Some tests failed.\n");
        }
        
        summary
    }
}

/// Run all tests with default configuration
pub async fn run_tests() -> Result<TestResults, Box<dyn std::error::Error>> {
    let config = Config::default();
    let runner = TestRunner::new(config);
    runner.run_all_tests().await
}

/// Run only unit tests
pub async fn run_unit_tests_only() -> Result<TestResults, Box<dyn std::error::Error>> {
    let config = Config::default();
    let runner = TestRunner::new(config);
    runner.run_unit_tests().await
}

/// Run only integration tests
pub async fn run_integration_tests_only() -> Result<TestResults, Box<dyn std::error::Error>> {
    let config = Config::default();
    let runner = TestRunner::new(config);
    runner.run_integration_tests().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_test_runner_creation() {
        let config = Config::default();
        let runner = TestRunner::new(config);
        assert!(runner.config.consensus.max_validators > 0);
    }

    #[tokio::test]
    async fn test_test_results_creation() {
        let results = TestResults::new();
        assert!(!results.all_tests_passed());
        assert_eq!(results.unit_test_duration, std::time::Duration::ZERO);
        assert_eq!(results.integration_test_duration, std::time::Duration::ZERO);
    }

    #[tokio::test]
    async fn test_test_results_summary() {
        let mut results = TestResults::new();
        results.unit_tests_passed = true;
        results.integration_tests_passed = true;
        
        let summary = results.summary();
        assert!(summary.contains("PASSED"));
        assert!(summary.contains("All tests passed"));
    }
}
