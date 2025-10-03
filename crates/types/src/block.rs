use crate::hashtimer::{HashTimer, IppanTimeMicros};
use crate::transaction::Transaction;
use blake3::Hasher as Blake3;
use hex;
use serde::{Deserialize, Serialize};
use serde_bytes;

/// Unique identifier for a consensus round.
pub type RoundId = u64;
/// Unique identifier for a validator (32-byte public key hash).
pub type ValidatorId = [u8; 32];
/// Canonical identifier for a block header (32-byte digest).
pub type BlockId = [u8; 32];

const HASH_CONTEXT: &str = "block";
const HASH_DOMAIN: &[u8] = b"block";

/// Block header capturing the round-scoped DAG metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlockHeader {
    /// Canonical identifier for this block (hash of the header fields).
    pub id: BlockId,
    /// Validator that created the block.
    pub creator: ValidatorId,
    /// Round identifier.
    pub round: RoundId,
    /// HashTimer anchoring the block in IPPAN time.
    pub hashtimer: HashTimer,
    /// Parent block identifiers drawn from round-1.
    pub parent_ids: Vec<BlockId>,
    /// Hex-encoded parent hashes for DA summaries.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prev_hashes: Vec<String>,
    /// Ordered payload identifiers (e.g., transaction batch digests).
    pub payload_ids: Vec<[u8; 32]>,
    /// Merkle root over payload identifiers.
    pub merkle_payload: [u8; 32],
    /// Merkle root over parent identifiers.
    pub merkle_parents: [u8; 32],
    /// Merkle/Verkle root of transactions committed in the block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tx_root: Option<String>,
    /// Merkle root of erasure-coded body shards.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub erasure_root: Option<String>,
    /// Root of receipts proving execution/state transitions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt_root: Option<String>,
    /// Global state root after executing the block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_root: Option<String>,
    /// Aggregate signatures (proposer + round validators).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub validator_sigs: Vec<String>,
    /// Optional VRF proof used for committee sampling or priority.
    #[serde(default, with = "serde_bytes")]
    pub vrf_proof: Vec<u8>,
}

/// A round-scoped DAG vertex along with its detached payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    /// Metadata header.
    pub header: BlockHeader,
    /// Signature issued by the creator (Ed25519 today, aggregated later).
    #[serde(default, with = "serde_bytes")]
    pub signature: Vec<u8>,
    /// Optional inlined transactions; maintained for compatibility with the
    /// current execution pipeline until the availability layer is wired in.
    #[serde(default)]
    pub transactions: Vec<Transaction>,
    /// Optional metadata with the parent hash strings for UI consumption.
    #[serde(default)]
    pub prev_hashes: Vec<String>,
}

fn decode_block_id(hash: &str) -> Option<BlockId> {
    let trimmed = hash
        .strip_prefix("0x")
        .or_else(|| hash.strip_prefix("0X"))
        .unwrap_or(hash);
    let bytes = hex::decode(trimmed).ok()?;
    if bytes.len() != 32 {
        return None;
    }
    let mut id = [0u8; 32];
    id.copy_from_slice(&bytes);
    Some(id)
}

impl BlockHeader {
    /// Compute the canonical identifier for the supplied header components.
    #[allow(clippy::too_many_arguments)]
    fn compute_id(
        creator: &ValidatorId,
        round: RoundId,
        hashtimer: &HashTimer,
        parent_ids: &[BlockId],
        payload_ids: &[[u8; 32]],
        merkle_payload: &[u8; 32],
        merkle_parents: &[u8; 32],
        vrf_proof: &[u8],
    ) -> BlockId {
        let mut hasher = Blake3::new();
        hasher.update(creator);
        hasher.update(&round.to_be_bytes());
        hasher.update(&hashtimer.time_prefix);
        hasher.update(&hashtimer.hash_suffix);
        hasher.update(merkle_payload);
        hasher.update(merkle_parents);
        for parent in parent_ids {
            hasher.update(parent);
        }
        for payload in payload_ids {
            hasher.update(payload);
        }
        hasher.update(vrf_proof);

        let hash = hasher.finalize();
        let mut id = [0u8; 32];
        id.copy_from_slice(&hash.as_bytes()[0..32]);
        id
    }
}

