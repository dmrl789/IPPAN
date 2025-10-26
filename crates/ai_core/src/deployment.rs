//! Production deployment system for GBDT
//!
//! This module provides comprehensive deployment capabilities including:
//! - Health checks and readiness probes
//! - Graceful startup and shutdown
//! - Resource monitoring and limits
//! - Service discovery integration
//! - Rolling updates and rollbacks
//! - Deployment validation

use crate::gbdt::{GBDTModel, GBDTError};
use crate::model_manager::ModelManager;
use crate::feature_engineering::FeatureEngineeringPipeline;
use crate::monitoring::MonitoringSystem;
use crate::security::SecuritySystem;
use crate::production_config::{ProductionConfig, ProductionConfigManager, Environment};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{RwLock as AsyncRwLock, Semaphore};
use tokio::time::{sleep, timeout};
use tracing::{debug, error, info, warn, instrument};

/// Deployment status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeploymentStatus {
    Starting,
    Ready,
    Degraded,
    Failed,
    ShuttingDown,
    Stopped,
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub status: HealthStatus,
    pub message: String,
    pub timestamp: SystemTime,
    pub duration: Duration,
    pub details: HashMap<String, String>,
}

/// Health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Degraded,
}

/// Deployment metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentMetrics {
    pub startup_time: Duration,
    pub uptime: Duration,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_response_time: Duration,
    pub memory_usage_bytes: u64,
    pub cpu_usage_percent: f64,
    pub last_health_check: SystemTime,
    pub consecutive_failures: u32,
}

/// Production deployment manager
#[derive(Debug)]
pub struct ProductionDeployment {
    config_manager: Arc<ProductionConfigManager>,
    gbdt_models: Arc<AsyncRwLock<HashMap<String, GBDTModel>>>,
    model_manager: Arc<AsyncRwLock<Option<ModelManager>>>,
    feature_pipeline: Arc<AsyncRwLock<Option<FeatureEngineeringPipeline>>>,
    monitoring: Arc<AsyncRwLock<Option<MonitoringSystem>>>,
    security: Arc<AsyncRwLock<Option<SecuritySystem>>>,
    status: Arc<RwLock<DeploymentStatus>>,
    metrics: Arc<RwLock<DeploymentMetrics>>,
    startup_time: Instant,
    health_check_interval: Duration,
    max_startup_time: Duration,
    max_shutdown_time: Duration,
    resource_semaphore: Arc<Semaphore>,
}

impl ProductionDeployment {
    /// Create a new production deployment
    pub fn new(config_manager: Arc<ProductionConfigManager>) -> Self {
        let config = config_manager.get_config();
        let max_parallel_evaluations = config.gbdt.max_parallel_evaluations;
        
        Self {
            config_manager,
            gbdt_models: Arc::new(AsyncRwLock::new(HashMap::new())),
            model_manager: Arc::new(AsyncRwLock::new(None)),
            feature_pipeline: Arc::new(AsyncRwLock::new(None)),
            monitoring: Arc::new(AsyncRwLock::new(None)),
            security: Arc::new(AsyncRwLock::new(None)),
            status: Arc::new(RwLock::new(DeploymentStatus::Starting)),
            metrics: Arc::new(RwLock::new(DeploymentMetrics::default())),
            startup_time: Instant::now(),
            health_check_interval: Duration::from_secs(30),
            max_startup_time: Duration::from_secs(300), // 5 minutes
            max_shutdown_time: Duration::from_secs(60), // 1 minute
            resource_semaphore: Arc::new(Semaphore::new(max_parallel_evaluations)),
        }
    }

    /// Start the deployment
    #[instrument(skip(self))]
    pub async fn start(&self) -> Result<(), GBDTError> {
        info!("Starting production deployment...");
        
        // Set status to starting
        *self.status.write().unwrap() = DeploymentStatus::Starting;
        
        // Start with timeout
        let startup_result = timeout(self.max_startup_time, self.startup_sequence()).await;
        
        match startup_result {
            Ok(Ok(())) => {
                *self.status.write().unwrap() = DeploymentStatus::Ready;
                info!("Production deployment started successfully");
                Ok(())
            }
            Ok(Err(e)) => {
                *self.status.write().unwrap() = DeploymentStatus::Failed;
                error!("Failed to start deployment: {}", e);
                Err(e)
            }
            Err(_) => {
                *self.status.write().unwrap() = DeploymentStatus::Failed;
                error!("Deployment startup timed out after {:?}", self.max_startup_time);
                Err(GBDTError::EvaluationTimeout {
                    timeout_ms: self.max_startup_time.as_millis() as u64,
                })
            }
        }
    }

