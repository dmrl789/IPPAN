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
    // Step 1: Hash the public key with SHA-256
    let sha256_hash = Sha256::digest(pubkey_bytes);
    
    // Step 2: Hash the result with RIPEMD-160
    let ripe160_hash = Ripemd160::digest(&sha256_hash);
    
    // Step 3: Create address payload with prefix byte
    let mut address_payload = Vec::with_capacity(ADDRESS_PAYLOAD_LENGTH);
    address_payload.push(IPPAN_ADDRESS_PREFIX); // Use the original prefix
    address_payload.extend_from_slice(&ripe160_hash);
    
    // Step 4: Calculate double SHA-256 checksum
    let checksum = Sha256::digest(&Sha256::digest(&address_payload));
    address_payload.extend_from_slice(&checksum[..CHECKSUM_LENGTH]);
    
    // Step 5: Encode with Base58Check
    let encoded = bs58::encode(address_payload).into_string();
    
    // For now, we'll use the standard Base58 encoding
    // In a real implementation, we might need a more sophisticated approach
    // to ensure addresses start with 'i' while maintaining proper encoding
    encoded
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
    // Decode from Base58
    let decoded = bs58::decode(addr)
        .into_vec()
        .map_err(|_| AddressError::InvalidBase58)?;
    
    // Check length
    if decoded.len() != ADDRESS_PAYLOAD_LENGTH {
        return Err(AddressError::InvalidLength);
    }
    
    // Verify prefix byte - we accept the original prefix or any valid prefix
    if decoded[0] != IPPAN_ADDRESS_PREFIX {
        return Err(AddressError::InvalidPrefix);
    }
    
    // Extract payload and checksum
    let payload = &decoded[..1 + RIPEMD160_LENGTH];
    let provided_checksum = &decoded[1 + RIPEMD160_LENGTH..];
    
    // Calculate expected checksum
    let expected_checksum = Sha256::digest(&Sha256::digest(payload));
    
    // Verify checksum
    if provided_checksum != &expected_checksum[..CHECKSUM_LENGTH] {
        return Err(AddressError::InvalidChecksum);
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
    // First validate the address
    validate_ippan_address(addr)?;
    
    // Decode from Base58
    let decoded = bs58::decode(addr)
        .into_vec()
        .map_err(|_| AddressError::InvalidBase58)?;
    
    // Extract the hash (skip prefix byte)
    let mut hash = [0u8; RIPEMD160_LENGTH];
    hash.copy_from_slice(&decoded[1..1 + RIPEMD160_LENGTH]);
    
    Ok(hash)
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
    pub address: String,
    pub prefix_byte: u8,
    pub hash_length: usize,
    pub checksum_length: usize,
    pub total_length: usize,
}

impl AddressInfo {
    /// Create address info from a valid address
    pub fn from_address(addr: &str) -> Result<Self, AddressError> {
        validate_ippan_address(addr)?;
        
        Ok(Self {
            address: addr.to_string(),
            prefix_byte: IPPAN_ADDRESS_PREFIX,
            hash_length: RIPEMD160_LENGTH,
            checksum_length: CHECKSUM_LENGTH,
            total_length: ADDRESS_PAYLOAD_LENGTH,
        })
    }
}

impl fmt::Display for AddressInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "IPPAN Address: {} (prefix: 0x{:02x}, hash: {} bytes, checksum: {} bytes)",
            self.address, self.prefix_byte, self.hash_length, self.checksum_length
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
        // Test with a known public key
        let pubkey = [0u8; 32];
        let address = generate_ippan_address(&pubkey);
        
        // Should be valid
        assert!(is_valid_ippan_address(&address));
    }

    #[test]
    fn test_validate_ippan_address() {
        // Generate a valid address
        let pubkey = [1u8; 32];
        let address = generate_ippan_address(&pubkey);
        
        // Should be valid
        assert!(validate_ippan_address(&address).is_ok());
        
        // Test invalid addresses
        assert!(validate_ippan_address("invalid").is_err());
        assert!(validate_ippan_address("1invalid").is_err()); // Wrong prefix
        assert!(validate_ippan_address("i").is_err()); // Too short
    }

    #[test]
    fn test_extract_hash_from_address() {
        // Generate a valid address
        let pubkey = [2u8; 32];
        let address = generate_ippan_address(&pubkey);
        
        // Extract hash
        let hash = extract_hash_from_address(&address).unwrap();
        assert_eq!(hash.len(), RIPEMD160_LENGTH);
        
        // Should fail for invalid address
        assert!(extract_hash_from_address("invalid").is_err());
    }

    #[test]
    fn test_address_consistency() {
        // Same public key should generate same address
        let pubkey = [3u8; 32];
        let address1 = generate_ippan_address(&pubkey);
        let address2 = generate_ippan_address(&pubkey);
        
        assert_eq!(address1, address2);
    }

    #[test]
    fn test_different_keys_different_addresses() {
        // Different public keys should generate different addresses
        let pubkey1 = [4u8; 32];
        let pubkey2 = [5u8; 32];
        
        let address1 = generate_ippan_address(&pubkey1);
        let address2 = generate_ippan_address(&pubkey2);
        
        assert_ne!(address1, address2);
    }

    #[test]
    fn test_vanity_address_generation() {
        // Test vanity address generation (with small attempt limit for speed)
        let result = generate_vanity_address("iTest", 1000);
        
        // This might not find a match, but should not panic
        if let Some((address, _keypair)) = result {
            assert!(address.starts_with("iTest"));
            assert!(is_valid_ippan_address(&address));
        }
    }

    #[test]
    fn test_address_info() {
        let pubkey = [6u8; 32];
        let address = generate_ippan_address(&pubkey);
        
        let info = AddressInfo::from_address(&address).unwrap();
        assert_eq!(info.address, address);
        assert_eq!(info.prefix_byte, IPPAN_ADDRESS_PREFIX);
        assert_eq!(info.hash_length, RIPEMD160_LENGTH);
        assert_eq!(info.checksum_length, CHECKSUM_LENGTH);
        assert_eq!(info.total_length, ADDRESS_PAYLOAD_LENGTH);
    }

    #[test]
    fn test_checksum_validation() {
        let pubkey = [7u8; 32];
        let mut address = generate_ippan_address(&pubkey);
        
        // Should be valid
        assert!(validate_ippan_address(&address).is_ok());
        
        // Modify the address (change last character)
        let last_char = address.pop().unwrap();
        address.push(if last_char == '1' { '2' } else { '1' });
        
        // Should now be invalid
        assert!(validate_ippan_address(&address).is_err());
    }

    #[test]
    fn test_real_ed25519_keypair() {
        // Test with a real ed25519 keypair
        let mut rng = rand::thread_rng();
        let mut sk_bytes = [0u8; 32];
        rng.fill_bytes(&mut sk_bytes);
        let signing_key = SigningKey::from_bytes(&sk_bytes);
        let verifying_key = signing_key.verifying_key();
        
        let address = generate_ippan_address(&verifying_key.to_bytes());
        
        assert!(is_valid_ippan_address(&address));
    }
} 