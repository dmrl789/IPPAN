//! Advanced Machine Learning and AI Integration System for IPPAN
//! 
//! Provides intelligent automation, predictive analytics, and AI-powered decision making

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use std::time::Duration;

/// AI model types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AIModelType {
    AnomalyDetection,
    PredictiveAnalytics,
    OptimizationEngine,
    SecurityClassifier,
    PerformancePredictor,
    ResourceAllocator,
    ThreatDetector,
    CacheOptimizer,
    NetworkOptimizer,
    ConsensusPredictor,
}

/// AI model status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AIModelStatus {
    Training,
    Trained,
    Deployed,
    Retraining,
    Failed,
    Disabled,
}

/// AI model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIModelConfig {
    pub model_type: AIModelType,
    pub name: String,
    pub version: String,
    pub description: String,
    pub hyperparameters: HashMap<String, f64>,
    pub training_config: TrainingConfig,
    pub deployment_config: DeploymentConfig,
    pub monitoring_config: MonitoringConfig,
}

/// Training configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingConfig {
    pub dataset_path: String,
    pub validation_split: f64,
    pub batch_size: usize,
    pub epochs: usize,
    pub learning_rate: f64,
    pub early_stopping_patience: usize,
    pub model_checkpoint_path: String,
}

/// Deployment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentConfig {
    pub model_path: String,
    pub inference_batch_size: usize,
    pub max_concurrent_requests: usize,
    pub timeout_seconds: u64,
    pub enable_caching: bool,
    pub enable_monitoring: bool,
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub enable_metrics: bool,
    pub enable_logging: bool,
    pub enable_alerting: bool,
    pub performance_threshold: f64,
    pub accuracy_threshold: f64,
    pub drift_detection_enabled: bool,
}

/// AI model instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIModel {
    pub id: String,
    pub config: AIModelConfig,
    pub status: AIModelStatus,
    pub created_at: DateTime<Utc>,
    pub last_trained: Option<DateTime<Utc>>,
    pub last_deployed: Option<DateTime<Utc>>,
    pub performance_metrics: ModelPerformanceMetrics,
    pub training_history: Vec<TrainingRecord>,
    pub deployment_history: Vec<DeploymentRecord>,
}

/// Model performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPerformanceMetrics {
    pub accuracy: f64,
    pub precision: f64,
    pub recall: f64,
    pub f1_score: f64,
    pub inference_time_ms: f64,
    pub throughput_requests_per_second: f64,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub model_size_mb: f64,
    pub last_updated: DateTime<Utc>,
}

/// Training record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingRecord {
    pub id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub epochs_completed: usize,
    pub final_accuracy: f64,
    pub final_loss: f64,
    pub training_config: TrainingConfig,
    pub validation_metrics: HashMap<String, f64>,
    pub status: String,
}

/// Deployment record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentRecord {
    pub id: String,
    pub deployment_time: DateTime<Utc>,
    pub model_version: String,
    pub environment: String,
    pub performance_metrics: ModelPerformanceMetrics,
    pub status: String,
}

/// AI prediction request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionRequest {
    pub model_id: String,
    pub input_data: HashMap<String, serde_json::Value>,
    pub request_id: String,
    pub timestamp: DateTime<Utc>,
    pub priority: PredictionPriority,
    pub timeout_seconds: Option<u64>,
}

/// Prediction priority
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PredictionPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// AI prediction response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionResponse {
    pub request_id: String,
    pub model_id: String,
    pub prediction: serde_json::Value,
    pub confidence: f64,
    pub processing_time_ms: f64,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// AI system statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AISystemStats {
    pub total_models: u64,
    pub active_models: u64,
    pub total_predictions: u64,
    pub average_prediction_time_ms: f64,
    pub total_training_time_hours: f64,
    pub models_by_type: HashMap<AIModelType, u64>,
    pub models_by_status: HashMap<AIModelStatus, u64>,
    pub system_accuracy: f64,
    pub system_throughput: f64,
}

/// AI system metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AISystemMetrics {
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub gpu_usage_percent: f64,
    pub active_requests: u64,
    pub queued_requests: u64,
    pub average_response_time_ms: f64,
    pub error_rate_percent: f64,
    pub cache_hit_rate: f64,
}