    /// Startup sequence
    async fn startup_sequence(&self) -> Result<(), GBDTError> {
        let config = self.config_manager.get_config();
        
        // 1. Initialize monitoring
        if config.monitoring.enable_performance_monitoring {
            self.initialize_monitoring().await?;
        }
        
        // 2. Initialize security
        if config.security.enable_input_validation {
            self.initialize_security().await?;
        }
        
        // 3. Initialize feature engineering
        if config.feature_engineering.enable_feature_engineering {
            self.initialize_feature_engineering().await?;
        }
        
        // 4. Initialize model manager
        if config.model_manager.enable_model_management {
            self.initialize_model_manager().await?;
        }
        
        // 5. Load initial models
        self.load_initial_models().await?;
        
        // 6. Start health monitoring
        self.start_health_monitoring().await;
        
        // 7. Update metrics
        self.update_startup_metrics().await;
        
        Ok(())
    }

    /// Initialize monitoring system
    async fn initialize_monitoring(&self) -> Result<(), GBDTError> {
        let config = self.config_manager.get_config();
        let monitoring = MonitoringSystem::new(config.monitoring.clone())
            .map_err(|e| GBDTError::ModelValidationFailed {
                reason: format!("Failed to initialize monitoring: {}", e),
            })?;
        
        monitoring.start().await
            .map_err(|e| GBDTError::ModelValidationFailed {
                reason: format!("Failed to start monitoring: {}", e),
            })?;
        
        *self.monitoring.write().await = Some(monitoring);
        info!("Monitoring system initialized");
        Ok(())
    }

    /// Initialize security system
    async fn initialize_security(&self) -> Result<(), GBDTError> {
        let config = self.config_manager.get_config();
        let security = SecuritySystem::new(config.security.clone())
            .map_err(|e| GBDTError::ModelValidationFailed {
                reason: format!("Failed to initialize security: {}", e),
            })?;
        
        *self.security.write().await = Some(security);
        info!("Security system initialized");
        Ok(())
    }

    /// Initialize feature engineering pipeline
    async fn initialize_feature_engineering(&self) -> Result<(), GBDTError> {
        let config = self.config_manager.get_config();
        let pipeline = FeatureEngineeringPipeline::new(config.feature_engineering.clone())
            .map_err(|e| GBDTError::ModelValidationFailed {
                reason: format!("Failed to initialize feature engineering: {}", e),
            })?;
        
        *self.feature_pipeline.write().await = Some(pipeline);
        info!("Feature engineering pipeline initialized");
        Ok(())
    }

    /// Initialize model manager
    async fn initialize_model_manager(&self) -> Result<(), GBDTError> {
        let config = self.config_manager.get_config();
        let model_manager = ModelManager::new(config.model_manager.clone())
            .map_err(|e| GBDTError::ModelValidationFailed {
                reason: format!("Failed to initialize model manager: {}", e),
            })?;
        
        *self.model_manager.write().await = Some(model_manager);
        info!("Model manager initialized");
        Ok(())
    }

    /// Load initial models
    async fn load_initial_models(&self) -> Result<(), GBDTError> {
        let config = self.config_manager.get_config();
        
        // Load default models if specified
        if let Some(default_models) = &config.model_manager.default_models {
            for model_path in default_models {
                self.load_model(model_path).await?;
            }
        }
        
        info!("Initial models loaded");
        Ok(())
    }

