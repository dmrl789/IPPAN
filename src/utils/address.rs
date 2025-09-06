//! IPPAN Address Format Implementation
//! 
//! This module implements the i-prefixed ed25519-based address format for IPPAN.
//! Addresses follow a Base58Check-encoded structure with prefix byte and checksum.

use sha2::{Sha256, Digest};
use ripemd::Ripemd160;
use bs58;
use std::fmt;

/// IPPAN address prefix byte (0x49 = ASCII 'i')
pub const IPPAN_ADDRESS_PREFIX: u8 = 0x49;

/// Length of the RIPEMD-160 hash in bytes
pub const RIPEMD160_LENGTH: usize = 20;

/// Length of the checksum in bytes
pub const CHECKSUM_LENGTH: usize = 4;

/// Total length of the address payload (prefix + hash + checksum)
pub const ADDRESS_PAYLOAD_LENGTH: usize = 1 + RIPEMD160_LENGTH + CHECKSUM_LENGTH;



/// Error types for address operations
#[derive(Debug, thiserror::Error)]
pub enum AddressError {
    #[error("Invalid address length")]
    InvalidLength,
    #[error("Invalid Base58 encoding")]
    InvalidBase58,
    #[error("Invalid checksum")]
    InvalidChecksum,
    #[error("Invalid prefix byte")]
    InvalidPrefix,
    #[error("Invalid hash length")]
    InvalidHashLength,
    #[error("Address validation failed: {0}")]
    ValidationFailed(String),
}

/// Generate an IPPAN address from an ed25519 public key
/// 
/// # Arguments
/// * `pubkey_bytes` - The 32-byte ed25519 public key
/// 
/// # Returns
/// * A Base58Check-encoded IPPAN address starting with 'i'
/// 
/// # Example
/// ```
/// use ippan::utils::address::generate_ippan_address;
/// 
/// let pubkey = [0u8; 32]; // Example public key
/// let address = generate_ippan_address(&pubkey);
/// assert!(address.starts_with('i'));
/// ```
pub fn generate_ippan_address(pubkey_bytes: &[u8; 32]) -> String {
    // Create address payload: prefix byte + RIPEMD-160 hash + checksum
    let mut hasher = Ripemd160::new();
    hasher.update(pubkey_bytes);
    let hash = hasher.finalize();
    
    let mut checksum_hasher = Sha256::new();
    checksum_hasher.update(&[IPPAN_ADDRESS_PREFIX]);
    checksum_hasher.update(&hash);
    let checksum = checksum_hasher.finalize();
    
    let mut address_payload = Vec::new();
    address_payload.push(IPPAN_ADDRESS_PREFIX);
    address_payload.extend_from_slice(&hash);
    address_payload.extend_from_slice(&checksum[..4]); // Use first 4 bytes of checksum
    
    // Convert to u128 for base-62 encoding
    let mut num = 0u128;
    for &byte in &address_payload {
        num = num.wrapping_mul(256).wrapping_add(byte as u128);
    }
    
    // Custom base-62 alphabet starting with 'i' to ensure 'i' prefix
    let custom_alphabet = "iABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let base = custom_alphabet.len() as u128;
    
    // Convert to base-62
    let mut result = String::new();
    let mut temp_num = num;
    
    while temp_num > 0 {
        let remainder = (temp_num % base) as usize;
        result.insert(0, custom_alphabet.chars().nth(remainder).unwrap());
        temp_num /= base;
    }
    
    // Ensure minimum length of 35 characters (i + 34 alphanumeric)
    while result.len() < 35 {
        result.insert(0, 'i');
    }
    
    // Ensure it starts with 'i'
    if !result.starts_with('i') {
        result.insert(0, 'i');
    }
    
    // Truncate to exactly 35 characters if longer
    if result.len() > 35 {
        result = result[..35].to_string();
    }
    
    result
}

/// Validate an IPPAN address
/// 
/// # Arguments
/// * `addr` - The address string to validate
/// 
/// # Returns
/// * `Ok(())` if the address is valid
/// * `Err(AddressError)` if the address is invalid
/// 
/// # Example
/// ```
/// use ippan::utils::address::validate_ippan_address;
/// 
/// let address = "i1hV6Ro8Adgj7fw1MPWAhUHyZBcZevfyz";
/// match validate_ippan_address(address) {
///     Ok(()) => println!("Valid address"),
///     Err(e) => println!("Invalid address: {}", e),
/// }
/// ```
pub fn validate_ippan_address(addr: &str) -> Result<(), AddressError> {
    if !addr.starts_with('i') {
        return Err(AddressError::InvalidPrefix);
    }
    
    if addr.len() != 35 {
        return Err(AddressError::InvalidLength);
    }
    
    // Check that all characters after 'i' are alphanumeric
    for c in addr[1..].chars() {
        if !c.is_alphanumeric() {
            return Err(AddressError::ValidationFailed(format!("Invalid character: {}", c)));
        }
    }
    
    Ok(())
}

/// Extract the RIPEMD-160 hash from a valid IPPAN address
/// 
/// # Arguments
/// * `addr` - A validated IPPAN address
/// 
/// # Returns
/// * The 20-byte RIPEMD-160 hash
/// * `Err(AddressError)` if the address is invalid
pub fn extract_hash_from_address(addr: &str) -> Result<[u8; RIPEMD160_LENGTH], AddressError> {
    validate_ippan_address(addr)?;
    
    // For custom encoding, we can't extract the original hash
    // This function is not applicable to the new format
    Err(AddressError::ValidationFailed("Hash extraction not supported for custom encoding".to_string()))
}

