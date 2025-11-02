use serde::{Deserialize, Serialize};

/// Errors that can occur when parsing an IPPAN address string.
#[derive(Debug, thiserror::Error)]
pub enum AddressError {
    #[error("address must start with 'i'")]
    InvalidPrefix,
    #[error("address must be {expected} characters, got {actual}")]
    InvalidLength { expected: usize, actual: usize },
    #[error("address payload is not valid hexadecimal")]
    InvalidHex(#[from] hex::FromHexError),
    #[error("address payload must be exactly 32 bytes")]
    InvalidPayloadLength,
}

/// Number of raw bytes contained in an address.
pub const ADDRESS_BYTES: usize = 32;
/// Expected string length of an encoded address (prefix + 64 hex chars).
pub const ADDRESS_STRING_LENGTH: usize = 1 + ADDRESS_BYTES * 2;

/// Encode a 32-byte account identifier into the human readable IPPAN format.
///
/// The encoded address always begins with the character `i` followed by the
/// hexadecimal representation of the raw bytes.
pub fn encode_address(bytes: &[u8; ADDRESS_BYTES]) -> String {
    let mut encoded = String::with_capacity(ADDRESS_STRING_LENGTH);
    encoded.push('i');
    encoded.push_str(&hex::encode(bytes));
    encoded
}

/// Attempt to decode a human readable IPPAN address string into the raw bytes.
pub fn decode_address(address: &str) -> Result<[u8; ADDRESS_BYTES], AddressError> {
    if !address.starts_with('i') {
        return Err(AddressError::InvalidPrefix);
    }

    if address.len() != ADDRESS_STRING_LENGTH {
        return Err(AddressError::InvalidLength {
            expected: ADDRESS_STRING_LENGTH,
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

#[cfg(all(test, feature = "enable-tests"))]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_roundtrip() {
        let bytes = [0xABu8; ADDRESS_BYTES];
        let encoded = encode_address(&bytes);
        assert!(encoded.starts_with('i'));
        assert_eq!(encoded.len(), ADDRESS_STRING_LENGTH);

        let decoded = decode_address(&encoded).expect("address should decode");
        assert_eq!(decoded, bytes);
    }

    #[test]
    fn invalid_prefix_rejected() {
        let bad = "x".to_string() + &"00".repeat(ADDRESS_BYTES);
        let err = decode_address(&bad).unwrap_err();
        assert!(matches!(err, AddressError::InvalidPrefix));
    }

    #[test]
    fn invalid_length_rejected() {
        let bad = "i".to_string() + &"00".repeat(ADDRESS_BYTES - 1);
        let err = decode_address(&bad).unwrap_err();
        assert!(matches!(err, AddressError::InvalidLength { .. }));
    }

    #[test]
    fn invalid_hex_rejected() {
        let bad = format!("i{}", "gg".repeat(ADDRESS_BYTES));
        let err = decode_address(&bad).unwrap_err();
        assert!(matches!(err, AddressError::InvalidHex(_)));
    }
}
