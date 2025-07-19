//! Comprehensive tests for IPPAN address functionality
//! 
//! This module tests the i-prefixed ed25519-based address format implementation.

use ippan::utils::address::*;
use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::RngCore;

#[test]
fn test_address_format_specifications() {
    // Test that addresses follow the exact specifications
    
    // Test with a known public key
    let pubkey = [0u8; 32];
    let address = generate_ippan_address(&pubkey);
    
    // Must start with 'i'
    assert!(address.starts_with('i'), "Address must start with 'i', got: {}", address);
    
    // Must be valid Base58Check
    assert!(is_valid_ippan_address(&address), "Generated address must be valid");
    
    // Must be reasonable length (typically 34-35 chars for this format)
    assert!(address.len() >= 30 && address.len() <= 40, 
            "Address length should be reasonable, got: {} chars", address.len());
    
    println!("Generated address: {}", address);
}

#[test]
fn test_address_validation_edge_cases() {
    // Test various invalid address formats
    
    // Empty string
    assert!(validate_ippan_address("").is_err());
    
    // Too short
    assert!(validate_ippan_address("i").is_err());
    assert!(validate_ippan_address("i1").is_err());
    
    // Too long
    let long_address = "i".to_string() + &"1".repeat(100);
    assert!(validate_ippan_address(&long_address).is_err());
    
    // Invalid Base58 characters
    assert!(validate_ippan_address("i1hV6Ro8Adgj7fw1MPWAhUHyZBcZevfyz0").is_err()); // Contains '0'
    
    // Wrong prefix
    assert!(validate_ippan_address("1hV6Ro8Adgj7fw1MPWAhUHyZBcZevfyz").is_err()); // Bitcoin-style
    assert!(validate_ippan_address("0x1hV6Ro8Adgj7fw1MPWAhUHyZBcZevfyz").is_err()); // Ethereum-style
    
    // Valid address should pass
    let pubkey = [1u8; 32];
    let valid_address = generate_ippan_address(&pubkey);
    assert!(validate_ippan_address(&valid_address).is_ok());
}

#[test]
fn test_address_consistency_and_uniqueness() {
    // Test that same public key always generates same address
    let pubkey1 = [2u8; 32];
    let address1 = generate_ippan_address(&pubkey1);
    let address2 = generate_ippan_address(&pubkey1);
    assert_eq!(address1, address2, "Same public key should generate same address");
    
    // Test that different public keys generate different addresses
    let pubkey2 = [3u8; 32];
    let address3 = generate_ippan_address(&pubkey2);
    assert_ne!(address1, address3, "Different public keys should generate different addresses");
    
    // Test multiple different keys
    for i in 0..10 {
        let mut pubkey = [0u8; 32];
        pubkey[0] = i;
        let address = generate_ippan_address(&pubkey);
        assert!(address.starts_with('i'), "Address {} should start with 'i'", i);
        assert!(is_valid_ippan_address(&address), "Address {} should be valid", i);
    }
}

#[test]
fn test_checksum_validation() {
    // Test that checksum validation works correctly
    
    let pubkey = [4u8; 32];
    let original_address = generate_ippan_address(&pubkey);
    
    // Should be valid
    assert!(validate_ippan_address(&original_address).is_ok());
    
    // Modify the address by changing characters
    let mut modified_address = original_address.clone();
    let chars: Vec<char> = modified_address.chars().collect();
    
    // Try changing each character
    for i in 0..chars.len() {
        let mut test_address = chars.clone();
        // Change to a different Base58 character
        test_address[i] = if chars[i] == '1' { '2' } else { '1' };
        let test_string: String = test_address.into_iter().collect();
        
        // Should now be invalid
        assert!(validate_ippan_address(&test_string).is_err(), 
                "Modified address should be invalid: {}", test_string);
    }
}

#[test]
fn test_hash_extraction() {
    // Test extracting RIPEMD-160 hash from address
    
    let pubkey = [5u8; 32];
    let address = generate_ippan_address(&pubkey);
    
    // Extract hash
    let hash = extract_hash_from_address(&address).unwrap();
    
    // Hash should be correct length
    assert_eq!(hash.len(), RIPEMD160_LENGTH, "Hash should be {} bytes", RIPEMD160_LENGTH);
    
    // Hash should be consistent
    let hash2 = extract_hash_from_address(&address).unwrap();
    assert_eq!(hash, hash2, "Hash extraction should be consistent");
    
    // Invalid address should fail
    assert!(extract_hash_from_address("invalid").is_err());
}

#[test]
fn test_real_ed25519_keypairs() {
    // Test with real ed25519 keypairs
    
    for _ in 0..5 {
        let mut rng = rand::thread_rng();
        let signing_key = SigningKey::generate(&mut rng);
        let verifying_key = signing_key.verifying_key();
        
        let address = generate_ippan_address(&verifying_key.to_bytes());
        
        // Should be valid IPPAN address
        assert!(address.starts_with('i'), "Real ed25519 address should start with 'i'");
        assert!(is_valid_ippan_address(&address), "Real ed25519 address should be valid");
        
        // Should be able to extract hash
        let hash = extract_hash_from_address(&address).unwrap();
        assert_eq!(hash.len(), RIPEMD160_LENGTH);
    }
}

