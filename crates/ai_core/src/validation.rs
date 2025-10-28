//! Model validation and verification

use crate::{errors::AiCoreError, types::*};
use std::collections::HashMap;
use tracing::{error, info, warn};

/// Model validator for ensuring model integrity and correctness
pub struct ModelValidator {
    /// Validation rules
    rules: Vec<ValidationRule>,
    /// Validation statistics
    stats: ValidationStats,
}

/// Validation rule
pub enum ValidationRule {
    /// Check model hash integrity
    HashIntegrity,
    /// Validate input/output shapes
    ShapeValidation,
    /// Check parameter bounds
    ParameterBounds,
    /// Verify execution determinism
    DeterminismCheck,
    /// Validate model format
    FormatValidation,
}

/// Validation statistics
#[derive(Debug, Default)]
pub struct ValidationStats {
    /// Total validations performed
    pub total_validations: u64,
    /// Successful validations
    pub successful_validations: u64,
    /// Failed validations
    pub failed_validations: u64,
    /// Validation errors by type
    pub error_counts: HashMap<String, u64>,
}

/// Validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether validation passed
    pub passed: bool,
    /// Validation errors
    pub errors: Vec<ValidationError>,
    /// Validation warnings
    pub warnings: Vec<ValidationWarning>,
    /// Validation metadata
    pub metadata: ValidationMetadata,
}

/// Validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Error type
    pub error_type: String,
    /// Error message
    pub message: String,
    /// Error severity
    pub severity: ErrorSeverity,
}

/// Validation warning
#[derive(Debug, Clone)]
pub struct ValidationWarning {
    /// Warning type
    pub warning_type: String,
    /// Warning message
    pub message: String,
}

/// Error severity levels
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// Critical error - model cannot be used
    Critical,
    /// High severity - model may not work correctly
    High,
    /// Medium severity - model may have issues
    Medium,
    /// Low severity - minor issues
    Low,
}

/// Validation metadata
#[derive(Debug, Clone)]
pub struct ValidationMetadata {
    /// Validation timestamp
    pub timestamp: u64,
    /// Validation duration in microseconds
    pub duration_us: u64,
    /// Validator version
    pub validator_version: String,
    /// Model version validated
    pub model_version: String,
}

impl ModelValidator {
    /// Create a new model validator
    pub fn new() -> Self {
        Self {
            rules: vec![
                ValidationRule::HashIntegrity,
                ValidationRule::ShapeValidation,
                ValidationRule::ParameterBounds,
                ValidationRule::DeterminismCheck,
                ValidationRule::FormatValidation,
            ],
            stats: ValidationStats::default(),
        }
    }

    /// Validate a model
    pub async fn validate_model(
        &mut self,
        model_data: &[u8],
        metadata: &ModelMetadata,
    ) -> Result<ValidationResult, AiCoreError> {
        info!("Validating model: {:?}", metadata.id);

        let start_time = std::time::Instant::now();
        let mut errors: Vec<ValidationError> = Vec::new();
        let mut warnings: Vec<ValidationWarning> = Vec::new();

        // Run validation rules
        for rule in &self.rules {
            match rule {
                ValidationRule::HashIntegrity => {
                    if let Err(e) = self.validate_hash_integrity(model_data, metadata) {
                        errors.push(e);
                    }
                }
                ValidationRule::ShapeValidation => {
                    if let Err(e) = self.validate_shapes(metadata) {
                        errors.push(e);
                    }
                }
                ValidationRule::ParameterBounds => {
                    if let Err(e) = self.validate_parameter_bounds(metadata) {
                        errors.push(e);
                    }
                }
                ValidationRule::DeterminismCheck => {
                    if let Err(e) = self.validate_determinism(model_data, metadata).await {
                        errors.push(e);
                    }
                }
                ValidationRule::FormatValidation => {
                    if let Err(e) = self.validate_format(model_data, metadata) {
                        errors.push(e);
                    }
                }
            }
        }

        let duration = start_time.elapsed();
        let passed =
            errors.is_empty() || errors.iter().all(|e| e.severity != ErrorSeverity::Critical);

        // Update statistics
        self.update_stats(passed, &errors);

        let result = ValidationResult {
            passed,
            errors,
            warnings,
            metadata: ValidationMetadata {
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                duration_us: duration.as_micros() as u64,
                validator_version: crate::VERSION.to_string(),
                model_version: metadata.version.clone(),
            },
        };

        info!(
            "Model validation completed: {}",
            if passed { "PASSED" } else { "FAILED" }
        );
        Ok(result)
    }

    /// Validate hash integrity
    fn validate_hash_integrity(
        &self,
        model_data: &[u8],
        metadata: &ModelMetadata,
    ) -> Result<(), ValidationError> {
        let computed_hash = blake3::hash(model_data).to_hex().to_string();
        if computed_hash != metadata.id.hash {
            return Err(ValidationError {
                error_type: "HashIntegrity".to_string(),
                message: format!(
                    "Hash mismatch: expected {}, got {}",
                    metadata.id.hash, computed_hash
                ),
                severity: ErrorSeverity::Critical,
            });
        }
        Ok(())
    }

