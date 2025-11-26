//! Production-ready configuration management

use crate::{fixed_math::parse_decimal_env, AIServiceConfig, AIServiceError};
use ippan_ai_core::Fixed;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

/// Configuration manager for production environments
#[derive(Debug, Clone)]
pub struct ConfigManager {
    config: AIServiceConfig,
    environment: Environment,
    secrets: HashMap<String, String>,
}

/// Environment type
#[derive(Debug, Clone, PartialEq)]
pub enum Environment {
    Development,
    Staging,
    Production,
    Testing,
}

impl ConfigManager {
    /// Create a new configuration manager
    pub fn new() -> Result<Self, AIServiceError> {
        let environment = Self::detect_environment()?;
        let mut config = Self::load_config(&environment)?;
        let mut secrets = Self::load_secrets(&environment)?;

        Self::apply_env_overrides(&mut config);
        Self::apply_secret_overrides(&mut config, &mut secrets)?;

        Ok(Self {
            config,
            environment,
            secrets,
        })
    }

    /// Get the current configuration
    pub fn get_config(&self) -> &AIServiceConfig {
        &self.config
    }

    /// Get the current environment
    pub fn get_environment(&self) -> &Environment {
        &self.environment
    }

    /// Get a secret value
    pub fn get_secret(&self, key: &str) -> Option<&String> {
        self.secrets.get(key)
    }

    /// Update configuration at runtime
    pub fn update_config(&mut self, new_config: AIServiceConfig) {
        self.config = new_config;
    }

    /// Reload configuration from files
    pub fn reload(&mut self) -> Result<(), AIServiceError> {
        self.config = Self::load_config(&self.environment)?;
        self.secrets = Self::load_secrets(&self.environment)?;
        Ok(())
    }

    /// Detect the current environment
    fn detect_environment() -> Result<Environment, AIServiceError> {
        let env_str = env::var("IPPAN_ENV")
            .or_else(|_| env::var("ENVIRONMENT"))
            .unwrap_or_else(|_| "development".to_string());

        match env_str.to_lowercase().as_str() {
            "production" | "prod" => Ok(Environment::Production),
            "staging" | "stage" => Ok(Environment::Staging),
            "testing" | "test" => Ok(Environment::Testing),
            "development" | "dev" => Ok(Environment::Development),
            _ => Err(AIServiceError::Internal(format!(
                "Unknown environment: {env_str}"
            ))),
        }
    }

    /// Load configuration based on environment
    fn load_config(environment: &Environment) -> Result<AIServiceConfig, AIServiceError> {
        let config_path = match environment {
            Environment::Production => "config/production.toml",
            Environment::Staging => "config/staging.toml",
            Environment::Testing => "config/testing.toml",
            Environment::Development => "config/development.toml",
        };

        if Path::new(config_path).exists() {
            Self::load_config_from_file(config_path)
        } else {
            Self::load_config_from_env()
        }
    }

    /// Load configuration from TOML file
    fn load_config_from_file(path: &str) -> Result<AIServiceConfig, AIServiceError> {
        let content = fs::read_to_string(path).map_err(|e| {
            AIServiceError::Io(format!("Failed to read config file {path}: {e}"))
        })?;

        let config: ConfigFile = toml::from_str(&content).map_err(|e| {
            AIServiceError::SerializationError(format!("Failed to parse config file: {e}"))
        })?;

        Ok(config.into())
    }

