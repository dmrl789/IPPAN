#![no_main]
use libfuzzer_sys::fuzz_target;

// Fuzz target for RPC handle registration parsing
// Tests handle format validation under adversarial input

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Test handle validation logic
        let _is_valid = s.starts_with('@') 
            && s.len() > 1 
            && s.len() <= 32
            && s[1..].chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-');
        
        // Try JSON parsing
        let _: Result<serde_json::Value, _> = serde_json::from_str(s);
    }
});
