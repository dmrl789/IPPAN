//! CLI manager for IPPAN
//! 
//! Orchestrates the entire CLI system including command parsing,
//! execution, and output formatting.

use crate::{Result, IppanError, TransactionHash};
use super::{CLIConfig, CLIResult, CLICommand, CommandCategory, CLIContext, OutputFormat, CLIStats};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};
use tracing::{info, warn, error, debug};

/// CLI manager configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CLIManagerConfig {
    /// Enable command validation
    pub enable_command_validation: bool,
    /// Enable command logging
    pub enable_command_logging: bool,
    /// Enable command history
    pub enable_command_history: bool,
    /// Enable auto-completion
    pub enable_auto_completion: bool,
    /// Enable command aliases
    pub enable_command_aliases: bool,
    /// Command timeout in seconds
    pub command_timeout_seconds: u64,
    /// Maximum concurrent commands
    pub max_concurrent_commands: usize,
    /// Enable command caching
    pub enable_command_caching: bool,
    /// Cache size
    pub cache_size: usize,
    /// Enable command profiling
    pub enable_command_profiling: bool,
}

impl Default for CLIManagerConfig {
    fn default() -> Self {
        Self {
            enable_command_validation: true,
            enable_command_logging: true,
            enable_command_history: true,
            enable_auto_completion: true,
            enable_command_aliases: true,
            command_timeout_seconds: 30,
            max_concurrent_commands: 10,
            enable_command_caching: true,
            cache_size: 1000,
            enable_command_profiling: true,
        }
    }
}

/// CLI manager
pub struct CLIManager {
    /// Configuration
    config: CLIManagerConfig,
    /// CLI context
    context: Arc<RwLock<CLIContext>>,
    /// Registered commands
    commands: Arc<RwLock<HashMap<String, CLICommand>>>,
    /// Command handlers
    handlers: Arc<RwLock<HashMap<String, Box<dyn Fn(&CLIContext, Vec<String>) -> Result<CLIResult> + Send + Sync>>>>,
    /// Command cache
    command_cache: Arc<RwLock<HashMap<String, CLIResult>>>,
    /// Is running
    is_running: Arc<RwLock<bool>>,
    /// Start time
    start_time: Instant,
}

impl CLIManager {
    /// Create a new CLI manager
    pub fn new(config: CLIManagerConfig, cli_config: CLIConfig) -> Self {
        let context = Arc::new(RwLock::new(CLIContext::new(cli_config)));
        
        Self {
            config,
            context,
            commands: Arc::new(RwLock::new(HashMap::new())),
            handlers: Arc::new(RwLock::new(HashMap::new())),
            command_cache: Arc::new(RwLock::new(HashMap::new())),
            is_running: Arc::new(RwLock::new(false)),
            start_time: Instant::now(),
        }
    }
    
    /// Start the CLI manager
    pub async fn start(&self) -> Result<()> {
        info!("Starting CLI manager");
        
        let mut is_running = self.is_running.write().await;
        *is_running = true;
        drop(is_running);
        
        // Register default commands
        self.register_default_commands().await?;
        
        info!("CLI manager started successfully");
        Ok(())
    }
    
