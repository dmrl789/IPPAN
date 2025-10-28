//! Model execution engine

use crate::{
    errors::{AiCoreError, Result},
    types::*,
};
use std::collections::HashMap;
use tracing::{error, info, warn};

/// Model execution engine
pub struct ExecutionEngine {
    /// Loaded models cache
    models: HashMap<ModelId, ModelMetadata>,
    /// Execution statistics
    stats: ExecutionStats,
}

/// Execution statistics
#[derive(Debug, Default)]
pub struct ExecutionStats {
    /// Total executions
    pub total_executions: u64,
    /// Successful executions
    pub successful_executions: u64,
    /// Failed executions
    pub failed_executions: u64,
    /// Average execution time in microseconds
    pub avg_execution_time_us: u64,
    /// Total memory usage in bytes
    pub total_memory_usage: u64,
}

impl ExecutionEngine {
    /// Create a new execution engine
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
            stats: ExecutionStats::default(),
        }
    }

    /// Load a model into the execution engine
    pub async fn load_model(&mut self, model_data: &[u8], metadata: ModelMetadata) -> Result<()> {
        info!("Loading model: {:?}", metadata.id);

        // Validate model data
        self.validate_model_data(model_data, &metadata)?;

        // Store model metadata
        self.models.insert(metadata.id.clone(), metadata);

        info!(
            "Model loaded successfully. Total loaded: {}",
            self.models.len()
        );
        Ok(())
    }

    /// Execute a model with given input
    pub async fn execute(
        &mut self,
        context: ExecutionContext,
        input: ModelInput,
    ) -> Result<ExecutionResult> {
        info!("Executing model: {:?}", context.model_id);

        // Retrieve model
        let model_metadata = self
            .models
            .get(&context.model_id)
            .ok_or_else(|| AiCoreError::ExecutionFailed("Model not found".to_string()))?;

        // Validate input
        self.validate_input(&input, model_metadata)?;

        // Execute model
        let start_time = std::time::Instant::now();
        let output = self
            .execute_model_deterministic(model_metadata, &input, &context)
            .await?;
        let execution_time = start_time.elapsed();

        // Update statistics
        self.update_stats(execution_time.as_micros() as u64, true);

        // Assemble execution result
        let result = ExecutionResult {
            data_type: output.dtype,
            execution_time_us: execution_time.as_micros() as u64,
            memory_usage: (output.data.len() + input.data.len()) as u64,
            output,
            context,
            success: true,
            error: None,
            metadata: HashMap::new(),
        };

        info!("Model execution completed successfully");
        Ok(result)
    }

    /// Validate model data integrity
    fn validate_model_data(&self, data: &[u8], metadata: &ModelMetadata) -> Result<()> {
        if data.len() as u64 != metadata.size_bytes {
            return Err(AiCoreError::ValidationFailed(
                "Model data size mismatch".to_string(),
            ));
        }

        let computed_hash = blake3::hash(data).to_hex().to_string();
        if computed_hash != metadata.id.hash {
            return Err(AiCoreError::ValidationFailed(
                "Model hash verification failed".to_string(),
            ));
        }

        Ok(())
    }

    /// Validate input data
    fn validate_input(&self, input: &ModelInput, metadata: &ModelMetadata) -> Result<()> {
        if input.shape != metadata.input_shape {
            return Err(AiCoreError::InvalidParameters(format!(
                "Input shape mismatch: expected {:?}, got {:?}",
                metadata.input_shape, input.shape
            )));
        }

        let expected_size = input.shape.iter().product::<usize>() * input.dtype.size_bytes();
        if input.data.len() != expected_size {
            return Err(AiCoreError::InvalidParameters(
                "Input data size mismatch".to_string(),
            ));
        }

        Ok(())
    }

    /// Execute model with deterministic behavior
    async fn execute_model_deterministic(
        &self,
        metadata: &ModelMetadata,
        input: &ModelInput,
        context: &ExecutionContext,
    ) -> Result<ModelOutput> {
        let start_time = std::time::Instant::now();

        // Apply deterministic seed if present
        if let Some(seed) = context.seed {
            info!("Using deterministic seed: {}", seed);
            std::env::set_var("AI_DETERMINISTIC_SEED", seed.to_string());
        }

        info!(
            "Executing model: {:?}, input size: {} bytes",
            metadata.id,
            input.data.len()
        );

        let output_size =
            metadata.output_shape.iter().product::<usize>() * input.dtype.size_bytes();
        let mut output_data = vec![0u8; output_size];

        // Placeholder deterministic inference logic
        if metadata.architecture == "gbdt" {
            info!("Executing GBDT model");
            let features = self.convert_input_to_features(input)?;
            for (i, byte) in output_data.iter_mut().enumerate() {
                *byte = ((features.iter().sum::<i64>() as usize + i) % 256) as u8;
            }
        } else {
            info!("Executing generic model: {}", metadata.architecture);
            let mut sum: u64 = 0;
            for chunk in input.data.chunks(8) {
                let mut arr = [0u8; 8];
                for (i, &byte) in chunk.iter().enumerate() {
                    if i < arr.len() {
                        arr[i] = byte;
                    }
                }
                sum = sum.wrapping_add(u64::from_le_bytes(arr));
            }
            for (i, byte) in output_data.iter_mut().enumerate() {
                *byte = ((sum as usize + i) % 256) as u8;
            }
        }

        let execution_time = start_time.elapsed();
        let execution_hash = self.compute_execution_hash(metadata, input, context)?;

        info!("Model execution completed in {:?}", execution_time);

        Ok(ModelOutput {
            data: output_data,
            dtype: input.dtype,
            shape: metadata.output_shape.clone(),
            confidence: 1.0,
            metadata: ExecutionMetadata {
                execution_id: context.id.clone(),
                model_id: metadata.id.to_string(),
                start_time: 0,
                end_time: 0,
                duration_us: execution_time.as_micros() as u64,
                memory_usage: metadata.size_bytes + input.data.len() as u64,
                cpu_usage: 0.0,
                success: true,
                error: None,
                metadata: {
                    let mut m = HashMap::new();
                    m.insert("execution_hash".to_string(), execution_hash);
                    m.insert("model_version".to_string(), metadata.id.version.clone());
                    m
                },
            },
        })
    }

    /// Convert input data to feature vector for GBDT models
    fn convert_input_to_features(&self, input: &ModelInput) -> Result<Vec<i64>> {
        let feature_count = input.data.len() / 8;
        let mut features = Vec::with_capacity(feature_count);

        match input.dtype {
            DataType::Int64 => {
                for chunk in input.data.chunks(8) {
                    if chunk.len() == 8 {
                        let mut arr = [0u8; 8];
                        arr.copy_from_slice(chunk);
                        features.push(i64::from_le_bytes(arr));
                    }
                }
            }
            DataType::Int32 => {
                for chunk in input.data.chunks(4) {
                    if chunk.len() == 4 {
                        let mut arr = [0u8; 4];
                        arr.copy_from_slice(chunk);
                        features.push(i32::from_le_bytes(arr) as i64);
                    }
                }
            }
            DataType::Float32 => {
                for chunk in input.data.chunks(4) {
                    if chunk.len() == 4 {
                        let mut arr = [0u8; 4];
                        arr.copy_from_slice(chunk);
                        let float_val = f32::from_le_bytes(arr);
                        features.push((float_val * 10000.0) as i64);
                    }
                }
            }
            _ => {
                return Err(AiCoreError::InvalidParameters(
                    "Unsupported data type for GBDT features".to_string(),
                ));
            }
        }

        Ok(features)
    }

    /// Estimate CPU cycles from execution time
    fn estimate_cpu_cycles(&self, duration: std::time::Duration) -> u64 {
        const CPU_FREQ_HZ: u64 = 3_000_000_000; // 3 GHz
        let seconds = duration.as_secs_f64();
        (seconds * CPU_FREQ_HZ as f64) as u64
    }

    /// Compute deterministic execution hash
    fn compute_execution_hash(
        &self,
        metadata: &ModelMetadata,
        input: &ModelInput,
        context: &ExecutionContext,
    ) -> Result<String> {
        let mut hasher = blake3::Hasher::new();

        hasher.update(metadata.id.hash.as_bytes());
        hasher.update(metadata.id.version.as_bytes());
        hasher.update(&input.data);
        hasher.update(
            &input
                .shape
                .iter()
                .flat_map(|x| x.to_le_bytes())
                .collect::<Vec<_>>(),
        );
        hasher.update(&context.seed.unwrap_or(0).to_le_bytes());

        for (key, value) in &context.parameters {
            hasher.update(key.as_bytes());
            hasher.update(value.as_bytes());
        }

        Ok(hasher.finalize().to_hex().to_string())
    }

    /// Update execution statistics
    fn update_stats(&mut self, execution_time_us: u64, success: bool) {
        self.stats.total_executions += 1;
        if success {
            self.stats.successful_executions += 1;
        } else {
            self.stats.failed_executions += 1;
        }

        self.stats.avg_execution_time_us = if self.stats.total_executions == 0 {
            execution_time_us
        } else {
            (self.stats.avg_execution_time_us * (self.stats.total_executions - 1)
                + execution_time_us)
                / self.stats.total_executions
        };
    }

    /// Get execution statistics
    pub fn get_stats(&self) -> &ExecutionStats {
        &self.stats
    }

    /// Get loaded model count
    pub fn model_count(&self) -> usize {
        self.models.len()
    }
}

// Removed duplicate size_bytes implementation (now defined on types::DataType)
