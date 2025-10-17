//! IPPAN Block and header structures with HashTimer integration.
//!
//! Blocks in the IPPAN BlockDAG embed a signed [`HashTimer`], providing a
//! cryptographically verifiable anchor into the deterministic network time
//! stream. The helpers in this module construct new blocks, compute their
//! hashes, and validate incoming blocks before they are inserted into the DAG.

use std::convert::TryInto;

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
        let signature = match Signature::from_bytes(&signature_bytes) {
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
        let mut next = Vec::with_capacity((level.len() + 1) / 2);
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
    hasher.update(&median_time_us.to_be_bytes());
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
    hasher.update(&header.median_time_us.to_be_bytes());
    hasher.update(&header.merkle_root);
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
}
