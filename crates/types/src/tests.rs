use super::*;
use crate::{Block, HashTimer, Transaction, IppanTimeMicros};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hashtimer_creation() {
        let hashtimer = HashTimer::now_tx("test", b"payload", [1u8; 32], b"node");
        assert_eq!(hashtimer.time_prefix.len(), 14);
        assert_eq!(hashtimer.hash_suffix.len(), 50);
        assert_eq!(hashtimer.to_hex().len(), 64);
    }

    #[test]
    fn test_hashtimer_deterministic() {
        let time = IppanTimeMicros(1000000);
        let domain = "test";
        let payload = b"payload";
        let nonce = [1u8; 32];
        let node_id = b"node";

        let h1 = HashTimer::derive(domain, time, domain.as_bytes(), payload, &nonce, node_id);
        let h2 = HashTimer::derive(domain, time, domain.as_bytes(), payload, &nonce, node_id);
        
        assert_eq!(h1.to_hex(), h2.to_hex());
    }

    #[test]
    fn test_hashtimer_ordering() {
        let early = HashTimer::now_tx("test", b"payload1", [1u8; 32], b"node");
        std::thread::sleep(std::time::Duration::from_micros(10));
        let later = HashTimer::now_tx("test", b"payload2", [2u8; 32], b"node");
        
        assert!(early.time_prefix < later.time_prefix);
    }

    #[test]
    fn test_transaction_creation() {
        let tx = Transaction::new([1u8; 32], [2u8; 32], 1000, 1);
        assert_eq!(tx.from, [1u8; 32]);
        assert_eq!(tx.to, [2u8; 32]);
        assert_eq!(tx.amount, 1000);
        assert_eq!(tx.nonce, 1);
        assert!(!tx.hash().iter().all(|&b| b == 0));
    }

    #[test]
    fn test_transaction_hash_deterministic() {
        let tx1 = Transaction::new([1u8; 32], [2u8; 32], 1000, 1);
        let tx2 = Transaction::new([1u8; 32], [2u8; 32], 1000, 1);
        
        // Hashes should be different due to different hashtimers (time-based)
        assert_ne!(tx1.hash(), tx2.hash());
    }

    #[test]
    fn test_transaction_validation() {
        let tx = Transaction::new([1u8; 32], [2u8; 32], 1000, 1);
        assert!(tx.is_valid());
        
        // Test invalid transaction (zero amount)
        let invalid_tx = Transaction::new([1u8; 32], [2u8; 32], 0, 1);
        assert!(!invalid_tx.is_valid());
    }

    #[test]
    fn test_block_creation() {
        let tx1 = Transaction::new([1u8; 32], [2u8; 32], 1000, 1);
        let tx2 = Transaction::new([3u8; 32], [4u8; 32], 2000, 1);
        let transactions = vec![tx1, tx2];
        
        let block = Block::new(vec![[0u8; 32]], transactions.clone(), 1, [5u8; 32]);
        assert_eq!(block.header.round, 1);
        assert_eq!(block.header.proposer, [5u8; 32]);
        assert_eq!(block.transactions.len(), 2);
        assert!(!block.hash().iter().all(|&b| b == 0));
    }

    #[test]
    fn test_block_validation() {
        let tx = Transaction::new([1u8; 32], [2u8; 32], 1000, 1);
        let block = Block::new(vec![[0u8; 32]], vec![tx], 1, [5u8; 32]);
        assert!(block.is_valid());
        
        // Test invalid block (empty transactions but non-genesis)
        let invalid_block = Block::new(vec![[0u8; 32]], vec![], 1, [5u8; 32]);
        assert!(!invalid_block.is_valid());
    }

    #[test]
    fn test_ippan_time_monotonic() {
        let t1 = IppanTimeMicros::now();
        std::thread::sleep(std::time::Duration::from_micros(10));
        let t2 = IppanTimeMicros::now();
        
        assert!(t2.0 > t1.0);
    }

    #[test]
    fn test_transaction_topics() {
        let mut tx = Transaction::new([1u8; 32], [2u8; 32], 1000, 1);
        tx.topics = vec!["transfer".to_string(), "payment".to_string()];
        
        assert_eq!(tx.topics.len(), 2);
        assert!(tx.topics.contains(&"transfer".to_string()));
    }

    #[test]
    fn test_confidential_transaction() {
        let mut tx = Transaction::new([1u8; 32], [2u8; 32], 1000, 1);
        
        let envelope = crate::ConfidentialEnvelope {
            enc_algo: "AES-256-GCM".to_string(),
            iv: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
            ciphertext: vec![0xde, 0xad, 0xbe, 0xef],
            access_keys: vec![crate::AccessKey {
                recipient_pub: vec![0x01; 32],
                enc_key: vec![0x02; 32],
            }],
        };
        
        tx.confidential = Some(envelope);
        assert!(tx.confidential.is_some());
        assert!(tx.is_valid());
    }

    #[test]
    fn test_zk_proof_transaction() {
        let mut tx = Transaction::new([1u8; 32], [2u8; 32], 1000, 1);
        
        let mut public_inputs = std::collections::HashMap::new();
        public_inputs.insert("balance".to_string(), "1000".to_string());
        
        let zk_proof = crate::ZkProof {
            proof: vec![0x01, 0x02, 0x03, 0x04],
            public_inputs,
        };
        
        tx.zk_proof = Some(zk_proof);
        assert!(tx.zk_proof.is_some());
        assert!(tx.is_valid());
    }
}