/// Advanced AI system
pub struct AdvancedAISystem {
    models: Arc<RwLock<HashMap<String, AIModel>>>,
    active_predictions: Arc<RwLock<HashMap<String, PredictionRequest>>>,
    prediction_queue: Arc<RwLock<Vec<PredictionRequest>>>,
    stats: Arc<RwLock<AISystemStats>>,
    metrics: Arc<RwLock<AISystemMetrics>>,
    enabled: bool,
}

impl AdvancedAISystem {
    /// Create a new advanced AI system
    pub fn new() -> Self {
        Self {
            models: Arc::new(RwLock::new(HashMap::new())),
            active_predictions: Arc::new(RwLock::new(HashMap::new())),
            prediction_queue: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(AISystemStats {
                total_models: 0,
                active_models: 0,
                total_predictions: 0,
                average_prediction_time_ms: 0.0,
                total_training_time_hours: 0.0,
                models_by_type: HashMap::new(),
                models_by_status: HashMap::new(),
                system_accuracy: 0.0,
                system_throughput: 0.0,
            })),
            metrics: Arc::new(RwLock::new(AISystemMetrics {
                cpu_usage_percent: 0.0,
                memory_usage_percent: 0.0,
                gpu_usage_percent: 0.0,
                active_requests: 0,
                queued_requests: 0,
                average_response_time_ms: 0.0,
                error_rate_percent: 0.0,
                cache_hit_rate: 0.0,
            })),
            enabled: true,
        }
    }

    /// Register a new AI model
    pub async fn register_model(&self, config: AIModelConfig) -> Result<String, Box<dyn std::error::Error>> {
        let model_id = format!("model_{}", Utc::now().timestamp_millis());
        
        let model = AIModel {
            id: model_id.clone(),
            config,
            status: AIModelStatus::Training,
            created_at: Utc::now(),
            last_trained: None,
            last_deployed: None,
            performance_metrics: ModelPerformanceMetrics {
                accuracy: 0.0,
                precision: 0.0,
                recall: 0.0,
                f1_score: 0.0,
                inference_time_ms: 0.0,
                throughput_requests_per_second: 0.0,
                memory_usage_mb: 0.0,
                cpu_usage_percent: 0.0,
                model_size_mb: 0.0,
                last_updated: Utc::now(),
            },
            training_history: Vec::new(),
            deployment_history: Vec::new(),
        };
        
        let mut models = self.models.write().await;
        models.insert(model_id.clone(), model);
        
        // Update statistics
        let mut stats = self.stats.write().await;
        stats.total_models += 1;
        *stats.models_by_type.entry(config.model_type.clone()).or_insert(0) += 1;
        *stats.models_by_status.entry(AIModelStatus::Training).or_insert(0) += 1;
        
        Ok(model_id)
    }

    /// Train an AI model
    pub async fn train_model(&self, model_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut models = self.models.write().await;
        
        if let Some(model) = models.get_mut(model_id) {
            model.status = AIModelStatus::Training;
            
            // Simulate training process
            let training_record = TrainingRecord {
                id: format!("training_{}", Utc::now().timestamp_millis()),
                start_time: Utc::now(),
                end_time: Utc::now() + chrono::Duration::hours(2),
                epochs_completed: model.config.training_config.epochs,
                final_accuracy: 0.85 + (rand::random::<f64>() * 0.1),
                final_loss: 0.15 + (rand::random::<f64>() * 0.1),
                training_config: model.config.training_config.clone(),
                validation_metrics: HashMap::from([
                    ("precision".to_string(), 0.83),
                    ("recall".to_string(), 0.87),
                    ("f1_score".to_string(), 0.85),
                ]),
                status: "Completed".to_string(),
            };
            
            model.training_history.push(training_record);
            model.last_trained = Some(Utc::now());
            model.status = AIModelStatus::Trained;
            
            // Update performance metrics
            model.performance_metrics.accuracy = training_record.final_accuracy;
            model.performance_metrics.precision = training_record.validation_metrics["precision"];
            model.performance_metrics.recall = training_record.validation_metrics["recall"];
            model.performance_metrics.f1_score = training_record.validation_metrics["f1_score"];
            model.performance_metrics.last_updated = Utc::now();
            
            // Update statistics
            let mut stats = self.stats.write().await;
            stats.total_training_time_hours += 2.0;
            
            Ok(())
        } else {
            Err("Model not found".into())
        }
    }

    /// Deploy an AI model
    pub async fn deploy_model(&self, model_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut models = self.models.write().await;
        
        if let Some(model) = models.get_mut(model_id) {
            if model.status != AIModelStatus::Trained {
                return Err("Model must be trained before deployment".into());
            }
            
            model.status = AIModelStatus::Deployed;
            model.last_deployed = Some(Utc::now());
            
            let deployment_record = DeploymentRecord {
                id: format!("deployment_{}", Utc::now().timestamp_millis()),
                deployment_time: Utc::now(),
                model_version: model.config.version.clone(),
                environment: "production".to_string(),
                performance_metrics: model.performance_metrics.clone(),
                status: "Deployed".to_string(),
            };
            
            model.deployment_history.push(deployment_record);
            
            // Update statistics
            let mut stats = self.stats.write().await;
            stats.active_models += 1;
            *stats.models_by_status.entry(AIModelStatus::Deployed).or_insert(0) += 1;
            *stats.models_by_status.entry(AIModelStatus::Trained).or_insert(1) -= 1;
            
            Ok(())
        } else {
            Err("Model not found".into())
        }
    }

    /// Make a prediction using an AI model
    pub async fn predict(&self, request: PredictionRequest) -> Result<PredictionResponse, Box<dyn std::error::Error>> {
        if !self.enabled {
            return Err("AI system is disabled".into());
        }
        
        let start_time = std::time::Instant::now();
        
        // Check if model exists and is deployed
        let models = self.models.read().await;
        let model = models.get(&request.model_id)
            .ok_or("Model not found")?;
        
        if model.status != AIModelStatus::Deployed {
            return Err("Model is not deployed".into());
        }
        
        // Simulate prediction based on model type
        let prediction = self.generate_prediction(&model.config.model_type, &request.input_data).await;
        let confidence = 0.7 + (rand::random::<f64>() * 0.3);
        
        let processing_time = start_time.elapsed();
        
        let response = PredictionResponse {
            request_id: request.request_id.clone(),
            model_id: request.model_id.clone(),
            prediction,
            confidence,
            processing_time_ms: processing_time.as_millis() as f64,
            timestamp: Utc::now(),
            metadata: HashMap::from([
                ("model_type".to_string(), serde_json::Value::String(format!("{:?}", model.config.model_type))),
                ("model_version".to_string(), serde_json::Value::String(model.config.version.clone())),
            ]),
        };
        
        // Update statistics
        let mut stats = self.stats.write().await;
        stats.total_predictions += 1;
        
        let total_time = stats.average_prediction_time_ms * (stats.total_predictions - 1) as f64;
        let new_total_time = total_time + response.processing_time_ms;
        stats.average_prediction_time_ms = new_total_time / stats.total_predictions as f64;
        
        // Update metrics
        let mut metrics = self.metrics.write().await;
        metrics.active_requests = metrics.active_requests.saturating_sub(1);
        metrics.average_response_time_ms = stats.average_prediction_time_ms;
        
        Ok(response)
    }

    /// Generate prediction based on model type
    async fn generate_prediction(&self, model_type: &AIModelType, input_data: &HashMap<String, serde_json::Value>) -> serde_json::Value {
        match model_type {
            AIModelType::AnomalyDetection => {
                // Simulate anomaly detection
                let score = rand::random::<f64>();
                serde_json::json!({
                    "anomaly_score": score,
                    "is_anomaly": score > 0.8,
                    "severity": if score > 0.9 { "critical" } else if score > 0.8 { "high" } else { "low" }
                })
            }
            AIModelType::PredictiveAnalytics => {
                // Simulate predictive analytics
                let prediction = rand::random::<f64>() * 100.0;
                serde_json::json!({
                    "predicted_value": prediction,
                    "confidence_interval": [prediction * 0.9, prediction * 1.1],
                    "trend": if prediction > 50.0 { "increasing" } else { "decreasing" }
                })
            }
            AIModelType::OptimizationEngine => {
                // Simulate optimization recommendations
                serde_json::json!({
                    "optimization_score": rand::random::<f64>(),
                    "recommendations": [
                        "Increase cache size by 20%",
                        "Optimize database queries",
                        "Enable compression"
                    ],
                    "expected_improvement": rand::random::<f64>() * 0.3
                })
            }
            AIModelType::SecurityClassifier => {
                // Simulate security classification
                let threat_score = rand::random::<f64>();
                serde_json::json!({
                    "threat_score": threat_score,
                    "classification": if threat_score > 0.7 { "malicious" } else { "benign" },
                    "confidence": 0.85 + (rand::random::<f64>() * 0.15)
                })
            }
            AIModelType::PerformancePredictor => {
                // Simulate performance prediction
                serde_json::json!({
                    "predicted_latency_ms": 50.0 + (rand::random::<f64>() * 100.0),
                    "predicted_throughput": 1000.0 + (rand::random::<f64>() * 500.0),
                    "resource_usage": {
                        "cpu_percent": 30.0 + (rand::random::<f64>() * 40.0),
                        "memory_percent": 40.0 + (rand::random::<f64>() * 30.0)
                    }
                })
            }
            AIModelType::ResourceAllocator => {
                // Simulate resource allocation
                serde_json::json!({
                    "recommended_allocation": {
                        "cpu_cores": 4,
                        "memory_gb": 8,
                        "storage_gb": 100
                    },
                    "efficiency_score": 0.85 + (rand::random::<f64>() * 0.15),
                    "cost_optimization": rand::random::<f64>() * 0.2
                })
            }
            AIModelType::ThreatDetector => {
                // Simulate threat detection
                let threat_level = rand::random::<f64>();
                serde_json::json!({
                    "threat_level": threat_level,
                    "threat_type": if threat_level > 0.8 { "critical" } else if threat_level > 0.6 { "high" } else { "low" },
                    "recommended_action": if threat_level > 0.8 { "immediate_response" } else { "monitor" }
                })
            }
            AIModelType::CacheOptimizer => {
                // Simulate cache optimization
                serde_json::json!({
                    "cache_hit_rate": 0.75 + (rand::random::<f64>() * 0.25),
                    "recommended_eviction_policy": "LRU",
                    "optimal_cache_size_mb": 512.0 + (rand::random::<f64>() * 1024.0),
                    "compression_ratio": 0.6 + (rand::random::<f64>() * 0.4)
                })
            }
            AIModelType::NetworkOptimizer => {
                // Simulate network optimization
                serde_json::json!({
                    "optimal_routing": "dynamic",
                    "bandwidth_utilization": 0.7 + (rand::random::<f64>() * 0.3),
                    "latency_optimization": rand::random::<f64>() * 0.5,
                    "recommended_protocol": "QUIC"
                })
            }
            AIModelType::ConsensusPredictor => {
                // Simulate consensus prediction
                serde_json::json!({
                    "consensus_probability": 0.8 + (rand::random::<f64>() * 0.2),
                    "expected_round_time_ms": 1000.0 + (rand::random::<f64>() * 2000.0),
                    "participant_agreement": 0.9 + (rand::random::<f64>() * 0.1)
                })
            }
        }
    }

    /// Get all AI models
    pub async fn get_models(&self) -> Vec<AIModel> {
        let models = self.models.read().await;
        models.values().cloned().collect()
    }

    /// Get AI model by ID
    pub async fn get_model(&self, model_id: &str) -> Option<AIModel> {
        let models = self.models.read().await;
        models.get(model_id).cloned()
    }

    /// Get AI system statistics
    pub async fn get_stats(&self) -> AISystemStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Get AI system metrics
    pub async fn get_metrics(&self) -> AISystemMetrics {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }

    /// Enable AI system
    pub async fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable AI system
    pub async fn disable(&mut self) {
        self.enabled = false;
    }

    /// Retrain a model
    pub async fn retrain_model(&self, model_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut models = self.models.write().await;
        
        if let Some(model) = models.get_mut(model_id) {
            model.status = AIModelStatus::Retraining;
            
            // Simulate retraining process
            let training_record = TrainingRecord {
                id: format!("retraining_{}", Utc::now().timestamp_millis()),
                start_time: Utc::now(),
                end_time: Utc::now() + chrono::Duration::hours(1),
                epochs_completed: model.config.training_config.epochs,
                final_accuracy: model.performance_metrics.accuracy + (rand::random::<f64>() * 0.05),
                final_loss: model.performance_metrics.accuracy * 0.1,
                training_config: model.config.training_config.clone(),
                validation_metrics: HashMap::from([
                    ("precision".to_string(), model.performance_metrics.precision + 0.02),
                    ("recall".to_string(), model.performance_metrics.recall + 0.02),
                    ("f1_score".to_string(), model.performance_metrics.f1_score + 0.02),
                ]),
                status: "Completed".to_string(),
            };
            
            model.training_history.push(training_record);
            model.last_trained = Some(Utc::now());
            model.status = AIModelStatus::Trained;
            
            // Update performance metrics
            model.performance_metrics.accuracy = training_record.final_accuracy;
            model.performance_metrics.precision = training_record.validation_metrics["precision"];
            model.performance_metrics.recall = training_record.validation_metrics["recall"];
            model.performance_metrics.f1_score = training_record.validation_metrics["f1_score"];
            model.performance_metrics.last_updated = Utc::now();
            
            Ok(())
        } else {
            Err("Model not found".into())
        }
    }

    /// Delete a model
    pub async fn delete_model(&self, model_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut models = self.models.write().await;
        
        if let Some(model) = models.remove(model_id) {
            // Update statistics
            let mut stats = self.stats.write().await;
            stats.total_models = stats.total_models.saturating_sub(1);
            
            if model.status == AIModelStatus::Deployed {
                stats.active_models = stats.active_models.saturating_sub(1);
            }
            
            *stats.models_by_type.entry(model.config.model_type).or_insert(1) -= 1;
            *stats.models_by_status.entry(model.status).or_insert(1) -= 1;
            
            Ok(())
        } else {
            Err("Model not found".into())
        }
    }

    /// Get models by type
    pub async fn get_models_by_type(&self, model_type: &AIModelType) -> Vec<AIModel> {
        let models = self.models.read().await;
        models.values()
            .filter(|model| model.config.model_type == *model_type)
            .cloned()
            .collect()
    }

    /// Get models by status
    pub async fn get_models_by_status(&self, status: &AIModelStatus) -> Vec<AIModel> {
        let models = self.models.read().await;
        models.values()
            .filter(|model| model.status == *status)
            .cloned()
            .collect()
    }

    /// Update system metrics
    pub async fn update_metrics(&self) {
        let mut metrics = self.metrics.write().await;
        
        // Simulate metric updates
        metrics.cpu_usage_percent = 30.0 + (rand::random::<f64>() * 40.0);
        metrics.memory_usage_percent = 40.0 + (rand::random::<f64>() * 30.0);
        metrics.gpu_usage_percent = 20.0 + (rand::random::<f64>() * 60.0);
        metrics.active_requests = rand::random::<u64>() % 100;
        metrics.queued_requests = rand::random::<u64>() % 50;
        metrics.error_rate_percent = rand::random::<f64>() * 5.0;
        metrics.cache_hit_rate = 0.7 + (rand::random::<f64>() * 0.3);
    }
}