    /// Load configuration from environment variables
    fn load_config_from_env() -> Result<AIServiceConfig, AIServiceError> {
        Ok(AIServiceConfig {
            enable_llm: env::var("ENABLE_LLM")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            enable_analytics: env::var("ENABLE_ANALYTICS")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            enable_smart_contracts: env::var("ENABLE_SMART_CONTRACTS")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            enable_monitoring: env::var("ENABLE_MONITORING")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            llm_config: crate::LLMConfig {
                api_endpoint: env::var("LLM_API_ENDPOINT")
                    .unwrap_or_else(|_| "https://api.openai.com/v1".to_string()),
                api_key: env::var("LLM_API_KEY")
                    .unwrap_or_else(|_| "your-api-key-here".to_string()),
                model_name: env::var("LLM_MODEL").unwrap_or_else(|_| "gpt-4".to_string()),
                max_tokens: env::var("LLM_MAX_TOKENS")
                    .unwrap_or_else(|_| "4000".to_string())
                    .parse()
                    .unwrap_or(4000),
                temperature: parse_decimal_env(
                    &env::var("LLM_TEMPERATURE").unwrap_or_else(|_| "0.7".to_string()),
                    Fixed::from_ratio(7, 10),
                ),
                timeout_seconds: env::var("LLM_TIMEOUT")
                    .unwrap_or_else(|_| "30".to_string())
                    .parse()
                    .unwrap_or(30),
            },
            analytics_config: crate::AnalyticsConfig {
                enable_realtime: env::var("ANALYTICS_REALTIME")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
                retention_days: env::var("ANALYTICS_RETENTION_DAYS")
                    .unwrap_or_else(|_| "30".to_string())
                    .parse()
                    .unwrap_or(30),
                analysis_interval: env::var("ANALYTICS_INTERVAL")
                    .unwrap_or_else(|_| "60".to_string())
                    .parse()
                    .unwrap_or(60),
                enable_predictive: env::var("ANALYTICS_PREDICTIVE")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
            },
        })
    }

    /// Load secrets from environment or secret files
    fn load_secrets(environment: &Environment) -> Result<HashMap<String, String>, AIServiceError> {
        let mut secrets = HashMap::new();

        // Load from environment variables
        for (key, value) in env::vars() {
            if key.starts_with("IPPAN_SECRET_") {
                let secret_key = key.strip_prefix("IPPAN_SECRET_").unwrap().to_lowercase();
                secrets.insert(secret_key, value);
            }
        }

        // Load from secret files if they exist
        let secret_file = match environment {
            Environment::Production => "/run/secrets/ippan-secrets",
            Environment::Staging => "secrets/staging.env",
            Environment::Testing => "secrets/testing.env",
            Environment::Development => "secrets/development.env",
        };

        if Path::new(secret_file).exists() {
            let content = fs::read_to_string(secret_file)
                .map_err(|e| AIServiceError::Io(format!("Failed to read secrets file: {e}")))?;

            for line in content.lines() {
                if let Some((key, value)) = line.split_once('=') {
                    secrets.insert(key.trim().to_lowercase(), value.trim().to_string());
                }
            }
        }

        Ok(secrets)
    }

    fn apply_env_overrides(config: &mut AIServiceConfig) {
        if let Ok(value) = env::var("ENABLE_LLM") {
            if let Some(parsed) = Self::parse_bool(&value) {
                config.enable_llm = parsed;
            }
        }

        if let Ok(value) = env::var("ENABLE_ANALYTICS") {
            if let Some(parsed) = Self::parse_bool(&value) {
                config.enable_analytics = parsed;
            }
        }

        if let Ok(value) = env::var("ENABLE_SMART_CONTRACTS") {
            if let Some(parsed) = Self::parse_bool(&value) {
                config.enable_smart_contracts = parsed;
            }
        }

        if let Ok(value) = env::var("ENABLE_MONITORING") {
            if let Some(parsed) = Self::parse_bool(&value) {
                config.enable_monitoring = parsed;
            }
        }

        if let Ok(value) = env::var("LLM_API_ENDPOINT") {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                config.llm_config.api_endpoint = trimmed.to_string();
            }
        }

