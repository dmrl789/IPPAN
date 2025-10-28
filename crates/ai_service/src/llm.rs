//! LLM integration module

use crate::errors::AIServiceError;
use crate::types::{LLMConfig, LLMRequest, LLMResponse, LLMUsage};
use serde_json::json;
use std::time::Duration;

/// LLM service client
pub struct LLMService {
    config: LLMConfig,
    client: reqwest::Client,
}

impl LLMService {
    /// Create a new LLM service
    pub fn new(config: LLMConfig) -> Result<Self, AIServiceError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| AIServiceError::NetworkError(e.to_string()))?;

        Ok(Self { config, client })
    }

    /// Generate text using LLM
    pub async fn generate(&self, request: LLMRequest) -> Result<LLMResponse, AIServiceError> {
        let payload = json!({
            "model": self.config.model_name,
            "messages": [
                {
                    "role": "user",
                    "content": request.prompt
                }
            ],
            "max_tokens": request.max_tokens.unwrap_or(self.config.max_tokens),
            "temperature": request.temperature.unwrap_or(self.config.temperature),
            "stream": request.stream
        });

        let response = self
            .client
            .post(&format!("{}/chat/completions", self.config.api_endpoint))
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AIServiceError::LLMError(format!(
                "API request failed: {} - {}",
                status, error_text
            )));
        }

        let response_json: serde_json::Value = response.json().await?;

        let choice = response_json["choices"][0]
            .as_object()
            .ok_or_else(|| AIServiceError::LLMError("Invalid response format".to_string()))?;

        let message = choice["message"]["content"]
            .as_str()
            .ok_or_else(|| AIServiceError::LLMError("No content in response".to_string()))?;

        let usage = response_json["usage"]
            .as_object()
            .ok_or_else(|| AIServiceError::LLMError("No usage data in response".to_string()))?;

        let finish_reason = choice["finish_reason"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        Ok(LLMResponse {
            text: message.to_string(),
            usage: LLMUsage {
                prompt_tokens: usage["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                completion_tokens: usage["completion_tokens"].as_u64().unwrap_or(0) as u32,
                total_tokens: usage["total_tokens"].as_u64().unwrap_or(0) as u32,
            },
            model: self.config.model_name.clone(),
            finish_reason,
        })
    }

    /// Analyze blockchain data with natural language
    pub async fn analyze_blockchain_data(
        &self,
        data: &serde_json::Value,
        analysis_prompt: &str,
    ) -> Result<String, AIServiceError> {
        let context = json!({
            "data": data,
            "analysis_type": "blockchain_analysis"
        });

        let request = LLMRequest {
            prompt: format!(
                "Analyze the following blockchain data and provide insights:\n\n{}\n\nData: {}",
                analysis_prompt,
                serde_json::to_string_pretty(data)?
            ),
            context: Some(context.as_object().unwrap().clone().into_iter().collect()),
            max_tokens: Some(2000),
            temperature: Some(0.3),
            stream: false,
        };

        let response = self.generate(request).await?;
        Ok(response.text)
    }

    /// Generate smart contract documentation
    pub async fn generate_contract_docs(
        &self,
        contract_code: &str,
        language: &str,
    ) -> Result<String, AIServiceError> {
        let request = LLMRequest {
            prompt: format!(
                "Generate comprehensive documentation for this {} smart contract:\n\n```{}\n{}\n```\n\nInclude:\n- Overview and purpose\n- Function descriptions\n- Parameters and return values\n- Usage examples\n- Security considerations",
                language,
                language,
                contract_code
            ),
            context: None,
            max_tokens: Some(3000),
            temperature: Some(0.2),
            stream: false,
        };

        let response = self.generate(request).await?;
        Ok(response.text)
    }

    /// Generate transaction explanation
    pub async fn explain_transaction(
        &self,
        transaction: &serde_json::Value,
    ) -> Result<String, AIServiceError> {
        let request = LLMRequest {
            prompt: format!(
                "Explain this blockchain transaction in simple terms:\n\n{}",
                serde_json::to_string_pretty(transaction)?
            ),
            context: None,
            max_tokens: Some(1000),
            temperature: Some(0.5),
            stream: false,
        };

        let response = self.generate(request).await?;
        Ok(response.text)
    }

    /// Generate monitoring alert explanation
    pub async fn explain_alert(
        &self,
        alert_data: &serde_json::Value,
    ) -> Result<String, AIServiceError> {
        let request = LLMRequest {
            prompt: format!(
                "Explain this blockchain monitoring alert and suggest potential solutions:\n\n{}",
                serde_json::to_string_pretty(alert_data)?
            ),
            context: None,
            max_tokens: Some(1500),
            temperature: Some(0.4),
            stream: false,
        };

        let response = self.generate(request).await?;
        Ok(response.text)
    }

    /// Generate optimization suggestions
    pub async fn suggest_optimizations(
        &self,
        performance_data: &serde_json::Value,
    ) -> Result<String, AIServiceError> {
        let request = LLMRequest {
            prompt: format!(
                "Analyze this blockchain performance data and suggest optimizations:\n\n{}",
                serde_json::to_string_pretty(performance_data)?
            ),
            context: None,
            max_tokens: Some(2000),
            temperature: Some(0.3),
            stream: false,
        };

        let response = self.generate(request).await?;
        Ok(response.text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_llm_service_creation() {
        let config = LLMConfig {
            api_endpoint: "https://api.openai.com/v1".to_string(),
            api_key: "test-key".to_string(),
            model_name: "gpt-4".to_string(),
            max_tokens: 1000,
            temperature: 0.7,
            timeout_seconds: 30,
        };

        let service = LLMService::new(config);
        assert!(service.is_ok());
    }

    #[tokio::test]
    async fn test_generate_request_structure() {
        let request = LLMRequest {
            prompt: "Test prompt".to_string(),
            context: None,
            max_tokens: Some(100),
            temperature: Some(0.5),
            stream: false,
        };

        assert_eq!(request.prompt, "Test prompt");
        assert_eq!(request.max_tokens, Some(100));
        assert_eq!(request.temperature, Some(0.5));
        assert!(!request.stream);
    }
}
