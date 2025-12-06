#![no_main]
use libfuzzer_sys::fuzz_target;
use ippan_ai_core::serde_canon::{to_canonical_json, hash_canonical, hash_canonical_hex};
use serde_json::Value;

fuzz_target!(|data: &[u8]| {
    // Cap input size to prevent unbounded memory allocation
    if data.len() > 1_000_000 {
        return;
    }

    // Test 1: Parse as JSON and canonicalize
    if let Ok(s) = std::str::from_utf8(data) {
        if let Ok(json_value) = serde_json::from_str::<Value>(s) {
            // Should never panic - returns Result
            let _canonical = to_canonical_json(&json_value);
        }
    }

    // Test 2: Try to deserialize as arbitrary JSON structure
    if let Ok(json_value) = serde_json::from_slice::<Value>(data) {
        // Test canonicalization roundtrip
        let canonical1 = to_canonical_json(&json_value);
        let canonical2 = to_canonical_json(&json_value);
        
        // Same input should produce same canonical output
        if let (Ok(c1), Ok(c2)) = (&canonical1, &canonical2) {
            assert_eq!(c1, c2);
        }

        // Test hashing
        let hash1 = hash_canonical(&json_value);
        let hash2 = hash_canonical(&json_value);
        
        // Same input should produce same hash
        if let (Ok(h1), Ok(h2)) = (&hash1, &hash2) {
            assert_eq!(h1, h2);
        }

        // Test hex hash
        let hex1 = hash_canonical_hex(&json_value);
        let hex2 = hash_canonical_hex(&json_value);
        
        if let (Ok(h1), Ok(h2)) = (&hex1, &hex2) {
            assert_eq!(h1, h2);
            // Hex should be 64 chars (32 bytes)
            assert_eq!(h1.len(), 64);
        }
    }
});

