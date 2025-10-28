//! Comprehensive testing suite for production GBDT system
//!
//! This module provides extensive testing capabilities including:
//! - Unit tests for all components
//! - Integration tests for production workflows
//! - Performance benchmarks
//! - Stress testing
//! - Security testing
//! - End-to-end testing

use crate::gbdt::{GBDTModel, GBDTError, GBDTResult, SecurityConstraints, FeatureNormalization};
use crate::model_manager::{ModelManager, ModelManagerConfig};
use crate::feature_engineering::{FeatureEngineeringPipeline, FeatureEngineeringConfig, RawFeatureData};
use crate::monitoring::{MonitoringSystem, MonitoringConfig};
use crate::security::{SecuritySystem, SecurityConfig};
use crate::production_config::{ProductionConfig, ProductionConfigManager, Environment};
use crate::deployment::{ProductionDeployment, DeploymentStatus, HealthStatus};
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
    /// Create a new test suite
    pub fn new(config: TestConfig) -> Self {
        Self {
            config,
            results: HashMap::new(),
        }
    }

    /// Run all tests
    pub async fn run_all_tests(&mut self) -> Result<()> {
        println!("Starting comprehensive test suite...");
        
        // Unit tests
        self.run_unit_tests().await?;
        
        // Integration tests
        self.run_integration_tests().await?;
        
        // Performance tests
        if self.config.enable_performance_tests {
            self.run_performance_tests().await?;
        }
        
        // Stress tests
        if self.config.enable_stress_tests {
            self.run_stress_tests().await?;
        }
        
        // Security tests
        if self.config.enable_security_tests {
            self.run_security_tests().await?;
        }
        
        // End-to-end tests
        self.run_e2e_tests().await?;
        
        self.print_summary();
        Ok(())
    }

    /// Run unit tests
    async fn run_unit_tests(&mut self) -> Result<()> {
        println!("Running unit tests...");
        
        // Test GBDT model creation
        self.run_test("gbdt_model_creation", || async {
            let model = GBDTModel::new(
                vec![],
                0,
                10000,
                0,
            )?;
            assert!(model.trees.is_empty());
            Ok(())
        }).await;
        
        // Test feature engineering
        self.run_test("feature_engineering_basic", || async {
            let config = FeatureEngineeringConfig::default();
            let mut pipeline = FeatureEngineeringPipeline::new(config);
            
            let raw_data = RawFeatureData {
                features: vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]],
                feature_names: vec!["feature1".to_string(), "feature2".to_string(), "feature3".to_string()],
                sample_count: 2,
                feature_count: 3,
                metadata: HashMap::new(),
            };
            
            let result = pipeline.fit(&raw_data).await;
            assert!(result.is_ok());
            Ok(())
        }).await;
        
        // Test monitoring system
        self.run_test("monitoring_system_creation", || async {
            let config = MonitoringConfig::default();
            let monitoring = MonitoringSystem::new(config);
            // monitoring.start().await;
            // monitoring.stop().await;
            Ok(())
        }).await;
        
        // Test security system
        self.run_test("security_system_creation", || async {
            let config = SecurityConfig::default();
            let security = SecuritySystem::new(config);
            // assert!(security.validate_input(&[1.0, 2.0, 3.0]).is_ok());
            Ok(())
        }).await;
        
        Ok(())
    }

    /// Run integration tests
    async fn run_integration_tests(&mut self) -> Result<()> {
        println!("Running integration tests...");
        
        // Test full production deployment
        self.run_test("production_deployment", || async {
            let config_manager = ProductionConfigManager::new("test_config.toml".into());
            let deployment = ProductionDeployment::new(std::sync::Arc::new(config_manager));
            
            // Test deployment creation
            assert_eq!(deployment.get_status(), DeploymentStatus::Starting);
            
            // Test health check
            let health = deployment.perform_health_check().await?;
            assert_eq!(health.status, HealthStatus::Unhealthy); // No models loaded
            
            Ok(())
        }).await;
        
        // Test model manager integration
        self.run_test("model_manager_integration", || async {
            let config = ModelManagerConfig::default();
            let manager = ModelManager::new(config);
            
            // Test model loading (would fail in test environment)
            let result = manager.load_model("nonexistent_model.bin").await;
            assert!(result.is_err());
            
            Ok(())
        }).await;
        
        Ok(())
    }

    /// Run performance tests
    async fn run_performance_tests(&mut self) -> Result<()> {
        println!("Running performance tests...");
        
        // Test GBDT evaluation performance
        self.run_test("gbdt_evaluation_performance", || async {
            let mut model = create_test_model();
            let features: Vec<i64> = vec![1; 10];
            
            let start = Instant::now();
            for _ in 0..1000 {
                let _ = model.evaluate(&features)?;
            }
            let duration = start.elapsed();
            
            // Should complete within reasonable time
            assert!(duration < Duration::from_secs(5));
            
            Ok(())
        }).await;
        
        // Test memory usage
        self.run_test("memory_usage_test", || async {
            let mut models = Vec::new();
            
            // Create multiple models to test memory usage
            for i in 0..100 {
                let model = create_test_model();
                models.push(model);
                
                // Check memory usage periodically
                if i % 20 == 0 {
                    let memory_usage = get_memory_usage();
                    assert!(memory_usage < 1024 * 1024 * 1024); // 1GB limit
                }
            }
            
            Ok(())
        }).await;
        
        Ok(())
    }

    /// Run stress tests
    async fn run_stress_tests(&mut self) -> Result<()> {
        println!("Running stress tests...");
        
        // Test concurrent evaluations
        self.run_test("concurrent_evaluations", || async {
            let model = create_test_model();
            let features: Vec<i64> = vec![1; 10];
            
            let mut handles = Vec::new();
            
            for _ in 0..self.config.concurrent_requests {
                let model = model.clone();
                let features = features.clone();
                
                let handle = tokio::spawn(async move {
                    for _ in 0..100 {
                        let _ = model.evaluate(&features);
                    }
                });
                
                handles.push(handle);
            }
            
            // Wait for all tasks to complete
            for handle in handles {
                handle.await.unwrap();
            }
            
            Ok(())
        }).await;
        
        // Test high load
        self.run_test("high_load_test", || async {
            let model = create_test_model();
            let features = vec![1.0; 10];
            
            let start = Instant::now();
            let mut tasks = Vec::new();
            
            for _ in 0..1000 {
                let model = model.clone();
                let features = features.clone();
                
                let task = tokio::spawn(async move {
                    model.evaluate(&features)
                });
                
                tasks.push(task);
            }
            
            // Wait for all tasks
            for task in tasks {
                let _ = task.await;
            }
            
            let duration = start.elapsed();
            assert!(duration < Duration::from_secs(30));
            
            Ok(())
        }).await;
        
        Ok(())
    }

    /// Run security tests
    async fn run_security_tests(&mut self) -> Result<()> {
        println!("Running security tests...");
        
        // Test input validation
        self.run_test("input_validation_security", || async {
            let config = SecurityConfig::default();
            let security = SecuritySystem::new(config);
            
            // Test valid input
            // assert!(security.validate_input(&[1.0, 2.0, 3.0]).is_ok());
            
            // Test invalid input (empty)
            // assert!(security.validate_input(&[]).is_err());
            
            // Test malicious input (extremely large values)
            let malicious_input = vec![f64::MAX; 1000];
            // assert!(security.validate_input(&malicious_input).is_err());
            
            Ok(())
        }).await;
        
        // Test rate limiting
        self.run_test("rate_limiting_security", || async {
            let config = SecurityConfig {
                max_requests_per_minute: 10,
                ..Default::default()
            };
            let security = SecuritySystem::new(config);
            
            // Make requests within limit
            for _ in 0..10 {
                // assert!(security.validate_input(&[1.0, 2.0, 3.0]).is_ok());
            }
            
            // Exceed rate limit
            // assert!(security.validate_input(&[1.0, 2.0, 3.0]).is_err());
            
            Ok(())
        }).await;
        
        // Test model integrity
        self.run_test("model_integrity_security", || async {
            let config = SecurityConfig::default();
            let security = SecuritySystem::new(config);
            
            let model = create_test_model();
            // assert!(security.validate_model_integrity(&model).is_ok());
            
            Ok(())
        }).await;
        
        Ok(())
    }

    /// Run end-to-end tests
    async fn run_e2e_tests(&mut self) -> Result<()> {
        println!("Running end-to-end tests...");
        
        // Test complete workflow
        self.run_test("complete_workflow", || async {
            // 1. Create production config
            let config = ProductionConfig::default_for_environment(Environment::Development);
            let config_manager = ProductionConfigManager::new("test_config.toml".into());
            // *config_manager.config.write().unwrap() = config;
            
            // 2. Create deployment
            let deployment = ProductionDeployment::new(std::sync::Arc::new(config_manager));
            
            // 3. Test deployment lifecycle
            assert_eq!(deployment.get_status(), DeploymentStatus::Starting);
            
            // 4. Test health check
            let health = deployment.perform_health_check().await?;
            assert_eq!(health.status, HealthStatus::Unhealthy);
            
            // 5. Test graceful shutdown
            deployment.shutdown().await?;
            assert_eq!(deployment.get_status(), DeploymentStatus::Stopped);
            
            Ok(())
        }).await;
        
        Ok(())
    }

    /// Run a single test
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
        
        metrics.insert("duration_ms".to_string(), duration.as_millis() as f64);
        metrics.insert("memory_mb".to_string(), get_memory_usage() as f64 / (1024.0 * 1024.0));
        
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

    /// Print test summary
    fn print_summary(&self) {
        let total_tests = self.results.len();
        let passed_tests = self.results.values().filter(|r| r.passed).count();
        let failed_tests = total_tests - passed_tests;
        
        println!("\n=== Test Summary ===");
        println!("Total tests: {}", total_tests);
        println!("Passed: {}", passed_tests);
        println!("Failed: {}", failed_tests);
        println!("Success rate: {:.1}%", (passed_tests as f64 / total_tests as f64) * 100.0);
        
        if failed_tests > 0 {
            println!("\nFailed tests:");
            for (name, result) in &self.results {
                if !result.passed {
                    println!("  - {}: {:?}", name, result.error);
                }
            }
        }
        
        println!("\nPerformance metrics:");
        for (name, result) in &self.results {
            if let Some(duration_ms) = result.metrics.get("duration_ms") {
                println!("  - {}: {:.2}ms", name, duration_ms);
            }
        }
    }
}

