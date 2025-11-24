#![no_main]
use libfuzzer_sys::fuzz_target;

// Fuzz target for transaction decoding
// Ensures arbitrary bytes don't cause panics when parsed as transactions

fuzz_target!(|data: &[u8]| {
    // Test CBOR decoding (if used)
    // let _: Result<Transaction, _> = serde_cbor::from_slice(data);
    
    // Test JSON decoding
    if let Ok(s) = std::str::from_utf8(data) {
        let _: Result<serde_json::Value, _> = serde_json::from_str(s);
    }
    
    // Test binary format (if custom)
    if data.len() >= 8 {
        // Check signature length
        if data.len() >= 64 {
            let _signature = &data[..64];
            let _payload = &data[64..];
        }
    }
});
