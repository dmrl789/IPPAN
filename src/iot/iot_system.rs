//! Advanced Internet of Things (IoT) and Edge Computing System for IPPAN
//! 
//! Provides IoT device management, edge computing capabilities, and real-time sensor data processing

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use std::time::Duration;

/// IoT device types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum IoTDeviceType {
    Sensor,
    Actuator,
    Gateway,
    EdgeNode,
    Controller,
    Monitor,
    Camera,
    Microphone,
    Speaker,
    Display,
    Robot,
    Vehicle,
    Drone,
    SmartMeter,
    SmartLight,
    SmartThermostat,
    SmartLock,
    SmartAppliance,
}

/// IoT device status
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum IoTDeviceStatus {
    Online,
    Offline,
    Maintenance,
    Error,
    Disabled,
    Updating,
}

/// Edge computing node types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EdgeNodeType {
    Gateway,
    Fog,
    Edge,
    MicroEdge,
    NanoEdge,
}

/// Sensor data types
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum SensorDataType {
    Temperature,
    Humidity,
    Pressure,
    Light,
    Motion,
    Sound,
    Vibration,
    Location,
    Accelerometer,
    Gyroscope,
    Magnetometer,
    GPS,
    Camera,
    Microphone,
    Custom(String),
}

/// IoT device configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoTDeviceConfig {
    pub device_type: IoTDeviceType,
    pub name: String,
    pub description: String,
    pub location: String,
    pub manufacturer: String,
    pub model: String,
    pub firmware_version: String,
    pub hardware_version: String,
    pub capabilities: Vec<String>,
    pub sensors: Vec<SensorDataType>,
    pub actuators: Vec<String>,
    pub communication_protocol: String,
    pub power_source: String,
    pub battery_level: Option<f64>,
    pub update_interval_ms: u64,
    pub data_retention_days: u32,
    pub encryption_enabled: bool,
    pub authentication_required: bool,
}

/// IoT device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoTDevice {
    pub id: String,
    pub config: IoTDeviceConfig,
    pub status: IoTDeviceStatus,
    pub last_seen: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub total_data_points: u64,
    pub total_commands: u64,
    pub error_count: u64,
    pub sensor_data: HashMap<SensorDataType, Vec<SensorDataPoint>>,
    pub commands: Vec<IoTCommand>,
    pub alerts: Vec<IoTAlert>,
}

/// Sensor data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorDataPoint {
    pub id: String,
    pub device_id: String,
    pub sensor_type: SensorDataType,
    pub value: f64,
    pub unit: String,
    pub timestamp: DateTime<Utc>,
    pub location: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// IoT command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoTCommand {
    pub id: String,
    pub device_id: String,
    pub command_type: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub timestamp: DateTime<Utc>,
    pub status: CommandStatus,
    pub response: Option<String>,
    pub execution_time_ms: Option<u64>,
}

/// Command status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandStatus {
    Pending,
    Executing,
    Completed,
    Failed,
    Cancelled,
}

/// IoT alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoTAlert {
    pub id: String,
    pub device_id: String,
    pub alert_type: String,
    pub severity: AlertSeverity,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub acknowledged: bool,
    pub resolved: bool,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Alert severity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Edge computing node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeNode {
    pub id: String,
    pub node_type: EdgeNodeType,
    pub name: String,
    pub location: String,
    pub capabilities: Vec<String>,
    pub connected_devices: Vec<String>,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub storage_usage: f64,
    pub network_bandwidth: f64,
    pub status: IoTDeviceStatus,
    pub last_updated: DateTime<Utc>,
    pub processing_jobs: Vec<EdgeProcessingJob>,
}

/// Edge processing job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeProcessingJob {
    pub id: String,
    pub node_id: String,
    pub job_type: String,
    pub input_data: HashMap<String, serde_json::Value>,
    pub output_data: Option<HashMap<String, serde_json::Value>>,
    pub status: CommandStatus,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub execution_time_ms: Option<u64>,
    pub error_message: Option<String>,
}