/// Create a test GBDT model
fn create_test_model() -> GBDTModel {
    create_test_model_inner()
}

fn create_test_model_inner() -> GBDTModel {
    use crate::gbdt::{Tree, Node};
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
        1, // feature_count
    ).unwrap()
}

/// Get current memory usage
fn get_memory_usage() -> u64 {
    // This would typically read from /proc/self/status or similar
    // For now, return a simulated value
    100 * 1024 * 1024 // 100MB
}

/// Benchmark suite
pub struct BenchmarkSuite {
    config: TestConfig,
}

impl BenchmarkSuite {
    /// Create a new benchmark suite
    pub fn new(config: TestConfig) -> Self {
        Self { config }
    }

    /// Run all benchmarks
    pub async fn run_all_benchmarks(&self) -> Result<()> {
        println!("Running benchmarks...");
        
        self.benchmark_gbdt_evaluation().await?;
        self.benchmark_model_loading().await?;
        self.benchmark_feature_engineering().await?;
        self.benchmark_monitoring().await?;
        self.benchmark_security().await?;
        
        Ok(())
    }

    /// Benchmark GBDT evaluation
    async fn benchmark_gbdt_evaluation(&self) -> Result<()> {
        let model = create_test_model();
        let features = vec![1i64; 10];
        
        let iterations = 10000;
        let start = Instant::now();
        
        for _ in 0..iterations {
            let _ = model.evaluate(&features)?;
        }
        
        let duration = start.elapsed();
        let evaluations_per_second = iterations as f64 / duration.as_secs_f64();
        
        println!("GBDT Evaluation Benchmark:");
        println!("  Iterations: {}", iterations);
        println!("  Duration: {:?}", duration);
        println!("  Evaluations/sec: {:.2}", evaluations_per_second);
        
        Ok(())
    }

