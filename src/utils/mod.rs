//! Utility modules for IPPAN
//! 
//! This module provides common utilities used throughout the IPPAN codebase.

pub mod address;
pub mod config;
pub mod crypto;
pub mod logging;
pub mod time;
pub mod performance;
pub mod optimization;

pub use address::*;
pub use config::*;
pub use crypto::*;
pub use logging::*;
pub use time::*;
pub use performance::*;
pub use optimization::*;

use std::time::{SystemTime, UNIX_EPOCH};
use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};

/// Generate a unique identifier based on current time and random data
pub fn generate_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    
    let random_bytes: [u8; 8] = rand::random();
    let combined = format!("{}{:016x}", timestamp, u64::from_le_bytes(random_bytes));
    
    // Hash the combined string for a shorter, consistent ID
    let mut hasher = Sha256::new();
    hasher.update(combined.as_bytes());
    let result = hasher.finalize();
    
    // Return first 16 characters of hex string
    hex::encode(&result[..8])
}

/// Calculate the hash of data
pub fn hash_data(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    result.into()
}

/// Convert bytes to hex string
pub fn bytes_to_hex(bytes: &[u8]) -> String {
    hex::encode(bytes)
}

/// Convert hex string to bytes
pub fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    Ok(hex::decode(hex)?)
}

/// Serialize data to bytes
pub fn serialize<T: Serialize>(data: &T) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    Ok(bincode::serialize(data)?)
}

/// Deserialize data from bytes
pub fn deserialize<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T, Box<dyn std::error::Error>> {
    Ok(bincode::deserialize(bytes)?)
}

/// Format bytes as human readable string
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    format!("{:.2} {}", size, UNITS[unit_index])
}

/// Format IPN amount in smallest units to human readable
pub fn format_ipn(amount: u64) -> String {
    let ipn = amount as f64 / 100_000_000.0; // 8 decimals
    format!("{:.8} IPN", ipn)
}

/// Parse IPN amount from string to smallest units
pub fn parse_ipn(amount_str: &str) -> Result<u64, Box<dyn std::error::Error>> {
    let amount: f64 = amount_str.parse()?;
    let smallest_units = (amount * 100_000_000.0) as u64;
    Ok(smallest_units)
}

/// Retry operation with exponential backoff
pub async fn retry_with_backoff<F, Fut, T, E>(
    mut operation: F,
    max_retries: usize,
    initial_delay: std::time::Duration,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Debug,
{
    let mut delay = initial_delay;
    
    for attempt in 0..=max_retries {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                if attempt == max_retries {
                    return Err(e);
                }
                
                tracing::warn!("Operation failed (attempt {}/{}): {:?}, retrying in {:?}", 
                    attempt + 1, max_retries + 1, e, delay);
                
                tokio::time::sleep(delay).await;
                delay *= 2; // Exponential backoff
            }
        }
    }
    
    unreachable!()
}

/// Validate domain name format
pub fn validate_domain_name(domain: &str) -> bool {
    if domain.is_empty() || domain.len() > 253 {
        return false;
    }
    
    // Check for valid characters and structure
    let parts: Vec<&str> = domain.split('.').collect();
    if parts.len() < 2 {
        return false;
    }
    
    for part in parts {
        if part.is_empty() || part.len() > 63 {
            return false;
        }
        
        // Check for valid characters (alphanumeric and hyphens, but not at start/end)
        if !part.chars().all(|c| c.is_alphanumeric() || c == '-') {
            return false;
        }
        
        if part.starts_with('-') || part.ends_with('-') {
            return false;
        }
    }
    
    true
}

/// Validate IPN address format
pub fn validate_ipn_address(address: &str) -> bool {
    // IPN addresses are Ed25519 public keys encoded in base58
    if address.len() != 44 {
        return false;
    }
    
    // Check if it's valid base58
    bs58::decode(address).into_vec().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_id() {
        let id1 = generate_id();
        let id2 = generate_id();
        
        assert_eq!(id1.len(), 16);
        assert_eq!(id2.len(), 16);
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_hash_data() {
        let data = b"test data";
        let hash = hash_data(data);
        
        assert_eq!(hash.len(), 32);
        
        // Same data should produce same hash
        let hash2 = hash_data(data);
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_format_ipn() {
        assert_eq!(format_ipn(100_000_000), "1.00000000 IPN");
        assert_eq!(format_ipn(1_000_000), "0.01000000 IPN");
        assert_eq!(format_ipn(0), "0.00000000 IPN");
    }

    #[test]
    fn test_parse_ipn() {
        assert_eq!(parse_ipn("1.0").unwrap(), 100_000_000);
        assert_eq!(parse_ipn("0.01").unwrap(), 1_000_000);
        assert_eq!(parse_ipn("0").unwrap(), 0);
    }

    #[test]
    fn test_validate_domain_name() {
        assert!(validate_domain_name("example.ipn"));
        assert!(validate_domain_name("test.example.ipn"));
        assert!(validate_domain_name("valid-domain.ipn"));
        
        assert!(!validate_domain_name(""));
        assert!(!validate_domain_name("invalid"));
        assert!(!validate_domain_name(".ipn"));
        assert!(!validate_domain_name("example."));
        assert!(!validate_domain_name("-invalid.ipn"));
        assert!(!validate_domain_name("invalid-.ipn"));
    }
}