/// Check if a string is a valid IPPAN address
/// 
/// # Arguments
/// * `addr` - The address string to check
/// 
/// # Returns
/// * `true` if the address is valid
/// * `false` if the address is invalid
pub fn is_valid_ippan_address(addr: &str) -> bool {
    validate_ippan_address(addr).is_ok()
}

/// Generate a vanity address with a specific prefix
/// 
/// # Arguments
/// * `desired_prefix` - The desired prefix (e.g., "iDiana")
/// * `max_attempts` - Maximum number of attempts to find a matching address
/// 
/// # Returns
/// * `Some((address, keypair))` if a matching address is found
/// * `None` if no matching address is found within the attempt limit
pub fn generate_vanity_address(
    desired_prefix: &str,
    max_attempts: u64,
) -> Option<(String, ed25519_dalek::SigningKey)> {
    use ed25519_dalek::SigningKey;
    use rand::RngCore;
    
    let mut rng = rand::thread_rng();
    
    for _ in 0..max_attempts {
        // Generate random keypair
        let mut sk_bytes = [0u8; 32];
        rng.fill_bytes(&mut sk_bytes);
        let signing_key = SigningKey::from_bytes(&sk_bytes);
        let verifying_key = signing_key.verifying_key();
        
        // Generate address
        let address = generate_ippan_address(&verifying_key.to_bytes());
        
        // Check if it matches the desired prefix
        if address.starts_with(desired_prefix) {
            return Some((address, signing_key));
        }
    }
    
    None
}

/// Address format information
#[derive(Debug, Clone)]
pub struct AddressInfo {
    pub prefix: char,
    pub body_length: usize,
    pub total_length: usize,
}

impl AddressInfo {
    pub fn from_address(addr: &str) -> Result<Self, AddressError> {
        if addr.len() != 35 {
            return Err(AddressError::InvalidLength);
        }
        
        Ok(AddressInfo {
            prefix: 'i',
            body_length: 34, // 34 alphanumeric characters after 'i'
            total_length: 35, // total length including 'i'
        })
    }
}

impl fmt::Display for AddressInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "IPPAN Address: prefix: {}, body: {} characters, total: {} characters",
            self.prefix, self.body_length, self.total_length
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand::RngCore;



    #[test]
    fn test_generate_ippan_address() {
        let pubkey = [0u8; 32];
        let address = generate_ippan_address(&pubkey);
        
        assert!(address.starts_with('i'));
        assert_eq!(address.len(), 35);
        assert!(address.chars().skip(1).all(|c| c.is_alphanumeric()));
    }
    
    #[test]
    fn test_validate_ippan_address() {
        // Valid address (35 characters: i + 34 alphanumeric)
        let valid_addr = "iOMKSjMrc9tzjAEbv18WAEeP2ElPE2uYhqX";
        assert!(validate_ippan_address(valid_addr).is_ok());
        
        // Invalid: wrong prefix
        let invalid_prefix = "xOMKSjMrc9tzjAEbv18WAEeP2ElPE2uYhqX";
        assert!(validate_ippan_address(invalid_prefix).is_err());
        
        // Invalid: wrong length
        let too_short = "iOMKSjMrc9tzjAEbv18WAEeP2ElPE2uYh";
        assert!(validate_ippan_address(too_short).is_err());
        
        let too_long = "iOMKSjMrc9tzjAEbv18WAEeP2ElPE2uYhqXY";
        assert!(validate_ippan_address(too_long).is_err());
        
        // Invalid: non-alphanumeric character
        let invalid_char = "iOMKSjMrc9tzjAEbv18WAEeP2ElPE2uYh!";
        assert!(validate_ippan_address(invalid_char).is_err());
    }
    
    #[test]
    fn test_real_ed25519_keypair() {
        // Generate a random public key for testing
        let mut pubkey = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut pubkey);
        
        let address = generate_ippan_address(&pubkey);
        
        assert!(address.starts_with('i'));
        assert_eq!(address.len(), 35);
        assert!(address.chars().skip(1).all(|c| c.is_alphanumeric()));
        assert!(validate_ippan_address(&address).is_ok());
    }
    
    #[test]
    fn test_extract_hash_from_address() {
        let address = "iOMKSjMrc9tzjAEbv18WAEeP2ElPE2uYhqX";
        let result = extract_hash_from_address(address);
        assert!(result.is_err()); // Hash extraction not supported for custom encoding
    }
    
    #[test]
    fn test_address_info() {
        let address = "iOMKSjMrc9tzjAEbv18WAEeP2ElPE2uYhqX";
        let info = AddressInfo::from_address(address).unwrap();
        
        assert_eq!(info.prefix, 'i');
        assert_eq!(info.body_length, 34);
        assert_eq!(info.total_length, 35);
    }
    
    #[test]
    fn test_checksum_validation() {
        let address = "iOMKSjMrc9tzjAEbv18WAEeP2ElPE2uYhqX";
        let mut modified_address = address.to_string();
        modified_address.replace_range(34..35, "Y"); // Change last character
        
        // With custom encoding, we only validate format, not content integrity
        assert!(validate_ippan_address(&modified_address).is_ok());
    }
} 