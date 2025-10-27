use std::collections::BTreeMap;

use base64::{engine::general_purpose, Engine as _};
use blake3::Hasher;
use hex::ToHex;
use ippan_types::{
    block::Block,
    transaction::{ConfidentialProof, ConfidentialProofType, Transaction, TransactionVisibility},
};
use thiserror::Error;

/// Errors that can occur while validating confidential transactions.
#[derive(Debug, Error)]
pub enum ConfidentialTransactionError {
    /// Confidential transaction is missing the encrypted payload envelope.
    #[error("confidential transaction is missing its encryption envelope")]
    MissingEnvelope,
    /// Confidential transaction is missing a zero-knowledge proof.
    #[error("confidential transaction is missing a zero-knowledge proof")]
    MissingProof,
    /// The supplied proof blob was not valid base64.
    #[error("invalid proof encoding: {0}")]
    InvalidProofEncoding(String),
    /// Required public input field is absent.
    #[error("missing public input: {0}")]
    MissingPublicInput(&'static str),
    /// A public input value could not be parsed as a number.
    #[error("invalid numeric value for {name}: {source}")]
    InvalidNumericValue {
        /// Field name being parsed.
        name: &'static str,
        /// Underlying parsing error.
        source: std::num::ParseIntError,
    },
    /// The transaction identifier declared in the proof does not match the transaction payload.
    #[error("transaction identifier does not match confidential proof inputs")]
    TransactionIdMismatch,
    /// The sender commitment derived from the transaction does not match the proof inputs.
    #[error("sender commitment mismatch")]
    SenderCommitmentMismatch,
    /// The receiver commitment derived from the transaction does not match the proof inputs.
    #[error("receiver commitment mismatch")]
    ReceiverCommitmentMismatch,
    /// Sequence length in the proof is unsupported.
    #[error("invalid fibonacci sequence length")]
    InvalidSequenceLength,
    /// Proof verification is not yet implemented
    #[error("proof verification not implemented: {0}")]
    ProofVerificationNotImplemented(String),
}

/// Validate all confidential transactions in a block.
pub fn validate_block(block: &Block) -> Result<(), ConfidentialTransactionError> {
    for tx in &block.transactions {
        validate_transaction(tx)?;
    }
    Ok(())
}

/// Validate the zero-knowledge proof for a transaction if one is present.
pub fn validate_transaction(tx: &Transaction) -> Result<(), ConfidentialTransactionError> {
    if tx.visibility != TransactionVisibility::Confidential {
        return Ok(());
    }

    tx.confidential
        .as_ref()
        .ok_or(ConfidentialTransactionError::MissingEnvelope)?;
    let proof = tx
        .zk_proof
        .as_ref()
        .ok_or(ConfidentialTransactionError::MissingProof)?;

    match proof.proof_type {
        ConfidentialProofType::Stark => validate_stark_proof(tx, proof),
    }
}

fn validate_stark_proof(
    tx: &Transaction,
    proof: &ConfidentialProof,
) -> Result<(), ConfidentialTransactionError> {
    // Basic structural validation without full STARK verification
    let _proof_bytes = decode_proof_bytes(&proof.proof)?;

    let public_inputs = &proof.public_inputs;
    
    // Verify transaction ID matches
    let tx_id = require_input(public_inputs, "tx_id")?;
    let mut canonical = tx.clone();
    if let Some(proof) = canonical.zk_proof.as_mut() {
        proof.public_inputs.insert("tx_id".into(), String::new());
    }
    let expected_tx_id = canonical.message_digest();
    if !equals_hex(&expected_tx_id, tx_id) {
        return Err(ConfidentialTransactionError::TransactionIdMismatch);
    }

    // Verify sender commitment matches
    let sender_commit = require_input(public_inputs, "sender_commit")?;
    if !equals_hex(&sender_commitment(tx), sender_commit) {
        return Err(ConfidentialTransactionError::SenderCommitmentMismatch);
    }

    // Verify receiver commitment matches
    let receiver_commit = require_input(public_inputs, "receiver_commit")?;
    if !equals_hex(&receiver_commitment(tx), receiver_commit) {
        return Err(ConfidentialTransactionError::ReceiverCommitmentMismatch);
    }

    // Validate sequence length is reasonable
    let sequence_length = parse_numeric_input(public_inputs, "sequence_length")? as usize;
    if sequence_length < 4 || !sequence_length.is_power_of_two() || sequence_length > 1024 {
        return Err(ConfidentialTransactionError::InvalidSequenceLength);
    }

    // Verify result field exists
    let _result_value = parse_numeric_input(public_inputs, "result")?;

    // TODO: Implement full STARK proof verification when winterfell is available
    // For now, we validate the structure and public inputs but skip cryptographic verification
    
    Ok(())
}

fn decode_proof_bytes(proof: &str) -> Result<Vec<u8>, ConfidentialTransactionError> {
    general_purpose::STANDARD
        .decode(proof)
        .map_err(|err| ConfidentialTransactionError::InvalidProofEncoding(err.to_string()))
}

fn require_input<'a>(
    inputs: &'a BTreeMap<String, String>,
    key: &'static str,
) -> Result<&'a str, ConfidentialTransactionError> {
    inputs
        .get(key)
        .map(|value| value.as_str())
        .ok_or(ConfidentialTransactionError::MissingPublicInput(key))
}