        if let Ok(value) = env::var("LLM_MODEL") {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                config.llm_config.model_name = trimmed.to_string();
            }
        }

        if let Ok(value) = env::var("LLM_MAX_TOKENS") {
            if let Ok(parsed) = value.trim().parse::<u32>() {
                config.llm_config.max_tokens = parsed;
            }
        }

        if let Ok(value) = env::var("LLM_TEMPERATURE") {
            config.llm_config.temperature =
                parse_decimal_env(value.as_str(), config.llm_config.temperature);
        }

        if let Ok(value) = env::var("LLM_TIMEOUT") {
            if let Ok(parsed) = value.trim().parse::<u64>() {
                config.llm_config.timeout_seconds = parsed;
            }
        }

        if let Ok(value) = env::var("ANALYTICS_REALTIME") {
            if let Some(parsed) = Self::parse_bool(&value) {
                config.analytics_config.enable_realtime = parsed;
            }
        }

        if let Ok(value) = env::var("ANALYTICS_RETENTION_DAYS") {
            if let Ok(parsed) = value.trim().parse::<u32>() {
                config.analytics_config.retention_days = parsed;
            }
        }

        if let Ok(value) = env::var("ANALYTICS_INTERVAL") {
            if let Ok(parsed) = value.trim().parse::<u64>() {
                config.analytics_config.analysis_interval = parsed;
            }
        }

        if let Ok(value) = env::var("ANALYTICS_PREDICTIVE") {
            if let Some(parsed) = Self::parse_bool(&value) {
                config.analytics_config.enable_predictive = parsed;
            }
        }
    }

    fn apply_secret_overrides(
        config: &mut AIServiceConfig,
        secrets: &mut HashMap<String, String>,
    ) -> Result<(), AIServiceError> {
        let env_api_key = env::var("LLM_API_KEY")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        let secret_api_key = secrets
            .get("llm_api_key")
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        if let Some(api_key) = env_api_key.or(secret_api_key) {
            config.llm_config.api_key = api_key.clone();
            secrets.insert("llm_api_key".to_string(), api_key);
            Ok(())
        } else {
            Err(AIServiceError::ConfigError(
                "Missing required secret: LLM_API_KEY. Provide via environment variable or secret store."
                    .to_string(),
            ))
        }
    }

    fn parse_bool(value: &str) -> Option<bool> {
        let value = value.trim();
        if value.eq_ignore_ascii_case("true")
            || value.eq_ignore_ascii_case("yes")
            || value.eq_ignore_ascii_case("y")
            || value == "1"
        {
            Some(true)
        } else if value.eq_ignore_ascii_case("false")
            || value.eq_ignore_ascii_case("no")
            || value.eq_ignore_ascii_case("n")
            || value == "0"
        {
            Some(false)
        } else {
            None
        }
    }
}

/// Configuration file structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConfigFile {
    service: ServiceConfig,
    llm: LLMConfigFile,
    analytics: AnalyticsConfigFile,
    #[serde(default)]
    logging: LoggingConfig,
    #[serde(default)]
    security: SecurityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ServiceConfig {
    enable_llm: bool,
    enable_analytics: bool,
    enable_smart_contracts: bool,
    enable_monitoring: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LLMConfigFile {
    api_endpoint: String,
    model_name: String,
    max_tokens: u32,
    temperature: toml::Value,
    timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnalyticsConfigFile {
    enable_realtime: bool,
    retention_days: u32,
    analysis_interval: u64,
    enable_predictive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LoggingConfig {
    level: String,
    format: String,
    output: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "pretty".to_string(),
            output: "stdout".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SecurityConfig {
    enable_encryption: bool,
    enable_authentication: bool,
    session_timeout: u64,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_encryption: false,
            enable_authentication: false,
            session_timeout: 3600,
        }
    }
}

impl From<ConfigFile> for AIServiceConfig {
    fn from(config: ConfigFile) -> Self {
        AIServiceConfig {
            enable_llm: config.service.enable_llm,
            enable_analytics: config.service.enable_analytics,
            enable_smart_contracts: config.service.enable_smart_contracts,
            enable_monitoring: config.service.enable_monitoring,
            llm_config: crate::LLMConfig {
                api_endpoint: config.llm.api_endpoint,
                api_key: "".to_string(), // Will be loaded from secrets
                model_name: config.llm.model_name,
                max_tokens: config.llm.max_tokens,
                temperature: parse_decimal_env(
                    &config.llm.temperature.to_string(),
                    Fixed::from_ratio(7, 10),
                ),
                timeout_seconds: config.llm.timeout_seconds,
            },
            analytics_config: crate::AnalyticsConfig {
                enable_realtime: config.analytics.enable_realtime,
                retention_days: config.analytics.retention_days,
                analysis_interval: config.analytics.analysis_interval,
                enable_predictive: config.analytics.enable_predictive,
            },
        }
    }
}