impl Block {
    /// Create a new round-scoped block with the provided parents and
    /// transactions. The transactions are flattened into payload identifiers
    /// while remaining available on the struct for the current execution path.
    pub fn new(
        parent_ids: Vec<BlockId>,
        transactions: Vec<Transaction>,
        round: RoundId,
        creator: ValidatorId,
    ) -> Self {
        let payload_ids: Vec<[u8; 32]> = transactions.iter().map(|tx| tx.hash()).collect();
        let merkle_payload = Self::compute_merkle_root_from_hashes(&payload_ids);
        let merkle_parents = Self::compute_merkle_root_from_hashes(&parent_ids);
        let hashtimer_nonce =
            Self::derive_hashtimer_nonce(round, &creator, &parent_ids, &payload_ids);
        let prev_hashes: Vec<String> = parent_ids.iter().map(hex::encode).collect();
        let hashtimer_payload = Self::build_hashtimer_payload(
            round,
            &creator,
            &parent_ids,
            &payload_ids,
            &merkle_payload,
            &merkle_parents,
        );
        let time = IppanTimeMicros::now();
        let hashtimer = HashTimer::derive(
            HASH_CONTEXT,
            time,
            HASH_DOMAIN,
            &hashtimer_payload,
            &hashtimer_nonce,
            &creator,
        );

        let id = BlockHeader::compute_id(
            &creator,
            round,
            &hashtimer,
            &parent_ids,
            &payload_ids,
            &merkle_payload,
            &merkle_parents,
            &[],
        );

        let tx_root_hex = hex::encode(merkle_payload);

        let header = BlockHeader {
            id,
            creator,
            round,
            hashtimer,
            parent_ids,
            prev_hashes: prev_hashes.clone(),
            payload_ids,
            merkle_payload,
            merkle_parents,
            tx_root: Some(tx_root_hex),
            erasure_root: None,
            receipt_root: None,
            state_root: None,
            validator_sigs: Vec::new(),
            vrf_proof: Vec::new(),
        };

        Self {
            header,
            signature: Vec::new(),
            transactions,
            prev_hashes,
        }
    }

    /// Convenience constructor for creating a block with a single parent.
    pub fn with_parent(
        parent_id: BlockId,
        transactions: Vec<Transaction>,
        round: RoundId,
        creator: ValidatorId,
    ) -> Self {
        Self::new(vec![parent_id], transactions, round, creator)
    }

    /// Compute the merkle root for an arbitrary slice of 32-byte hashes.
    pub fn compute_merkle_root_from_hashes(items: &[BlockId]) -> [u8; 32] {
        if items.is_empty() {
            return [0u8; 32];
        }

        if items.len() == 1 {
            return items[0];
        }

        let mut current_level: Vec<[u8; 32]> = items.to_vec();
        while current_level.len() > 1 {
            let mut next_level = Vec::with_capacity(current_level.len().div_ceil(2));
            for chunk in current_level.chunks(2) {
                let left = chunk[0];
                let right = if chunk.len() == 2 { chunk[1] } else { chunk[0] };
                let mut hasher = Blake3::new();
                hasher.update(&left);
                hasher.update(&right);
                let hash = hasher.finalize();
                let mut combined = [0u8; 32];
                combined.copy_from_slice(&hash.as_bytes()[0..32]);
                next_level.push(combined);
            }
            current_level = next_level;
        }

        current_level[0]
    }

    /// Deterministically derive the nonce used in the HashTimer payload.
    fn derive_hashtimer_nonce(
        round: RoundId,
        creator: &ValidatorId,
        parent_ids: &[BlockId],
        payload_ids: &[[u8; 32]],
    ) -> [u8; 32] {
        let mut hasher = Blake3::new();
        hasher.update(&round.to_be_bytes());
        hasher.update(creator);
        for parent in parent_ids {
            hasher.update(parent);
        }
        for payload in payload_ids {
            hasher.update(payload);
        }
        let hash = hasher.finalize();
        let mut nonce = [0u8; 32];
        nonce.copy_from_slice(&hash.as_bytes()[0..32]);
        nonce
    }

    /// Build the payload that is hashed into the HashTimer.
    fn build_hashtimer_payload(
        round: RoundId,
        creator: &ValidatorId,
        parent_ids: &[BlockId],
        payload_ids: &[[u8; 32]],
        merkle_payload: &[u8; 32],
        merkle_parents: &[u8; 32],
    ) -> Vec<u8> {
        let mut payload = Vec::with_capacity(
            8 + creator.len() + (parent_ids.len() + payload_ids.len()) * 32 + 64,
        );
        payload.extend_from_slice(&round.to_be_bytes());
        payload.extend_from_slice(creator);
        payload.extend_from_slice(merkle_payload);
        payload.extend_from_slice(merkle_parents);
        for parent in parent_ids {
            payload.extend_from_slice(parent);
        }
        for item in payload_ids {
            payload.extend_from_slice(item);
        }
        payload
    }

