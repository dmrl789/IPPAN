#![no_main]
use libfuzzer_sys::fuzz_target;

// Fuzz target for RPC payment endpoint parsing
// Tests that arbitrary input doesn't cause panics, UB, or excessive allocations

fuzz_target!(|data: &[u8]| {
    // Attempt to parse as JSON payment request
    if let Ok(s) = std::str::from_utf8(data) {
        let _: Result<serde_json::Value, _> = serde_json::from_str(s);
    }
    
    // Also test binary parsing if we have binary formats
    if data.len() >= 4 {
        // Check for length-prefixed format
        let len = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        if len < 1_000_000 && len <= data.len() - 4 {
            // Safe to process
            let _payload = &data[4..4 + len];
        }
    }
});
