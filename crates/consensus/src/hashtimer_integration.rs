//! HashTimer Integration for DLC Consensus
//!
//! Provides temporal ordering and deterministic time anchoring
//! for the Deterministic Learning Consensus model.

use blake3::Hasher as Blake3;
use ippan_types::{HashTimer, IppanTimeMicros};

fn derive_nonce(domain: &str, time: IppanTimeMicros, parts: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Blake3::new();
    hasher.update(domain.as_bytes());
    hasher.update(&time.0.to_be_bytes());
    for part in parts {
        hasher.update(part);
    }
    let hash = hasher.finalize();
    let mut nonce = [0u8; 32];
    nonce.copy_from_slice(hash.as_bytes());
    nonce
}

/// Generate a round HashTimer for deterministic temporal ordering
pub fn generate_round_hashtimer(
    round_id: u64,
    previous_hash: &[u8; 32],
    validator_id: &[u8; 32],
) -> HashTimer {
    let current_time = IppanTimeMicros::now();
    generate_round_hashtimer_at(round_id, previous_hash, validator_id, current_time)
}

/// Generate a round HashTimer using an explicit time (useful for tests)
pub fn generate_round_hashtimer_at(
    round_id: u64,
    previous_hash: &[u8; 32],
    validator_id: &[u8; 32],
    time: IppanTimeMicros,
) -> HashTimer {
    let domain = "dlc_round";

    let mut payload = Vec::new();
    let round_bytes = round_id.to_be_bytes();
    payload.extend_from_slice(&round_bytes);
    payload.extend_from_slice(previous_hash);

    let nonce = derive_nonce(
        domain,
        time,
        &[
            round_bytes.as_ref(),
            previous_hash.as_ref(),
            validator_id.as_ref(),
        ],
    );

    HashTimer::derive(
        domain,
        time,
        domain.as_bytes(),
        &payload,
        &nonce,
        validator_id,
    )
}

/// Generate a block proposal HashTimer
pub fn generate_block_hashtimer(
    block_id: &[u8; 32],
    round_id: u64,
    proposer_id: &[u8; 32],
) -> HashTimer {
    let current_time = IppanTimeMicros::now();
    generate_block_hashtimer_at(block_id, round_id, proposer_id, current_time)
}

/// Generate a block proposal HashTimer using an explicit time (useful for tests)
pub fn generate_block_hashtimer_at(
    block_id: &[u8; 32],
    round_id: u64,
    proposer_id: &[u8; 32],
    time: IppanTimeMicros,
) -> HashTimer {
    let domain = "dlc_block_proposal";

    let mut payload = Vec::new();
    let round_bytes = round_id.to_be_bytes();
    payload.extend_from_slice(block_id);
    payload.extend_from_slice(&round_bytes);

    let nonce = derive_nonce(
        domain,
        time,
        &[
            block_id.as_ref(),
            round_bytes.as_ref(),
            proposer_id.as_ref(),
        ],
    );

    HashTimer::derive(
        domain,
        time,
        domain.as_bytes(),
        &payload,
        &nonce,
        proposer_id,
    )
}

/// Verify temporal ordering of blocks using HashTimer
pub fn verify_temporal_ordering(
    block_hashtimer: &HashTimer,
    round_start_time: IppanTimeMicros,
    round_duration_ms: u64,
) -> bool {
    let block_time = block_hashtimer.time();
    let round_end_time = IppanTimeMicros(round_start_time.0 + (round_duration_ms * 1000));

    // Block must be within the round window
    block_time >= round_start_time && block_time <= round_end_time
}

/// Check if round should close based on temporal finality
pub fn should_close_round(round_start: IppanTimeMicros, finality_window_ms: u64) -> bool {
    let current_time = IppanTimeMicros::now();
    let elapsed_us = current_time.0.saturating_sub(round_start.0);
    let finality_window_us = finality_window_ms * 1000;

    elapsed_us >= finality_window_us
}

/// Generate deterministic seed from HashTimer for verifier selection
pub fn derive_selection_seed(hashtimer: &HashTimer) -> [u8; 32] {
    let mut hasher = Blake3::new();
    hasher.update(b"DLC_VERIFIER_SELECTION_SEED");
    hasher.update(&hashtimer.digest());

    let hash = hasher.finalize();
    let mut seed = [0u8; 32];
    seed.copy_from_slice(hash.as_bytes());
    seed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_hashtimer_generation() {
        let round_id = 1;
        let previous_hash = [0u8; 32];
        let validator_id = [1u8; 32];

        let hashtimer = generate_round_hashtimer(round_id, &previous_hash, &validator_id);
        assert!(hashtimer.timestamp_us > 0);
    }

    #[test]
    fn test_temporal_ordering_verification() {
        let round_start = IppanTimeMicros::now();
        let round_duration_ms = 250;
        let validator_id = [1u8; 32];

        let hashtimer = generate_round_hashtimer(1, &[0u8; 32], &validator_id);

        let is_valid = verify_temporal_ordering(&hashtimer, round_start, round_duration_ms);
        assert!(is_valid);
    }

    #[test]
    fn test_round_closure() {
        let round_start = IppanTimeMicros(0);
        let finality_window_ms = 250;

        // Should close after window
        std::thread::sleep(std::time::Duration::from_millis(10));
        let should_close = should_close_round(round_start, finality_window_ms);
        assert!(should_close);
    }

    #[test]
    fn test_selection_seed_derivation() {
        let validator_id = [1u8; 32];
        let hashtimer = generate_round_hashtimer(1, &[0u8; 32], &validator_id);

        let seed1 = derive_selection_seed(&hashtimer);
        let seed2 = derive_selection_seed(&hashtimer);

        // Same hashtimer should give same seed
        assert_eq!(seed1, seed2);
    }
}
