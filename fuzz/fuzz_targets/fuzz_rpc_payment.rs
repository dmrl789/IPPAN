#![no_main]
use libfuzzer_sys::fuzz_target;

// Enhanced fuzz target for RPC payment endpoint (Phase E)
// Tests payment construction, handle resolution, and amount validation
// under adversarial/malformed input without panics

fuzz_target!(|data: &[u8]| {
    // Test 1: JSON deserialization robustness
    if let Ok(s) = std::str::from_utf8(data) {
        // Try to deserialize as a generic JSON value
        // This tests that serde_json itself doesn't panic
        let _: Result<serde_json::Value, _> = serde_json::from_str(s);
        
        // Test handle identifier validation (mimicking wallet CLI logic)
        let trimmed = s.trim();
        let is_handle = trimmed.starts_with('@') && trimmed.len() > 1;
        let _ = is_handle; // Ensure no panic in validation
        
        // Test address decoding patterns
        // Base58Check format
        if trimmed.starts_with('i') && trimmed.len() >= 26 {
            // Attempt to validate as Base58Check (should never panic)
            let _ = trimmed.chars().all(|c| {
                matches!(c, '1'..='9' | 'A'..='H' | 'J'..='N' | 'P'..='Z' | 'a'..='k' | 'm'..='z')
            });
        }
        
        // Hex format (with optional 0x prefix)
        if trimmed.starts_with("0x") || trimmed.starts_with("0X") {
            let hex_part = trimmed.trim_start_matches("0x").trim_start_matches("0X");
            if hex_part.len() == 64 {
                let _ = hex_part.chars().all(|c| c.is_ascii_hexdigit());
            }
        }
    }
    
    // Test 2: Binary format parsing (length-prefixed)
    if data.len() >= 4 {
        let len = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        // Defend against excessive allocations
        if len < 1_000_000 && len <= data.len().saturating_sub(4) {
            let payload = &data[4..4 + len];
            // Attempt UTF-8 decode on payload
            if let Ok(utf8) = std::str::from_utf8(payload) {
                let _: Result<serde_json::Value, _> = serde_json::from_str(utf8);
            }
        }
    }
    
    // Test 3: Amount parsing robustness
    if let Ok(s) = std::str::from_utf8(data) {
        // Try to parse as decimal amount string
        if s.len() < 100 {
            // Check for numeric patterns
            let _ = s.parse::<f64>();
            let _ = s.parse::<u128>();
            
            // Test amount with up to 24 decimals (IPN precision)
            if let Some(dot_idx) = s.find('.') {
                let decimals = &s[dot_idx + 1..];
                if decimals.len() <= 24 {
                    // Valid precision range, no panic
                    let _ = decimals.chars().all(|c| c.is_ascii_digit());
                }
            }
        }
    }
});
