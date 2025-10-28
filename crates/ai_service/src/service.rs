//! Main AI Service implementation

#[cfg(feature = "analytics")]
use crate::{
    analytics::AnalyticsService, monitoring::MonitoringService, optimization::OptimizationService,
};
use crate::{
    errors::AIServiceError,
    llm::LLMService,
    smart_contracts::SmartContractService,
    types::{
        AIServiceConfig, AnalyticsConfig, AnalyticsInsight, LLMConfig, LLMRequest, LLMResponse,
        MonitoringAlert, MonitoringConfig, SmartContractAnalysisRequest,
        SmartContractAnalysisResponse, TransactionOptimizationRequest,
        TransactionOptimizationResponse,
    },
};
use std::collections::HashMap;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

/// Main AI Service that coordinates all AI functionality
pub struct AIService {
    config: AIServiceConfig,
    llm_service: Option<LLMService>,
    #[cfg(feature = "analytics")]
    analytics_service: AnalyticsService,
    #[cfg(feature = "analytics")]
    monitoring_service: MonitoringService,
    smart_contract_service: SmartContractService,
    #[cfg(feature = "analytics")]
    optimization_service: OptimizationService,
    is_running: bool,
}

impl AIService {
    /// Create a new AI Service
    pub fn new(config: AIServiceConfig) -> Result<Self, AIServiceError> {
        // Initialize LLM service if enabled
        let llm_service = if config.enable_llm {
            Some(LLMService::new(config.llm_config.clone())?)
        } else {
            None
        };

        #[cfg(feature = "analytics")]
        let analytics_service = AnalyticsService::new(config.analytics_config.clone());
        #[cfg(feature = "analytics")]
        let monitoring_service = MonitoringService::new(config.monitoring_config.clone());
        let smart_contract_service = SmartContractService::new();
        #[cfg(feature = "analytics")]
        let optimization_service = OptimizationService::new();

        Ok(Self {
            config,
            llm_service,
            #[cfg(feature = "analytics")]
            analytics_service,
            #[cfg(feature = "analytics")]
            monitoring_service,
            smart_contract_service,
            #[cfg(feature = "analytics")]
            optimization_service,
            is_running: false,
        })
    }

    /// Start the AI Service
    pub async fn start(&mut self) -> Result<(), AIServiceError> {
        if self.is_running {
            return Err(AIServiceError::Internal(
                "Service is already running".to_string(),
            ));
        }

        info!("Starting AI Service v{}", crate::VERSION);

        #[cfg(feature = "analytics")]
        if self.config.enable_analytics {
            self.start_analytics_task().await?;
        }

        #[cfg(feature = "analytics")]
        if self.config.enable_monitoring {
            self.start_monitoring_task().await?;
        }

        self.is_running = true;
        info!("AI Service started successfully");

        Ok(())
    }

    /// Stop the AI Service
    pub async fn stop(&mut self) -> Result<(), AIServiceError> {
        if !self.is_running {
            return Err(AIServiceError::Internal(
                "Service is not running".to_string(),
            ));
        }

        info!("Stopping AI Service");
        self.is_running = false;
        info!("AI Service stopped");
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.is_running
    }

    pub fn get_status(&self) -> ServiceStatus {
        ServiceStatus {
            is_running: self.is_running,
            llm_enabled: self.config.enable_llm,
            analytics_enabled: self.config.enable_analytics,
            monitoring_enabled: self.config.enable_monitoring,
            smart_contracts_enabled: self.config.enable_smart_contracts,
            version: crate::VERSION.to_string(),
        }
    }

    // -------------------
    // LLM Methods
    // -------------------

    pub async fn generate_text(&self, request: LLMRequest) -> Result<LLMResponse, AIServiceError> {
        if !self.config.enable_llm {
            return Err(AIServiceError::ConfigError(
                "LLM features are disabled".to_string(),
            ));
        }

        let llm_service = self.llm_service.as_ref().ok_or_else(|| {
            AIServiceError::ConfigError("LLM service not initialized".to_string())
        })?;

        llm_service.generate(request).await
    }

    pub async fn analyze_blockchain_data(
        &self,
        data: &serde_json::Value,
        analysis_prompt: &str,
    ) -> Result<String, AIServiceError> {
        if !self.config.enable_llm {
            return Err(AIServiceError::ConfigError(
                "LLM features are disabled".to_string(),
            ));
        }

        let llm_service = self.llm_service.as_ref().ok_or_else(|| {
            AIServiceError::ConfigError("LLM service not initialized".to_string())
        })?;

        llm_service
            .analyze_blockchain_data(data, analysis_prompt)
            .await
    }

