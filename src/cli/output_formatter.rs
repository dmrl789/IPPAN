//! Output formatter for IPPAN CLI
//! 
//! Implements output formatting for different output types including
//! tables, JSON, YAML, CSV, and plain text.

use crate::{Result, IppanError, TransactionHash};
use super::OutputFormat;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};
use tracing::{info, warn, error, debug};

/// Output formatter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputFormatterConfig {
    /// Enable colored output
    pub enable_colored_output: bool,
    /// Default output format
    pub default_output_format: OutputFormat,
    /// Table column width
    pub table_column_width: usize,
    /// Table border style
    pub table_border_style: TableBorderStyle,
    /// JSON pretty print
    pub json_pretty_print: bool,
    /// YAML pretty print
    pub yaml_pretty_print: bool,
    /// CSV delimiter
    pub csv_delimiter: char,
    /// CSV quote character
    pub csv_quote_character: char,
    /// Enable output caching
    pub enable_output_caching: bool,
    /// Output cache size
    pub output_cache_size: usize,
}

impl Default for OutputFormatterConfig {
    fn default() -> Self {
        Self {
            enable_colored_output: true,
            default_output_format: OutputFormat::Table,
            table_column_width: 20,
            table_border_style: TableBorderStyle::Simple,
            json_pretty_print: true,
            yaml_pretty_print: true,
            csv_delimiter: ',',
            csv_quote_character: '"',
            enable_output_caching: true,
            output_cache_size: 1000,
        }
    }
}

/// Table border style
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TableBorderStyle {
    /// Simple border
    Simple,
    /// Double border
    Double,
    /// No border
    None,
    /// Custom border
    Custom,
}

/// Output formatter
pub struct OutputFormatter {
    /// Configuration
    config: OutputFormatterConfig,
    /// Output cache
    output_cache: Arc<RwLock<HashMap<String, String>>>,
    /// Statistics
    stats: Arc<RwLock<OutputFormatterStats>>,
    /// Start time
    start_time: Instant,
}

/// Output formatter statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputFormatterStats {
    /// Total outputs formatted
    pub total_outputs_formatted: u64,
    /// Successful formats
    pub successful_formats: u64,
    /// Failed formats
    pub failed_formats: u64,
    /// Average format time in milliseconds
    pub average_format_time_ms: f64,
    /// Most used formats
    pub most_used_formats: HashMap<String, u64>,
    /// Format success rate
    pub format_success_rate: f64,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Last format timestamp
    pub last_format_timestamp: Option<u64>,
}

impl Default for OutputFormatterStats {
    fn default() -> Self {
        Self {
            total_outputs_formatted: 0,
            successful_formats: 0,
            failed_formats: 0,
            average_format_time_ms: 0.0,
            most_used_formats: HashMap::new(),
            format_success_rate: 0.0,
            uptime_seconds: 0,
            last_format_timestamp: None,
        }
    }
}

