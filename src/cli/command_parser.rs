//! Command parser for IPPAN CLI
//! 
//! Implements command parsing, validation, and argument processing.

use crate::{Result, IppanError, TransactionHash};
use super::{CLICommand, CLIArgument, ArgumentType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};
use tracing::{info, warn, error, debug};

/// Command parser configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandParserConfig {
    /// Enable command validation
    pub enable_command_validation: bool,
    /// Enable argument validation
    pub enable_argument_validation: bool,
    /// Enable command aliases
    pub enable_command_aliases: bool,
    /// Enable command completion
    pub enable_command_completion: bool,
    /// Maximum command length
    pub max_command_length: usize,
    /// Maximum argument count
    pub max_argument_count: usize,
    /// Enable command history
    pub enable_command_history: bool,
    /// Command history size
    pub command_history_size: usize,
}

impl Default for CommandParserConfig {
    fn default() -> Self {
        Self {
            enable_command_validation: true,
            enable_argument_validation: true,
            enable_command_aliases: true,
            enable_command_completion: true,
            max_command_length: 1000,
            max_argument_count: 100,
            enable_command_history: true,
            command_history_size: 1000,
        }
    }
}

/// Parsed command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedCommand {
    /// Command name
    pub command_name: String,
    /// Command arguments
    pub arguments: Vec<ParsedArgument>,
    /// Original command line
    pub original_command_line: String,
    /// Parse timestamp
    pub parse_timestamp: u64,
    /// Parse success
    pub parse_success: bool,
    /// Parse error message
    pub parse_error_message: Option<String>,
}

/// Parsed argument
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedArgument {
    /// Argument name
    pub argument_name: String,
    /// Argument value
    pub argument_value: String,
    /// Argument type
    pub argument_type: ArgumentType,
    /// Is required
    pub is_required: bool,
    /// Validation success
    pub validation_success: bool,
    /// Validation error message
    pub validation_error_message: Option<String>,
}

/// Command parser
pub struct CommandParser {
    /// Configuration
    config: CommandParserConfig,
    /// Registered commands
    commands: Arc<RwLock<HashMap<String, CLICommand>>>,
    /// Command aliases
    command_aliases: Arc<RwLock<HashMap<String, String>>>,
    /// Statistics
    stats: Arc<RwLock<CommandParserStats>>,
    /// Start time
    start_time: Instant,
}

/// Command parser statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandParserStats {
    /// Total commands parsed
    pub total_commands_parsed: u64,
    /// Successful parses
    pub successful_parses: u64,
    /// Failed parses
    pub failed_parses: u64,
    /// Average parse time in milliseconds
    pub average_parse_time_ms: f64,
    /// Most parsed commands
    pub most_parsed_commands: HashMap<String, u64>,
    /// Parse success rate
    pub parse_success_rate: f64,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Last parse timestamp
    pub last_parse_timestamp: Option<u64>,
}

impl Default for CommandParserStats {
    fn default() -> Self {
        Self {
            total_commands_parsed: 0,
            successful_parses: 0,
            failed_parses: 0,
            average_parse_time_ms: 0.0,
            most_parsed_commands: HashMap::new(),
            parse_success_rate: 0.0,
            uptime_seconds: 0,
            last_parse_timestamp: None,
        }
    }
}