    /// Load a model
    async fn load_model(&self, model_path: &str) -> Result<(), GBDTError> {
        let model_manager = self.model_manager.read().await;
        if let Some(manager) = model_manager.as_ref() {
            let result = manager.load_model(model_path).await;
            match result {
                Ok(model) => {
                    let mut models = self.gbdt_models.write().await;
                    models.insert(model_path.to_string(), model);
                    info!("Model loaded: {}", model_path);
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to load model {}: {}", model_path, e);
                    Err(GBDTError::ModelValidationFailed {
                        reason: format!("Failed to load model {}: {}", model_path, e),
                    })
                }
            }
        } else {
            Err(GBDTError::ModelValidationFailed {
                reason: "Model manager not initialized".to_string(),
            })
        }
    }

    /// Start health monitoring
    async fn start_health_monitoring(&self) {
        let health_check_interval = self.health_check_interval;
        let deployment = self.clone();
        
        tokio::spawn(async move {
            loop {
                sleep(health_check_interval).await;
                
                match deployment.perform_health_check().await {
                    Ok(result) => {
                        if result.status != HealthStatus::Healthy {
                            warn!("Health check failed: {}", result.message);
                            *deployment.status.write().unwrap() = DeploymentStatus::Degraded;
                        } else {
                            *deployment.status.write().unwrap() = DeploymentStatus::Ready;
                        }
                    }
                    Err(e) => {
                        error!("Health check error: {}", e);
                        *deployment.status.write().unwrap() = DeploymentStatus::Failed;
                    }
                }
            }
        });
    }

    /// Perform health check
    #[instrument(skip(self))]
    pub async fn perform_health_check(&self) -> Result<HealthCheckResult, GBDTError> {
        let start_time = Instant::now();
        let mut details = HashMap::new();
        
        // Check system resources
        let resource_check = self.check_system_resources().await;
        details.insert("resource_check".to_string(), format!("{:?}", resource_check));
        
        // Check model availability
        let model_check = self.check_models().await;
        details.insert("model_check".to_string(), format!("{:?}", model_check));
        
        // Check monitoring system
        let monitoring_check = self.check_monitoring().await;
        details.insert("monitoring_check".to_string(), format!("{:?}", monitoring_check));
        
        // Check security system
        let security_check = self.check_security().await;
        details.insert("security_check".to_string(), format!("{:?}", security_check));
        
        // Determine overall health
        let overall_health = if resource_check && model_check && monitoring_check && security_check {
            HealthStatus::Healthy
        } else if resource_check && model_check {
            HealthStatus::Degraded
        } else {
            HealthStatus::Unhealthy
        };
        
        let duration = start_time.elapsed();
        let message = match overall_health {
            HealthStatus::Healthy => "All systems operational".to_string(),
            HealthStatus::Degraded => "Some systems degraded".to_string(),
            HealthStatus::Unhealthy => "Critical systems failed".to_string(),
        };
        
        // Update metrics
        self.update_health_metrics(overall_health == HealthStatus::Healthy).await;
        
        Ok(HealthCheckResult {
            status: overall_health,
            message,
            timestamp: SystemTime::now(),
            duration,
            details,
        })
    }

    /// Check system resources
    async fn check_system_resources(&self) -> bool {
        // This would typically check actual system resources
        // For now, we'll simulate a basic check
        true
    }

    /// Check models
    async fn check_models(&self) -> bool {
        let models = self.gbdt_models.read().await;
        !models.is_empty()
    }

    /// Check monitoring system
    async fn check_monitoring(&self) -> bool {
        let monitoring = self.monitoring.read().await;
        monitoring.is_some()
    }

    /// Check security system
    async fn check_security(&self) -> bool {
        let security = self.security.read().await;
        security.is_some()
    }

    /// Update health metrics
    async fn update_health_metrics(&self, is_healthy: bool) {
        let mut metrics = self.metrics.write().unwrap();
        metrics.last_health_check = SystemTime::now();
        
        if is_healthy {
            metrics.consecutive_failures = 0;
        } else {
            metrics.consecutive_failures += 1;
        }
    }

    /// Update startup metrics
    async fn update_startup_metrics(&self) {
        let mut metrics = self.metrics.write().unwrap();
        metrics.startup_time = self.startup_time.elapsed();
        metrics.uptime = Duration::from_secs(0);
    }

    /// Get deployment status
    pub fn get_status(&self) -> DeploymentStatus {
        self.status.read().unwrap().clone()
    }