/// IoT system statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoTSystemStats {
    pub total_devices: u64,
    pub online_devices: u64,
    pub offline_devices: u64,
    pub total_edge_nodes: u64,
    pub active_edge_nodes: u64,
    pub total_data_points: u64,
    pub total_commands: u64,
    pub total_alerts: u64,
    pub active_alerts: u64,
    pub devices_by_type: HashMap<IoTDeviceType, u64>,
    pub devices_by_status: HashMap<IoTDeviceStatus, u64>,
    pub data_points_by_sensor: HashMap<SensorDataType, u64>,
}

/// IoT system metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoTSystemMetrics {
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub network_bandwidth_mbps: f64,
    pub storage_usage_percent: f64,
    pub active_connections: u64,
    pub data_throughput_mbps: f64,
    pub average_response_time_ms: f64,
    pub error_rate_percent: f64,
    pub battery_level_average: f64,
    pub signal_strength_average: f64,
}

/// Advanced IoT system
pub struct AdvancedIoTSystem {
    devices: Arc<RwLock<HashMap<String, IoTDevice>>>,
    edge_nodes: Arc<RwLock<HashMap<String, EdgeNode>>>,
    sensor_data: Arc<RwLock<HashMap<String, Vec<SensorDataPoint>>>>,
    commands: Arc<RwLock<HashMap<String, IoTCommand>>>,
    alerts: Arc<RwLock<HashMap<String, IoTAlert>>>,
    stats: Arc<RwLock<IoTSystemStats>>,
    metrics: Arc<RwLock<IoTSystemMetrics>>,
    enabled: bool,
}