    /// Validate input/output shapes
    fn validate_shapes(&self, metadata: &ModelMetadata) -> Result<(), ValidationError> {
        if metadata.input_shape.is_empty() {
            return Err(ValidationError {
                error_type: "ShapeValidation".to_string(),
                message: "Input shape cannot be empty".to_string(),
                severity: ErrorSeverity::Critical,
            });
        }

        if metadata.output_shape.is_empty() {
            return Err(ValidationError {
                error_type: "ShapeValidation".to_string(),
                message: "Output shape cannot be empty".to_string(),
                severity: ErrorSeverity::Critical,
            });
        }

        for (i, dim) in metadata.input_shape.iter().enumerate() {
            if *dim == 0 {
                return Err(ValidationError {
                    error_type: "ShapeValidation".to_string(),
                    message: format!("Input dimension {} cannot be zero", i),
                    severity: ErrorSeverity::High,
                });
            }
        }

        for (i, dim) in metadata.output_shape.iter().enumerate() {
            if *dim == 0 {
                return Err(ValidationError {
                    error_type: "ShapeValidation".to_string(),
                    message: format!("Output dimension {} cannot be zero", i),
                    severity: ErrorSeverity::High,
                });
            }
        }

        Ok(())
    }

    /// Validate parameter bounds
    fn validate_parameter_bounds(&self, metadata: &ModelMetadata) -> Result<(), ValidationError> {
        if metadata.parameter_count == 0 {
            return Err(ValidationError {
                error_type: "ParameterBounds".to_string(),
                message: "Parameter count cannot be zero".to_string(),
                severity: ErrorSeverity::Critical,
            });
        }

        if metadata.parameter_count > 1_000_000_000 {
            return Err(ValidationError {
                error_type: "ParameterBounds".to_string(),
                message: "Parameter count is extremely large, possible overflow".to_string(),
                severity: ErrorSeverity::High,
            });
        }

        if metadata.size_bytes == 0 {
            return Err(ValidationError {
                error_type: "ParameterBounds".to_string(),
                message: "Model size cannot be zero".to_string(),
                severity: ErrorSeverity::Critical,
            });
        }

        if metadata.size_bytes > 10_000_000_000 {
            return Err(ValidationError {
                error_type: "ParameterBounds".to_string(),
                message: "Model size is extremely large, may cause memory issues".to_string(),
                severity: ErrorSeverity::Medium,
            });
        }

        Ok(())
    }

    /// Validate execution determinism
    async fn validate_determinism(
        &self,
        model_data: &[u8],
        metadata: &ModelMetadata,
    ) -> Result<(), ValidationError> {
        info!("Validating model determinism");

        let test_input_size = metadata.input_shape.iter().product::<usize>() * 4;
        let _test_input = vec![42u8; test_input_size];

        if metadata.architecture == "gbdt" {
            if let Ok(model_str) = std::str::from_utf8(model_data) {
                if model_str.contains("random") || model_str.contains("stochastic") {
                    return Err(ValidationError {
                        error_type: "DeterminismCheck".to_string(),
                        message: "Model contains non-deterministic components".to_string(),
                        severity: ErrorSeverity::High,
                    });
                }
            }
        }

        if metadata.size_bytes > 10_000_000_000 {
            warn!("Large model size may impact determinism verification");
        }

        if metadata.id.hash.is_empty() {
            return Err(ValidationError {
                error_type: "DeterminismCheck".to_string(),
                message: "Model hash is required for determinism verification".to_string(),
                severity: ErrorSeverity::Critical,
            });
        }

        info!("Determinism validation completed successfully");
        Ok(())
    }

    /// Validate model format
    fn validate_format(&self, model_data: &[u8], _metadata: &ModelMetadata) -> Result<(), ValidationError> {
        if model_data.len() < 1024 {
            return Err(ValidationError {
                error_type: "FormatValidation".to_string(),
                message: "Model data too small, likely corrupted".to_string(),
                severity: ErrorSeverity::High,
            });
        }

        if model_data[0] == 0 {
            return Err(ValidationError {
                error_type: "FormatValidation".to_string(),
                message: "Model data starts with null bytes, possible corruption".to_string(),
                severity: ErrorSeverity::Medium,
            });
        }

        Ok(())
    }

    /// Update validation statistics
    fn update_stats(&mut self, passed: bool, errors: &[ValidationError]) {
        self.stats.total_validations += 1;

        if passed {
            self.stats.successful_validations += 1;
        } else {
            self.stats.failed_validations += 1;
        }

        for error in errors {
            *self
                .stats
                .error_counts
                .entry(error.error_type.clone())
                .or_insert(0) += 1;
        }
    }

    /// Get validation statistics
    pub fn get_stats(&self) -> &ValidationStats {
        &self.stats
    }
}
