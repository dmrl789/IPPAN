//! Type definitions for the AI Core module

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Model identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelId {
    /// Model name
    pub name: String,
    /// Model version
    pub version: String,
    /// Model hash for integrity verification
    pub hash: String,
}

/// Model metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    /// Model identifier
    pub id: ModelId,
    /// Model architecture
    pub architecture: String,
    /// Input shape
    pub input_shape: Vec<usize>,
    /// Output shape
    pub output_shape: Vec<usize>,
    /// Model parameters count
    pub parameter_count: u64,
    /// Model size in bytes
    pub size_bytes: u64,
    /// Creation timestamp
    pub created_at: u64,
    /// Model description
    pub description: Option<String>,
}

/// Model input data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInput {
    /// Input data as raw bytes
    pub data: Vec<u8>,
    /// Input shape
    pub shape: Vec<usize>,
    /// Data type
    pub dtype: DataType,
}

/// Model output data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelOutput {
    /// Output data as raw bytes
    pub data: Vec<u8>,
    /// Output shape
    pub shape: Vec<usize>,
    /// Data type
    pub dtype: DataType,
    /// Execution metadata
    pub metadata: ExecutionMetadata,
}

/// Data type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataType {
    /// 32-bit floating point
    Float32,
    /// 64-bit floating point
    Float64,
    /// 8-bit integer
    Int8,
    /// 16-bit integer
    Int16,
    /// 32-bit integer
    Int32,
    /// 64-bit integer
    Int64,
    /// Unsigned 8-bit integer
    UInt8,
    /// Unsigned 16-bit integer
    UInt16,
    /// Unsigned 32-bit integer
    UInt32,
    /// Unsigned 64-bit integer
    UInt64,
}

/// Execution metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetadata {
    /// Execution time in microseconds
    pub execution_time_us: u64,
    /// Memory usage in bytes
    pub memory_usage_bytes: u64,
    /// CPU cycles used
    pub cpu_cycles: u64,
    /// Deterministic execution hash
    pub execution_hash: String,
    /// Model version used
    pub model_version: String,
}

/// Model execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// Model identifier
    pub model_id: ModelId,
    /// Execution parameters
    pub parameters: HashMap<String, String>,
    /// Deterministic seed
    pub seed: Option<u64>,
    /// Execution timeout in seconds
    pub timeout_seconds: Option<u64>,
}

/// Model execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Model output
    pub output: ModelOutput,
    /// Execution context
    pub context: ExecutionContext,
    /// Success flag
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}