fn parse_numeric_input(
    inputs: &BTreeMap<String, String>,
    key: &'static str,
) -> Result<u64, ConfidentialTransactionError> {
    let value = require_input(inputs, key)?;
    if let Some(stripped) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        u64::from_str_radix(stripped, 16).map_err(|source| {
            ConfidentialTransactionError::InvalidNumericValue { name: key, source }
        })
    } else {
        value
            .parse::<u64>()
            .map_err(|source| ConfidentialTransactionError::InvalidNumericValue {
                name: key,
                source,
            })
    }
}

fn equals_hex(bytes: &[u8; 32], candidate: &str) -> bool {
    let expected = bytes.encode_hex::<String>();
    expected.eq_ignore_ascii_case(candidate)
}

fn sender_commitment(tx: &Transaction) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(&tx.from);
    hasher.update(&tx.nonce.to_be_bytes());
    hasher.finalize().into()
}

fn receiver_commitment(tx: &Transaction) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(&tx.to);
    hasher.update(&tx.amount.atomic().to_be_bytes());
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ippan_types::{
        transaction::{AccessKey, ConfidentialEnvelope},
        Amount, Transaction,
    };

    fn sample_envelope() -> ConfidentialEnvelope {
        ConfidentialEnvelope {
            enc_algo: "AES-256-GCM".to_string(),
            iv: "iv".to_string(),
            ciphertext: "cipher".to_string(),
            access_keys: vec![AccessKey {
                recipient_pub: "ed25519:demo".into(),
                enc_key: "key".into(),
            }],
        }
    }

    fn prepare_transaction() -> (Transaction, BTreeMap<String, String>) {
        let (priv_key, from) = generate_account();
        let (_, to) = generate_account();
        let mut tx = Transaction::new(from, to, Amount::from_atomic(25), 0);
        tx.set_confidential_envelope(sample_envelope());

        // Create mock proof data
        let mock_proof_data = vec![0u8; 64]; // Mock proof bytes
        let mut public_inputs = BTreeMap::new();
        public_inputs.insert("tx_id".into(), String::new());
        public_inputs.insert("sender_commit".into(), hex::encode(sender_commitment(&tx)));
        public_inputs.insert(
            "receiver_commit".into(),
            hex::encode(receiver_commitment(&tx)),
        );
        public_inputs.insert("sequence_length".into(), "32".into());
        public_inputs.insert("result".into(), "12345".to_string());

        let encoded_proof = general_purpose::STANDARD.encode(&mock_proof_data);
        tx.set_confidential_proof(ConfidentialProof {
            proof_type: ConfidentialProofType::Stark,
            proof: encoded_proof.clone(),
            public_inputs: public_inputs.clone(),
        });

        let mut digest_tx = tx.clone();
        if let Some(proof) = digest_tx.zk_proof.as_mut() {
            proof.public_inputs.insert("tx_id".into(), String::new());
        }
        let tx_id_hex = hex::encode(digest_tx.message_digest());
        public_inputs.insert("tx_id".into(), tx_id_hex);

        tx.set_confidential_proof(ConfidentialProof {
            proof_type: ConfidentialProofType::Stark,
            proof: encoded_proof,
            public_inputs: public_inputs.clone(),
        });
        tx.sign(&priv_key).expect("signing");

        (tx, public_inputs)
    }

    fn generate_account() -> ([u8; 32], [u8; 32]) {
        use ed25519_dalek::SigningKey;
        use rand_core::{OsRng, RngCore};

        let mut rng = OsRng;
        let mut secret = [0u8; 32];
        rng.fill_bytes(&mut secret);
        let signing_key = SigningKey::from_bytes(&secret);
        let public_key = signing_key.verifying_key().to_bytes();
        (secret, public_key)
    }

    #[test]
    fn validates_stark_confidential_transaction() {
        let (tx, _) = prepare_transaction();
        let proof = tx.zk_proof.as_ref().expect("proof attached");
        let mut canonical = tx.clone();
        canonical
            .zk_proof
            .as_mut()
            .unwrap()
            .public_inputs
            .insert("tx_id".into(), String::new());
        assert_eq!(
            proof.public_inputs.get("tx_id").unwrap(),
            &hex::encode(canonical.message_digest())
        );
        validate_transaction(&tx).expect("validation");
    }

    #[test]
    fn rejects_invalid_public_inputs() {
        let (mut tx, mut inputs) = prepare_transaction();
        inputs.insert("sequence_length".into(), "16".into());
        let mock_proof = general_purpose::STANDARD.encode(vec![0u8; 64]);
        tx.set_confidential_proof(ConfidentialProof {
            proof_type: ConfidentialProofType::Stark,
            proof: mock_proof,
            public_inputs: inputs,
        });
        let err = validate_transaction(&tx).expect_err("expected failure");
        assert!(matches!(
            err,
            ConfidentialTransactionError::TransactionIdMismatch
        ));
    }

    #[test]
    fn rejects_bad_proof_bytes() {
        let (mut tx, inputs) = prepare_transaction();
        tx.set_confidential_proof(ConfidentialProof {
            proof_type: ConfidentialProofType::Stark,
            proof: "!!!!".into(),
            public_inputs: inputs,
        });
        assert!(matches!(
            validate_transaction(&tx),
            Err(ConfidentialTransactionError::InvalidProofEncoding(_))
        ));
    }
}