impl CommandParser {
    /// Create a new command parser
    pub fn new(config: CommandParserConfig) -> Self {
        Self {
            config,
            commands: Arc::new(RwLock::new(HashMap::new())),
            command_aliases: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(CommandParserStats::default())),
            start_time: Instant::now(),
        }
    }
    
    /// Register a command
    pub async fn register_command(&self, command: CLICommand) -> Result<()> {
        info!("Registering command: {}", command.name);
        
        // Register command
        {
            let mut commands = self.commands.write().await;
            commands.insert(command.name.clone(), command.clone());
        }
        
        // Register aliases
        if self.config.enable_command_aliases {
            for alias in &command.aliases {
                let mut aliases = self.command_aliases.write().await;
                aliases.insert(alias.clone(), command.name.clone());
            }
        }
        
        info!("Command registered successfully");
        Ok(())
    }
    
    /// Parse command line
    pub async fn parse_command_line(&self, command_line: &str) -> Result<ParsedCommand> {
        let start_time = Instant::now();
        
        info!("Parsing command line: {}", command_line);
        
        // Validate command line length
        if command_line.len() > self.config.max_command_length {
            return Ok(ParsedCommand {
                command_name: String::new(),
                arguments: vec![],
                original_command_line: command_line.to_string(),
                parse_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                parse_success: false,
                parse_error_message: Some(format!("Command line too long: {} characters", command_line.len())),
            });
        }
        
        // Split command line into parts
        let parts: Vec<&str> = command_line.trim().split_whitespace().collect();
        
        if parts.is_empty() {
            return Ok(ParsedCommand {
                command_name: String::new(),
                arguments: vec![],
                original_command_line: command_line.to_string(),
                parse_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                parse_success: false,
                parse_error_message: Some("Empty command line".to_string()),
            });
        }
        
        let command_name = parts[0].to_string();
        let argument_strings = parts[1..].to_vec();
        
        // Check for command aliases
        let resolved_command_name = if self.config.enable_command_aliases {
            let aliases = self.command_aliases.read().await;
            aliases.get(&command_name).cloned().unwrap_or(command_name)
        } else {
            command_name
        };
        
        // Get command definition
        let command_definition = {
            let commands = self.commands.read().await;
            commands.get(&resolved_command_name).cloned()
        };
        
        let command_definition = match command_definition {
            Some(cmd) => cmd,
            None => {
                return Ok(ParsedCommand {
                    command_name: resolved_command_name.clone(),
                    arguments: vec![],
                    original_command_line: command_line.to_string(),
                    parse_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                    parse_success: false,
                    parse_error_message: Some(format!("Command '{}' not found", resolved_command_name)),
                });
            }
        };
        
        // Parse arguments
        let argument_strings: Vec<String> = argument_strings.iter().map(|s| s.to_string()).collect();
        let arguments = self.parse_arguments(&command_definition, &argument_strings).await?;
        
        let parse_time = start_time.elapsed().as_millis() as u64;
        
        let parsed_command = ParsedCommand {
            command_name: resolved_command_name,
            arguments,
            original_command_line: command_line.to_string(),
            parse_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            parse_success: true,
            parse_error_message: None,
        };
        
        // Update statistics
        self.update_statistics(&parsed_command, parse_time).await;
        
        info!("Command parsed successfully in {}ms", parse_time);
        Ok(parsed_command)
    }
    
    /// Parse arguments
    async fn parse_arguments(&self, command: &CLICommand, argument_strings: &[String]) -> Result<Vec<ParsedArgument>> {
        let mut parsed_arguments = Vec::new();
        
        // Parse required arguments
        for (i, arg_def) in command.required_arguments.iter().enumerate() {
            if i < argument_strings.len() {
                let argument_value = argument_strings[i].clone();
                let parsed_arg = self.parse_argument(arg_def, &argument_value).await?;
                parsed_arguments.push(parsed_arg);
            } else {
                return Err(IppanError::CLI(
                    format!("Missing required argument: {}", arg_def.name)
                ));
            }
        }
        
        // Parse optional arguments
        let optional_start_index = command.required_arguments.len();
        for (i, arg_def) in command.optional_arguments.iter().enumerate() {
            let arg_index = optional_start_index + i;
            if arg_index < argument_strings.len() {
                let argument_value = argument_strings[arg_index].clone();
                let parsed_arg = self.parse_argument(arg_def, &argument_value).await?;
                parsed_arguments.push(parsed_arg);
            } else if let Some(default_value) = &arg_def.default_value {
                let parsed_arg = self.parse_argument(arg_def, default_value).await?;
                parsed_arguments.push(parsed_arg);
            }
        }
        
        Ok(parsed_arguments)
    }
    
    /// Parse individual argument
    async fn parse_argument(&self, arg_def: &CLIArgument, value: &str) -> Result<ParsedArgument> {
        let mut parsed_arg = ParsedArgument {
            argument_name: arg_def.name.clone(),
            argument_value: value.to_string(),
            argument_type: arg_def.argument_type.clone(),
            is_required: arg_def.required,
            validation_success: true,
            validation_error_message: None,
        };
        
        // Validate argument if enabled
        if self.config.enable_argument_validation {
            match self.validate_argument_value(value, &arg_def.argument_type) {
                Ok(_) => {
                    parsed_arg.validation_success = true;
                }
                Err(e) => {
                    parsed_arg.validation_success = false;
                    parsed_arg.validation_error_message = Some(e.to_string());
                }
            }
        }
        
        Ok(parsed_arg)
    }
    
    /// Validate argument value
    fn validate_argument_value(&self, value: &str, arg_type: &ArgumentType) -> Result<()> {
        match arg_type {
            ArgumentType::String => {
                // String is always valid
                Ok(())
            }
            ArgumentType::Integer => {
                value.parse::<i64>()
                    .map_err(|_| IppanError::CLI(format!("Invalid integer: {}", value)))?;
                Ok(())
            }
            ArgumentType::Float => {
                value.parse::<f64>()
                    .map_err(|_| IppanError::CLI(format!("Invalid float: {}", value)))?;
                Ok(())
            }
            ArgumentType::Boolean => {
                match value.to_lowercase().as_str() {
                    "true" | "false" | "1" | "0" | "yes" | "no" => Ok(()),
                    _ => Err(IppanError::CLI(format!("Invalid boolean: {}", value))),
                }
            }
            ArgumentType::FilePath => {
                if std::path::Path::new(value).exists() {
                    Ok(())
                } else {
                    Err(IppanError::CLI(format!("File not found: {}", value)))
                }
            }
            ArgumentType::DirectoryPath => {
                if std::path::Path::new(value).is_dir() {
                    Ok(())
                } else {
                    Err(IppanError::CLI(format!("Directory not found: {}", value)))
                }
            }
            ArgumentType::Url => {
                url::Url::parse(value)
                    .map_err(|_| IppanError::CLI(format!("Invalid URL: {}", value)))?;
                Ok(())
            }
            ArgumentType::Email => {
                if value.contains('@') && value.contains('.') {
                    Ok(())
                } else {
                    Err(IppanError::CLI(format!("Invalid email: {}", value)))
                }
            }
            ArgumentType::IpAddress => {
                value.parse::<std::net::IpAddr>()
                    .map_err(|_| IppanError::CLI(format!("Invalid IP address: {}", value)))?;
                Ok(())
            }
            ArgumentType::Port => {
                let port = value.parse::<u16>()
                    .map_err(|_| IppanError::CLI(format!("Invalid port: {}", value)))?;
                if port > 0 {
                    Ok(())
                } else {
                    Err(IppanError::CLI(format!("Port must be greater than 0: {}", value)))
                }
            }
            ArgumentType::Hash => {
                if value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit()) {
                    Ok(())
                } else {
                    Err(IppanError::CLI(format!("Invalid hash: {}", value)))
                }
            }
            ArgumentType::Address => {
                if value.starts_with('i') && value.len() >= 10 {
                    Ok(())
                } else {
                    Err(IppanError::CLI(format!("Invalid address: {}", value)))
                }
            }
        }
    }
    
    /// Get command suggestions
    pub async fn get_command_suggestions(&self, partial_command: &str) -> Result<Vec<String>> {
        let commands = self.commands.read().await;
        let mut suggestions = Vec::new();
        
        for command_name in commands.keys() {
            if command_name.starts_with(partial_command) {
                suggestions.push(command_name.clone());
            }
        }
        
        // Sort suggestions alphabetically
        suggestions.sort();
        
        Ok(suggestions)
    }
    
    /// Get available commands
    pub async fn get_available_commands(&self) -> Result<Vec<CLICommand>> {
        let commands = self.commands.read().await;
        Ok(commands.values().cloned().collect())
    }
    
    /// Update statistics
    async fn update_statistics(&self, parsed_command: &ParsedCommand, parse_time_ms: u64) {
        let mut stats = self.stats.write().await;
        
        stats.total_commands_parsed += 1;
        if parsed_command.parse_success {
            stats.successful_parses += 1;
        } else {
            stats.failed_parses += 1;
        }
        
        // Update averages
        let total = stats.total_commands_parsed as f64;
        stats.average_parse_time_ms = 
            (stats.average_parse_time_ms * (total - 1.0) + parse_time_ms as f64) / total;
        
        // Update most parsed commands
        *stats.most_parsed_commands.entry(parsed_command.command_name.clone()).or_insert(0) += 1;
        
        // Update success rate
        stats.parse_success_rate = stats.successful_parses as f64 / total;
        
        // Update timestamps
        stats.last_parse_timestamp = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
        stats.uptime_seconds = self.start_time.elapsed().as_secs();
    }
    
    /// Get command parser statistics
    pub async fn get_statistics(&self) -> Result<CommandParserStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_command_parser_config() {
        let config = CommandParserConfig::default();
        assert!(config.enable_command_validation);
        assert!(config.enable_argument_validation);
        assert!(config.enable_command_aliases);
        assert!(config.enable_command_completion);
        assert_eq!(config.max_command_length, 1000);
        assert_eq!(config.max_argument_count, 100);
        assert!(config.enable_command_history);
        assert_eq!(config.command_history_size, 1000);
    }
    
    #[tokio::test]
    async fn test_parsed_command() {
        let parsed_command = ParsedCommand {
            command_name: "test".to_string(),
            arguments: vec![],
            original_command_line: "test arg1 arg2".to_string(),
            parse_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            parse_success: true,
            parse_error_message: None,
        };
        
        assert_eq!(parsed_command.command_name, "test");
        assert!(parsed_command.arguments.is_empty());
        assert_eq!(parsed_command.original_command_line, "test arg1 arg2");
        assert!(parsed_command.parse_success);
        assert!(parsed_command.parse_error_message.is_none());
    }
    
    #[tokio::test]
    async fn test_parsed_argument() {
        let parsed_arg = ParsedArgument {
            argument_name: "test_arg".to_string(),
            argument_value: "test_value".to_string(),
            argument_type: ArgumentType::String,
            is_required: true,
            validation_success: true,
            validation_error_message: None,
        };
        
        assert_eq!(parsed_arg.argument_name, "test_arg");
        assert_eq!(parsed_arg.argument_value, "test_value");
        assert_eq!(parsed_arg.argument_type, ArgumentType::String);
        assert!(parsed_arg.is_required);
        assert!(parsed_arg.validation_success);
        assert!(parsed_arg.validation_error_message.is_none());
    }
    
    #[tokio::test]
    async fn test_command_parser_stats() {
        let stats = CommandParserStats {
            total_commands_parsed: 100,
            successful_parses: 95,
            failed_parses: 5,
            average_parse_time_ms: 10.0,
            most_parsed_commands: HashMap::new(),
            parse_success_rate: 0.95,
            uptime_seconds: 3600,
            last_parse_timestamp: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()),
        };
        
        assert_eq!(stats.total_commands_parsed, 100);
        assert_eq!(stats.successful_parses, 95);
        assert_eq!(stats.failed_parses, 5);
        assert_eq!(stats.average_parse_time_ms, 10.0);
        assert_eq!(stats.parse_success_rate, 0.95);
        assert_eq!(stats.uptime_seconds, 3600);
        assert!(stats.last_parse_timestamp.is_some());
    }
}