impl OutputFormatter {
    /// Create a new output formatter
    pub fn new(config: OutputFormatterConfig) -> Self {
        Self {
            config,
            output_cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(OutputFormatterStats::default())),
            start_time: Instant::now(),
        }
    }
    
    /// Format output
    pub async fn format_output(&self, data: &serde_json::Value, format: OutputFormat) -> Result<String> {
        let start_time = Instant::now();
        
        info!("Formatting output in {:?} format", format);
        
        // Check cache if enabled
        if self.config.enable_output_caching {
            let cache_key = self.generate_cache_key(data, &format);
            let cached_output = {
                let cache = self.output_cache.read().await;
                cache.get(&cache_key).cloned()
            };
            
            if let Some(cached) = cached_output {
                info!("Output retrieved from cache");
                return Ok(cached);
            }
        }
        
        // Format output
        let formatted_output = match format {
            OutputFormat::Table => self.format_as_table(data).await?,
            OutputFormat::Json => self.format_as_json(data).await?,
            OutputFormat::Yaml => self.format_as_yaml(data).await?,
            OutputFormat::Csv => self.format_as_csv(data).await?,
            OutputFormat::Plain => self.format_as_plain(data).await?,
            OutputFormat::Pretty => self.format_as_pretty(data).await?,
        };
        
        // Cache output if enabled
        if self.config.enable_output_caching {
            let cache_key = self.generate_cache_key(data, &format);
            let mut cache = self.output_cache.write().await;
            cache.insert(cache_key, formatted_output.clone());
            
            // Trim cache if it exceeds size limit
            if cache.len() > self.config.output_cache_size {
                let keys_to_remove: Vec<String> = cache.keys().take(cache.len() - self.config.output_cache_size).cloned().collect();
                for key in keys_to_remove {
                    cache.remove(&key);
                }
            }
        }
        
        let format_time = start_time.elapsed().as_millis() as u64;
        
        // Update statistics
        self.update_statistics(&format, format_time, true).await;
        
        info!("Output formatted successfully in {}ms", format_time);
        Ok(formatted_output)
    }
    
    /// Format as table
    async fn format_as_table(&self, data: &serde_json::Value) -> Result<String> {
        let mut output = String::new();
        
        if let Some(array) = data.as_array() {
            if array.is_empty() {
                return Ok("No data to display".to_string());
            }
            
            // Get headers from first object
            if let Some(first_obj) = array[0].as_object() {
                let headers: Vec<&String> = first_obj.keys().collect();
                
                // Calculate column widths
                let mut column_widths = Vec::new();
                for header in &headers {
                    let mut max_width = header.len();
                    for item in array {
                        if let Some(obj) = item.as_object() {
                            if let Some(value) = obj.get(*header) {
                                let value_str = self.value_to_string(value);
                                max_width = max_width.max(value_str.len());
                            }
                        }
                    }
                    column_widths.push(max_width.min(self.config.table_column_width));
                }
                
                // Print table header
                if self.config.table_border_style != TableBorderStyle::None {
                    output.push_str(&self.create_table_border(&column_widths));
                }
                
                // Print headers
                output.push_str("|");
                for (i, header) in headers.iter().enumerate() {
                    let padded_header = self.pad_string(header, column_widths[i]);
                    output.push_str(&format!(" {} |", padded_header));
                }
                output.push('\n');
                
                if self.config.table_border_style != TableBorderStyle::None {
                    output.push_str(&self.create_table_border(&column_widths));
                }
                
                // Print data rows
                for item in array {
                    if let Some(obj) = item.as_object() {
                        output.push_str("|");
                        for (i, header) in headers.iter().enumerate() {
                            let value = obj.get(*header).map(|v| self.value_to_string(v)).unwrap_or_default();
                            let padded_value = self.pad_string(&value, column_widths[i]);
                            output.push_str(&format!(" {} |", padded_value));
                        }
                        output.push('\n');
                    }
                }
                
                if self.config.table_border_style != TableBorderStyle::None {
                    output.push_str(&self.create_table_border(&column_widths));
                }
            }
        } else if let Some(obj) = data.as_object() {
            // Format single object as key-value table
            for (key, value) in obj {
                let value_str = self.value_to_string(value);
                output.push_str(&format!("{:<20} : {}\n", key, value_str));
            }
        } else {
            // Format primitive value
            output.push_str(&self.value_to_string(data));
        }
        
        Ok(output)
    }
    
    /// Format as JSON
    async fn format_as_json(&self, data: &serde_json::Value) -> Result<String> {
        if self.config.json_pretty_print {
            serde_json::to_string_pretty(data)
                .map_err(|e| IppanError::CLI(format!("Failed to format as JSON: {}", e)))
        } else {
            serde_json::to_string(data)
                .map_err(|e| IppanError::CLI(format!("Failed to format as JSON: {}", e)))
        }
    }
    
    /// Format as YAML
    async fn format_as_yaml(&self, data: &serde_json::Value) -> Result<String> {
        serde_yaml::to_string(data)
            .map_err(|e| IppanError::CLI(format!("Failed to format as YAML: {}", e)))
    }
    
    /// Format as CSV
    async fn format_as_csv(&self, data: &serde_json::Value) -> Result<String> {
        let mut output = String::new();
        
        if let Some(array) = data.as_array() {
            if array.is_empty() {
                return Ok(String::new());
            }
            
            // Get headers from first object
            if let Some(first_obj) = array[0].as_object() {
                let headers: Vec<&String> = first_obj.keys().collect();
                
                // Print headers
                for (i, header) in headers.iter().enumerate() {
                    if i > 0 {
                        output.push(self.config.csv_delimiter);
                    }
                    output.push(self.config.csv_quote_character);
                    output.push_str(header);
                    output.push(self.config.csv_quote_character);
                }
                output.push('\n');
                
                // Print data rows
                for item in array {
                    if let Some(obj) = item.as_object() {
                        for (i, header) in headers.iter().enumerate() {
                            if i > 0 {
                                output.push(self.config.csv_delimiter);
                            }
                            let value = obj.get(*header).map(|v| self.value_to_string(v)).unwrap_or_default();
                            output.push(self.config.csv_quote_character);
                            output.push_str(&value);
                            output.push(self.config.csv_quote_character);
                        }
                        output.push('\n');
                    }
                }
            }
        } else {
            return Err(IppanError::CLI("CSV format requires array data".to_string()));
        }
        
        Ok(output)
    }
    
    /// Format as plain text
    async fn format_as_plain(&self, data: &serde_json::Value) -> Result<String> {
        Ok(self.value_to_string(data))
    }
    
    /// Format as pretty text
    async fn format_as_pretty(&self, data: &serde_json::Value) -> Result<String> {
        if let Some(obj) = data.as_object() {
            let mut output = String::new();
            for (key, value) in obj {
                let value_str = self.value_to_string(value);
                if self.config.enable_colored_output {
                    output.push_str(&format!("\x1b[36m{}:\x1b[0m {}\n", key, value_str));
                } else {
                    output.push_str(&format!("{}: {}\n", key, value_str));
                }
            }
            Ok(output)
        } else {
            Ok(self.value_to_string(data))
        }
    }
    
    /// Convert value to string
    fn value_to_string(&self, value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::Null => "null".to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| self.value_to_string(v)).collect();
                format!("[{}]", items.join(", "))
            }
            serde_json::Value::Object(obj) => {
                let pairs: Vec<String> = obj.iter()
                    .map(|(k, v)| format!("{}: {}", k, self.value_to_string(v)))
                    .collect();
                format!("{{{}}}", pairs.join(", "))
            }
        }
    }
    
    /// Pad string to specified width
    fn pad_string(&self, s: &str, width: usize) -> String {
        if s.len() > width {
            format!("{}...", &s[..width.saturating_sub(3)])
        } else {
            format!("{:<width$}", s, width = width)
        }
    }
    
    /// Create table border
    fn create_table_border(&self, column_widths: &[usize]) -> String {
        match self.config.table_border_style {
            TableBorderStyle::Simple => {
                let mut border = String::from("+");
                for width in column_widths {
                    border.push_str(&format!("-{}-+", "-".repeat(*width)));
                }
                border.push('\n');
                border
            }
            TableBorderStyle::Double => {
                let mut border = String::from("+");
                for width in column_widths {
                    border.push_str(&format!("={}=+", "=".repeat(*width)));
                }
                border.push('\n');
                border
            }
            TableBorderStyle::None => String::new(),
            TableBorderStyle::Custom => {
                // Custom border implementation
                let mut border = String::from("┌");
                for width in column_widths {
                    border.push_str(&format!("─{}─┬", "─".repeat(*width)));
                }
                border.pop(); // Remove last '┬'
                border.push('┐');
                border.push('\n');
                border
            }
        }
    }
    
    /// Generate cache key
    fn generate_cache_key(&self, data: &serde_json::Value, format: &OutputFormat) -> String {
        let data_hash = format!("{:x}", md5::compute(serde_json::to_string(data).unwrap_or_default()));
        format!("{}:{}", format!("{:?}", format), data_hash)
    }
    
    /// Update statistics
    async fn update_statistics(&self, format: &OutputFormat, format_time_ms: u64, success: bool) {
        let mut stats = self.stats.write().await;
        
        stats.total_outputs_formatted += 1;
        if success {
            stats.successful_formats += 1;
        } else {
            stats.failed_formats += 1;
        }
        
        // Update averages
        let total = stats.total_outputs_formatted as f64;
        stats.average_format_time_ms = 
            (stats.average_format_time_ms * (total - 1.0) + format_time_ms as f64) / total;
        
        // Update most used formats
        let format_str = format!("{:?}", format);
        *stats.most_used_formats.entry(format_str).or_insert(0) += 1;
        
        // Update success rate
        stats.format_success_rate = stats.successful_formats as f64 / total;
        
        // Update timestamps
        stats.last_format_timestamp = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
        stats.uptime_seconds = self.start_time.elapsed().as_secs();
    }
    
    /// Get output formatter statistics
    pub async fn get_statistics(&self) -> Result<OutputFormatterStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_output_formatter_config() {
        let config = OutputFormatterConfig::default();
        assert!(config.enable_colored_output);
        assert_eq!(config.default_output_format, OutputFormat::Table);
        assert_eq!(config.table_column_width, 20);
        assert_eq!(config.table_border_style, TableBorderStyle::Simple);
        assert!(config.json_pretty_print);
        assert!(config.yaml_pretty_print);
        assert_eq!(config.csv_delimiter, ',');
        assert_eq!(config.csv_quote_character, '"');
        assert!(config.enable_output_caching);
        assert_eq!(config.output_cache_size, 1000);
    }
    
    #[tokio::test]
    async fn test_output_formatter_stats() {
        let stats = OutputFormatterStats {
            total_outputs_formatted: 100,
            successful_formats: 95,
            failed_formats: 5,
            average_format_time_ms: 5.0,
            most_used_formats: HashMap::new(),
            format_success_rate: 0.95,
            uptime_seconds: 3600,
            last_format_timestamp: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()),
        };
        
        assert_eq!(stats.total_outputs_formatted, 100);
        assert_eq!(stats.successful_formats, 95);
        assert_eq!(stats.failed_formats, 5);
        assert_eq!(stats.average_format_time_ms, 5.0);
        assert_eq!(stats.format_success_rate, 0.95);
        assert_eq!(stats.uptime_seconds, 3600);
        assert!(stats.last_format_timestamp.is_some());
    }
}
