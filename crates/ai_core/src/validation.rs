//! Model validation and verification
//!
//! Provides deterministic validation for model integrity, format correctness,
//! and execution determinism within the IPPAN/FinDAG AI core.

use crate::{errors::AiCoreError, types::*};
use std::collections::HashMap;
use tracing::{info, warn};

/// Model validator for ensuring model integrity and correctness
pub struct ModelValidator {
    /// Validation rules
    rules: Vec<ValidationRule>,
    /// Validation statistics
    stats: ValidationStats,
}

/// Validation rule set
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
    pub total_validations: u64,
    pub successful_validations: u64,
    pub failed_validations: u64,
    pub error_counts: HashMap<String, u64>,
}

/// Validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub passed: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
    pub metadata: ValidationMetadata,
}

/// Validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub error_type: String,
    pub message: String,
    pub severity: ErrorSeverity,
}

/// Validation warning
#[derive(Debug, Clone)]
pub struct ValidationWarning {
    pub warning_type: String,
    pub message: String,
}

/// Error severity levels
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorSeverity {
    Critical,
    High,
    Medium,
    Low,
}

/// Validation metadata
#[derive(Debug, Clone)]
pub struct ValidationMetadata {
    pub timestamp: u64,
    pub duration_us: u64,
    pub validator_version: String,
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

    /// Validate a model’s integrity and metadata
    pub async fn validate_model(
        &mut self,
        model_data: &[u8],
        metadata: &ModelMetadata,
    ) -> Result<ValidationResult, AiCoreError> {
        info!("Validating model: {:?}", metadata.id);

        let start = std::time::Instant::now();
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

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

        let duration = start.elapsed();
        let passed =
            errors.is_empty() || errors.iter().all(|e| e.severity != ErrorSeverity::Critical);

        self.update_stats(passed, &errors);

        Ok(ValidationResult {
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
        })
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
                error_type: "HashIntegrity".into(),
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
                error_type: "ShapeValidation".into(),
                message: "Input shape cannot be empty".into(),
                severity: ErrorSeverity::Critical,
            });
        }
        if metadata.output_shape.is_empty() {
            return Err(ValidationError {
                error_type: "ShapeValidation".into(),
                message: "Output shape cannot be empty".into(),
                severity: ErrorSeverity::Critical,
            });
        }

        for (i, &dim) in metadata.input_shape.iter().enumerate() {
            if dim == 0 {
                return Err(ValidationError {
                    error_type: "ShapeValidation".into(),
                    message: format!("Input dimension {} cannot be zero", i),
                    severity: ErrorSeverity::High,
                });
            }
        }
        for (i, &dim) in metadata.output_shape.iter().enumerate() {
            if dim == 0 {
                return Err(ValidationError {
                    error_type: "ShapeValidation".into(),
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
                error_type: "ParameterBounds".into(),
                message: "Parameter count cannot be zero".into(),
                severity: ErrorSeverity::Critical,
            });
        }

        if metadata.parameter_count > 1_000_000_000 {
            return Err(ValidationError {
                error_type: "ParameterBounds".into(),
                message: "Parameter count too high; possible overflow".into(),
                severity: ErrorSeverity::High,
            });
        }

        if metadata.size_bytes == 0 {
            return Err(ValidationError {
                error_type: "ParameterBounds".into(),
                message: "Model size cannot be zero".into(),
                severity: ErrorSeverity::Critical,
            });
        }

        if metadata.size_bytes > 10_000_000_000 {
            return Err(ValidationError {
                error_type: "ParameterBounds".into(),
                message: "Model size extremely large, may affect performance".into(),
                severity: ErrorSeverity::Medium,
            });
        }
        Ok(())
    }

    /// Validate deterministic behavior
    async fn validate_determinism(
        &self,
        model_data: &[u8],
        metadata: &ModelMetadata,
    ) -> Result<(), ValidationError> {
        info!("Validating model determinism...");

        if metadata.architecture == "gbdt" {
            if let Ok(model_str) = std::str::from_utf8(model_data) {
                if model_str.contains("random") || model_str.contains("stochastic") {
                    return Err(ValidationError {
                        error_type: "DeterminismCheck".into(),
                        message: "Model contains non-deterministic components".into(),
                        severity: ErrorSeverity::High,
                    });
                }
            }
        }

        if metadata.size_bytes > 10_000_000_000 {
            warn!("Large model may impact deterministic validation speed");
        }

        // Verify deterministic hash presence
        if metadata.id.hash.is_empty() {
            return Err(ValidationError {
                error_type: "DeterminismCheck".into(),
                message: "Missing model hash for determinism verification".into(),
                severity: ErrorSeverity::Critical,
            });
        }

        info!("Determinism validation passed");
        Ok(())
    }

    /// Validate model format
    fn validate_format(
        &self,
        model_data: &[u8],
        _metadata: &ModelMetadata,
    ) -> Result<(), ValidationError> {
        if model_data.len() < 1024 {
            return Err(ValidationError {
                error_type: "FormatValidation".into(),
                message: "Model file too small; possible corruption".into(),
                severity: ErrorSeverity::High,
            });
        }

        if model_data.first() == Some(&0) {
            return Err(ValidationError {
                error_type: "FormatValidation".into(),
                message: "Model data begins with null byte(s); likely corrupted".into(),
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

        for err in errors {
            *self.stats.error_counts.entry(err.error_type.clone()).or_insert(0) += 1;
        }
    }

    /// Get validation stats
    pub fn get_stats(&self) -> &ValidationStats {
        &self.stats
    }
}
