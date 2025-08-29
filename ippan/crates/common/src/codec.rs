use serde::{Deserialize, Serialize};
use crate::{Error, Result, Transaction, crypto::Hash};

/// Network message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    /// Transaction message
    Transaction(Transaction),
    
    /// Block message
    Block(BlockMessage),
    
    /// Round message
    Round(RoundMessage),
    
    /// Time sync message
    TimeSync(TimeSyncMessage),
    
    /// Ping message
    Ping(PingMessage),
    
    /// Pong message
    Pong(PongMessage),
}

/// Block message for network transmission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockMessage {
    pub block_id: Hash,
    pub round_id: u64,
    pub block_time_us: u64,
    pub builder_id: Hash,
    pub tx_count: u32,
    pub merkle_root: Hash,
    pub hashtimer: Hash,
    pub parent_hashes: Vec<Hash>,
    pub transaction_ids: Vec<Hash>,
}

/// Round message for network transmission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundMessage {
    pub round_id: u64,
    pub start_time_us: u64,
    pub end_time_us: u64,
    pub verifier_set: Vec<Hash>,
    pub finality_threshold: usize,
    pub block_ids: Vec<Hash>,
    pub finality_signatures: Vec<(Hash, Vec<u8>)>, // (verifier_id, signature)
}

/// Time sync message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSyncMessage {
    pub peer_id: String,
    pub local_time_us: u64,
    pub sequence: u64,
}

/// Ping message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingMessage {
    pub sequence: u64,
    pub timestamp_us: u64,
}

/// Pong message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PongMessage {
    pub sequence: u64,
    pub timestamp_us: u64,
    pub response_time_us: u64,
}

/// Encode a network message to binary
pub fn encode_message<T: Serialize>(message: &T) -> Result<Vec<u8>> {
    bincode::serialize(message).map_err(|e| Error::Serialization(e.to_string()))
}

/// Decode a network message from binary
pub fn decode_message<T: for<'de> Deserialize<'de>>(data: &[u8]) -> Result<T> {
    bincode::deserialize(data).map_err(|e| Error::Serialization(e.to_string()))
}

/// Encode a transaction to binary
pub fn encode_transaction(tx: &Transaction) -> Result<Vec<u8>> {
    encode_message(tx)
}

/// Decode a transaction from binary
pub fn decode_transaction(data: &[u8]) -> Result<Transaction> {
    decode_message(data)
}

/// Encode a network message to hex string
pub fn encode_message_hex<T: Serialize>(message: &T) -> Result<String> {
    let binary = encode_message(message)?;
    Ok(hex::encode(binary))
}

/// Decode a network message from hex string
pub fn decode_message_hex<T: for<'de> Deserialize<'de>>(hex_str: &str) -> Result<T> {
    let binary = hex::decode(hex_str).map_err(|e| Error::Serialization(e.to_string()))?;
    decode_message(&binary)
}

/// Get message type identifier
pub fn get_message_type(data: &[u8]) -> Result<&'static str> {
    // Try to deserialize as NetworkMessage to determine type
    match decode_message::<NetworkMessage>(data) {
        Ok(msg) => match msg {
            NetworkMessage::Transaction(_) => Ok("Transaction"),
            NetworkMessage::Block(_) => Ok("Block"),
            NetworkMessage::Round(_) => Ok("Round"),
            NetworkMessage::TimeSync(_) => Ok("TimeSync"),
            NetworkMessage::Ping(_) => Ok("Ping"),
            NetworkMessage::Pong(_) => Ok("Pong"),
        },
        Err(_) => {
            // Try as Transaction directly
            match decode_message::<Transaction>(data) {
                Ok(_) => Ok("Transaction"),
                Err(_) => Ok("Unknown"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::KeyPair;

    #[test]
    fn test_encode_decode_transaction() {
        let keypair = KeyPair::generate();
        let tx = Transaction::new(
            keypair.public_key,
            [1u8; 32],
            1000,
            1,
            1234567890,
            [2u8; 32],
            [3u8; 64],
        );
        
        let encoded = encode_transaction(&tx).unwrap();
        let decoded = decode_transaction(&encoded).unwrap();
        
        assert_eq!(tx.ver, decoded.ver);
        assert_eq!(tx.from_pub, decoded.from_pub);
        assert_eq!(tx.to_addr, decoded.to_addr);
        assert_eq!(tx.amount, decoded.amount);
        assert_eq!(tx.nonce, decoded.nonce);
        assert_eq!(tx.ippan_time_us, decoded.ippan_time_us);
        assert_eq!(tx.hashtimer, decoded.hashtimer);
        assert_eq!(tx.sig, decoded.sig);
    }

    #[test]
    fn test_encode_decode_network_message() {
        let keypair = KeyPair::generate();
        let tx = Transaction::new(
            keypair.public_key,
            [1u8; 32],
            1000,
            1,
            1234567890,
            [2u8; 32],
            [3u8; 64],
        );
        
        let message = NetworkMessage::Transaction(tx);
        let encoded = encode_message(&message).unwrap();
        let decoded: NetworkMessage = decode_message(&encoded).unwrap();
        
        match decoded {
            NetworkMessage::Transaction(decoded_tx) => {
                match message {
                    NetworkMessage::Transaction(original_tx) => {
                        assert_eq!(original_tx.ver, decoded_tx.ver);
                        assert_eq!(original_tx.amount, decoded_tx.amount);
                    }
                    _ => panic!("Unexpected message type"),
                }
            }
            _ => panic!("Unexpected decoded message type"),
        }
    }

    #[test]
    fn test_hex_encoding() {
        let ping = PingMessage {
            sequence: 123,
            timestamp_us: 456789,
        };
        
        let hex_str = encode_message_hex(&ping).unwrap();
        let decoded: PingMessage = decode_message_hex(&hex_str).unwrap();
        
        assert_eq!(ping.sequence, decoded.sequence);
        assert_eq!(ping.timestamp_us, decoded.timestamp_us);
    }

    #[test]
    fn test_message_type_detection() {
        let keypair = KeyPair::generate();
        let tx = Transaction::new(
            keypair.public_key,
            [1u8; 32],
            1000,
            1,
            1234567890,
            [2u8; 32],
            [3u8; 64],
        );
        
        let message = NetworkMessage::Transaction(tx);
        let encoded = encode_message(&message).unwrap();
        
        let msg_type = get_message_type(&encoded).unwrap();
        assert_eq!(msg_type, "Transaction");
    }
}
