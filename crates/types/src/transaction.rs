use crate::currency::Amount;
use crate::handle::HandleOperation;
use crate::{HashTimer, IppanTimeMicros};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use serde_bytes;
use std::collections::BTreeMap;

/// Visibility options for transaction payloads.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum TransactionVisibility {
    /// Transaction payload is plaintext and globally readable.
    #[default]
    Public,
    /// Transaction payload is encrypted and only accessible to entitled parties.
    Confidential,
}

impl TransactionVisibility {
    fn as_byte(self) -> u8 {
        match self {
            Self::Public => 0,
            Self::Confidential => 1,
        }
    }
}

/// Envelope entry mapping an entitled recipient to an encrypted symmetric key.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccessKey {
    /// Recipient public key (e.g., encoded Ed25519 key).
    pub recipient_pub: String,
    /// Symmetric key encrypted to the recipient (base64 or hex).
    pub enc_key: String,
}

/// Confidential transaction payload metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfidentialEnvelope {
    /// Encryption algorithm identifier (e.g., "AES-256-GCM").
    pub enc_algo: String,
    /// Initialization vector / nonce (base64 or hex string).
    pub iv: String,
    /// Ciphertext of the original payload (base64 or hex string).
    pub ciphertext: String,
    /// One entry per entitled reader.
    pub access_keys: Vec<AccessKey>,
}

/// Supported zero-knowledge proof systems for confidential payload validation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConfidentialProofType {
    /// Zero-knowledge STARK proof.
    Stark,
}

impl ConfidentialProofType {
    fn as_byte(self) -> u8 {
        match self {
            ConfidentialProofType::Stark => 0,
        }
    }
}

/// Metadata describing a confidential proof attached to a transaction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfidentialProof {
    /// Type of proof supplied by the transaction author.
    #[serde(rename = "proof_type")]
    pub proof_type: ConfidentialProofType,
    /// Base64- or hex-encoded serialized proof bytes.
    pub proof: String,
    /// Public inputs bound to the proof (sorted for deterministic hashing).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub public_inputs: BTreeMap<String, String>,
}

/// A transaction in the IPPAN blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Transaction ID (32 bytes)
    pub id: [u8; 32],
    /// Sender address (32 bytes)
    pub from: [u8; 32],
    /// Recipient address (32 bytes)
    pub to: [u8; 32],
    /// Amount to transfer (in atomic units with 24 decimal precision)
    pub amount: Amount,
    /// Nonce for replay protection
    pub nonce: u64,
    /// HashTimer for temporal ordering and validation
    pub hashtimer: HashTimer,
    /// Timestamp when transaction was created
    pub timestamp: IppanTimeMicros,
    /// Visibility flag describing how the payload should be handled.
    #[serde(default)]
    pub visibility: TransactionVisibility,
    /// Optional cleartext topics/tags for routing or indexing.
    #[serde(default)]
    pub topics: Vec<String>,
    /// Optional embedded handle operation payload.
    #[serde(default)]
    pub handle_op: Option<HandleOperation>,
    /// Optional confidential payload envelope.
    #[serde(default)]
    pub confidential: Option<ConfidentialEnvelope>,
    /// Optional zero-knowledge proof metadata.
    #[serde(default)]
    pub zk_proof: Option<ConfidentialProof>,
    /// Transaction signature (64 bytes)
    #[serde(with = "serde_bytes")]
    pub signature: [u8; 64],
}

impl Transaction {
    /// Create a new transaction
    pub fn new(from: [u8; 32], to: [u8; 32], amount: Amount, nonce: u64) -> Self {
        let timestamp = IppanTimeMicros::now();
        let payload = Self::create_payload(&from, &to, amount, nonce, None);
        let hashtimer = HashTimer::derive(
            "transaction",
            timestamp,
            b"transaction",
            &payload,
            &nonce.to_be_bytes(),
            &from,
        );

        Self {
            id: [0u8; 32], // Will be computed after signing
            from,
            to,
            amount,
            nonce,
            visibility: TransactionVisibility::Public,
            topics: Vec::new(),
            handle_op: None,
            confidential: None,
            zk_proof: None,
            signature: [0u8; 64], // Will be set after signing
            hashtimer,
            timestamp,
        }
    }