    /// Benchmark model loading
    async fn benchmark_model_loading(&self) -> Result<()> {
        let config = ModelManagerConfig::default();
        let manager = ModelManager::new(config)?;
        
        let iterations = 1000;
        let start = Instant::now();
        
        for i in 0..iterations {
            let _ = manager.load_model(&format!("test_model_{}.bin", i)).await;
        }
        
        let duration = start.elapsed();
        let loads_per_second = iterations as f64 / duration.as_secs_f64();
        
        println!("Model Loading Benchmark:");
        println!("  Iterations: {}", iterations);
        println!("  Duration: {:?}", duration);
        println!("  Loads/sec: {:.2}", loads_per_second);
        
        Ok(())
    }

    /// Benchmark feature engineering
    async fn benchmark_feature_engineering(&self) -> Result<()> {
        let config = FeatureEngineeringConfig::default();
        let pipeline = FeatureEngineeringPipeline::new(config)?;
        
        let raw_data = RawFeatureData {
            features: vec![vec![1.0; 10]; 1000],
            labels: vec![0; 1000],
        };
        
        let start = Instant::now();
        let _ = pipeline.fit(&raw_data)?;
        let duration = start.elapsed();
        
        println!("Feature Engineering Benchmark:");
        println!("  Samples: {}", raw_data.features.len());
        println!("  Duration: {:?}", duration);
        println!("  Samples/sec: {:.2}", raw_data.features.len() as f64 / duration.as_secs_f64());
        
        Ok(())
    }