    /// Return the canonical identifier for this block.
    pub fn hash(&self) -> BlockId {
        self.header.id
    }

    /// Approximate block size (header plus inlined transactions).
    pub fn size(&self) -> usize {
        let header_size = 32 // id
            + 32 // creator
            + 8  // round
            + 7  // time prefix
            + 25 // hash suffix
            + self.header.parent_ids.len() * 32
            + self.header.payload_ids.len() * 32
            + 32 // merkle_payload
            + 32 // merkle_parents
            + self.header.vrf_proof.len()
            + self.signature.len();
        let tx_size = self.transactions.len() * 200; // heuristic for now
        let metadata_size: usize = self.prev_hashes.iter().map(|hash| hash.len()).sum();
        header_size + tx_size + metadata_size
    }

    /// Verify block integrity against the deterministic header rules.
    pub fn is_valid(&self) -> bool {
        // Merkle roots must match inputs.
        if self.header.merkle_payload
            != Self::compute_merkle_root_from_hashes(&self.header.payload_ids)
        {
            return false;
        }

        if self.header.merkle_parents
            != Self::compute_merkle_root_from_hashes(&self.header.parent_ids)
        {
            return false;
        }

        // Header.prev_hashes must correspond to parent_ids (allowing optional 0x and casing).
        let decoded_prev: Option<Vec<BlockId>> = self
            .header
            .prev_hashes
            .iter()
            .map(|hash| decode_block_id(hash))
            .collect();

        let Some(decoded_prev) = decoded_prev else {
            return false;
        };

        if decoded_prev != self.header.parent_ids {
            return false;
        }

        // tx_root, if present, must equal hex(merkle_payload).
        if let Some(tx_root) = &self.header.tx_root {
            if *tx_root != hex::encode(self.header.merkle_payload) {
                return false;
            }
        }

        // Optional UI-level prev_hashes on the outer Block struct:
        // - allow empty (legacy)
        // - if present, allow 0x prefix and uppercase, but must match expected_prev.
        if !self.prev_hashes.is_empty() {
            let expected_prev: Vec<String> =
                self.header.parent_ids.iter().map(hex::encode).collect();
            if self.prev_hashes.len() != self.header.parent_ids.len() {
                return false;
            }

            let normalized: Option<Vec<String>> = self
                .prev_hashes
                .iter()
                .map(|s| decode_block_id(s).map(hex::encode))
                .collect();

            let Some(normalized) = normalized else {
                return false;
            };

            if normalized != expected_prev {
                return false;
            }
        }

        // Transactions, if present, must match payload_ids and be valid.
        if !self.transactions.is_empty() {
            let computed_payload_ids: Vec<[u8; 32]> =
                self.transactions.iter().map(|tx| tx.hash()).collect();
            if computed_payload_ids != self.header.payload_ids {
                return false;
            }

            if self.transactions.iter().any(|tx| !tx.is_valid()) {
                return false;
            }
        }

        // HashTimer must be reproducible from header components (same time).
        let hashtimer_payload = Self::build_hashtimer_payload(
            self.header.round,
            &self.header.creator,
            &self.header.parent_ids,
            &self.header.payload_ids,
            &self.header.merkle_payload,
            &self.header.merkle_parents,
        );
        let expected_nonce = Self::derive_hashtimer_nonce(
            self.header.round,
            &self.header.creator,
            &self.header.parent_ids,
            &self.header.payload_ids,
        );
        let expected_hashtimer = HashTimer::derive(
            HASH_CONTEXT,
            self.header.hashtimer.time(),
            HASH_DOMAIN,
            &hashtimer_payload,
            &expected_nonce,
            &self.header.creator,
        );

        if expected_hashtimer != self.header.hashtimer {
            return false;
        }

        // Header ID must match.
        let expected_id = BlockHeader::compute_id(
            &self.header.creator,
            self.header.round,
            &self.header.hashtimer,
            &self.header.parent_ids,
            &self.header.payload_ids,
            &self.header.merkle_payload,
            &self.header.merkle_parents,
            &self.header.vrf_proof,
        );

        if expected_id != self.header.id {
            return false;
        }

        // Ensure the declared time is not from the future.
        self.header.hashtimer.time().0 <= IppanTimeMicros::now().0
    }

    /// Update erasure/receipt/state roots after executing the block.
    pub fn set_data_availability_roots(
        &mut self,
        erasure_root: Option<String>,
        receipt_root: Option<String>,
        state_root: Option<String>,
    ) {
        self.header.erasure_root = erasure_root;
        self.header.receipt_root = receipt_root;
        self.header.state_root = state_root;
    }