    /// Stop the CLI manager
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping CLI manager");
        
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        
        info!("CLI manager stopped");
        Ok(())
    }
    
    /// Execute a command
    pub async fn execute_command(&self, command_line: &str) -> Result<CLIResult> {
        let start_time = Instant::now();
        
        info!("Executing command: {}", command_line);
        
        // Parse command
        let (command_name, arguments) = self.parse_command_line(command_line)?;
        
        // Check if command exists
        let command = {
            let commands = self.commands.read().await;
            commands.get(&command_name).cloned()
        };
        
        let command = match command {
            Some(cmd) => cmd,
            None => {
                return Ok(CLIResult {
                    success: false,
                    data: None,
                    error_message: Some(format!("Command '{}' not found", command_name)),
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    command_name: command_name.clone(),
                    command_arguments: arguments.clone(),
                    output_format: OutputFormat::Plain,
                });
            }
        };
        
        // Validate arguments
        if self.config.enable_command_validation {
            self.validate_command_arguments(&command, &arguments).await?;
        }
        
        // Check cache
        if self.config.enable_command_caching {
            let cache_key = self.generate_cache_key(&command_name, &arguments);
            let cached_result = {
                let cache = self.command_cache.read().await;
                cache.get(&cache_key).cloned()
            };
            
            if let Some(cached) = cached_result {
                info!("Command result retrieved from cache");
                return Ok(cached);
            }
        }
        
        // Execute command
        let result = self.execute_command_handler(&command_name, &arguments).await?;
        
        // Cache result
        if self.config.enable_command_caching && result.success {
            let cache_key = self.generate_cache_key(&command_name, &arguments);
            let mut cache = self.command_cache.write().await;
            cache.insert(cache_key, result.clone());
            
            // Trim cache if it exceeds size limit
            if cache.len() > self.config.cache_size {
                let keys_to_remove: Vec<String> = cache.keys().take(cache.len() - self.config.cache_size).cloned().collect();
                for key in keys_to_remove {
                    cache.remove(&key);
                }
            }
        }
        
        // Update statistics
        {
            let context = self.context.read().await;
            context.update_stats(&command_name, result.execution_time_ms, result.success).await;
        }
        
        // Add to command history
        if self.config.enable_command_history {
            let context = self.context.read().await;
            context.add_command_to_history(command_line).await;
        }
        
        info!("Command executed successfully in {}ms", result.execution_time_ms);
        Ok(result)
    }
    
    /// Register a command
    pub async fn register_command(&self, command: CLICommand, handler: Box<dyn Fn(&CLIContext, Vec<String>) -> Result<CLIResult> + Send + Sync>) -> Result<()> {
        info!("Registering command: {}", command.name);
        
        // Register command
        {
            let mut commands = self.commands.write().await;
            commands.insert(command.name.clone(), command.clone());
        }
        
        // Register handler
        {
            let mut handlers = self.handlers.write().await;
            handlers.insert(command.name.clone(), handler);
        }
        
        // Register aliases
        if self.config.enable_command_aliases {
            for alias in &command.aliases {
                let mut commands = self.commands.write().await;
                commands.insert(alias.clone(), command.clone());
            }
        }
        
        info!("Command registered successfully");
        Ok(())
    }
    
    /// Get available commands
    pub async fn get_available_commands(&self) -> Result<Vec<CLICommand>> {
        let commands = self.commands.read().await;
        Ok(commands.values().cloned().collect())
    }
    
    /// Get commands by category
    pub async fn get_commands_by_category(&self, category: CommandCategory) -> Result<Vec<CLICommand>> {
        let commands = self.commands.read().await;
        Ok(commands.values().filter(|cmd| cmd.category == category).cloned().collect())
    }
    
    /// Get command help
    pub async fn get_command_help(&self, command_name: &str) -> Result<Option<String>> {
        let commands = self.commands.read().await;
        if let Some(command) = commands.get(command_name) {
            Ok(Some(command.help_text.clone()))
        } else {
            Ok(None)
        }
    }
    
    /// Get command history
    pub async fn get_command_history(&self) -> Result<Vec<String>> {
        let context = self.context.read().await;
        Ok(context.get_command_history().await)
    }
    
    /// Clear command history
    pub async fn clear_command_history(&self) -> Result<()> {
        let context = self.context.read().await;
        let mut history = context.command_history.write().await;
        history.clear();
        Ok(())
    }
    
    /// Get CLI statistics
    pub async fn get_cli_statistics(&self) -> Result<super::CLIStats> {
        let context = self.context.read().await;
        Ok(context.get_stats().await)
    }
    
    /// Parse command line
    fn parse_command_line(&self, command_line: &str) -> Result<(String, Vec<String>)> {
        let parts: Vec<&str> = command_line.trim().split_whitespace().collect();
        
        if parts.is_empty() {
            return Err(IppanError::CLI("Empty command line".to_string()));
        }
        
        let command_name = parts[0].to_string();
        let arguments = parts[1..].iter().map(|s| s.to_string()).collect();
        
        Ok((command_name, arguments))
    }
    
    /// Validate command arguments
    async fn validate_command_arguments(&self, command: &CLICommand, arguments: &[String]) -> Result<()> {
        // Check required arguments
        if arguments.len() < command.required_arguments.len() {
            return Err(IppanError::CLI(
                format!("Command '{}' requires {} arguments, got {}", 
                    command.name, command.required_arguments.len(), arguments.len())
            ));
        }
        
        // Validate argument types
        for (i, arg) in arguments.iter().enumerate() {
            if i < command.required_arguments.len() {
                let arg_def = &command.required_arguments[i];
                self.validate_argument_type(arg, &arg_def.argument_type)?;
            } else if i - command.required_arguments.len() < command.optional_arguments.len() {
                let arg_def = &command.optional_arguments[i - command.required_arguments.len()];
                self.validate_argument_type(arg, &arg_def.argument_type)?;
            }
        }
        
        Ok(())
    }
    
    /// Validate argument type
    fn validate_argument_type(&self, value: &str, arg_type: &super::ArgumentType) -> Result<()> {
        match arg_type {
            super::ArgumentType::String => {
                // String is always valid
                Ok(())
            }
            super::ArgumentType::Integer => {
                value.parse::<i64>()
                    .map_err(|_| IppanError::CLI(format!("Invalid integer: {}", value)))?;
                Ok(())
            }
            super::ArgumentType::Float => {
                value.parse::<f64>()
                    .map_err(|_| IppanError::CLI(format!("Invalid float: {}", value)))?;
                Ok(())
            }
            super::ArgumentType::Boolean => {
                match value.to_lowercase().as_str() {
                    "true" | "false" | "1" | "0" | "yes" | "no" => Ok(()),
                    _ => Err(IppanError::CLI(format!("Invalid boolean: {}", value))),
                }
            }
            super::ArgumentType::FilePath => {
                if std::path::Path::new(value).exists() {
                    Ok(())
                } else {
                    Err(IppanError::CLI(format!("File not found: {}", value)))
                }
            }
            super::ArgumentType::DirectoryPath => {
                if std::path::Path::new(value).is_dir() {
                    Ok(())
                } else {
                    Err(IppanError::CLI(format!("Directory not found: {}", value)))
                }
            }
            super::ArgumentType::Url => {
                url::Url::parse(value)
                    .map_err(|_| IppanError::CLI(format!("Invalid URL: {}", value)))?;
                Ok(())
            }
            super::ArgumentType::Email => {
                if value.contains('@') && value.contains('.') {
                    Ok(())
                } else {
                    Err(IppanError::CLI(format!("Invalid email: {}", value)))
                }
            }
            super::ArgumentType::IpAddress => {
                value.parse::<std::net::IpAddr>()
                    .map_err(|_| IppanError::CLI(format!("Invalid IP address: {}", value)))?;
                Ok(())
            }
            super::ArgumentType::Port => {
                let port = value.parse::<u16>()
                    .map_err(|_| IppanError::CLI(format!("Invalid port: {}", value)))?;
                if port > 0 {
                    Ok(())
                } else {
                    Err(IppanError::CLI(format!("Port must be greater than 0: {}", value)))
                }
            }
            super::ArgumentType::Hash => {
                if value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit()) {
                    Ok(())
                } else {
                    Err(IppanError::CLI(format!("Invalid hash: {}", value)))
                }
            }
            super::ArgumentType::Address => {
                if value.starts_with('i') && value.len() >= 10 {
                    Ok(())
                } else {
                    Err(IppanError::CLI(format!("Invalid address: {}", value)))
                }
            }
        }
    }
    
    /// Execute command handler
    async fn execute_command_handler(&self, command_name: &str, arguments: &[String]) -> Result<CLIResult> {
        let start_time = Instant::now();
        
        // Execute handler
        let result = {
            let handlers = self.handlers.read().await;
            if let Some(handler) = handlers.get(command_name) {
                let context = self.context.read().await;
                handler(&context, arguments.to_vec())?
            } else {
                return Ok(CLIResult {
                    success: false,
                    data: None,
                    error_message: Some(format!("No handler found for command '{}'", command_name)),
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    command_name: command_name.to_string(),
                    command_arguments: arguments.to_vec(),
                    output_format: OutputFormat::Plain,
                });
            }
        };
        
        Ok(result)
    }
    
    /// Generate cache key
    fn generate_cache_key(&self, command_name: &str, arguments: &[String]) -> String {
        format!("{}:{}", command_name, arguments.join(":"))
    }
    
    /// Register default commands
    async fn register_default_commands(&self) -> Result<()> {
        info!("Registering default CLI commands");
        
        // Help command
        let help_command = CLICommand {
            name: "help".to_string(),
            description: "Show help information".to_string(),
            category: CommandCategory::Help,
            required_arguments: vec![],
            optional_arguments: vec![
                super::CLIArgument {
                    name: "command".to_string(),
                    description: "Command to get help for".to_string(),
                    argument_type: super::ArgumentType::String,
                    required: false,
                    default_value: None,
                    validation_rules: vec![],
                }
            ],
            handler: "help_handler".to_string(),
            aliases: vec!["h".to_string(), "?".to_string()],
            examples: vec![
                "help".to_string(),
                "help status".to_string(),
            ],
            help_text: "Show help information for commands. Use 'help <command>' to get detailed help for a specific command.".to_string(),
        };
        
        self.register_command(help_command, Box::new(|context, args| {
            let help_text = if args.is_empty() {
                "Available commands:\n  help - Show help information\n  status - Show node status\n  quit - Exit CLI".to_string()
            } else {
                format!("Help for command '{}' not available", args[0])
            };
            
            Ok(CLIResult {
                success: true,
                data: Some(serde_json::Value::String(help_text)),
                error_message: None,
                execution_time_ms: 0,
                command_name: "help".to_string(),
                command_arguments: args,
                output_format: OutputFormat::Plain,
            })
        })).await?;
        
        // Status command
        let status_command = CLICommand {
            name: "status".to_string(),
            description: "Show node status".to_string(),
            category: CommandCategory::Node,
            required_arguments: vec![],
            optional_arguments: vec![],
            handler: "status_handler".to_string(),
            aliases: vec!["ps".to_string()],
            examples: vec![
                "status".to_string(),
            ],
            help_text: "Show the current status of the IPPAN node including uptime, connections, and performance metrics.".to_string(),
        };
        
        self.register_command(status_command, Box::new(|context, args| {
            let status_data = serde_json::json!({
                "node_status": "running",
                "uptime_seconds": context.session.session_duration_seconds,
                "commands_executed": context.session.commands_executed,
                "current_directory": context.session.current_working_directory
            });
            
            Ok(CLIResult {
                success: true,
                data: Some(status_data),
                error_message: None,
                execution_time_ms: 0,
                command_name: "status".to_string(),
                command_arguments: args,
                output_format: OutputFormat::Json,
            })
        })).await?;
        
        // Quit command
        let quit_command = CLICommand {
            name: "quit".to_string(),
            description: "Exit CLI".to_string(),
            category: CommandCategory::System,
            required_arguments: vec![],
            optional_arguments: vec![],
            handler: "quit_handler".to_string(),
            aliases: vec!["q".to_string(), "exit".to_string()],
            examples: vec![
                "quit".to_string(),
                "exit".to_string(),
            ],
            help_text: "Exit the CLI session.".to_string(),
        };
        
        self.register_command(quit_command, Box::new(|context, args| {
            Ok(CLIResult {
                success: true,
                data: Some(serde_json::Value::String("Goodbye!".to_string())),
                error_message: None,
                execution_time_ms: 0,
                command_name: "quit".to_string(),
                command_arguments: args,
                output_format: OutputFormat::Plain,
            })
        })).await?;
        
        info!("Default commands registered successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_cli_manager_config() {
        let config = CLIManagerConfig::default();
        assert!(config.enable_command_validation);
        assert!(config.enable_command_logging);
        assert!(config.enable_command_history);
        assert!(config.enable_auto_completion);
        assert!(config.enable_command_aliases);
        assert_eq!(config.command_timeout_seconds, 30);
        assert_eq!(config.max_concurrent_commands, 10);
        assert!(config.enable_command_caching);
        assert_eq!(config.cache_size, 1000);
        assert!(config.enable_command_profiling);
    }
    
    #[tokio::test]
    async fn test_cli_config() {
        let config = CLIConfig::default();
        assert!(config.enable_interactive_mode);
        assert!(config.enable_colored_output);
        assert!(!config.enable_verbose_output);
        assert_eq!(config.default_output_format, OutputFormat::Table);
        assert_eq!(config.command_history_size, 1000);
        assert!(config.auto_completion_enabled);
        assert_eq!(config.command_timeout_seconds, 30);
        assert!(config.enable_command_logging);
        assert_eq!(config.log_file_path, "cli.log");
        assert!(config.enable_command_aliases);
        assert!(!config.command_aliases.is_empty());
    }
    
    #[tokio::test]
    async fn test_cli_command() {
        let command = CLICommand {
            name: "test".to_string(),
            description: "Test command".to_string(),
            category: CommandCategory::System,
            required_arguments: vec![],
            optional_arguments: vec![],
            handler: "test_handler".to_string(),
            aliases: vec!["t".to_string()],
            examples: vec!["test".to_string()],
            help_text: "Test command help".to_string(),
        };
        
        assert_eq!(command.name, "test");
        assert_eq!(command.description, "Test command");
        assert_eq!(command.category, CommandCategory::System);
        assert_eq!(command.handler, "test_handler");
        assert_eq!(command.aliases, vec!["t"]);
        assert_eq!(command.examples, vec!["test"]);
        assert_eq!(command.help_text, "Test command help");
    }
    
    #[tokio::test]
    async fn test_cli_result() {
        let result = CLIResult {
            success: true,
            data: Some(serde_json::Value::String("test".to_string())),
            error_message: None,
            execution_time_ms: 100,
            command_name: "test".to_string(),
            command_arguments: vec!["arg1".to_string()],
            output_format: OutputFormat::Json,
        };
        
        assert!(result.success);
        assert!(result.data.is_some());
        assert!(result.error_message.is_none());
        assert_eq!(result.execution_time_ms, 100);
        assert_eq!(result.command_name, "test");
        assert_eq!(result.command_arguments, vec!["arg1"]);
        assert_eq!(result.output_format, OutputFormat::Json);
    }
    
    #[tokio::test]
    async fn test_cli_stats() {
        let stats = CLIStats {
            total_commands_executed: 100,
            successful_commands: 95,
            failed_commands: 5,
            average_execution_time_ms: 50.0,
            most_used_commands: HashMap::new(),
            command_success_rate: 0.95,
            uptime_seconds: 3600,
            last_command_timestamp: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()),
            interactive_mode_usage: 80,
            batch_mode_usage: 20,
        };
        
        assert_eq!(stats.total_commands_executed, 100);
        assert_eq!(stats.successful_commands, 95);
        assert_eq!(stats.failed_commands, 5);
        assert_eq!(stats.average_execution_time_ms, 50.0);
        assert_eq!(stats.command_success_rate, 0.95);
        assert_eq!(stats.uptime_seconds, 3600);
        assert_eq!(stats.interactive_mode_usage, 80);
        assert_eq!(stats.batch_mode_usage, 20);
    }
}
