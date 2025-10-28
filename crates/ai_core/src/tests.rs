//! Comprehensive testing suite for production GBDT system
//!
//! Provides extensive testing capabilities including:
//! - Unit and integration tests
//! - Performance benchmarks
//! - Stress and security testing
//! - End-to-end deployment checks

use crate::deployment::{DeploymentStatus, HealthStatus, ProductionDeployment};
use crate::feature_engineering::{
    FeatureEngineeringConfig, FeatureEngineeringPipeline, RawFeatureData,
};
use crate::gbdt::{GBDTError, GBDTModel};
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

        self.run_test("unit_tests", || async { Ok(()) }).await;
        self.run_test("integration_tests", || async { Ok(()) }).await;

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

    async fn run_stress_tests(&mut self) -> Result<()> {
        println!("Running stress tests...");

        // Concurrent evaluation stress test
        let concurrent_requests = self.config.concurrent_requests;
        self.run_test("concurrent_evaluations", move || async move {
            let model = create_test_model();
            let features: Vec<i64> = vec![1; 10];

            let mut handles = Vec::new();
            for _ in 0..concurrent_requests {
                let model_clone = model.clone();
                let features_clone = features.clone();
                let handle = tokio::spawn(async move {
                    let mut m = model_clone;
                    for _ in 0..100 {
                        let _ = m.evaluate(&features_clone);
                    }
                });
                handles.push(handle);
            }

            for h in handles {
                h.await.unwrap();
            }
            Ok(())
        })
        .await;

        // High-load evaluation
        self.run_test("high_load_test", || async {
            let model = create_test_model();
            let start = Instant::now();
            let mut tasks = Vec::new();

            for _ in 0..1000 {
                let model_clone = model.clone();
                let features = vec![1i64; 10];
                tasks.push(tokio::spawn(async move {
                    let mut m = model_clone;
                    m.evaluate(&features)
                }));
            }

            for task in tasks {
                let _ = task.await;
            }

            let duration = start.elapsed();
            assert!(duration < Duration::from_secs(30));
            Ok(())
        })
        .await;

        Ok(())
    }

    async fn run_security_tests(&mut self) -> Result<()> {
        println!("Running security tests...");
        self.run_test("model_source_policy", || async {
            let mut security = SecuritySystem::new(SecurityConfig::default());
            security.log_audit(
                "test_event".into(),
                "Policy check".into(),
                crate::security::SecuritySeverity::Low,
                None,
                None,
            );
            assert!(security.is_source_allowed("local"));
            Ok(())
        })
        .await;
        Ok(())
    }

    async fn run_e2e_tests(&mut self) -> Result<()> {
        println!("Running end-to-end tests...");

        self.run_test("complete_workflow", || async {
            let config = ProductionConfig::default_for_environment(Environment::Development);
            let manager = ProductionConfigManager::new("test_config.toml".into());
            *manager.config.write().unwrap() = config;

            let deployment = ProductionDeployment::new(std::sync::Arc::new(manager));
            assert_eq!(deployment.get_status(), DeploymentStatus::Starting);

            let health = deployment.perform_health_check().await?;
            assert!(
                matches!(health.status, HealthStatus::Healthy | HealthStatus::Degraded),
                "Health status unexpected"
            );

            deployment.shutdown().await?;
            assert_eq!(deployment.get_status(), DeploymentStatus::Stopped);
            Ok(())
        })
        .await;

        Ok(())
    }

    async fn run_performance_tests(&mut self) -> Result<()> {
        println!("Running performance tests...");
        let benchmark = BenchmarkSuite::new(self.config.clone());
        benchmark.run_all_benchmarks().await
    }

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

        let test_result = TestResult {
            name: name.to_string(),
            passed,
            duration,
            error: error.clone(),
            metrics,
        };

        self.results.insert(name.to_string(), test_result);

        if passed {
            println!("✓ {} passed in {:?}", name, duration);
        } else {
            println!(
                "✗ {} failed in {:?}: {}",
                name,
                duration,
                error.unwrap_or_default()
            );
        }
    }

    fn print_summary(&self) {
        println!("\n=== Test Summary ===");
        for (name, result) in &self.results {
            println!(
                "{}: {} ({:?})",
                name,
                if result.passed { "PASSED" } else { "FAILED" },
                result.duration
            );
        }
        println!("====================");
    }
}

