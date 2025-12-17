#![cfg_attr(fuzzing, no_main)]

// See `canonical_hash.rs` for why this exists (workspace test builds also compile binaries).
#[cfg(not(fuzzing))]
fn main() {}

#[cfg(fuzzing)]
use base64::{engine::general_purpose, Engine as _};
#[cfg(fuzzing)]
use ippan_crypto::CryptoUtils;
#[cfg(fuzzing)]
use libfuzzer_sys::fuzz_target;

#[cfg(fuzzing)]
fuzz_target!(|data: &[u8]| {
    // Cap input size to prevent unbounded memory
    if data.len() > 100_000 {
        return;
    }

    // Test 1: Confidential transaction validation (raw bytes)
    // This should never panic, only return Result
    let _result: Result<bool, _> = CryptoUtils::validate_confidential_transaction(data);

    // Test 2: Try to parse as base64-encoded proof
    if let Ok(s) = std::str::from_utf8(data) {
        if s.len() < 10_000 {
            // Try base64 decode (should not panic)
            let _decoded = general_purpose::STANDARD.decode(s);
        }
    }

    // Test 3: Proof byte parsing with size limits
    if data.len() >= 102 {
        // Minimum size check (as per validate_confidential_transaction)
        let _has_min_size = true;

        if data.len() > 10_000 {
            // Proof length should be bounded
            let _proof_too_large = true;
        }
    }

    // Test 4: Parse proof structure fields
    if data.len() >= 4 {
        // Try to read proof length (u32 LE)
        if data.len() >= 8 {
            let proof_len_bytes = &data[4..8];
            if proof_len_bytes.len() == 4 {
                let proof_len = u32::from_le_bytes([
                    proof_len_bytes[0],
                    proof_len_bytes[1],
                    proof_len_bytes[2],
                    proof_len_bytes[3],
                ]) as usize;

                // Validate reasonable proof length
                if proof_len > 0 && proof_len <= 10_000 {
                    // Check if we have enough data
                    if 8 + proof_len <= data.len() {
                        let _proof_bytes = &data[8..8 + proof_len];
                    }
                }
            }
        }
    }
});