    pub async fn generate_contract_docs(
        &self,
        contract_code: &str,
        language: &str,
    ) -> Result<String, AIServiceError> {
        if !self.config.enable_llm {
            return Err(AIServiceError::ConfigError(
                "LLM features are disabled".to_string(),
            ));
        }

        let llm_service = self.llm_service.as_ref().ok_or_else(|| {
            AIServiceError::ConfigError("LLM service not initialized".to_string())
        })?;

        llm_service
            .generate_contract_docs(contract_code, language)
            .await
    }

    // -------------------
    // Analytics Methods
    // -------------------

    pub fn add_analytics_data(
        &mut self,
        metric: String,
        value: f64,
        unit: String,
        tags: HashMap<String, String>,
    ) {
        #[cfg(feature = "analytics")]
        if self.config.enable_analytics {
            self.analytics_service
                .add_data_point(metric, value, unit, tags);
        }
    }

    pub async fn get_analytics_insights(
        &mut self,
    ) -> Result<Vec<AnalyticsInsight>, AIServiceError> {
        if !self.config.enable_analytics {
            return Err(AIServiceError::ConfigError(
                "Analytics features are disabled".to_string(),
            ));
        }
        #[cfg(feature = "analytics")]
        {
            return self.analytics_service.analyze().await;
        }
        #[allow(unreachable_code)]
        Err(AIServiceError::ConfigError(
            "Analytics feature not compiled".to_string(),
        ))
    }

    pub fn get_all_insights(&self) -> &[AnalyticsInsight] {
        #[cfg(feature = "analytics")]
        {
            self.analytics_service.get_insights()
        }
        #[cfg(not(feature = "analytics"))]
        {
            &[]
        }
    }

    // -------------------
    // Monitoring Methods
    // -------------------

    pub fn add_monitoring_metric(&mut self, metric_name: String, value: f64) {
        #[cfg(feature = "analytics")]
        if self.config.enable_monitoring {
            self.monitoring_service.add_metric(metric_name, value);
        }
    }

    pub async fn check_monitoring_alerts(
        &mut self,
    ) -> Result<Vec<MonitoringAlert>, AIServiceError> {
        if !self.config.enable_monitoring {
            return Err(AIServiceError::ConfigError(
                "Monitoring features are disabled".to_string(),
            ));
        }
        #[cfg(feature = "analytics")]
        {
            return self.monitoring_service.check_alerts().await;
        }
        #[allow(unreachable_code)]
        Err(AIServiceError::ConfigError(
            "Monitoring feature not compiled".to_string(),
        ))
    }

    pub fn get_monitoring_alerts(&self) -> &[MonitoringAlert] {
        #[cfg(feature = "analytics")]
        {
            self.monitoring_service.get_alerts()
        }
        #[cfg(not(feature = "analytics"))]
        {
            &[]
        }
    }

    pub fn acknowledge_alert(&mut self, alert_id: &str) -> Result<(), AIServiceError> {
        #[cfg(feature = "analytics")]
        {
            self.monitoring_service.acknowledge_alert(alert_id)
        }
        #[cfg(not(feature = "analytics"))]
        {
            Err(AIServiceError::ConfigError(
                "Monitoring features are disabled".to_string(),
            ))
        }
    }

    pub fn resolve_alert(
        &mut self,
        alert_id: &str,
        resolution: String,
    ) -> Result<(), AIServiceError> {
        #[cfg(feature = "analytics")]
        {
            self.monitoring_service.resolve_alert(alert_id, resolution)
        }
        #[cfg(not(feature = "analytics"))]
        {
            Err(AIServiceError::ConfigError(
                "Monitoring features are disabled".to_string(),
            ))
        }
    }

    // -------------------
    // Smart Contract & Optimization
    // -------------------

    pub async fn analyze_smart_contract(
        &self,
        request: SmartContractAnalysisRequest,
    ) -> Result<SmartContractAnalysisResponse, AIServiceError> {
        if !self.config.enable_smart_contracts {
            return Err(AIServiceError::ConfigError(
                "Smart contract features are disabled".to_string(),
            ));
        }
        self.smart_contract_service.analyze_contract(request).await
    }

