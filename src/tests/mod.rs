//! Test suite for IPPAN
//! 
//! Comprehensive testing for all subsystems

pub mod integration;
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
            total_duration: std::time::Duration::ZERO,
        }
    }

    /// Check if all tests passed
    pub fn all_tests_passed(&self) -> bool {
        self.unit_tests_passed && self.integration_tests_passed
    }

    /// Get summary string
    pub fn summary(&self) -> String {
        let mut summary = String::new();
        
        summary.push_str("📊 Test Results Summary:\n");
        summary.push_str(&format!("  Unit Tests: {}\n", 
            if self.unit_tests_passed { "✅ PASSED" } else { "❌ FAILED" }));
        
        if let Some(ref error) = self.unit_test_error {
            summary.push_str(&format!("    Error: {}\n", error));
        }
        
        summary.push_str(&format!("    Duration: {:?}\n", self.unit_test_duration));
        
        summary.push_str(&format!("  Integration Tests: {}\n", 
            if self.integration_tests_passed { "✅ PASSED" } else { "❌ FAILED" }));
        
        if let Some(ref error) = self.integration_test_error {
            summary.push_str(&format!("    Error: {}\n", error));
        }
        
        summary.push_str(&format!("    Duration: {:?}\n", self.integration_test_duration));
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
