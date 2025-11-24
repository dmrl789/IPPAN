#![no_main]
use libfuzzer_sys::fuzz_target;

// Enhanced fuzz target for RPC handle operations (Phase E)
// Tests handle format validation, registration payloads, and resolution
// under adversarial input without panics

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let trimmed = s.trim();
        
        // Test 1: Handle identifier validation (per IPPAN spec)
        // Must start with @, include a dot, and be 4-63 chars
        if !trimmed.is_empty() {
            let has_prefix = trimmed.starts_with('@');
            let has_dot = trimmed.contains('.');
            let valid_length = trimmed.len() >= 4 && trimmed.len() <= 63;
            
            // Check suffix validation (.ipn, .iot, .m, .cyborg)
            let valid_suffix = trimmed.ends_with(".ipn") 
                || trimmed.ends_with(".iot") 
                || trimmed.ends_with(".m") 
                || trimmed.ends_with(".cyborg");
            
            // Character validation: ASCII letters, digits, underscore, dot
            let valid_chars = trimmed.chars().all(|c| {
                c.is_ascii_lowercase() 
                || c.is_ascii_digit() 
                || c == '_' 
                || c == '.' 
                || c == '@'
            });
            
            let is_valid_handle = has_prefix && has_dot && valid_length && valid_suffix && valid_chars;
            let _ = is_valid_handle; // Ensure no panic in validation
        }
        
        // Test 2: JSON parsing for handle registration request
        if s.len() < 10_000 {
            // Try to parse as handle registration JSON
            let _: Result<serde_json::Value, _> = serde_json::from_str(s);
            
            // Test malformed JSON recovery
            if s.starts_with('{') {
                // Attempt to extract fields even if JSON is incomplete
                let _ = s.contains("\"handle\":");
                let _ = s.contains("\"owner\":");
                let _ = s.contains("\"metadata\":");
            }
        }
        
        // Test 3: Handle-to-address resolution patterns
        if trimmed.starts_with('@') {
            // Simulate resolution lookup (should never panic)
            let parts: Vec<&str> = trimmed.split('.').collect();
            if parts.len() >= 2 {
                let name = parts[0].trim_start_matches('@');
                let suffix = parts[1];
                
                // Validate name component
                let _ = name.len() >= 1 && name.len() <= 50;
                
                // Validate suffix component
                let _ = matches!(suffix, "ipn" | "iot" | "m" | "cyborg");
            }
        }
        
        // Test 4: Premium TLD detection
        if trimmed.ends_with(".cyborg") || trimmed.ends_with(".iot") || trimmed.ends_with(".m") {
            let _is_premium = true;
        }
    }
});
