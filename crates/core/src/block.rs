//! IPPAN Block and header structures with HashTimer integration.
//!
//! Blocks in the IPPAN BlockDAG embed a signed [`HashTimer`], providing a
//! cryptographically verifiable anchor into the deterministic network time
//! stream. The helpers in this module construct new blocks, compute their
//! hashes, and validate incoming blocks before they are inserted into the DAG.

use std::convert::{TryFrom, TryInto};

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use ippan_time::{now_us, sign_hashtimer, verify_hashtimer, HashTimer};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[cfg(test)]
use rand_core::OsRng;

/// Binary merkle roots and block signatures are all 32 or 64 byte arrays.
const SIGNATURE_BYTES: usize = 64;
const PUBLIC_KEY_BYTES: usize = 32;

/// Block header describing a BlockDAG vertex.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlockHeader {
    /// Semantic version for future migrations.
    pub version: u8,
    /// Creator's Ed25519 verifying key as raw bytes.
    pub creator: Vec<u8>,
    /// Parent block hashes referenced by this block.
    pub parent_hashes: Vec<[u8; 32]>,
    /// HashTimer proving when the block was authored.
    pub hash_timer: HashTimer,
    /// Snapshot of the local median IPPAN time when authored.
    pub median_time_us: i64,
    /// Merkle root over the transactions embedded in the block body.
    pub merkle_root: [u8; 32],
    /// Ed25519 signature produced by the creator across the header fields.
    pub signature: Vec<u8>,
}

/// Block structure containing serialized transactions alongside the header.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Block {
    /// Header metadata with HashTimer and signature information.
    pub header: BlockHeader,
    /// Transactions or payload fragments carried by this block.
    pub transactions: Vec<Vec<u8>>,
}

impl Block {
    /// Create a new block signed by the provided Ed25519 [`SigningKey`].
    pub fn new(
        signing_key: &SigningKey,
        parent_hashes: Vec<[u8; 32]>,
        transactions: Vec<Vec<u8>>,
    ) -> Self {
        let hash_timer = sign_hashtimer(signing_key);
        let median_time_us = now_us();
        let merkle_root = compute_merkle_root(&transactions);
        let creator_key = signing_key.verifying_key().to_bytes().to_vec();

        let digest = signing_digest(
            1,
            &creator_key,
            &parent_hashes,
            &hash_timer,
            median_time_us,
            &merkle_root,
        );
        let signature = signing_key.sign(&digest).to_bytes().to_vec();

        let header = BlockHeader {
            version: 1,
            creator: creator_key,
            parent_hashes,
            hash_timer,
            median_time_us,
            merkle_root,
            signature,
        };

        Block {
            header,
            transactions,
        }
    }

    /// Compute the canonical hash of this block's header.
    pub fn hash(&self) -> [u8; 32] {
        header_hash(&self.header)
    }

    /// Verify the HashTimer, merkle root, and signature for this block.
    pub fn verify(&self) -> bool {
        if !verify_hashtimer(&self.header.hash_timer) {
            return false;
        }

        if compute_merkle_root(&self.transactions) != self.header.merkle_root {
            return false;
        }

        let public_key_bytes: [u8; PUBLIC_KEY_BYTES] =
            match self.header.creator.as_slice().try_into() {
                Ok(bytes) => bytes,
                Err(_) => return false,
            };
        let verifying_key = match VerifyingKey::from_bytes(&public_key_bytes) {
            Ok(key) => key,
            Err(_) => return false,
        };

        let signature_bytes: [u8; SIGNATURE_BYTES] =
            match self.header.signature.as_slice().try_into() {
                Ok(bytes) => bytes,
                Err(_) => return false,
            };

        let signature = match Signature::try_from(&signature_bytes[..]) {
            Ok(sig) => sig,
            Err(_) => return false,
        };

        let digest = signing_digest(
            self.header.version,
            &self.header.creator,
            &self.header.parent_hashes,
            &self.header.hash_timer,
            self.header.median_time_us,
            &self.header.merkle_root,
        );

        verifying_key.verify(&digest, &signature).is_ok()
    }
}

/// Compute a deterministic merkle root over the provided transactions.
fn compute_merkle_root(transactions: &[Vec<u8>]) -> [u8; 32] {
    if transactions.is_empty() {
        return [0u8; 32];
    }

    let mut level: Vec<[u8; 32]> = transactions
        .iter()
        .map(|tx| Sha256::digest(tx).into())
        .collect();

    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len().div_ceil(2));
        for chunk in level.chunks(2) {
            let mut hasher = Sha256::new();
            hasher.update(chunk[0]);
            if chunk.len() == 2 {
                hasher.update(chunk[1]);
            } else {
                hasher.update(chunk[0]);
            }
            next.push(hasher.finalize().into());
        }
        level = next;
    }

    level[0]
}