    /// Append an aggregated validator signature entry.
    pub fn push_validator_signature(&mut self, signature: String) {
        self.header.validator_sigs.push(signature);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;

    fn sample_transactions() -> Vec<Transaction> {
        let secret1 = SigningKey::from_bytes(&[7u8; 32]);
        let secret2 = SigningKey::from_bytes(&[8u8; 32]);
        let mut tx1 = Transaction::new(secret1.verifying_key().to_bytes(), [4u8; 32], 1000, 1);
        tx1.sign(&secret1.to_bytes()).expect("sign tx1");
        let mut tx2 = Transaction::new(secret2.verifying_key().to_bytes(), [6u8; 32], 2000, 2);
        tx2.sign(&secret2.to_bytes()).expect("sign tx2");
        vec![tx1, tx2]
    }

    #[test]
    fn test_block_creation() {
        let parent = [1u8; 32];
        let creator = [2u8; 32];
        let round = 1;
        let block = Block::new(vec![parent], sample_transactions(), round, creator);

        assert_eq!(block.header.creator, creator);
        assert_eq!(block.header.round, round);
        assert_eq!(block.header.parent_ids, vec![parent]);
        assert_eq!(block.header.prev_hashes, vec![hex::encode(parent)]);
        assert_eq!(block.transactions.len(), 2);
        assert_eq!(block.header.payload_ids.len(), 2);
        assert_eq!(block.hash(), block.header.id);
        assert_eq!(block.header.merkle_payload.len(), 32);
        assert_eq!(block.header.merkle_parents.len(), 32);
        assert_eq!(block.header.hashtimer.to_hex().len(), 64);
        assert_eq!(
            block.header.tx_root.as_ref().map(|root| root.len()),
            Some(64)
        );
        assert!(block.header.validator_sigs.is_empty());
    }

    #[test]
    fn test_empty_block() {
        let creator = [2u8; 32];
        let round = 1;
        let block = Block::new(vec![], vec![], round, creator);

        assert!(block.transactions.is_empty());
        assert!(block.header.payload_ids.is_empty());
        assert_eq!(block.header.merkle_payload, [0u8; 32]);
        assert_eq!(block.header.merkle_parents, [0u8; 32]);
        assert!(block.header.prev_hashes.is_empty());
        assert_eq!(
            block.header.tx_root.as_ref().map(|root| root.len()),
            Some(64)
        );
    }

    #[test]
    fn test_merkle_root_computation() {
        let hashes = vec![[1u8; 32], [2u8; 32], [3u8; 32]];
        let root = Block::compute_merkle_root_from_hashes(&hashes);
        assert_ne!(root, [0u8; 32]);
    }

    #[test]
    fn test_block_validation() {
        let parent = [7u8; 32];
        let creator = [8u8; 32];
        let block = Block::new(vec![parent], sample_transactions(), 5, creator);
        assert!(block.is_valid());
    }

    #[test]
    fn block_validation_allows_empty_prev_hashes() {
        let parent = [9u8; 32];
        let creator = [10u8; 32];
        let mut block = Block::new(vec![parent], sample_transactions(), 6, creator);
        block.prev_hashes.clear();

        assert!(block.is_valid());
    }

    #[test]
    fn block_validation_rejects_mismatched_prev_hashes() {
        let parent = [11u8; 32];
        let creator = [12u8; 32];
        let mut block = Block::new(vec![parent], sample_transactions(), 7, creator);
        let mut wrong_hash = hex::encode(parent);
        wrong_hash.replace_range(0..2, "ff");
        block.prev_hashes = vec![wrong_hash];

        assert!(!block.is_valid());
    }

    #[test]
    fn block_validation_accepts_prefixed_prev_hashes() {
        let parent = [13u8; 32];
        let creator = [14u8; 32];
        let mut block = Block::new(vec![parent], sample_transactions(), 8, creator);
        block.prev_hashes = vec![format!("0x{}", hex::encode_upper(parent))];

        assert!(block.is_valid());
    }

    #[test]
    fn block_validation_accepts_header_prev_hash_variants() {
        let parent = [15u8; 32];
        let creator = [16u8; 32];
        let mut block = Block::new(vec![parent], sample_transactions(), 9, creator);
        block.header.prev_hashes = block
            .header
            .prev_hashes
            .iter()
            .map(|hash| format!("0X{}", hash.to_uppercase()))
            .collect();

        assert!(block.is_valid());
    }
}
