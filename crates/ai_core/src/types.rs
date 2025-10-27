//! Common types for AI Core

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Model identifier
pub type ModelId = String;

/// Model metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    /// Model name
    pub name: String,
    /// Model version
    pub version: String,
    /// Model description
    pub description: String,
    /// Model author
    pub author: String,
    /// Model license
    pub license: String,
    /// Model tags
    pub tags: Vec<String>,
    /// Model creation time
    pub created_at: u64,
    /// Model last updated time
    pub updated_at: u64,
}

/// Model input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInput {
    /// Input data
    pub data: Vec<u8>,
    /// Input shape
    pub shape: Vec<usize>,
    /// Input data type
    pub dtype: DataType,
    /// Input metadata
    pub metadata: HashMap<String, String>,
}

/// Model output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelOutput {
    /// Output data
    pub data: Vec<u8>,
    /// Output shape
    pub shape: Vec<usize>,
    /// Output data type
    pub dtype: DataType,
    /// Output metadata
    pub metadata: HashMap<String, String>,
    /// Confidence score
    pub confidence: f64,
}

/// Data type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataType {
    /// Text data
    Text,
    /// Binary data
    Binary,
    /// JSON data
    Json,
    /// Numeric data
    Numeric,
    /// Image data
    Image,
    /// Audio data
    Audio,
    /// Video data
    Video,
    /// 64-bit integer
    Int64,
    /// 32-bit integer
    Int32,
    /// 16-bit integer
    Int16,
    /// 8-bit integer
    Int8,
    /// 32-bit float
    Float32,
    /// 64-bit float
    Float64,
}

impl DataType {
    /// Get the size in bytes for this data type
    pub fn size_bytes(&self) -> usize {
        match self {
            DataType::Text => 1, // Variable length, assume 1 byte per character
            DataType::Binary => 1,
            DataType::Json => 1,
            DataType::Numeric => 8, // Default to 64-bit
            DataType::Image => 1,
            DataType::Audio => 1,
            DataType::Video => 1,
            DataType::Int64 => 8,
            DataType::Int32 => 4,
            DataType::Int16 => 2,
            DataType::Int8 => 1,
            DataType::Float32 => 4,
            DataType::Float64 => 8,
        }
    }
}

/// Execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// Context ID
    pub id: String,
    /// Context metadata
    pub metadata: HashMap<String, String>,
    /// Execution parameters
    pub parameters: HashMap<String, String>,
    /// Execution timeout
    pub timeout_ms: u64,
}

/// Execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Result data
    pub data: Vec<u8>,
    /// Result type
    pub data_type: DataType,
    /// Execution time (microseconds)
    pub execution_time_us: u64,
    /// Memory usage (bytes)
    pub memory_usage: u64,
    /// Success flag
    pub success: bool,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Result metadata
    pub metadata: HashMap<String, String>,
}

/// Execution metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetadata {
    /// Execution ID
    pub id: String,
    /// Model ID
    pub model_id: String,
    /// Execution start time
    pub start_time: u64,
    /// Execution end time
    pub end_time: u64,
    /// Execution duration (microseconds)
    pub duration_us: u64,
    /// Memory usage (bytes)
    pub memory_usage: u64,
    /// CPU usage percentage
    pub cpu_usage: f64,
    /// Success flag
    pub success: bool,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}