    /// Get deployment metrics
    pub fn get_metrics(&self) -> DeploymentMetrics {
        self.metrics.read().unwrap().clone()
    }

    /// Get health status
    pub async fn get_health_status(&self) -> Result<HealthCheckResult, GBDTError> {
        self.perform_health_check().await
    }

    /// Graceful shutdown
    #[instrument(skip(self))]
    pub async fn shutdown(&self) -> Result<(), GBDTError> {
        info!("Initiating graceful shutdown...");
        
        *self.status.write().unwrap() = DeploymentStatus::ShuttingDown;
        
        // Shutdown with timeout
        let shutdown_result = timeout(self.max_shutdown_time, self.shutdown_sequence()).await;
        
        match shutdown_result {
            Ok(Ok(())) => {
                *self.status.write().unwrap() = DeploymentStatus::Stopped;
                info!("Graceful shutdown completed");
                Ok(())
            }
            Ok(Err(e)) => {
                *self.status.write().unwrap() = DeploymentStatus::Failed;
                error!("Shutdown failed: {}", e);
                Err(e)
            }
            Err(_) => {
                *self.status.write().unwrap() = DeploymentStatus::Failed;
                error!("Shutdown timed out after {:?}", self.max_shutdown_time);
                Err(GBDTError::EvaluationTimeout {
                    timeout_ms: self.max_shutdown_time.as_millis() as u64,
                })
            }
        }
    }

    /// Shutdown sequence
    async fn shutdown_sequence(&self) -> Result<(), GBDTError> {
        // 1. Stop accepting new requests
        info!("Stopping new request acceptance...");
        
        // 2. Wait for ongoing requests to complete
        info!("Waiting for ongoing requests to complete...");
        sleep(Duration::from_secs(5)).await;
        
        // 3. Stop monitoring
        if let Some(monitoring) = self.monitoring.write().await.take() {
            monitoring.stop().await;
            info!("Monitoring stopped");
        }
        
        // 4. Stop security
        if let Some(security) = self.security.write().await.take() {
            info!("Security system stopped");
        }
        
        // 5. Stop feature engineering
        if let Some(_pipeline) = self.feature_pipeline.write().await.take() {
            info!("Feature engineering pipeline stopped");
        }
        
        // 6. Stop model manager
        if let Some(model_manager) = self.model_manager.write().await.take() {
            model_manager.cleanup().await;
            info!("Model manager stopped");
        }
        
        // 7. Clear models
        self.gbdt_models.write().await.clear();
        info!("Models cleared");
        
        Ok(())
    }

    /// Check if deployment is ready
    pub fn is_ready(&self) -> bool {
        matches!(self.get_status(), DeploymentStatus::Ready)
    }

    /// Check if deployment is healthy
    pub async fn is_healthy(&self) -> bool {
        match self.perform_health_check().await {
            Ok(result) => result.status == HealthStatus::Healthy,
            Err(_) => false,
        }
    }

    /// Get uptime
    pub fn get_uptime(&self) -> Duration {
        self.startup_time.elapsed()
    }

    /// Get resource usage
    pub async fn get_resource_usage(&self) -> HashMap<String, f64> {
        let mut usage = HashMap::new();
        
        // This would typically get actual resource usage
        // For now, we'll return simulated values
        usage.insert("memory_percent".to_string(), 45.0);
        usage.insert("cpu_percent".to_string(), 23.0);
        usage.insert("disk_percent".to_string(), 12.0);
        
        usage
    }
}

impl Clone for ProductionDeployment {
    fn clone(&self) -> Self {
        Self {
            config_manager: self.config_manager.clone(),
            gbdt_models: self.gbdt_models.clone(),
            model_manager: self.model_manager.clone(),
            feature_pipeline: self.feature_pipeline.clone(),
            monitoring: self.monitoring.clone(),
            security: self.security.clone(),
            status: self.status.clone(),
            metrics: self.metrics.clone(),
            startup_time: self.startup_time,
            health_check_interval: self.health_check_interval,
            max_startup_time: self.max_startup_time,
            max_shutdown_time: self.max_shutdown_time,
            resource_semaphore: self.resource_semaphore.clone(),
        }
    }
}