impl AdvancedIoTSystem {
    /// Create a new advanced IoT system
    pub fn new() -> Self {
        Self {
            devices: Arc::new(RwLock::new(HashMap::new())),
            edge_nodes: Arc::new(RwLock::new(HashMap::new())),
            sensor_data: Arc::new(RwLock::new(HashMap::new())),
            commands: Arc::new(RwLock::new(HashMap::new())),
            alerts: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(IoTSystemStats {
                total_devices: 0,
                online_devices: 0,
                offline_devices: 0,
                total_edge_nodes: 0,
                active_edge_nodes: 0,
                total_data_points: 0,
                total_commands: 0,
                total_alerts: 0,
                active_alerts: 0,
                devices_by_type: HashMap::new(),
                devices_by_status: HashMap::new(),
                data_points_by_sensor: HashMap::new(),
            })),
            metrics: Arc::new(RwLock::new(IoTSystemMetrics {
                cpu_usage_percent: 0.0,
                memory_usage_percent: 0.0,
                network_bandwidth_mbps: 0.0,
                storage_usage_percent: 0.0,
                active_connections: 0,
                data_throughput_mbps: 0.0,
                average_response_time_ms: 0.0,
                error_rate_percent: 0.0,
                battery_level_average: 100.0,
                signal_strength_average: 100.0,
            })),
            enabled: true,
        }
    }

    /// Register an IoT device
    pub async fn register_device(&self, config: IoTDeviceConfig) -> Result<String, Box<dyn std::error::Error>> {
        if !self.enabled {
            return Err("IoT system is disabled".into());
        }
        
        let device_id = format!("iot_device_{}", Utc::now().timestamp_millis());
        let config_clone = config.clone();
        
        let device = IoTDevice {
            id: device_id.clone(),
            config,
            status: IoTDeviceStatus::Online,
            last_seen: Utc::now(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            total_data_points: 0,
            total_commands: 0,
            error_count: 0,
            sensor_data: HashMap::new(),
            commands: Vec::new(),
            alerts: Vec::new(),
        };
        
        let mut devices = self.devices.write().await;
        devices.insert(device_id.clone(), device);
        
        // Update statistics
        let mut stats = self.stats.write().await;
        stats.total_devices += 1;
        stats.online_devices += 1;
        *stats.devices_by_type.entry(config_clone.device_type.clone()).or_insert(0) += 1;
        *stats.devices_by_status.entry(IoTDeviceStatus::Online).or_insert(0) += 1;
        
        Ok(device_id)
    }

    /// Send sensor data from device
    pub async fn send_sensor_data(&self, device_id: &str, sensor_type: SensorDataType, value: f64, unit: &str) -> Result<(), Box<dyn std::error::Error>> {
        if !self.enabled {
            return Err("IoT system is disabled".into());
        }
        
        let data_point = SensorDataPoint {
            id: format!("data_{}", Utc::now().timestamp_millis()),
            device_id: device_id.to_string(),
            sensor_type: sensor_type.clone(),
            value,
            unit: unit.to_string(),
            timestamp: Utc::now(),
            location: None,
            metadata: HashMap::new(),
        };
        
        // Store sensor data
        let mut sensor_data = self.sensor_data.write().await;
        let device_data = sensor_data.entry(device_id.to_string()).or_insert_with(Vec::new);
        device_data.push(data_point.clone());
        
        // Update device statistics
        let mut devices = self.devices.write().await;
        if let Some(device) = devices.get_mut(device_id) {
            device.last_seen = Utc::now();
            device.total_data_points += 1;
            device.sensor_data.entry(sensor_type.clone()).or_insert_with(Vec::new).push(data_point);
        }
        
        // Update system statistics
        let mut stats = self.stats.write().await;
        stats.total_data_points += 1;
        *stats.data_points_by_sensor.entry(sensor_type).or_insert(0) += 1;
        
        Ok(())
    }

    /// Send command to device
    pub async fn send_command(&self, device_id: &str, command_type: &str, parameters: HashMap<String, serde_json::Value>) -> Result<String, Box<dyn std::error::Error>> {
        if !self.enabled {
            return Err("IoT system is disabled".into());
        }
        
        let command_id = format!("command_{}", Utc::now().timestamp_millis());
        
        let command = IoTCommand {
            id: command_id.clone(),
            device_id: device_id.to_string(),
            command_type: command_type.to_string(),
            parameters,
            timestamp: Utc::now(),
            status: CommandStatus::Pending,
            response: None,
            execution_time_ms: None,
        };
        
        // Store command
        let mut commands = self.commands.write().await;
        commands.insert(command_id.clone(), command.clone());
        
        // Update device statistics
        let mut devices = self.devices.write().await;
        if let Some(device) = devices.get_mut(device_id) {
            device.total_commands += 1;
            device.commands.push(command);
        }
        
        // Update system statistics
        let mut stats = self.stats.write().await;
        stats.total_commands += 1;
        
        Ok(command_id)
    }

    /// Execute command on device
    pub async fn execute_command(&self, command_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut commands = self.commands.write().await;
        
        let command = commands.get_mut(command_id)
            .ok_or("Command not found")?;
        
        command.status = CommandStatus::Executing;
        
        // Simulate command execution
        let start_time = std::time::Instant::now();
        
        // Simulate execution delay
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        let execution_time = start_time.elapsed();
        
        command.status = CommandStatus::Completed;
        command.response = Some("Command executed successfully".to_string());
        command.execution_time_ms = Some(execution_time.as_millis() as u64);
        
        Ok(())
    }

    /// Create alert for device
    pub async fn create_alert(&self, device_id: &str, alert_type: &str, severity: AlertSeverity, message: &str) -> Result<String, Box<dyn std::error::Error>> {
        if !self.enabled {
            return Err("IoT system is disabled".into());
        }
        
        let alert_id = format!("alert_{}", Utc::now().timestamp_millis());
        
        let alert = IoTAlert {
            id: alert_id.clone(),
            device_id: device_id.to_string(),
            alert_type: alert_type.to_string(),
            severity,
            message: message.to_string(),
            timestamp: Utc::now(),
            acknowledged: false,
            resolved: false,
            metadata: HashMap::new(),
        };
        
        // Store alert
        let mut alerts = self.alerts.write().await;
        alerts.insert(alert_id.clone(), alert.clone());
        
        // Update device statistics
        let mut devices = self.devices.write().await;
        if let Some(device) = devices.get_mut(device_id) {
            device.alerts.push(alert);
        }
        
        // Update system statistics
        let mut stats = self.stats.write().await;
        stats.total_alerts += 1;
        stats.active_alerts += 1;
        
        Ok(alert_id)
    }

    /// Register edge node
    pub async fn register_edge_node(&self, node_type: EdgeNodeType, name: &str, location: &str, capabilities: Vec<String>) -> Result<String, Box<dyn std::error::Error>> {
        if !self.enabled {
            return Err("IoT system is disabled".into());
        }
        
        let node_id = format!("edge_node_{}", Utc::now().timestamp_millis());
        
        let edge_node = EdgeNode {
            id: node_id.clone(),
            node_type,
            name: name.to_string(),
            location: location.to_string(),
            capabilities,
            connected_devices: Vec::new(),
            cpu_usage: 0.0,
            memory_usage: 0.0,
            storage_usage: 0.0,
            network_bandwidth: 0.0,
            status: IoTDeviceStatus::Online,
            last_updated: Utc::now(),
            processing_jobs: Vec::new(),
        };
        
        let mut edge_nodes = self.edge_nodes.write().await;
        edge_nodes.insert(node_id.clone(), edge_node);
        
        // Update statistics
        let mut stats = self.stats.write().await;
        stats.total_edge_nodes += 1;
        stats.active_edge_nodes += 1;
        
        Ok(node_id)
    }

    /// Submit edge processing job
    pub async fn submit_edge_job(&self, node_id: &str, job_type: &str, input_data: HashMap<String, serde_json::Value>) -> Result<String, Box<dyn std::error::Error>> {
        if !self.enabled {
            return Err("IoT system is disabled".into());
        }
        
        let job_id = format!("edge_job_{}", Utc::now().timestamp_millis());
        
        let job = EdgeProcessingJob {
            id: job_id.clone(),
            node_id: node_id.to_string(),
            job_type: job_type.to_string(),
            input_data,
            output_data: None,
            status: CommandStatus::Pending,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            execution_time_ms: None,
            error_message: None,
        };
        
        // Add job to edge node
        let mut edge_nodes = self.edge_nodes.write().await;
        if let Some(node) = edge_nodes.get_mut(node_id) {
            node.processing_jobs.push(job);
        }
        
        Ok(job_id)
    }

    /// Get device by ID
    pub async fn get_device(&self, device_id: &str) -> Option<IoTDevice> {
        let devices = self.devices.read().await;
        devices.get(device_id).cloned()
    }

    /// Get all devices
    pub async fn get_devices(&self) -> Vec<IoTDevice> {
        let devices = self.devices.read().await;
        devices.values().cloned().collect()
    }

    /// Get devices by type
    pub async fn get_devices_by_type(&self, device_type: &IoTDeviceType) -> Vec<IoTDevice> {
        let devices = self.devices.read().await;
        devices.values()
            .filter(|device| device.config.device_type == *device_type)
            .cloned()
            .collect()
    }

    /// Get devices by status
    pub async fn get_devices_by_status(&self, status: &IoTDeviceStatus) -> Vec<IoTDevice> {
        let devices = self.devices.read().await;
        devices.values()
            .filter(|device| device.status == *status)
            .cloned()
            .collect()
    }

    /// Get sensor data for device
    pub async fn get_sensor_data(&self, device_id: &str) -> Vec<SensorDataPoint> {
        let sensor_data = self.sensor_data.read().await;
        sensor_data.get(device_id).cloned().unwrap_or_default()
    }

    /// Get commands for device
    pub async fn get_commands(&self, device_id: &str) -> Vec<IoTCommand> {
        let commands = self.commands.read().await;
        commands.values()
            .filter(|command| command.device_id == device_id)
            .cloned()
            .collect()
    }

    /// Get alerts for device
    pub async fn get_alerts(&self, device_id: &str) -> Vec<IoTAlert> {
        let alerts = self.alerts.read().await;
        alerts.values()
            .filter(|alert| alert.device_id == device_id)
            .cloned()
            .collect()
    }

    /// Get edge node by ID
    pub async fn get_edge_node(&self, node_id: &str) -> Option<EdgeNode> {
        let edge_nodes = self.edge_nodes.read().await;
        edge_nodes.get(node_id).cloned()
    }

    /// Get all edge nodes
    pub async fn get_edge_nodes(&self) -> Vec<EdgeNode> {
        let edge_nodes = self.edge_nodes.read().await;
        edge_nodes.values().cloned().collect()
    }

    /// Get IoT system statistics
    pub async fn get_stats(&self) -> IoTSystemStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Get IoT system metrics
    pub async fn get_metrics(&self) -> IoTSystemMetrics {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }

    /// Enable IoT system
    pub async fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable IoT system
    pub async fn disable(&mut self) {
        self.enabled = false;
    }
}