    /// Attach cleartext topics/tags to the transaction body.
    pub fn set_topics(&mut self, topics: Vec<String>) {
        self.topics = topics;
    }

    /// Attach a handle operation payload (e.g. registration) to the transaction.
    pub fn set_handle_operation(&mut self, operation: HandleOperation) {
        self.handle_op = Some(operation);
        self.refresh_hashtimer();
    }

    /// Returns the embedded handle operation, if any.
    pub fn handle_operation(&self) -> Option<&HandleOperation> {
        self.handle_op.as_ref()
    }

    /// Attach a confidential envelope and mark the transaction as confidential.
    pub fn set_confidential_envelope(&mut self, envelope: ConfidentialEnvelope) {
        self.visibility = TransactionVisibility::Confidential;
        self.confidential = Some(envelope);
    }

    /// Remove any confidential envelope and revert to public visibility.
    pub fn clear_confidential_envelope(&mut self) {
        self.visibility = TransactionVisibility::Public;
        self.confidential = None;
        self.zk_proof = None;
    }

    /// Attach a zero-knowledge proof to the transaction.
    pub fn set_confidential_proof(&mut self, proof: ConfidentialProof) {
        self.zk_proof = Some(proof);
    }

    /// Remove any zero-knowledge proof metadata from the transaction.
    pub fn clear_confidential_proof(&mut self) {
        self.zk_proof = None;
    }

    /// Recompute the transaction identifier from its contents.
    pub fn refresh_id(&mut self) {
        self.id = self.hash();
    }

    fn refresh_hashtimer(&mut self) {
        let payload = Self::create_payload(
            &self.from,
            &self.to,
            self.amount,
            self.nonce,
            self.handle_op.as_ref(),
        );
        self.hashtimer = HashTimer::derive(
            "transaction",
            self.timestamp,
            b"transaction",
            &payload,
            &self.nonce.to_be_bytes(),
            &self.from,
        );
    }

    /// Compute the canonical transaction hash using BLAKE3.
    fn compute_hash(&self) -> [u8; 32] {
        let hash = blake3::hash(&self.canonical_bytes());
        let mut result = [0u8; 32];
        result.copy_from_slice(hash.as_bytes());
        result
    }