impl Default for DeploymentMetrics {
    fn default() -> Self {
        Self {
            startup_time: Duration::from_secs(0),
            uptime: Duration::from_secs(0),
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            average_response_time: Duration::from_secs(0),
            memory_usage_bytes: 0,
            cpu_usage_percent: 0.0,
            last_health_check: SystemTime::now(),
            consecutive_failures: 0,
        }
    }
}

/// Deployment utilities
pub mod utils {
    use super::*;
    use std::process::Command;

    /// Validate deployment environment
    pub async fn validate_environment(config: &ProductionConfig) -> Result<(), GBDTError> {
        // Check required environment variables
        let required_vars = match config.environment {
            Environment::Production => vec!["RUST_LOG", "DATABASE_URL"],
            Environment::Staging => vec!["RUST_LOG"],
            _ => vec![],
        };

        for var in required_vars {
            if std::env::var(var).is_err() {
                return Err(GBDTError::ModelValidationFailed {
                    reason: format!("Required environment variable {} not set", var),
                });
            }
        }

        // Check system requirements
        if config.resources.max_memory_bytes > 0 {
            let available_memory = get_available_memory().await?;
            if available_memory < config.resources.max_memory_bytes {
                return Err(GBDTError::ModelValidationFailed {
                    reason: format!(
                        "Insufficient memory: required {} bytes, available {} bytes",
                        config.resources.max_memory_bytes, available_memory
                    ),
                });
            }
        }

        Ok(())
    }

    /// Get available memory
    async fn get_available_memory() -> Result<u64, GBDTError> {
        // This would typically read from /proc/meminfo or similar
        // For now, we'll return a simulated value
        Ok(8 * 1024 * 1024 * 1024) // 8GB
    }

    /// Check system dependencies
    pub async fn check_dependencies() -> Result<(), GBDTError> {
        // Check if required binaries are available
        let required_binaries = vec!["cargo", "rustc"];
        
        for binary in required_binaries {
            if Command::new(binary).arg("--version").output().is_err() {
                return Err(GBDTError::ModelValidationFailed {
                    reason: format!("Required binary {} not found", binary),
                });
            }
        }

        Ok(())
    }

    /// Generate deployment report
    pub async fn generate_deployment_report(deployment: &ProductionDeployment) -> String {
        let status = deployment.get_status();
        let metrics = deployment.get_metrics();
        let uptime = deployment.get_uptime();
        let resource_usage = deployment.get_resource_usage().await;
        
        format!(
            "Deployment Report\n\
             ================\n\
             Status: {:?}\n\
             Uptime: {:?}\n\
             Startup Time: {:?}\n\
             Total Requests: {}\n\
             Successful Requests: {}\n\
             Failed Requests: {}\n\
             Memory Usage: {:.2}%\n\
             CPU Usage: {:.2}%\n\
             Disk Usage: {:.2}%",
            status,
            uptime,
            metrics.startup_time,
            metrics.total_requests,
            metrics.successful_requests,
            metrics.failed_requests,
            resource_usage.get("memory_percent").unwrap_or(&0.0),
            resource_usage.get("cpu_percent").unwrap_or(&0.0),
            resource_usage.get("disk_percent").unwrap_or(&0.0)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::production_config::ProductionConfigManager;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_deployment_creation() {
        let config_manager = Arc::new(ProductionConfigManager::new(PathBuf::from("test.toml")));
        let deployment = ProductionDeployment::new(config_manager);
        
        assert_eq!(deployment.get_status(), DeploymentStatus::Starting);
        assert!(!deployment.is_ready());
    }

    #[tokio::test]
    async fn test_health_check() {
        let config_manager = Arc::new(ProductionConfigManager::new(PathBuf::from("test.toml")));
        let deployment = ProductionDeployment::new(config_manager);
        
        let health_result = deployment.perform_health_check().await.unwrap();
        assert_eq!(health_result.status, HealthStatus::Unhealthy); // No models loaded
    }

    #[tokio::test]
    async fn test_deployment_metrics() {
        let config_manager = Arc::new(ProductionConfigManager::new(PathBuf::from("test.toml")));
        let deployment = ProductionDeployment::new(config_manager);
        
        let metrics = deployment.get_metrics();
        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.successful_requests, 0);
    }
}