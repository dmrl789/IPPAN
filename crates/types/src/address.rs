use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Errors that can occur when parsing an IPPAN address string.
#[derive(Debug, thiserror::Error)]
pub enum AddressError {
    #[error("address must start with 'i' or '1'")]
    InvalidPrefix,
    #[error("address must be between {min} and {max} characters, got {actual}")]
    InvalidLength {
        min: usize,
        max: usize,
        actual: usize,
    },
    #[error("address payload is not valid hexadecimal")]
    InvalidHex(#[from] hex::FromHexError),
    #[error("address payload must be exactly 32 bytes")]
    InvalidPayloadLength,
    #[error("invalid Base58 encoding")]
    InvalidBase58(#[from] bs58::decode::Error),
    #[error("invalid checksum in address")]
    InvalidChecksum,
    #[error("address version mismatch")]
    InvalidVersion,
}

/// Number of raw bytes contained in an address.
pub const ADDRESS_BYTES: usize = 32;
/// Expected string length of an encoded address (prefix + 64 hex chars).
pub const ADDRESS_STRING_LENGTH: usize = 1 + ADDRESS_BYTES * 2;
/// IPPAN address version byte
pub const ADDRESS_VERSION: u8 = 0x00;
/// Checksum length in bytes (first 4 bytes of double SHA256)
pub const CHECKSUM_LENGTH: usize = 4;

/// Calculate a checksum for Base58Check encoding using double SHA256
fn calculate_checksum(data: &[u8]) -> [u8; CHECKSUM_LENGTH] {
    let hash1 = Sha256::digest(data);
    let hash2 = Sha256::digest(&hash1);
    let mut checksum = [0u8; CHECKSUM_LENGTH];
    checksum.copy_from_slice(&hash2[..CHECKSUM_LENGTH]);
    checksum
}

/// Encode a 32-byte account identifier into Base58Check format with IPPAN prefix
///
/// The encoded address format: version_byte (1) + public_key (32) + checksum (4)
/// This is then Base58 encoded for a compact, checksummed representation.
pub fn encode_address_base58check(bytes: &[u8; ADDRESS_BYTES]) -> String {
    let mut payload = Vec::with_capacity(1 + ADDRESS_BYTES + CHECKSUM_LENGTH);

    // Add version byte
    payload.push(ADDRESS_VERSION);

    // Add public key bytes
    payload.extend_from_slice(bytes);

    // Calculate and add checksum
    let checksum = calculate_checksum(&payload);
    payload.extend_from_slice(&checksum);

    // Encode to Base58
    bs58::encode(payload).into_string()
}

/// Decode a Base58Check IPPAN address string into raw bytes
pub fn decode_address_base58check(address: &str) -> Result<[u8; ADDRESS_BYTES], AddressError> {
    // Decode from Base58
    let decoded = bs58::decode(address).into_vec()?;

    // Verify minimum length (version + address + checksum)
    if decoded.len() != 1 + ADDRESS_BYTES + CHECKSUM_LENGTH {
        return Err(AddressError::InvalidPayloadLength);
    }

    // Verify version byte
    if decoded[0] != ADDRESS_VERSION {
        return Err(AddressError::InvalidVersion);
    }

    // Extract components
    let payload = &decoded[..1 + ADDRESS_BYTES];
    let checksum = &decoded[1 + ADDRESS_BYTES..];

    // Verify checksum
    let calculated_checksum = calculate_checksum(payload);
    if checksum != calculated_checksum {
        return Err(AddressError::InvalidChecksum);
    }

    // Extract address bytes
    let mut address_bytes = [0u8; ADDRESS_BYTES];
    address_bytes.copy_from_slice(&decoded[1..1 + ADDRESS_BYTES]);

    Ok(address_bytes)
}

/// Encode a 32-byte account identifier into the human readable IPPAN format.
///
/// Uses Base58Check encoding with version byte and checksum for integrity verification.
/// The encoded address is compact and includes error detection.
pub fn encode_address(bytes: &[u8; ADDRESS_BYTES]) -> String {
    encode_address_base58check(bytes)
}

/// Attempt to decode a human readable IPPAN address string into the raw bytes.
///
/// Supports both Base58Check format (new) and legacy hex format (backward compatibility).
pub fn decode_address(address: &str) -> Result<[u8; ADDRESS_BYTES], AddressError> {
    // Try Base58Check format first (new format)
    if !address.starts_with('i') && !address.is_empty() {
        if let Ok(bytes) = decode_address_base58check(address) {
            return Ok(bytes);
        }
    }

    // Fall back to legacy hex format for backward compatibility
    if address.starts_with('i') && address.len() == ADDRESS_STRING_LENGTH {
        return decode_address_legacy_hex(address);
    }

    // If neither format works, try Base58Check anyway and return its error
    decode_address_base58check(address)
}

/// Legacy hex-based address decoding (for backward compatibility)
fn decode_address_legacy_hex(address: &str) -> Result<[u8; ADDRESS_BYTES], AddressError> {
    if !address.starts_with('i') {
        return Err(AddressError::InvalidPrefix);
    }

    if address.len() != ADDRESS_STRING_LENGTH {
        return Err(AddressError::InvalidLength {
            min: ADDRESS_STRING_LENGTH,
            max: ADDRESS_STRING_LENGTH,
            actual: address.len(),
        });
    }

    let payload = &address[1..];
    let decoded = hex::decode(payload)?;

    let bytes: [u8; ADDRESS_BYTES] = decoded
        .try_into()
        .map_err(|_| AddressError::InvalidPayloadLength)?;

    Ok(bytes)
}

/// Check whether the provided string is a valid IPPAN address.
pub fn is_valid_address(address: &str) -> bool {
    decode_address(address).is_ok()
}

/// Convenience wrapper for serialising/deserialising addresses as strings in JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Address(pub [u8; ADDRESS_BYTES]);

impl From<[u8; ADDRESS_BYTES]> for Address {
    fn from(value: [u8; ADDRESS_BYTES]) -> Self {
        Address(value)
    }
}

impl From<Address> for String {
    fn from(value: Address) -> Self {
        encode_address(&value.0)
    }
}

impl TryFrom<String> for Address {
    type Error = AddressError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        decode_address(&value).map(Address)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base58check_encode_decode_roundtrip() {
        let bytes = [0xABu8; ADDRESS_BYTES];
        let encoded = encode_address_base58check(&bytes);

        // Base58Check addresses should be shorter than hex format
        assert!(encoded.len() < ADDRESS_STRING_LENGTH);

        let decoded = decode_address_base58check(&encoded).expect("address should decode");
        assert_eq!(decoded, bytes);
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let bytes = [0xABu8; ADDRESS_BYTES];
        let encoded = encode_address(&bytes);

        // Should use Base58Check by default
        assert!(encoded.len() < ADDRESS_STRING_LENGTH);

        let decoded = decode_address(&encoded).expect("address should decode");
        assert_eq!(decoded, bytes);
    }

    #[test]
    fn test_legacy_hex_format_backward_compatibility() {
        // Test that legacy hex format can still be decoded
        let bytes = [0xABu8; ADDRESS_BYTES];
        let legacy_encoded = format!("i{}", hex::encode(&bytes));

        assert_eq!(legacy_encoded.len(), ADDRESS_STRING_LENGTH);
        assert!(legacy_encoded.starts_with('i'));

        let decoded = decode_address(&legacy_encoded).expect("legacy address should decode");
        assert_eq!(decoded, bytes);
    }

    #[test]
    fn test_base58check_invalid_checksum() {
        let bytes = [0xABu8; ADDRESS_BYTES];
        let mut encoded = encode_address_base58check(&bytes);

        // Corrupt the checksum by modifying the last character
        encoded.pop();
        encoded.push('X');

        let result = decode_address_base58check(&encoded);
        assert!(result.is_err());
    }

    #[test]
    fn test_base58check_invalid_version() {
        // Create an address with wrong version byte
        let mut payload = Vec::new();
        payload.push(0xFF); // Wrong version
        payload.extend_from_slice(&[0xABu8; ADDRESS_BYTES]);
        let checksum = calculate_checksum(&payload);
        payload.extend_from_slice(&checksum);

        let encoded = bs58::encode(payload).into_string();
        let result = decode_address_base58check(&encoded);
        assert!(matches!(result, Err(AddressError::InvalidVersion)));
    }

    #[test]
    fn test_invalid_prefix_rejected() {
        let bad = "x".to_string() + &"00".repeat(ADDRESS_BYTES);
        let err = decode_address(&bad).unwrap_err();
        // Should fail with Base58 decode error or other error
        assert!(err.to_string().len() > 0);
    }

    #[test]
    fn test_invalid_base58_characters() {
        // Base58 excludes 0, O, I, l to avoid confusion
        let bad = "000OOOIIILLL";
        let err = decode_address(bad).unwrap_err();
        assert!(matches!(err, AddressError::InvalidBase58(_)));
    }

    #[test]
    fn test_is_valid_address() {
        let bytes = [0x42u8; ADDRESS_BYTES];
        let valid_address = encode_address(&bytes);
        assert!(is_valid_address(&valid_address));

        assert!(!is_valid_address("invalid_address"));
        assert!(!is_valid_address(""));
    }

    #[test]
    fn test_different_keys_produce_different_addresses() {
        let bytes1 = [0x01u8; ADDRESS_BYTES];
        let bytes2 = [0x02u8; ADDRESS_BYTES];

        let addr1 = encode_address(&bytes1);
        let addr2 = encode_address(&bytes2);

        assert_ne!(addr1, addr2);
    }

    #[test]
    fn test_checksum_integrity() {
        // Verify that the checksum catches single bit errors
        let bytes = [0x55u8; ADDRESS_BYTES];
        let encoded = encode_address_base58check(&bytes);

        // Decode to get the raw bytes
        let raw = bs58::decode(&encoded).into_vec().unwrap();

        // Flip a bit in the address portion
        let mut corrupted = raw.clone();
        corrupted[1] ^= 0x01;

        let corrupted_address = bs58::encode(corrupted).into_string();
        let result = decode_address_base58check(&corrupted_address);

        // Should fail checksum validation
        assert!(result.is_err());
    }
}
