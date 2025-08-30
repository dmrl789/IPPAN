use ippan::crypto::{generate_hash_timer, hash};

fn main() {
    println!("=== IPPAN HashTimer Demo ===\n");
    
    // Example inputs
    let ippan_time_us = 1234567890123456u64;  // IPPAN time in microseconds
    let entropy = b"random_entropy_123";       // Random entropy bytes
    let tx_id = hash(b"example_transaction_id"); // Transaction ID (32 bytes)
    
    println!("Inputs:");
    println!("  IPPAN Time (us): {}", ippan_time_us);
    println!("  Entropy: {}", String::from_utf8_lossy(entropy));
    println!("  Transaction ID: {}", hex::encode(tx_id));
    println!();
    
    // Generate HashTimer
    let hash_timer = generate_hash_timer(ippan_time_us, entropy, &tx_id);
    
    println!("HashTimer (32 bytes):");
    println!("  Hex: {}", hex::encode(hash_timer));
    println!("  Length: {} bytes", hash_timer.len());
    println!();
    
    // Demonstrate determinism - same inputs produce same HashTimer
    let hash_timer2 = generate_hash_timer(ippan_time_us, entropy, &tx_id);
    println!("Verifying determinism:");
    println!("  HashTimer1 == HashTimer2: {}", hash_timer == hash_timer2);
    println!();
    
    // Show how different inputs produce different HashTimers
    let hash_timer3 = generate_hash_timer(ippan_time_us + 1, entropy, &tx_id);
    println!("Different IPPAN time produces different HashTimer:");
    println!("  HashTimer1 == HashTimer3: {}", hash_timer == hash_timer3);
    println!();
    
    let hash_timer4 = generate_hash_timer(ippan_time_us, b"different_entropy", &tx_id);
    println!("Different entropy produces different HashTimer:");
    println!("  HashTimer1 == HashTimer4: {}", hash_timer == hash_timer4);
    println!();
    
    let different_tx_id = hash(b"different_transaction_id");
    let hash_timer5 = generate_hash_timer(ippan_time_us, entropy, &different_tx_id);
    println!("Different transaction ID produces different HashTimer:");
    println!("  HashTimer1 == HashTimer5: {}", hash_timer == hash_timer5);
    println!();
    
    println!("=== HashTimer Formula ===");
    println!("HashTimer = Blake3(IPPAN_time_us || entropy || tx_id)");
    println!("  - IPPAN_time_us: 8 bytes (little-endian)");
    println!("  - entropy: variable length bytes");
    println!("  - tx_id: 32 bytes");
    println!("  - Output: 32 bytes");
}
