//! Model execution engine
//!
//! Provides deterministic model execution for AI inference inside IPPAN/FinDAG systems.

use crate::{
    errors::{AiCoreError, Result},
    types::*,
};
use std::collections::HashMap;
use tracing::info;

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
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub avg_execution_time_us: u64,
    pub total_memory_usage: u64,
}

impl Default for ExecutionEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutionEngine {
    /// Create new execution engine
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
            stats: ExecutionStats::default(),
        }
    }

    /// Load a model into memory
    pub async fn load_model(&mut self, model_data: &[u8], metadata: ModelMetadata) -> Result<()> {
        info!("Loading model: {:?}", metadata.id);
        self.validate_model_data(model_data, &metadata)?;
        self.models.insert(metadata.id.clone(), metadata);
        info!("Model loaded. Total models: {}", self.models.len());
        Ok(())
    }

    /// Execute a model with deterministic semantics
    pub async fn execute(
        &mut self,
        context: ExecutionContext,
        input: ModelInput,
    ) -> Result<ExecutionResult> {
        info!("Executing model: {:?}", context.model_id);

        let model_metadata = self
            .models
            .get(&context.model_id)
            .ok_or_else(|| AiCoreError::ExecutionFailed("Model not found".into()))?
            .clone();

        self.validate_input(&input, &model_metadata)?;

        let start_time = std::time::Instant::now();
        let output = self
            .execute_model_deterministic(&model_metadata, &input, &context)
            .await?;
        let exec_time = start_time.elapsed();

        self.update_stats(exec_time.as_micros() as u64, true);

        let result = ExecutionResult {
            context,
            success: true,
            error: None,
            metadata: HashMap::new(),
            execution_time_us: exec_time.as_micros() as u64,
            memory_usage: model_metadata.size_bytes,
            data_type: output.dtype,
            output,
        };

        info!("✅ Model execution completed successfully");
        Ok(result)
    }

    /// Validate model data integrity and hash
    fn validate_model_data(&self, data: &[u8], metadata: &ModelMetadata) -> Result<()> {
        if data.len() as u64 != metadata.size_bytes {
            return Err(AiCoreError::ValidationFailed("Model size mismatch".into()));
        }

        let hash = blake3::hash(data).to_hex().to_string();
        if hash != metadata.id.hash {
            return Err(AiCoreError::ValidationFailed("Model hash mismatch".into()));
        }
        Ok(())
    }

    /// Validate model input before execution
    fn validate_input(&self, input: &ModelInput, metadata: &ModelMetadata) -> Result<()> {
        if input.shape != metadata.input_shape {
            return Err(AiCoreError::InvalidParameters(format!(
                "Input shape mismatch: expected {:?}, got {:?}",
                metadata.input_shape, input.shape
            )));
        }

        let expected = input.shape.iter().product::<usize>() * input.dtype.size_bytes();
        if input.data.len() != expected {
            return Err(AiCoreError::InvalidParameters(
                "Input data size mismatch".into(),
            ));
        }
        Ok(())
    }

    /// Deterministic model execution (e.g., D-GBDT)
    async fn execute_model_deterministic(
        &self,
        metadata: &ModelMetadata,
        input: &ModelInput,
        context: &ExecutionContext,
    ) -> Result<ModelOutput> {
        let start_time = std::time::Instant::now();

        if let Some(seed) = context.seed {
            info!("Using deterministic seed: {}", seed);
            std::env::set_var("AI_DETERMINISTIC_SEED", seed.to_string());
        }

        info!(
            "Executing model {:?}, input size {} bytes",
            metadata.id,
            input.data.len()
        );

        let output_size =
            metadata.output_shape.iter().product::<usize>() * input.dtype.size_bytes();
        let mut output_data = vec![0u8; output_size];

        // Deterministic placeholder logic for GBDT or generic models
        if metadata.architecture == "gbdt" {
            info!("Executing GBDT model deterministically");
            let features = self.convert_input_to_features(input)?;
            for (i, byte) in output_data.iter_mut().enumerate() {
                *byte = ((features.iter().sum::<i64>() as usize + i) % 256) as u8;
            }
        } else {
            info!("Executing generic model: {}", metadata.architecture);
            let mut sum = 0u64;
            for chunk in input.data.chunks(8) {
                let mut arr = [0u8; 8];
                for (i, &b) in chunk.iter().enumerate() {
                    if i < 8 {
                        arr[i] = b;
                    }
                }
                sum = sum.wrapping_add(u64::from_le_bytes(arr));
            }
            for (i, b) in output_data.iter_mut().enumerate() {
                *b = ((sum as usize + i) % 256) as u8;
            }
        }

        let exec_time = start_time.elapsed();
        let execution_hash = self.compute_execution_hash(metadata, input, context)?;

        let metadata_block = ExecutionMetadata {
            execution_id: context.id.clone(),
            model_id: metadata.id.to_string(),
            start_time: 0,
            end_time: 0,
            duration_us: exec_time.as_micros() as u64,
            memory_usage: metadata.size_bytes + input.data.len() as u64,
            #[cfg(feature = "deterministic_math")]
            cpu_usage: crate::fixed::Fixed::ZERO,
            #[cfg(not(feature = "deterministic_math"))]
            cpu_usage: 0.0,
            success: true,
            error: None,
            metadata: {
                let mut m = HashMap::new();
                m.insert("execution_hash".into(), execution_hash);
                m.insert("model_version".into(), metadata.id.version.clone());
                m.insert(
                    "cpu_cycles".into(),
                    self.estimate_cpu_cycles(exec_time).to_string(),
                );
                m
            },
        };

        Ok(ModelOutput {
            data: output_data,
            dtype: input.dtype,
            shape: metadata.output_shape.clone(),
            #[cfg(feature = "deterministic_math")]
            confidence: crate::fixed::Fixed::ONE,
            #[cfg(not(feature = "deterministic_math"))]
            confidence: 1.0,
            metadata: metadata_block,
        })
    }

    /// Convert binary input to numeric feature vector
    fn convert_input_to_features(&self, input: &ModelInput) -> Result<Vec<i64>> {
        let mut features = Vec::new();
        match input.dtype {
            DataType::Int64 => {
                for c in input.data.chunks(8) {
                    if c.len() == 8 {
                        let mut arr = [0u8; 8];
                        arr.copy_from_slice(c);
                        features.push(i64::from_le_bytes(arr));
                    }
                }
            }
            DataType::Int32 => {
                for c in input.data.chunks(4) {
                    if c.len() == 4 {
                        let mut arr = [0u8; 4];
                        arr.copy_from_slice(c);
                        features.push(i32::from_le_bytes(arr) as i64);
                    }
                }
            }
            DataType::Float32 => {
                for c in input.data.chunks(4) {
                    if c.len() == 4 {
                        let mut arr = [0u8; 4];
                        arr.copy_from_slice(c);
                        let f = f32::from_le_bytes(arr);
                        features.push((f * 10_000.0) as i64);
                    }
                }
            }
            _ => {
                return Err(AiCoreError::InvalidParameters(
                    "Unsupported data type for GBDT features".into(),
                ));
            }
        }
        Ok(features)
    }

    /// Estimate CPU cycles from duration
    fn estimate_cpu_cycles(&self, d: std::time::Duration) -> u64 {
        const FREQ: u64 = 3_000_000_000; // 3 GHz
        (d.as_secs_f64() * FREQ as f64) as u64
    }

    /// Compute deterministic execution hash
    fn compute_execution_hash(
        &self,
        metadata: &ModelMetadata,
        input: &ModelInput,
        context: &ExecutionContext,
    ) -> Result<String> {
        let mut h = blake3::Hasher::new();
        h.update(metadata.id.hash.as_bytes());
        h.update(metadata.id.version.as_bytes());
        h.update(&input.data);
        h.update(
            &input
                .shape
                .iter()
                .flat_map(|x| x.to_le_bytes())
                .collect::<Vec<_>>(),
        );
        h.update(&context.seed.unwrap_or(0).to_le_bytes());
        for (k, v) in &context.parameters {
            h.update(k.as_bytes());
            h.update(v.as_bytes());
        }
        Ok(h.finalize().to_hex().to_string())
    }

    /// Update performance stats
    fn update_stats(&mut self, exec_time_us: u64, success: bool) {
        self.stats.total_executions += 1;
        if success {
            self.stats.successful_executions += 1;
        } else {
            self.stats.failed_executions += 1;
        }

        let n = self.stats.total_executions;
        if n == 0 {
            self.stats.avg_execution_time_us = exec_time_us;
        } else {
            self.stats.avg_execution_time_us =
                (self.stats.avg_execution_time_us * (n - 1) + exec_time_us) / n;
        }
    }

    pub fn get_stats(&self) -> &ExecutionStats {
        &self.stats
    }

    pub fn model_count(&self) -> usize {
        self.models.len()
    }
}