#[test]
fn test_vanity_address_generation() {
    // Test vanity address generation
    
    // Test with a short prefix
    let result = generate_vanity_address("iTest", 1000);
    if let Some((address, _keypair)) = result {
        assert!(address.starts_with("iTest"), "Vanity address should start with desired prefix");
        assert!(is_valid_ippan_address(&address), "Vanity address should be valid");
        println!("Found vanity address: {}", address);
    }
    
    // Test with a longer prefix (less likely to find)
    let result = generate_vanity_address("iDiana", 100);
    // This might not find a match, but should not panic
    if let Some((address, _keypair)) = result {
        assert!(address.starts_with("iDiana"));
        assert!(is_valid_ippan_address(&address));
        println!("Found long vanity address: {}", address);
    }
}

#[test]
fn test_address_info() {
    // Test AddressInfo struct
    
    let pubkey = [6u8; 32];
    let address = generate_ippan_address(&pubkey);
    
    let info = AddressInfo::from_address(&address).unwrap();
    
    // Check all fields
    assert_eq!(info.address, address);
    assert_eq!(info.prefix_byte, IPPAN_ADDRESS_PREFIX);
    assert_eq!(info.hash_length, RIPEMD160_LENGTH);
    assert_eq!(info.checksum_length, CHECKSUM_LENGTH);
    assert_eq!(info.total_length, ADDRESS_PAYLOAD_LENGTH);
    
    // Test display
    let display = format!("{}", info);
    assert!(display.contains(&address));
    assert!(display.contains("0x49")); // Should show prefix byte
}

#[test]
fn test_address_constants() {
    // Test that constants are correct
    
    assert_eq!(IPPAN_ADDRESS_PREFIX, 0x49, "Prefix byte should be 0x49 (ASCII 'i')");
    assert_eq!(RIPEMD160_LENGTH, 20, "RIPEMD-160 hash should be 20 bytes");
    assert_eq!(CHECKSUM_LENGTH, 4, "Checksum should be 4 bytes");
    assert_eq!(ADDRESS_PAYLOAD_LENGTH, 25, "Total payload should be 25 bytes (1 + 20 + 4)");
}

#[test]
fn test_error_types() {
    // Test error handling
    
    // Test invalid length
    match validate_ippan_address("i") {
        Err(AddressError::InvalidLength) => {},
        _ => panic!("Should return InvalidLength for short address"),
    }
    
    // Test invalid Base58
    match validate_ippan_address("i1hV6Ro8Adgj7fw1MPWAhUHyZBcZevfyz0") {
        Err(AddressError::InvalidBase58) => {},
        _ => panic!("Should return InvalidBase58 for invalid characters"),
    }
    
    // Test invalid prefix
    match validate_ippan_address("1hV6Ro8Adgj7fw1MPWAhUHyZBcZevfyz") {
        Err(AddressError::InvalidPrefix) => {},
        _ => panic!("Should return InvalidPrefix for wrong prefix"),
    }
}

#[test]
fn test_address_generation_performance() {
    // Test that address generation is reasonably fast
    
    let start = std::time::Instant::now();
    
    for _ in 0..100 {
        let pubkey = rand::random::<[u8; 32]>();
        let _address = generate_ippan_address(&pubkey);
    }
    
    let duration = start.elapsed();
    assert!(duration.as_millis() < 1000, "Address generation should be fast, took: {:?}", duration);
}

#[test]
fn test_address_format_compliance() {
    // Test that addresses comply with the specified format
    
    // Generate several addresses and verify format
    for i in 0..10 {
        let mut pubkey = [0u8; 32];
        pubkey[0] = i;
        let address = generate_ippan_address(&pubkey);
        
        // Must start with 'i'
        assert!(address.starts_with('i'), "Address {} must start with 'i'", i);
        
        // Must be valid
        assert!(is_valid_ippan_address(&address), "Address {} must be valid", i);
        
        // Must be Base58Check encoded
        let decoded = bs58::decode(&address).into_vec();
        assert!(decoded.is_ok(), "Address {} must be valid Base58", i);
        
        let decoded_bytes = decoded.unwrap();
        assert_eq!(decoded_bytes.len(), ADDRESS_PAYLOAD_LENGTH, 
                   "Address {} must have correct payload length", i);
        
        // Must have correct prefix byte
        assert_eq!(decoded_bytes[0], IPPAN_ADDRESS_PREFIX, 
                   "Address {} must have correct prefix byte", i);
    }
}

#[test]
fn test_address_examples() {
    // Test with specific examples to ensure consistency
    
    // Example 1: All zeros
    let pubkey1 = [0u8; 32];
    let address1 = generate_ippan_address(&pubkey1);
    println!("Example 1 (all zeros): {}", address1);
    assert!(address1.starts_with('i'));
    assert!(is_valid_ippan_address(&address1));
    
    // Example 2: All ones
    let pubkey2 = [1u8; 32];
    let address2 = generate_ippan_address(&pubkey2);
    println!("Example 2 (all ones): {}", address2);
    assert!(address2.starts_with('i'));
    assert!(is_valid_ippan_address(&address2));
    
    // Example 3: Sequential bytes
    let mut pubkey3 = [0u8; 32];
    for i in 0..32 {
        pubkey3[i] = i as u8;
    }
    let address3 = generate_ippan_address(&pubkey3);
    println!("Example 3 (sequential): {}", address3);
    assert!(address3.starts_with('i'));
    assert!(is_valid_ippan_address(&address3));
} 