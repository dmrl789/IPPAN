use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use crate::Result;
use crate::wallet::WalletManager;
use crate::utils::crypto;

/// M2M Payment System for IoT devices and AI agents
pub struct M2MPaymentSystem {
    /// Active payment channels
    payment_channels: HashMap<String, PaymentChannel>,
    /// Micro-payment transactions
    micro_transactions: Vec<MicroTransaction>,
    /// Payment channel counter
    channel_counter: u64,
}

/// Payment channel between two parties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentChannel {
    /// Channel ID
    pub channel_id: String,
    /// Sender address
    pub sender: String,
    /// Recipient address
    pub recipient: String,
    /// Total amount deposited
    pub total_deposit: u64,
    /// Amount already spent
    pub spent_amount: u64,
    /// Available balance
    pub available_balance: u64,
    /// Channel state
    pub state: ChannelState,
    /// Creation timestamp
    pub created_at: u64,
    /// Last update timestamp
    pub last_updated: u64,
    /// Channel timeout
    pub timeout: u64,
}

/// Payment channel state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChannelState {
    Open,
    Closing,
    Closed,
    Disputed,
}

/// Micro-transaction within a payment channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicroTransaction {
    /// Transaction ID
    pub tx_id: String,
    /// Channel ID
    pub channel_id: String,
    /// Amount in smallest units
    pub amount: u64,
    /// Transaction type
    pub tx_type: MicroTransactionType,
    /// Timestamp
    pub timestamp: u64,
    /// Fee amount (1% of transaction)
    pub fee_amount: u64,
    /// Sender signature
    pub sender_signature: Option<Vec<u8>>,
    /// Recipient signature
    pub recipient_signature: Option<Vec<u8>>,
}

/// Micro-transaction types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MicroTransactionType {
    /// Data transfer payment
    DataTransfer { bytes_transferred: u64 },
    /// Compute resource payment
    ComputeResource { cpu_seconds: f64, memory_mb: f64 },
    /// Storage payment
    Storage { bytes_stored: u64, duration_hours: u64 },
    /// API call payment
    ApiCall { endpoint: String, complexity: u32 },
    /// IoT sensor data payment
    SensorData { sensor_type: String, data_points: u32 },
    /// AI model inference payment
    ModelInference { model_name: String, input_tokens: u32 },
    /// Custom service payment
    CustomService { service_name: String, units: u32 },
}

/// Payment channel update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelUpdate {
    /// Channel ID
    pub channel_id: String,
    /// New balance
    pub new_balance: u64,
    /// Transaction amount
    pub tx_amount: u64,
    /// Sequence number
    pub sequence: u64,
    /// Sender signature
    pub sender_signature: Vec<u8>,
    /// Timestamp
    pub timestamp: u64,
}

impl M2MPaymentSystem {
    /// Create a new M2M payment system
    pub fn new() -> Self {
        Self {
            payment_channels: HashMap::new(),
            micro_transactions: Vec::new(),
            channel_counter: 0,
        }
    }

    /// Create a new payment channel
    pub fn create_payment_channel(
        &mut self,
        sender: String,
        recipient: String,
        deposit_amount: u64,
        timeout_hours: u64,
    ) -> Result<PaymentChannel> {
        if deposit_amount == 0 {
            return Err(crate::IppanError::Wallet("Deposit amount must be greater than 0".to_string()));
        }

        self.channel_counter += 1;
        let channel_id = format!("channel_{}_{}", sender, self.channel_counter);
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let channel = PaymentChannel {
            channel_id: channel_id.clone(),
            sender,
            recipient,
            total_deposit: deposit_amount,
            spent_amount: 0,
            available_balance: deposit_amount,
            state: ChannelState::Open,
            created_at: now,
            last_updated: now,
            timeout: now + (timeout_hours * 3600),
        };

        self.payment_channels.insert(channel_id.clone(), channel.clone());
        
        Ok(channel)
    }

