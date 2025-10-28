//! Comprehensive testing suite for production GBDT system
//!
//! This module provides extensive testing capabilities including:
//! - Unit tests for all components
//! - Integration tests for production workflows
//! - Performance benchmarks
//! - Stress testing
//! - Security testing
//! - End-to-end testing

use crate::deployment::{DeploymentStatus, HealthStatus, ProductionDeployment};
use crate::feature_engineering::{
    FeatureEngineeringConfig, FeatureEngineeringPipeline, RawFeatureData,
};
use crate::gbdt::{FeatureNormalization, GBDTError, GBDTModel, GBDTResult, SecurityConstraints};
use crate::model_manager::{ModelManager, ModelManagerConfig};
use crate::monitoring::{MonitoringConfig, MonitoringSystem};
use crate::production_config::{Environment, ProductionConfig, ProductionConfigManager};
use crate::security::{SecurityConfig, SecuritySystem};
use anyhow::Result;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::time::sleep;

/// Test configuration
#[derive(Debug, Clone)]
pub struct TestConfig {
    pub test_timeout: Duration,
    pub max_memory_mb: u64,
    pub max_cpu_percent: f64,
    pub enable_stress_tests: bool,
    pub enable_performance_tests: bool,
    pub enable_security_tests: bool,
    pub test_data_size: usize,
    pub concurrent_requests: usize,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            test_timeout: Duration::from_secs(30),
            max_memory_mb: 512,
            max_cpu_percent: 50.0,
            enable_stress_tests: true,
            enable_performance_tests: true,
            enable_security_tests: true,
            test_data_size: 1000,
            concurrent_requests: 10,
        }
    }
}

/// Test suite runner
pub struct TestSuite {
    config: TestConfig,
    results: HashMap<String, TestResult>,
}

/// Test result
#[derive(Debug, Clone)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub duration: Duration,
    pub error: Option<String>,
    pub metrics: HashMap<String, f64>,
}

impl TestSuite {
    pub fn new(config: TestConfig) -> Self {
        Self {
            config,
            results: HashMap::new(),
        }
    }

    pub async fn run_all_tests(&mut self) -> Result<()> {
        println!("Starting comprehensive test suite...");

        self.run_unit_tests().await?;
        self.run_integration_tests().await?;

        if self.config.enable_performance_tests {
            self.run_performance_tests().await?;
        }
        if self.config.enable_stress_tests {
            self.run_stress_tests().await?;
        }
        if self.config.enable_security_tests {
            self.run_security_tests().await?;
        }

        self.run_e2e_tests().await?;
        self.print_summary();
        Ok(())
    }

    // --- all test groups remain unchanged until run_test() ---

    async fn run_test<F, Fut>(&mut self, name: &str, test_fn: F)
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        let start = Instant::now();
        let mut metrics = HashMap::new();

        let result = tokio::time::timeout(self.config.test_timeout, test_fn()).await;

        let duration = start.elapsed();
        let (passed, error) = match result {
            Ok(Ok(())) => (true, None),
            Ok(Err(e)) => (false, Some(e.to_string())),
            Err(_) => (false, Some("Test timeout".to_string())),
        };

        metrics.insert(
            "memory_mb".to_string(),
            get_memory_usage() as f64 / (1024.0 * 1024.0),
        );

        let error_display = error.as_ref().map(|e| e.as_str()).unwrap_or("Unknown error");

        let test_result = TestResult {
            name: name.to_string(),
            passed,
            duration,
            error,
            metrics,
        };

        self.results.insert(name.to_string(), test_result);

        if passed {
            println!("✓ {} passed in {:?}", name, duration);
        } else {
            println!("✗ {} failed in {:?}: {}", name, duration, error_display);
        }
    }

    // --- other methods unchanged ---
}

/// Create a test GBDT model
fn create_test_model() -> GBDTModel {
    create_test_model_inner()
}

fn create_test_model_inner() -> GBDTModel {
    use crate::gbdt::{Node, Tree};
    GBDTModel::new(
        vec![Tree {
            nodes: vec![Node {
                feature_index: 0,
                threshold: 0,
                left: 0,
                right: 0,
                value: Some(100),
            }],
        }],
        0,
        10000,
        1,
    )
    .unwrap()
}

/// Get current memory usage
fn get_memory_usage() -> u64 {
    100 * 1024 * 1024 // Simulated 100MB
}

/// Benchmark suite
pub struct BenchmarkSuite {
    config: TestConfig,
}

impl BenchmarkSuite {
    pub fn new(config: TestConfig) -> Self {
        Self { config }
    }

    pub async fn run_all_benchmarks(&self) -> Result<()> {
        println!("Running benchmarks...");

        self.benchmark_gbdt_evaluation().await?;
        self.benchmark_model_loading().await?;
        self.benchmark_feature_engineering().await?;
        self.benchmark_monitoring().await?;
        self.benchmark_security().await?;
        Ok(())
    }

    async fn benchmark_gbdt_evaluation(&self) -> Result<()> {
        let model = create_test_model();
        let features = vec![1i64; 10]; // ✅ correct type for GBDT

        let iterations = 10000;
        let start = Instant::now();

        for _ in 0..iterations {
            let _ = model.evaluate(&features)?;
        }

        let duration = start.elapsed();
        let evals_per_sec = iterations as f64 / duration.as_secs_f64();

        println!("GBDT Evaluation Benchmark:");
        println!("  Iterations: {}", iterations);
        println!("  Duration: {:?}", duration);
        println!("  Evaluations/sec: {:.2}", evals_per_sec);

        Ok(())
    }

    // Remaining benchmark methods unchanged
}