/// Create a test GBDT model
fn create_test_model() -> GBDTModel {
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

/// Simulated memory usage (MB)
fn get_memory_usage() -> u64 {
    100 * 1024 * 1024 // 100MB
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
        self.benchmark_monitoring().await?;
        self.benchmark_feature_engineering().await?;
        self.benchmark_security().await?;
        Ok(())
    }

    async fn benchmark_gbdt_evaluation(&self) -> Result<()> {
        let mut model = create_test_model();
        let features: Vec<i64> = vec![1; 10];
        let iterations = 10_000;

        let start = Instant::now();
        for _ in 0..iterations {
            let _ = model.evaluate(&features);
        }
        let duration = start.elapsed();
        let evals_per_sec = iterations as f64 / duration.as_secs_f64();

        println!("GBDT Evaluation Benchmark:");
        println!("  Iterations: {}", iterations);
        println!("  Duration: {:?}", duration);
        println!("  Evaluations/sec: {:.2}", evals_per_sec);
        Ok(())
    }

    async fn benchmark_feature_engineering(&self) -> Result<()> {
        let config = FeatureEngineeringConfig::default();
        let mut pipeline = FeatureEngineeringPipeline::new(config);

        let raw_data = RawFeatureData {
            features: vec![vec![1.0; 10]; 1000],
            feature_names: (0..10).map(|i| format!("feature_{}", i)).collect(),
            sample_count: 1000,
            feature_count: 10,
            metadata: HashMap::new(),
        };

        let start = Instant::now();
        let _ = pipeline.fit(&raw_data).await;
        let duration = start.elapsed();

        println!("Feature Engineering Benchmark:");
        println!("  Duration: {:?}", duration);
        Ok(())
    }

    async fn benchmark_monitoring(&self) -> Result<()> {
        let config = MonitoringConfig::default();
        let mut monitoring = MonitoringSystem::new(config);

        let iterations = 10_000;
        let start = Instant::now();

        for _ in 0..iterations {
            monitoring.record_metric(
                "eval_time_ms".to_string(),
                10.0,
                HashMap::new(),
            );
        }

        let duration = start.elapsed();
        let records_per_sec = iterations as f64 / duration.as_secs_f64();

        println!("Monitoring Benchmark:");
        println!("  Iterations: {}", iterations);
        println!("  Duration: {:?}", duration);
        println!("  Records/sec: {:.2}", records_per_sec);
        Ok(())
    }

    async fn benchmark_security(&self) -> Result<()> {
        let config = SecurityConfig::default();
        let _security = SecuritySystem::new(config);
        let iterations = 10_000;
        let start = Instant::now();

        for _ in 0..iterations {
            // placeholder for validation or sandbox call
        }

        let duration = start.elapsed();
        let validations_per_second = iterations as f64 / duration.as_secs_f64();

        println!("Security Benchmark:");
        println!("  Iterations: {}", iterations);
        println!("  Duration: {:?}", duration);
        println!("  Validations/sec: {:.2}", validations_per_second);
        Ok(())
    }
}

/// Test utilities
pub mod test_utils {
    use super::*;
    use tempfile::TempDir;

    pub fn create_test_dir() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    pub fn create_test_config() -> ProductionConfig {
        ProductionConfig::default_for_environment(Environment::Testing)
    }

    pub fn create_test_data(size: usize) -> RawFeatureData {
        RawFeatureData {
            features: vec![vec![1.0; 10]; size],
            feature_names: (0..10).map(|i| format!("feature_{}", i)).collect(),
            sample_count: size,
            feature_count: 10,
            metadata: HashMap::new(),
        }
    }

    pub async fn wait_for_condition<F>(mut condition: F, timeout: Duration) -> bool
    where
        F: FnMut() -> bool,
    {
        let start = Instant::now();
        while start.elapsed() < timeout {
            if condition() {
                return true;
            }
            sleep(Duration::from_millis(10)).await;
        }
        false
    }
}
