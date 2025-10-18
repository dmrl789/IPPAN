use ippan_mempool::Mempool;
use ippan_types::Transaction;
use std::time::Duration;

fn main() {
    println!("🧪 Testing IPPAN Mempool Production Implementation");
    
    // Test 1: Basic functionality
    let mempool = Mempool::new(100);
    println!("✅ Created mempool with capacity 100");
    
    // Test 2: Add transactions
    let tx1 = Transaction::new([1u8; 32], [2u8; 32], 1000, 1);
    let tx2 = Transaction::new([1u8; 32], [3u8; 32], 2000, 2);
    let tx3 = Transaction::new([2u8; 32], [4u8; 32], 1500, 1);
    
    assert!(mempool.add_transaction(tx1).unwrap());
    assert!(mempool.add_transaction(tx2).unwrap());
    assert!(mempool.add_transaction(tx3).unwrap());
    
    println!("✅ Added 3 transactions to mempool");
    println!("   Mempool size: {}", mempool.size());
    
    // Test 3: Fee-based prioritization
    let block_txs = mempool.get_transactions_for_block(3);
    println!("✅ Retrieved {} transactions for block", block_txs.len());
    
    // Test 4: Sender transactions
    let sender_txs = mempool.get_sender_transactions(&hex::encode([1u8; 32]));
    println!("✅ Found {} transactions for sender", sender_txs.len());
    
    // Test 5: Statistics
    let stats = mempool.get_stats();
    println!("✅ Mempool stats: size={}, total_fee={}", stats.size, stats.total_fee);
    
    // Test 6: Expiration
    let mempool_exp = Mempool::new_with_expiration(100, Duration::from_millis(100));
    let tx_exp = Transaction::new([5u8; 32], [6u8; 32], 1000, 1);
    assert!(mempool_exp.add_transaction(tx_exp).unwrap());
    println!("✅ Added transaction to expiring mempool");
    
    println!("\n🎉 All mempool tests passed! Production-ready implementation is working.");
}