impl Default for AdvancedIoTSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_iot_system_creation() {
        let iot_system = AdvancedIoTSystem::new();
        assert!(iot_system.enabled);
    }

    #[tokio::test]
    async fn test_device_registration() {
        let iot_system = AdvancedIoTSystem::new();
        
        let config = IoTDeviceConfig {
            device_type: IoTDeviceType::Sensor,
            name: "Temperature Sensor".to_string(),
            description: "Temperature sensor for room monitoring".to_string(),
            location: "Living Room".to_string(),
            manufacturer: "SensorCorp".to_string(),
            model: "TempSense-100".to_string(),
            firmware_version: "1.0.0".to_string(),
            hardware_version: "1.0".to_string(),
            capabilities: vec!["temperature_reading".to_string()],
            sensors: vec![SensorDataType::Temperature],
            actuators: vec![],
            communication_protocol: "WiFi".to_string(),
            power_source: "Battery".to_string(),
            battery_level: Some(85.0),
            update_interval_ms: 5000,
            data_retention_days: 30,
            encryption_enabled: true,
            authentication_required: true,
        };
        
        let device_id = iot_system.register_device(config).await.unwrap();
        assert!(!device_id.is_empty());
        
        let devices = iot_system.get_devices().await;
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].id, device_id);
    }

    #[tokio::test]
    async fn test_sensor_data_sending() {
        let iot_system = AdvancedIoTSystem::new();
        
        // Register device first
        let config = IoTDeviceConfig {
            device_type: IoTDeviceType::Sensor,
            name: "Temperature Sensor".to_string(),
            description: "Temperature sensor".to_string(),
            location: "Room".to_string(),
            manufacturer: "SensorCorp".to_string(),
            model: "TempSense-100".to_string(),
            firmware_version: "1.0.0".to_string(),
            hardware_version: "1.0".to_string(),
            capabilities: vec!["temperature_reading".to_string()],
            sensors: vec![SensorDataType::Temperature],
            actuators: vec![],
            communication_protocol: "WiFi".to_string(),
            power_source: "Battery".to_string(),
            battery_level: Some(85.0),
            update_interval_ms: 5000,
            data_retention_days: 30,
            encryption_enabled: true,
            authentication_required: true,
        };
        
        let device_id = iot_system.register_device(config).await.unwrap();
        
        // Send sensor data
        iot_system.send_sensor_data(&device_id, SensorDataType::Temperature, 23.5, "°C").await.unwrap();
        
        let sensor_data = iot_system.get_sensor_data(&device_id).await;
        assert_eq!(sensor_data.len(), 1);
        assert_eq!(sensor_data[0].value, 23.5);
        assert_eq!(sensor_data[0].unit, "°C");
    }

    #[tokio::test]
    async fn test_command_sending() {
        let iot_system = AdvancedIoTSystem::new();
        
        // Register device first
        let config = IoTDeviceConfig {
            device_type: IoTDeviceType::Actuator,
            name: "Smart Light".to_string(),
            description: "Smart light bulb".to_string(),
            location: "Living Room".to_string(),
            manufacturer: "LightCorp".to_string(),
            model: "SmartLight-200".to_string(),
            firmware_version: "1.0.0".to_string(),
            hardware_version: "1.0".to_string(),
            capabilities: vec!["light_control".to_string()],
            sensors: vec![],
            actuators: vec!["light_switch".to_string()],
            communication_protocol: "WiFi".to_string(),
            power_source: "Mains".to_string(),
            battery_level: None,
            update_interval_ms: 1000,
            data_retention_days: 7,
            encryption_enabled: true,
            authentication_required: true,
        };
        
        let device_id = iot_system.register_device(config).await.unwrap();
        
        // Send command
        let mut parameters = HashMap::new();
        parameters.insert("brightness".to_string(), serde_json::Value::Number(serde_json::Number::from(80)));
        
        let command_id = iot_system.send_command(&device_id, "set_brightness", parameters).await.unwrap();
        assert!(!command_id.is_empty());
        
        let commands = iot_system.get_commands(&device_id).await;
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].id, command_id);
    }

    #[tokio::test]
    async fn test_alert_creation() {
        let iot_system = AdvancedIoTSystem::new();
        
        // Register device first
        let config = IoTDeviceConfig {
            device_type: IoTDeviceType::Sensor,
            name: "Temperature Sensor".to_string(),
            description: "Temperature sensor".to_string(),
            location: "Room".to_string(),
            manufacturer: "SensorCorp".to_string(),
            model: "TempSense-100".to_string(),
            firmware_version: "1.0.0".to_string(),
            hardware_version: "1.0".to_string(),
            capabilities: vec!["temperature_reading".to_string()],
            sensors: vec![SensorDataType::Temperature],
            actuators: vec![],
            communication_protocol: "WiFi".to_string(),
            power_source: "Battery".to_string(),
            battery_level: Some(85.0),
            update_interval_ms: 5000,
            data_retention_days: 30,
            encryption_enabled: true,
            authentication_required: true,
        };
        
        let device_id = iot_system.register_device(config).await.unwrap();
        
        // Create alert
        let alert_id = iot_system.create_alert(&device_id, "high_temperature", AlertSeverity::High, "Temperature is too high").await.unwrap();
        assert!(!alert_id.is_empty());
        
        let alerts = iot_system.get_alerts(&device_id).await;
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].id, alert_id);
    }

    #[tokio::test]
    async fn test_edge_node_registration() {
        let iot_system = AdvancedIoTSystem::new();
        
        let node_id = iot_system.register_edge_node(
            EdgeNodeType::Gateway,
            "Gateway-001",
            "Building A",
            vec!["data_processing".to_string(), "device_management".to_string()]
        ).await.unwrap();
        
        assert!(!node_id.is_empty());
        
        let edge_nodes = iot_system.get_edge_nodes().await;
        assert_eq!(edge_nodes.len(), 1);
        assert_eq!(edge_nodes[0].id, node_id);
    }

    #[tokio::test]
    async fn test_iot_system_stats() {
        let iot_system = AdvancedIoTSystem::new();
        
        // Register a device
        let config = IoTDeviceConfig {
            device_type: IoTDeviceType::Sensor,
            name: "Test Sensor".to_string(),
            description: "Test sensor".to_string(),
            location: "Test Room".to_string(),
            manufacturer: "TestCorp".to_string(),
            model: "Test-100".to_string(),
            firmware_version: "1.0.0".to_string(),
            hardware_version: "1.0".to_string(),
            capabilities: vec!["test_reading".to_string()],
            sensors: vec![SensorDataType::Temperature],
            actuators: vec![],
            communication_protocol: "WiFi".to_string(),
            power_source: "Battery".to_string(),
            battery_level: Some(85.0),
            update_interval_ms: 5000,
            data_retention_days: 30,
            encryption_enabled: true,
            authentication_required: true,
        };
        
        iot_system.register_device(config).await.unwrap();
        
        let stats = iot_system.get_stats().await;
        assert_eq!(stats.total_devices, 1);
        assert_eq!(stats.online_devices, 1);
    }
} 