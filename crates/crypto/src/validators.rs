//! Lightweight validators for confidential transactions and blocks

use anyhow::{anyhow, Result};
use ippan_types::{Transaction, ConfidentialEnvelope, ConfidentialProof, TransactionVisibility};

/// Validate a confidential transaction's envelope and basic proof structure.
///
/// This is a lightweight, mempool-friendly validation to filter malformed
/// payloads early. Full cryptographic checks are performed later in consensus.
pub fn validate_confidential_transaction(tx: &Transaction) -> Result<()> {
    if tx.visibility != TransactionVisibility::Confidential {
        return Ok(());
    }

    let envelope: &ConfidentialEnvelope = tx
        .confidential
        .as_ref()
        .ok_or_else(|| anyhow!("missing confidential envelope"))?;

    if envelope.enc_algo.is_empty() {
        return Err(anyhow!("missing encryption algorithm"));
    }
    if envelope.iv.is_empty() {
        return Err(anyhow!("missing IV"));
    }
    if envelope.ciphertext.is_empty() {
        return Err(anyhow!("missing ciphertext"));
    }

    if envelope.access_keys.is_empty() {
        return Err(anyhow!("no access keys provided"));
    }
    for (i, ak) in envelope.access_keys.iter().enumerate() {
        if ak.recipient_pub.is_empty() {
            return Err(anyhow!("access key #{i} missing recipient_pub"));
        }
        if ak.enc_key.is_empty() {
            return Err(anyhow!("access key #{i} missing enc_key"));
        }
    }

    // If a zk proof is present, minimally check fields are non-empty.
    if let Some(proof) = &tx.zk_proof {
        validate_confidential_proof(proof)?;
    }

    Ok(())
}

fn validate_confidential_proof(proof: &ConfidentialProof) -> Result<()> {
    if proof.proof.is_empty() {
        return Err(anyhow!("empty zk proof bytes"));
    }
    Ok(())
}

/// Validate a block's confidential transactions using the lightweight checks.
pub fn validate_confidential_block(txs: &[Transaction]) -> Result<()> {
    for tx in txs {
        validate_confidential_transaction(tx)?;
    }
    Ok(())
}