    /// Process a micro-payment within a channel
    pub fn process_micro_payment(
        &mut self,
        channel_id: &str,
        amount: u64,
        tx_type: MicroTransactionType,
    ) -> Result<MicroTransaction> {
        let channel = self.payment_channels.get_mut(channel_id)
            .ok_or_else(|| crate::IppanError::Wallet("Payment channel not found".to_string()))?;

        if channel.state != ChannelState::Open {
            return Err(crate::IppanError::Wallet("Payment channel is not open".to_string()));
        }

        if amount > channel.available_balance {
            return Err(crate::IppanError::Wallet("Insufficient balance in payment channel".to_string()));
        }

        // Calculate fee (1% of transaction)
        let fee_amount = amount / 100;
        let total_amount = amount + fee_amount;

        if total_amount > channel.available_balance {
            return Err(crate::IppanError::Wallet("Insufficient balance including fees".to_string()));
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Create micro-transaction
        let tx_id = format!("micro_tx_{}_{}", channel_id, now);
        let micro_tx = MicroTransaction {
            tx_id,
            channel_id: channel_id.to_string(),
            amount,
            tx_type,
            timestamp: now,
            fee_amount,
            sender_signature: None,
            recipient_signature: None,
        };

        // Update channel balance
        channel.spent_amount += total_amount;
        channel.available_balance -= total_amount;
        channel.last_updated = now;

        self.micro_transactions.push(micro_tx.clone());

        Ok(micro_tx)
    }

    /// Close a payment channel
    pub fn close_payment_channel(&mut self, channel_id: &str) -> Result<ChannelUpdate> {
        let channel = self.payment_channels.get_mut(channel_id)
            .ok_or_else(|| crate::IppanError::Wallet("Payment channel not found".to_string()))?;

        if channel.state != ChannelState::Open {
            return Err(crate::IppanError::Wallet("Payment channel is not open".to_string()));
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        channel.state = ChannelState::Closing;
        channel.last_updated = now;

        let update = ChannelUpdate {
            channel_id: channel_id.to_string(),
            new_balance: channel.available_balance,
            tx_amount: 0,
            sequence: 0,
            sender_signature: Vec::new(),
            timestamp: now,
        };

        Ok(update)
    }

    /// Get payment channel information
    pub fn get_payment_channel(&self, channel_id: &str) -> Option<&PaymentChannel> {
        self.payment_channels.get(channel_id)
    }

    /// Get all payment channels for an address
    pub fn get_channels_for_address(&self, address: &str) -> Vec<&PaymentChannel> {
        self.payment_channels
            .values()
            .filter(|channel| channel.sender == address || channel.recipient == address)
            .collect()
    }

    /// Get micro-transactions for a channel
    pub fn get_channel_transactions(&self, channel_id: &str) -> Vec<&MicroTransaction> {
        self.micro_transactions
            .iter()
            .filter(|tx| tx.channel_id == channel_id)
            .collect()
    }

    /// Calculate total fees collected
    pub fn get_total_fees_collected(&self) -> u64 {
        self.micro_transactions
            .iter()
            .map(|tx| tx.fee_amount)
            .sum()
    }

    /// Get payment statistics
    pub fn get_payment_statistics(&self) -> PaymentStatistics {
        let total_channels = self.payment_channels.len();
        let open_channels = self.payment_channels
            .values()
            .filter(|c| matches!(c.state, ChannelState::Open))
            .count();
        
        let total_transactions = self.micro_transactions.len();
        let total_volume = self.micro_transactions
            .iter()
            .map(|tx| tx.amount)
            .sum();
        
        let total_fees = self.get_total_fees_collected();

        PaymentStatistics {
            total_channels,
            open_channels,
            total_transactions,
            total_volume,
            total_fees,
            average_transaction_size: if total_transactions > 0 {
                total_volume / total_transactions as u64
            } else {
                0
            },
        }
    }

    /// Clean up expired channels
    pub fn cleanup_expired_channels(&mut self) -> usize {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let expired_channels: Vec<String> = self.payment_channels
            .iter()
            .filter(|(_, channel)| {
                matches!(channel.state, ChannelState::Open) && channel.timeout < now
            })
            .map(|(id, _)| id.clone())
            .collect();

        for channel_id in &expired_channels {
            if let Some(channel) = self.payment_channels.get_mut(channel_id) {
                channel.state = ChannelState::Closed;
            }
        }

        expired_channels.len()
    }
}

/// Payment statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentStatistics {
    /// Total number of payment channels
    pub total_channels: usize,
    /// Number of open channels
    pub open_channels: usize,
    /// Total number of micro-transactions
    pub total_transactions: usize,
    /// Total transaction volume
    pub total_volume: u64,
    /// Total fees collected
    pub total_fees: u64,
    /// Average transaction size
    pub average_transaction_size: u64,
}

/// IoT device payment handler
pub struct IoTDevicePayment {
    /// Device ID
    pub device_id: String,
    /// Payment channel ID
    pub channel_id: String,
    /// Device capabilities
    pub capabilities: Vec<DeviceCapability>,
}

/// Device capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceCapability {
    /// Temperature sensor
    TemperatureSensor,
    /// Humidity sensor
    HumiditySensor,
    /// Motion detector
    MotionDetector,
    /// Camera
    Camera,
    /// Microphone
    Microphone,
    /// GPS
    Gps,
    /// Actuator
    Actuator,
    /// Custom capability
    Custom(String),
}

impl IoTDevicePayment {
    /// Create new IoT device payment handler
    pub fn new(device_id: String, channel_id: String) -> Self {
        Self {
            device_id,
            channel_id,
            capabilities: Vec::new(),
        }
    }

    /// Add device capability
    pub fn add_capability(&mut self, capability: DeviceCapability) {
        self.capabilities.push(capability);
    }