    pub async fn optimize_transaction(
        &self,
        request: TransactionOptimizationRequest,
    ) -> Result<TransactionOptimizationResponse, AIServiceError> {
        #[cfg(feature = "analytics")]
        {
            self.optimization_service
                .optimize_transaction(request)
                .await
        }
        #[cfg(not(feature = "analytics"))]
        {
            Err(AIServiceError::ConfigError(
                "Optimization features are disabled".to_string(),
            ))
        }
    }

    pub fn get_optimization_recommendations(
        &self,
        tx_type: &str,
    ) -> Vec<crate::types::OptimizationSuggestion> {
        #[cfg(feature = "analytics")]
        {
            self.optimization_service
                .get_recommendations_for_type(tx_type)
        }
        #[cfg(not(feature = "analytics"))]
        {
            Vec::new()
        }
    }

    // -------------------
    // Background Tasks
    // -------------------

    #[cfg(feature = "analytics")]
    async fn start_analytics_task(&self) -> Result<(), AIServiceError> {
        let mut interval = interval(Duration::from_secs(
            self.config.analytics_config.analysis_interval,
        ));
        tokio::spawn(async move {
            loop {
                interval.tick().await;
                info!("Analytics task running");
            }
        });
        Ok(())
    }

    #[cfg(feature = "analytics")]
    async fn start_monitoring_task(&self) -> Result<(), AIServiceError> {
        let mut interval = interval(Duration::from_secs(
            self.config.monitoring_config.monitoring_interval,
        ));
        tokio::spawn(async move {
            loop {
                interval.tick().await;
                info!("Monitoring task running");
            }
        });
        Ok(())
    }

    // -------------------
    // Configuration
    // -------------------

    pub fn update_llm_config(&mut self, config: LLMConfig) -> Result<(), AIServiceError> {
        if self.config.enable_llm {
            self.llm_service = Some(LLMService::new(config.clone())?);
            self.config.llm_config = config;
        }
        Ok(())
    }

    pub fn update_analytics_config(&mut self, config: AnalyticsConfig) {
        self.config.analytics_config = config;
    }

    pub fn update_monitoring_config(&mut self, config: MonitoringConfig) {
        self.config.monitoring_config = config;
    }

    pub fn get_config(&self) -> &AIServiceConfig {
        &self.config
    }
}

/// Service status information
#[derive(Debug, Clone)]
pub struct ServiceStatus {
    pub is_running: bool,
    pub llm_enabled: bool,
    pub analytics_enabled: bool,
    pub monitoring_enabled: bool,
    pub smart_contracts_enabled: bool,
    pub version: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_ai_service_creation() {
        let config = AIServiceConfig::default();
        let service = AIService::new(config);
        assert!(service.is_ok());
    }

    #[tokio::test]
    async fn test_ai_service_start_stop() {
        let config = AIServiceConfig::default();
        let mut service = AIService::new(config).unwrap();

        assert!(!service.is_running());
        service.start().await.unwrap();
        assert!(service.is_running());
        service.stop().await.unwrap();
        assert!(!service.is_running());
    }

    #[tokio::test]
    async fn test_add_analytics_data() {
        let config = AIServiceConfig::default();
        let mut service = AIService::new(config).unwrap();

        let mut tags = HashMap::new();
        tags.insert("node".to_string(), "node1".to_string());

        service.add_analytics_data("cpu_usage".to_string(), 75.0, "percent".to_string(), tags);
        for i in 0..5 {
            let mut t = HashMap::new();
            t.insert("node".to_string(), format!("node{}", i));
            service.add_analytics_data(
                "cpu_usage".to_string(),
                70.0 + (i as f64),
                "percent".to_string(),
                t,
            );
        }
        let _ = service.get_analytics_insights().await.unwrap_or_default();
    }

    #[tokio::test]
    async fn test_add_monitoring_metric() {
        let config = AIServiceConfig::default();
        let mut service = AIService::new(config).unwrap();

        service.start().await.unwrap();
        service.add_monitoring_metric("memory_usage".to_string(), 85.0);
        service.update_monitoring_config(crate::types::MonitoringConfig {
            enable_anomaly_detection: true,
            alert_thresholds: std::iter::once(("memory_usage".to_string(), 10.0)).collect(),
            monitoring_interval: 1,
            enable_auto_remediation: false,
        });
        let _ = service.check_monitoring_alerts().await.unwrap();
    }
}