fn signing_digest(
    version: u8,
    creator: &[u8],
    parent_hashes: &[[u8; 32]],
    hash_timer: &HashTimer,
    median_time_us: i64,
    merkle_root: &[u8; 32],
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update([version]);
    hasher.update(creator);
    for parent in parent_hashes {
        hasher.update(parent);
    }
    hasher.update(hash_timer.digest());
    hasher.update(median_time_us.to_be_bytes());
    hasher.update(merkle_root);
    hasher.finalize().into()
}

fn header_hash(header: &BlockHeader) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update([header.version]);
    hasher.update(&header.creator);
    for parent in &header.parent_hashes {
        hasher.update(parent);
    }
    hasher.update(header.hash_timer.digest());
    hasher.update(header.median_time_us.to_be_bytes());
    hasher.update(header.merkle_root);
    hasher.update(&header.signature);
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_round_trip_verification() {
        let mut rng = OsRng;
        let signing_key = SigningKey::generate(&mut rng);
        let parent_hashes = vec![[1u8; 32], [2u8; 32]];
        let transactions = vec![b"tx-a".to_vec(), b"tx-b".to_vec()];

        let block = Block::new(&signing_key, parent_hashes.clone(), transactions.clone());
        assert!(block.verify());
        assert_ne!(block.hash(), [0u8; 32]);
        assert_eq!(block.header.parent_hashes, parent_hashes);
        assert_eq!(block.transactions, transactions);
    }

    #[test]
    fn tampered_transactions_fail_verification() {
        let mut rng = OsRng;
        let signing_key = SigningKey::generate(&mut rng);
        let parent_hashes = vec![[9u8; 32]];
        let transactions = vec![b"tx-a".to_vec()];

        let mut block = Block::new(&signing_key, parent_hashes, transactions);
        block.transactions.push(b"evil".to_vec());
        assert!(!block.verify());
    }

    #[test]
    fn tampered_signature_is_rejected() {
        let mut rng = OsRng;
        let signing_key = SigningKey::generate(&mut rng);
        let parent_hashes = vec![[0u8; 32]];
        let transactions = vec![b"tx".to_vec()];

        let mut block = Block::new(&signing_key, parent_hashes, transactions);
        block.header.signature[0] ^= 0xFF;
        assert!(!block.verify());
    }

    // =====================================================================
    // Deterministic Block Validation Tests - Critical Path Coverage
    // =====================================================================

    #[test]
    fn block_with_invalid_hashtimer_signature_rejected() {
        let mut rng = OsRng;
        let signing_key = SigningKey::generate(&mut rng);
        let parent_hashes = vec![[1u8; 32]];
        let transactions = vec![b"tx".to_vec()];

        let mut block = Block::new(&signing_key, parent_hashes, transactions);

        // Tamper with HashTimer signature
        if !block.header.hash_timer.signature.is_empty() {
            block.header.hash_timer.signature[0] ^= 0xFF;
        }

        assert!(
            !block.verify(),
            "Block with invalid HashTimer should be rejected"
        );
    }

    #[test]
    fn block_with_tampered_merkle_root_rejected() {
        let mut rng = OsRng;
        let signing_key = SigningKey::generate(&mut rng);
        let transactions = vec![b"tx1".to_vec(), b"tx2".to_vec()];
        let mut block = Block::new(&signing_key, vec![], transactions);

        // Tamper with merkle_root
        block.header.merkle_root[0] ^= 0xFF;

        assert!(
            !block.verify(),
            "Block with invalid merkle_root should be rejected"
        );
    }

    #[test]
    fn block_validation_deterministic_across_runs() {
        let mut rng = OsRng;
        let signing_key = SigningKey::generate(&mut rng);
        let transactions = vec![b"deterministic-tx".to_vec()];
        let block = Block::new(&signing_key, vec![[9u8; 32]], transactions);

        // Validate multiple times - must always return same result
        let first_result = block.verify();
        for _ in 0..100 {
            assert_eq!(
                block.verify(),
                first_result,
                "Block validation must be deterministic"
            );
        }
    }

    #[test]
    fn block_with_empty_transactions_valid() {
        let mut rng = OsRng;
        let signing_key = SigningKey::generate(&mut rng);
        let block = Block::new(&signing_key, vec![], vec![]);

        assert!(block.verify(), "Empty block should be valid");
        assert_eq!(
            block.header.merkle_root, [0u8; 32],
            "Empty block has zero merkle root"
        );
    }

    #[test]
    fn block_header_hash_collision_resistance() {
        let mut rng = OsRng;
        let signing_key = SigningKey::generate(&mut rng);

        let block1 = Block::new(&signing_key, vec![[1u8; 32]], vec![b"tx1".to_vec()]);
        let block2 = Block::new(&signing_key, vec![[2u8; 32]], vec![b"tx2".to_vec()]);

        assert_ne!(
            block1.hash(),
            block2.hash(),
            "Different blocks must have different hashes"
        );
    }

    #[test]
    fn block_with_single_parent_valid() {
        let mut rng = OsRng;
        let signing_key = SigningKey::generate(&mut rng);
        let parent = [42u8; 32];
        let block = Block::new(&signing_key, vec![parent], vec![]);

        assert!(block.verify());
        assert_eq!(block.header.parent_hashes.len(), 1);
        assert_eq!(block.header.parent_hashes[0], parent);
    }

    #[test]
    fn block_with_multiple_parents_valid() {
        let mut rng = OsRng;
        let signing_key = SigningKey::generate(&mut rng);
        let parents = vec![[1u8; 32], [2u8; 32], [3u8; 32]];
        let block = Block::new(&signing_key, parents.clone(), vec![]);

        assert!(block.verify());
        assert_eq!(block.header.parent_hashes.len(), 3);
        assert_eq!(block.header.parent_hashes, parents);
    }

    #[test]
    fn block_hash_deterministic_for_same_content() {
        let mut rng = OsRng;
        let signing_key = SigningKey::generate(&mut rng);
        let block = Block::new(&signing_key, vec![[1u8; 32]], vec![b"tx".to_vec()]);

        let hash1 = block.hash();
        let hash2 = block.hash();

        assert_eq!(hash1, hash2, "Block hash must be deterministic");
    }

    #[test]
    fn block_with_tampered_creator_rejected() {
        let mut rng = OsRng;
        let signing_key = SigningKey::generate(&mut rng);
        let mut block = Block::new(&signing_key, vec![], vec![]);

        // Tamper with creator public key
        block.header.creator[0] ^= 0xFF;

        assert!(
            !block.verify(),
            "Block with tampered creator should be rejected"
        );
    }

    #[test]
    fn merkle_root_empty_transactions() {
        let root = compute_merkle_root(&[]);
        assert_eq!(
            root, [0u8; 32],
            "Empty transaction list should have zero merkle root"
        );
    }

    #[test]
    fn merkle_root_single_transaction() {
        let tx = b"single-tx".to_vec();
        let root = compute_merkle_root(std::slice::from_ref(&tx));

        // Should equal hash of the single transaction
        let expected: [u8; 32] = Sha256::digest(&tx).into();
        assert_eq!(root, expected);
    }

    #[test]
    fn merkle_root_deterministic() {
        let txs = vec![b"tx1".to_vec(), b"tx2".to_vec(), b"tx3".to_vec()];

        let root1 = compute_merkle_root(&txs);
        let root2 = compute_merkle_root(&txs);

        assert_eq!(
            root1, root2,
            "Merkle root computation must be deterministic"
        );
    }

    #[test]
    fn merkle_root_changes_with_order() {
        let txs1 = vec![b"tx1".to_vec(), b"tx2".to_vec()];
        let txs2 = vec![b"tx2".to_vec(), b"tx1".to_vec()];

        let root1 = compute_merkle_root(&txs1);
        let root2 = compute_merkle_root(&txs2);

        assert_ne!(
            root1, root2,
            "Merkle root must change with transaction order"
        );
    }

    #[test]
    fn block_version_field_included_in_signature() {
        let mut rng = OsRng;
        let signing_key = SigningKey::generate(&mut rng);
        let mut block = Block::new(&signing_key, vec![], vec![]);

        // Change version after signing
        block.header.version = 2;

        assert!(
            !block.verify(),
            "Block with modified version should be rejected"
        );
    }

    #[test]
    fn block_median_time_included_in_signature() {
        let mut rng = OsRng;
        let signing_key = SigningKey::generate(&mut rng);
        let mut block = Block::new(&signing_key, vec![], vec![]);

        // Change median_time after signing
        block.header.median_time_us += 1000000;

        assert!(
            !block.verify(),
            "Block with modified time should be rejected"
        );
    }
}