    /// Process sensor data payment
    pub fn process_sensor_payment(
        &self,
        payment_system: &mut M2MPaymentSystem,
        sensor_type: String,
        data_points: u32,
    ) -> Result<MicroTransaction> {
        let base_amount = 1; // 1 satoshi per data point
        let amount = base_amount * data_points as u64;

        payment_system.process_micro_payment(
            &self.channel_id,
            amount,
            MicroTransactionType::SensorData {
                sensor_type,
                data_points,
            },
        )
    }

    /// Process compute resource payment
    pub fn process_compute_payment(
        &self,
        payment_system: &mut M2MPaymentSystem,
        cpu_seconds: f64,
        memory_mb: f64,
    ) -> Result<MicroTransaction> {
        let cpu_amount = (cpu_seconds * 100.0) as u64; // 100 satoshi per CPU second
        let memory_amount = (memory_mb * 10.0) as u64; // 10 satoshi per MB
        let amount = cpu_amount + memory_amount;

        payment_system.process_micro_payment(
            &self.channel_id,
            amount,
            MicroTransactionType::ComputeResource {
                cpu_seconds,
                memory_mb,
            },
        )
    }
}

/// AI agent payment handler
pub struct AIAgentPayment {
    /// Agent ID
    pub agent_id: String,
    /// Payment channel ID
    pub channel_id: String,
    /// Agent model
    pub model_name: String,
}

impl AIAgentPayment {
    /// Create new AI agent payment handler
    pub fn new(agent_id: String, channel_id: String, model_name: String) -> Self {
        Self {
            agent_id,
            channel_id,
            model_name,
        }
    }

    /// Process model inference payment
    pub fn process_inference_payment(
        &self,
        payment_system: &mut M2MPaymentSystem,
        input_tokens: u32,
    ) -> Result<MicroTransaction> {
        let amount = input_tokens as u64 * 2; // 2 satoshi per token

        payment_system.process_micro_payment(
            &self.channel_id,
            amount,
            MicroTransactionType::ModelInference {
                model_name: self.model_name.clone(),
                input_tokens,
            },
        )
    }

    /// Process API call payment
    pub fn process_api_payment(
        &self,
        payment_system: &mut M2MPaymentSystem,
        endpoint: String,
        complexity: u32,
    ) -> Result<MicroTransaction> {
        let amount = complexity as u64 * 5; // 5 satoshi per complexity unit

        payment_system.process_micro_payment(
            &self.channel_id,
            amount,
            MicroTransactionType::ApiCall {
                endpoint,
                complexity,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payment_channel_creation() {
        let mut payment_system = M2MPaymentSystem::new();
        let channel = payment_system.create_payment_channel(
            "alice".to_string(),
            "bob".to_string(),
            1000,
            24,
        ).unwrap();

        assert_eq!(channel.sender, "alice");
        assert_eq!(channel.recipient, "bob");
        assert_eq!(channel.total_deposit, 1000);
        assert_eq!(channel.available_balance, 1000);
        assert!(matches!(channel.state, ChannelState::Open));
    }

    #[test]
    fn test_micro_payment_processing() {
        let mut payment_system = M2MPaymentSystem::new();
        let channel = payment_system.create_payment_channel(
            "alice".to_string(),
            "bob".to_string(),
            1000,
            24,
        ).unwrap();

        let micro_tx = payment_system.process_micro_payment(
            &channel.channel_id,
            100,
            MicroTransactionType::DataTransfer { bytes_transferred: 1024 },
        ).unwrap();

        assert_eq!(micro_tx.amount, 100);
        assert_eq!(micro_tx.fee_amount, 1); // 1% fee
        assert_eq!(micro_tx.channel_id, channel.channel_id);
    }

    #[test]
    fn test_iot_device_payment() {
        let mut payment_system = M2MPaymentSystem::new();
        let channel = payment_system.create_payment_channel(
            "iot_device".to_string(),
            "data_consumer".to_string(),
            10000,
            168, // 1 week
        ).unwrap();

        let mut iot_payment = IoTDevicePayment::new(
            "sensor_001".to_string(),
            channel.channel_id.clone(),
        );
        iot_payment.add_capability(DeviceCapability::TemperatureSensor);

        let sensor_tx = iot_payment.process_sensor_payment(
            &mut payment_system,
            "temperature".to_string(),
            10,
        ).unwrap();

        assert_eq!(sensor_tx.amount, 10); // 1 satoshi per data point
    }

    #[test]
    fn test_ai_agent_payment() {
        let mut payment_system = M2MPaymentSystem::new();
        let channel = payment_system.create_payment_channel(
            "ai_agent".to_string(),
            "service_provider".to_string(),
            50000,
            168, // 1 week
        ).unwrap();

        let ai_payment = AIAgentPayment::new(
            "gpt_agent".to_string(),
            channel.channel_id.clone(),
            "gpt-4".to_string(),
        );

        let inference_tx = ai_payment.process_inference_payment(
            &mut payment_system,
            100, // 100 tokens
        ).unwrap();

        assert_eq!(inference_tx.amount, 200); // 2 satoshi per token
    }
} 