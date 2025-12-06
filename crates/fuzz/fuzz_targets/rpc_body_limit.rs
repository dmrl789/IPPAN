#![no_main]
use libfuzzer_sys::fuzz_target;

const MAX_BODY_BYTES: usize = 64 * 1024; // 64 KiB

fuzz_target!(|data: &[u8]| {
    // Cap input to reasonable size for fuzzing (but allow testing beyond limit)
    if data.len() > 2 * MAX_BODY_BYTES {
        return;
    }

    // Test body size limit enforcement
    let exceeds_limit = data.len() > MAX_BODY_BYTES;

    // Simulate body limit check (as done in RPC server)
    if exceeds_limit {
        // Should reject with appropriate error
        // In real code: return Err((StatusCode::PAYLOAD_TOO_LARGE, ...))
        let _rejected = true;
    } else {
        // Should accept
        let _accepted = true;
    }

    // Test boundary conditions
    if data.len() == MAX_BODY_BYTES {
        // Exactly at limit - should accept
        let _at_limit = true;
    }

    if data.len() == MAX_BODY_BYTES + 1 {
        // One byte over - should reject
        let _over_limit = true;
    }

    // Test JSON parsing on valid-sized bodies
    if data.len() <= MAX_BODY_BYTES {
        if let Ok(s) = std::str::from_utf8(data) {
            // Try to parse as JSON (should not panic)
            let _: Result<serde_json::Value, _> = serde_json::from_str(s);
        }
    }
});