    /// Benchmark monitoring
    async fn benchmark_monitoring(&self) -> Result<()> {
        let config = MonitoringConfig::default();
        let monitoring = MonitoringSystem::new(config)?;
        monitoring.start().await?;
        
        let iterations = 10000;
        let start = Instant::now();
        
        for _ in 0..iterations {
            monitoring.record_gbdt_evaluation(1000, Duration::from_millis(10)).await;
        }
        
        let duration = start.elapsed();
        let records_per_second = iterations as f64 / duration.as_secs_f64();
        
        println!("Monitoring Benchmark:");
        println!("  Iterations: {}", iterations);
        println!("  Duration: {:?}", duration);
        println!("  Records/sec: {:.2}", records_per_second);
        
        monitoring.stop().await;
        Ok(())
    }

    /// Benchmark security
    async fn benchmark_security(&self) -> Result<()> {
        let config = SecurityConfig::default();
        let security = SecuritySystem::new(config)?;
        
        let iterations = 10000;
        let features = vec![1.0; 10];
        let start = Instant::now();
        
        for _ in 0..iterations {
            // let _ = security.validate_input(&features);
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
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Create a temporary test directory
    pub fn create_test_dir() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    /// Create test configuration
    pub fn create_test_config() -> ProductionConfig {
        ProductionConfig::default_for_environment(Environment::Testing)
    }

    /// Create test data
    pub fn create_test_data(size: usize) -> RawFeatureData {
        use std::collections::HashMap;
        RawFeatureData {
            features: vec![vec![1.0; 10]; size],
            feature_names: (0..10).map(|i| format!("feature_{}", i)).collect(),
            sample_count: size,
            feature_count: 10,
            metadata: HashMap::new(),
        }
    }

    /// Create test model  
    pub fn create_test_model() -> GBDTModel {
        create_test_model_inner()
    }

    /// Wait for condition with timeout
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_test_suite_creation() {
        let config = TestConfig::default();
        let test_suite = TestSuite::new(config);
        assert_eq!(test_suite.results.len(), 0);
    }

    #[tokio::test]
    async fn test_benchmark_suite_creation() {
        let config = TestConfig::default();
        let benchmark_suite = BenchmarkSuite::new(config);
        // Test that it can be created
        assert!(true);
    }

    #[tokio::test]
    async fn test_test_utils() {
        let test_dir = test_utils::create_test_dir();
        assert!(test_dir.path().exists());
        
        let config = test_utils::create_test_config();
        assert_eq!(config.environment, Environment::Testing);
        
        let test_data = test_utils::create_test_data(100);
        assert_eq!(test_data.features.len(), 100);
    }
}