    /// Create payload for HashTimer computation
    fn create_payload(
        from: &[u8; 32],
        to: &[u8; 32],
        amount: Amount,
        nonce: u64,
        handle_op: Option<&HandleOperation>,
    ) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(from);
        payload.extend_from_slice(to);
        payload.extend_from_slice(&amount.atomic().to_be_bytes());
        payload.extend_from_slice(&nonce.to_be_bytes());
        if let Some(op) = handle_op {
            payload.push(1);
            Self::append_handle_operation(&mut payload, op);
        } else {
            payload.push(0);
        }
        payload
    }

    /// Bytes used for signature verification (excludes signature and id)
    fn message_bytes(&self) -> Vec<u8> {
        // Capacity hint only; it may be slightly overestimated and that's fine.
        let mut bytes = Vec::with_capacity(
            self.from.len() + self.to.len() + 16 + 8 + 40 + 8 + 1, // rough sizes
        );
        bytes.extend_from_slice(&self.from);
        bytes.extend_from_slice(&self.to);
        bytes.extend_from_slice(&self.amount.atomic().to_be_bytes());
        bytes.extend_from_slice(&self.nonce.to_be_bytes());
        // Serialize HashTimer: timestamp_us (8 bytes) + entropy (32 bytes) = 40 bytes
        bytes.extend_from_slice(&self.hashtimer.timestamp_us.to_be_bytes());
        bytes.extend_from_slice(&self.hashtimer.entropy);
        bytes.extend_from_slice(&self.timestamp.0.to_be_bytes());
        bytes.push(self.visibility.as_byte());
        Self::append_topics(&mut bytes, &self.topics);
        match &self.handle_op {
            Some(op) => {
                bytes.push(1);
                Self::append_handle_operation(&mut bytes, op);
            }
            None => bytes.push(0),
        }
        match &self.confidential {
            Some(envelope) => {
                bytes.push(1);
                Self::append_confidential(&mut bytes, envelope);
            }
            None => bytes.push(0),
        }
        match &self.zk_proof {
            Some(proof) => {
                bytes.push(1);
                Self::append_confidential_proof(&mut bytes, proof);
            }
            None => bytes.push(0),
        }
        bytes
    }

    /// Bytes used for hashing (includes signature)
    fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = self.message_bytes();
        bytes.extend_from_slice(&self.signature);
        bytes
    }

    /// Digest of the canonical transaction message without the signature bytes.
    pub fn message_digest(&self) -> [u8; 32] {
        let hash = blake3::hash(&self.message_bytes());
        let mut digest = [0u8; 32];
        digest.copy_from_slice(hash.as_bytes());
        digest
    }

    fn append_topics(bytes: &mut Vec<u8>, topics: &[String]) {
        bytes.extend_from_slice(&(topics.len() as u32).to_be_bytes());
        for topic in topics {
            Self::append_length_prefixed(bytes, topic.as_bytes());
        }
    }

    fn append_handle_operation(bytes: &mut Vec<u8>, op: &HandleOperation) {
        match op {
            HandleOperation::Register(data) => {
                bytes.push(0);
                Self::append_length_prefixed(bytes, data.handle.as_bytes());
                bytes.extend_from_slice(&data.owner);
                if let Some(exp) = data.expires_at {
                    bytes.push(1);
                    bytes.extend_from_slice(&exp.to_be_bytes());
                } else {
                    bytes.push(0);
                }
                bytes.extend_from_slice(&(data.metadata.len() as u32).to_be_bytes());
                for (key, value) in &data.metadata {
                    Self::append_length_prefixed(bytes, key.as_bytes());
                    Self::append_length_prefixed(bytes, value.as_bytes());
                }
                Self::append_length_prefixed(bytes, &data.signature);
            }
        }
    }

    fn append_confidential(bytes: &mut Vec<u8>, envelope: &ConfidentialEnvelope) {
        Self::append_length_prefixed(bytes, envelope.enc_algo.as_bytes());
        Self::append_length_prefixed(bytes, envelope.iv.as_bytes());
        Self::append_length_prefixed(bytes, envelope.ciphertext.as_bytes());
        bytes.extend_from_slice(&(envelope.access_keys.len() as u32).to_be_bytes());
        for access_key in &envelope.access_keys {
            Self::append_length_prefixed(bytes, access_key.recipient_pub.as_bytes());
            Self::append_length_prefixed(bytes, access_key.enc_key.as_bytes());
        }
    }

    fn append_confidential_proof(bytes: &mut Vec<u8>, proof: &ConfidentialProof) {
        bytes.push(proof.proof_type.as_byte());
        Self::append_length_prefixed(bytes, proof.proof.as_bytes());
        bytes.extend_from_slice(&(proof.public_inputs.len() as u32).to_be_bytes());
        for (key, value) in &proof.public_inputs {
            Self::append_length_prefixed(bytes, key.as_bytes());
            Self::append_length_prefixed(bytes, value.as_bytes());
        }
    }

    fn append_length_prefixed(bytes: &mut Vec<u8>, data: &[u8]) {
        bytes.extend_from_slice(&(data.len() as u32).to_be_bytes());
        bytes.extend_from_slice(data);
    }

    /// Sign the transaction using an Ed25519 private key
    pub fn sign(&mut self, private_key: &[u8; 32]) -> Result<(), String> {
        let signing_key = SigningKey::try_from(private_key.as_slice())
            .map_err(|e| format!("invalid private key: {e}"))?;
        let expected_public_key = signing_key.verifying_key().to_bytes();

        if self.from != expected_public_key {
            return Err("private key does not match transaction sender".into());
        }

        let message = self.message_bytes();
        let signature = signing_key.sign(&message);
        self.signature.copy_from_slice(&signature.to_bytes());
        self.id = self.hash();

        Ok(())
    }

    /// Verify the transaction signature
    pub fn verify(&self) -> bool {
        match VerifyingKey::from_bytes(&self.from) {
            Ok(verifying_key) => {
                let signature = Signature::from_bytes(&self.signature);
                verifying_key
                    .verify(&self.message_bytes(), &signature)
                    .is_ok()
            }
            Err(_) => false,
        }
    }

    /// Get transaction hash
    pub fn hash(&self) -> [u8; 32] {
        self.compute_hash()
    }

    /// Check if transaction is valid
    pub fn is_valid(&self) -> bool {
        // Basic validation checks
        let has_handle = self.handle_op.is_some();
        if self.visibility == TransactionVisibility::Confidential {
            if self.confidential.is_none() || self.zk_proof.is_none() {
                return false;
            }
        } else if !has_handle {
            if self.amount.is_zero() {
                return false;
            }

            if self.from == self.to {
                return false;
            }
        }

        // Verify signature
        if !self.verify() {
            return false;
        }

        // Ensure the id matches the canonical hash
        if self.id != self.hash() {
            return false;
        }

        // Verify HashTimer is valid and consistent with contents
        let payload = Self::create_payload(
            &self.from,
            &self.to,
            self.amount,
            self.nonce,
            self.handle_op.as_ref(),
        );
        let expected_hashtimer = HashTimer::derive(
            "transaction",
            self.timestamp,
            b"transaction",
            &payload,
            &self.nonce.to_be_bytes(),
            &self.from,
        );

        if expected_hashtimer != self.hashtimer {
            return false;
        }

        if let Some(op) = &self.handle_op {
            if op.validate_for_sender(&self.from).is_err() {
                return false;
            }
        }

        // HashTimer should not be from the future
        self.hashtimer.time().0 <= IppanTimeMicros::now().0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand_core::{OsRng, RngCore};

    fn generate_account() -> ([u8; 32], [u8; 32]) {
        let mut rng = OsRng;
        let mut secret = [0u8; 32];
        rng.fill_bytes(&mut secret);
        let signing_key = SigningKey::try_from(secret.as_slice()).unwrap();
        let public_key = signing_key.verifying_key().to_bytes();
        (secret, public_key)
    }

    #[test]
    fn test_transaction_creation() {
        let (_, from) = generate_account();
        let (_, to) = generate_account();
        let amount = Amount::from_micro_ipn(1000);
        let nonce = 1;

        let tx = Transaction::new(from, to, amount, nonce);

        assert_eq!(tx.from, from);
        assert_eq!(tx.to, to);
        assert_eq!(tx.amount, amount);
        assert_eq!(tx.nonce, nonce);
        assert!(tx.hashtimer.to_hex().len() == 64);
    }

    #[test]
    fn test_transaction_signing() {
        let (private_key, from) = generate_account();
        let (_, to) = generate_account();
        let mut tx = Transaction::new(from, to, Amount::from_micro_ipn(1000), 1);

        let result = tx.sign(&private_key);
        assert!(result.is_ok());
        assert_ne!(tx.signature, [0u8; 64]);
        assert_ne!(tx.id, [0u8; 32]);
    }

    #[test]
    fn test_transaction_verification() {
        let (private_key, from) = generate_account();
        let (_, to) = generate_account();
        let mut tx = Transaction::new(from, to, Amount::from_micro_ipn(1000), 1);

        tx.sign(&private_key).unwrap();
        assert!(tx.verify());
    }

    #[test]
    fn test_transaction_validation() {
        let (private_key, from) = generate_account();
        let (_, to) = generate_account();
        let mut tx = Transaction::new(from, to, Amount::from_micro_ipn(1000), 1);

        tx.sign(&private_key).unwrap();
        assert!(tx.is_valid());
    }

    #[test]
    fn test_invalid_transaction_zero_amount() {
        let (private_key, from) = generate_account();
        let (_, to) = generate_account();
        let mut tx = Transaction::new(from, to, Amount::zero(), 1);

        tx.sign(&private_key).unwrap();
        assert!(!tx.is_valid());
    }

    #[test]
    fn test_invalid_transaction_same_sender_recipient() {
        let (private_key, addr) = generate_account();
        let mut tx = Transaction::new(addr, addr, Amount::from_micro_ipn(1000), 1);

        tx.sign(&private_key).unwrap();
        assert!(!tx.is_valid());
    }
}
