use ippan_types::Transaction;

/// Maximum allowed protocol fee per transaction (ordering/validation).
/// This is enforced deterministically during block proposal and validation.
pub const MAX_FEE_PER_TX: u64 = 10_000_000; // 0.1 IPN if 8 decimals and base unit = µIPN

/// Estimate a transaction's fee using the same heuristic as the mempool.
///
/// NOTE: This is intentionally duplicated from the mempool to avoid a
/// dependency cycle. Keep the logic in sync with `ippan-mempool`.
pub fn estimate_fee_like_mempool(tx: &Transaction) -> u64 {
    let base_fee = 1000u64; // Base fee component for all transactions

    // Estimate transaction size using accessible public fields.
    let mut estimated_size = 0usize;

    // Fixed-size fields (id, from, to, amount, nonce, signature, hashtimer, timestamp).
    estimated_size += 32; // id
    estimated_size += 32; // from
    estimated_size += 32; // to
    estimated_size += 8; // amount
    estimated_size += 8; // nonce
    estimated_size += 64; // signature
    estimated_size += tx.hashtimer.time_prefix.len();
    estimated_size += tx.hashtimer.hash_suffix.len();
    estimated_size += std::mem::size_of_val(&tx.timestamp.0);

    // Dynamic fields.
    estimated_size += tx.topics.iter().map(|topic| topic.len()).sum::<usize>();

    if let Some(envelope) = &tx.confidential {
        estimated_size += envelope.enc_algo.len();
        estimated_size += envelope.iv.len();
        estimated_size += envelope.ciphertext.len();
        estimated_size += envelope
            .access_keys
            .iter()
            .map(|key| key.recipient_pub.len() + key.enc_key.len())
            .sum::<usize>();
    }

    if let Some(proof) = &tx.zk_proof {
        estimated_size += proof.proof.len();
        estimated_size += proof
            .public_inputs
            .iter()
            .map(|(key, value)| key.len() + value.len())
            .sum::<usize>();
    }

    let size_fee = estimated_size as u64 * 10; // Size-based fee (10 wei per byte)
    base_fee + size_fee
}

/// Check whether a transaction's estimated fee does not exceed the protocol cap.
pub fn within_cap(tx: &Transaction) -> bool {
    estimate_fee_like_mempool(tx) <= MAX_FEE_PER_TX
}