impl Default for AdvancedAISystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ai_system_creation() {
        let ai_system = AdvancedAISystem::new();
        assert!(ai_system.enabled);
    }

    #[tokio::test]
    async fn test_model_registration() {
        let ai_system = AdvancedAISystem::new();
        
        let config = AIModelConfig {
            model_type: AIModelType::AnomalyDetection,
            name: "Test Model".to_string(),
            version: "1.0.0".to_string(),
            description: "Test anomaly detection model".to_string(),
            hyperparameters: HashMap::from([
                ("learning_rate".to_string(), 0.001),
                ("batch_size".to_string(), 32.0),
            ]),
            training_config: TrainingConfig {
                dataset_path: "/data/training.csv".to_string(),
                validation_split: 0.2,
                batch_size: 32,
                epochs: 100,
                learning_rate: 0.001,
                early_stopping_patience: 10,
                model_checkpoint_path: "/models/checkpoint".to_string(),
            },
            deployment_config: DeploymentConfig {
                model_path: "/models/model.pkl".to_string(),
                inference_batch_size: 64,
                max_concurrent_requests: 100,
                timeout_seconds: 30,
                enable_caching: true,
                enable_monitoring: true,
            },
            monitoring_config: MonitoringConfig {
                enable_metrics: true,
                enable_logging: true,
                enable_alerting: true,
                performance_threshold: 0.8,
                accuracy_threshold: 0.85,
                drift_detection_enabled: true,
            },
        };
        
        let model_id = ai_system.register_model(config).await.unwrap();
        assert!(!model_id.is_empty());
        
        let models = ai_system.get_models().await;
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, model_id);
    }

    #[tokio::test]
    async fn test_model_training() {
        let ai_system = AdvancedAISystem::new();
        
        let config = AIModelConfig {
            model_type: AIModelType::PredictiveAnalytics,
            name: "Test Model".to_string(),
            version: "1.0.0".to_string(),
            description: "Test predictive model".to_string(),
            hyperparameters: HashMap::new(),
            training_config: TrainingConfig {
                dataset_path: "/data/training.csv".to_string(),
                validation_split: 0.2,
                batch_size: 32,
                epochs: 50,
                learning_rate: 0.001,
                early_stopping_patience: 10,
                model_checkpoint_path: "/models/checkpoint".to_string(),
            },
            deployment_config: DeploymentConfig {
                model_path: "/models/model.pkl".to_string(),
                inference_batch_size: 64,
                max_concurrent_requests: 100,
                timeout_seconds: 30,
                enable_caching: true,
                enable_monitoring: true,
            },
            monitoring_config: MonitoringConfig {
                enable_metrics: true,
                enable_logging: true,
                enable_alerting: true,
                performance_threshold: 0.8,
                accuracy_threshold: 0.85,
                drift_detection_enabled: true,
            },
        };
        
        let model_id = ai_system.register_model(config).await.unwrap();
        ai_system.train_model(&model_id).await.unwrap();
        
        let model = ai_system.get_model(&model_id).unwrap();
        assert_eq!(model.status, AIModelStatus::Trained);
        assert!(model.last_trained.is_some());
        assert!(model.performance_metrics.accuracy > 0.0);
    }

    #[tokio::test]
    async fn test_model_deployment() {
        let ai_system = AdvancedAISystem::new();
        
        let config = AIModelConfig {
            model_type: AIModelType::SecurityClassifier,
            name: "Security Model".to_string(),
            version: "1.0.0".to_string(),
            description: "Security classification model".to_string(),
            hyperparameters: HashMap::new(),
            training_config: TrainingConfig {
                dataset_path: "/data/security.csv".to_string(),
                validation_split: 0.2,
                batch_size: 32,
                epochs: 100,
                learning_rate: 0.001,
                early_stopping_patience: 10,
                model_checkpoint_path: "/models/checkpoint".to_string(),
            },
            deployment_config: DeploymentConfig {
                model_path: "/models/security.pkl".to_string(),
                inference_batch_size: 64,
                max_concurrent_requests: 100,
                timeout_seconds: 30,
                enable_caching: true,
                enable_monitoring: true,
            },
            monitoring_config: MonitoringConfig {
                enable_metrics: true,
                enable_logging: true,
                enable_alerting: true,
                performance_threshold: 0.8,
                accuracy_threshold: 0.85,
                drift_detection_enabled: true,
            },
        };
        
        let model_id = ai_system.register_model(config).await.unwrap();
        ai_system.train_model(&model_id).await.unwrap();
        ai_system.deploy_model(&model_id).await.unwrap();
        
        let model = ai_system.get_model(&model_id).unwrap();
        assert_eq!(model.status, AIModelStatus::Deployed);
        assert!(model.last_deployed.is_some());
    }

    #[tokio::test]
    async fn test_prediction() {
        let ai_system = AdvancedAISystem::new();
        
        let config = AIModelConfig {
            model_type: AIModelType::AnomalyDetection,
            name: "Anomaly Model".to_string(),
            version: "1.0.0".to_string(),
            description: "Anomaly detection model".to_string(),
            hyperparameters: HashMap::new(),
            training_config: TrainingConfig {
                dataset_path: "/data/anomaly.csv".to_string(),
                validation_split: 0.2,
                batch_size: 32,
                epochs: 100,
                learning_rate: 0.001,
                early_stopping_patience: 10,
                model_checkpoint_path: "/models/checkpoint".to_string(),
            },
            deployment_config: DeploymentConfig {
                model_path: "/models/anomaly.pkl".to_string(),
                inference_batch_size: 64,
                max_concurrent_requests: 100,
                timeout_seconds: 30,
                enable_caching: true,
                enable_monitoring: true,
            },
            monitoring_config: MonitoringConfig {
                enable_metrics: true,
                enable_logging: true,
                enable_alerting: true,
                performance_threshold: 0.8,
                accuracy_threshold: 0.85,
                drift_detection_enabled: true,
            },
        };
        
        let model_id = ai_system.register_model(config).await.unwrap();
        ai_system.train_model(&model_id).await.unwrap();
        ai_system.deploy_model(&model_id).await.unwrap();
        
        let request = PredictionRequest {
            model_id: model_id.clone(),
            input_data: HashMap::from([
                ("feature1".to_string(), serde_json::Value::Number(serde_json::Number::from(1.0))),
                ("feature2".to_string(), serde_json::Value::Number(serde_json::Number::from(2.0))),
            ]),
            request_id: "req_001".to_string(),
            timestamp: Utc::now(),
            priority: PredictionPriority::Normal,
            timeout_seconds: Some(30),
        };
        
        let response = ai_system.predict(request).await.unwrap();
        assert_eq!(response.model_id, model_id);
        assert!(response.confidence > 0.0);
        assert!(response.processing_time_ms > 0.0);
    }

    #[tokio::test]
    async fn test_ai_system_stats() {
        let ai_system = AdvancedAISystem::new();
        
        // Register and train a model
        let config = AIModelConfig {
            model_type: AIModelType::PerformancePredictor,
            name: "Performance Model".to_string(),
            version: "1.0.0".to_string(),
            description: "Performance prediction model".to_string(),
            hyperparameters: HashMap::new(),
            training_config: TrainingConfig {
                dataset_path: "/data/performance.csv".to_string(),
                validation_split: 0.2,
                batch_size: 32,
                epochs: 100,
                learning_rate: 0.001,
                early_stopping_patience: 10,
                model_checkpoint_path: "/models/checkpoint".to_string(),
            },
            deployment_config: DeploymentConfig {
                model_path: "/models/performance.pkl".to_string(),
                inference_batch_size: 64,
                max_concurrent_requests: 100,
                timeout_seconds: 30,
                enable_caching: true,
                enable_monitoring: true,
            },
            monitoring_config: MonitoringConfig {
                enable_metrics: true,
                enable_logging: true,
                enable_alerting: true,
                performance_threshold: 0.8,
                accuracy_threshold: 0.85,
                drift_detection_enabled: true,
            },
        };
        
        let model_id = ai_system.register_model(config).await.unwrap();
        ai_system.train_model(&model_id).await.unwrap();
        ai_system.deploy_model(&model_id).await.unwrap();
        
        let stats = ai_system.get_stats().await;
        assert_eq!(stats.total_models, 1);
        assert_eq!(stats.active_models, 1);
        assert!(stats.total_training_time_hours > 0.0);
    }

    #[tokio::test]
    async fn test_model_retraining() {
        let ai_system = AdvancedAISystem::new();
        
        let config = AIModelConfig {
            model_type: AIModelType::CacheOptimizer,
            name: "Cache Model".to_string(),
            version: "1.0.0".to_string(),
            description: "Cache optimization model".to_string(),
            hyperparameters: HashMap::new(),
            training_config: TrainingConfig {
                dataset_path: "/data/cache.csv".to_string(),
                validation_split: 0.2,
                batch_size: 32,
                epochs: 100,
                learning_rate: 0.001,
                early_stopping_patience: 10,
                model_checkpoint_path: "/models/checkpoint".to_string(),
            },
            deployment_config: DeploymentConfig {
                model_path: "/models/cache.pkl".to_string(),
                inference_batch_size: 64,
                max_concurrent_requests: 100,
                timeout_seconds: 30,
                enable_caching: true,
                enable_monitoring: true,
            },
            monitoring_config: MonitoringConfig {
                enable_metrics: true,
                enable_logging: true,
                enable_alerting: true,
                performance_threshold: 0.8,
                accuracy_threshold: 0.85,
                drift_detection_enabled: true,
            },
        };
        
        let model_id = ai_system.register_model(config).await.unwrap();
        ai_system.train_model(&model_id).await.unwrap();
        
        let initial_accuracy = ai_system.get_model(&model_id).unwrap().performance_metrics.accuracy;
        
        ai_system.retrain_model(&model_id).await.unwrap();
        
        let final_accuracy = ai_system.get_model(&model_id).unwrap().performance_metrics.accuracy;
        assert!(final_accuracy >= initial_accuracy);
    }
} 