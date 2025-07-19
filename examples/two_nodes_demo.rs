use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

// Import IPPAN modules
use ippan::{
    consensus::{
        ConsensusEngine, ConsensusConfig,
        blockdag::{Block, Transaction},
        hashtimer::HashTimer,
        ippan_time::IppanTimeManager,
    },
    wallet::{
        ed25519::Ed25519Wallet,
        payments::PaymentWallet,
    },
    utils::crypto,
    NodeId,
};

/// Demo function showing two nodes sending signed transactions
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 IPPAN Two-Node Transaction Demo");
    println!("===================================");
    
    // Step 1: Create two user nodes with wallets
    println!("\n📋 Step 1: Creating Nodes and Wallets");
    
    // Create Alice's node
    let alice_node_id = crypto::generate_node_id();
    let alice_ed25519 = Ed25519Wallet::new().await?;
    let alice_wallet = PaymentWallet::new(&alice_ed25519);
    let alice_pubkey = alice_ed25519.get_public_key();
    
    // Create Bob's node
    let bob_node_id = crypto::generate_node_id();
    let bob_ed25519 = Ed25519Wallet::new().await?;
    let bob_wallet = PaymentWallet::new(&bob_ed25519);
    let bob_pubkey = bob_ed25519.get_public_key();
    
    println!("✅ Alice Node: {}", hex::encode(&alice_node_id[..8]));
    println!("✅ Bob Node: {}", hex::encode(&bob_node_id[..8]));
    
    // Give Alice some initial funds
    alice_wallet.add_funds(1000).await?;
    bob_wallet.add_funds(500).await?;
    
    println!("💰 Alice initial balance: {} IPN", alice_wallet.get_available_balance().await?);
    println!("💰 Bob initial balance: {} IPN", bob_wallet.get_available_balance().await?);
    
    // Step 2: Set up consensus engine and validators
    println!("\n📋 Step 2: Setting up Consensus Engine");
    
    // Create consensus engine
    let consensus_config = ConsensusConfig {
        max_validators: 3,
        min_stake: 10,
        block_time: 10,
        max_time_drift: 30,
        min_nodes_for_time: 2,
    };
    
    let mut consensus = ConsensusEngine::new(consensus_config);
    
    // Create validator nodes
    let validator1_id = crypto::generate_node_id();
    let validator2_id = crypto::generate_node_id();
    let validator3_id = crypto::generate_node_id();
    
    // Add validators to consensus
    consensus.add_validator(validator1_id, 100)?;
    consensus.add_validator(validator2_id, 150)?;
    consensus.add_validator(validator3_id, 200)?;
    
    // Add time samples for IPPAN Time calculation
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_nanos() as u64;
    
    consensus.add_node_time(validator1_id, current_time);
    consensus.add_node_time(validator2_id, current_time + 1_000_000); // +1ms
    consensus.add_node_time(validator3_id, current_time - 500_000); // -0.5ms
    
    println!("✅ Consensus engine initialized");
    println!("✅ Validators: {}", consensus.get_validators().len());
    println!("✅ Current round: {}", consensus.current_round());
    println!("✅ IPPAN Time: {} ns", consensus.get_ippan_time());
    
    // Step 3: Create and sign transactions
    println!("\n📋 Step 3: Creating Signed Transactions");
    
    // Transaction 1: Alice → Bob (100 IPN)
    let tx1 = create_signed_transaction(
        &alice_ed25519,
        alice_pubkey,
        bob_pubkey,
        100,
        5,
        consensus.get_ippan_time(),
    ).await?;
    
    println!("💰 Transaction 1 created: Alice → Bob, 100 IPN (fee: 5)");
    println!("   Hash: {}", hex::encode(&tx1.hash[..8]));
    println!("   From: {}", hex::encode(&tx1.from[..8]));
    println!("   To: {}", hex::encode(&tx1.to[..8]));
    println!("   Amount: {}", tx1.amount);
    println!("   Fee: {}", tx1.fee);
    println!("   Signed: {}", tx1.signature.is_some());
    
    // Transaction 2: Bob → Alice (25 IPN)
    let tx2 = create_signed_transaction(
        &bob_ed25519,
        bob_pubkey,
        alice_pubkey,
        25,
        2,
        consensus.get_ippan_time(),
    ).await?;
    
    println!("💰 Transaction 2 created: Bob → Alice, 25 IPN (fee: 2)");
    println!("   Hash: {}", hex::encode(&tx2.hash[..8]));
    println!("   From: {}", hex::encode(&tx2.from[..8]));
    println!("   To: {}", hex::encode(&tx2.to[..8]));
    println!("   Amount: {}", tx2.amount);
    println!("   Fee: {}", tx2.fee);
    println!("   Signed: {}", tx2.signature.is_some());
    
    // Step 4: Validate transactions
    println!("\n📋 Step 4: Validating Transactions");
    
    let tx1_valid = consensus.validate_transaction(&tx1)?;
    let tx2_valid = consensus.validate_transaction(&tx2)?;
    
    println!("✅ Transaction 1 validation: {}", if tx1_valid { "VALID" } else { "INVALID" });
    println!("✅ Transaction 2 validation: {}", if tx2_valid { "VALID" } else { "INVALID" });
    
    // Step 5: Create blocks with transactions
    println!("\n📋 Step 5: Creating Blocks");
    
    // Round 1: Validator 1 creates block with transaction 1
    let block1 = consensus.create_block(vec![tx1.clone()], validator1_id).await?;
    println!("🔨 Block 1 created by Validator 1");
    println!("   Hash: {}", hex::encode(&block1.header.hash[..8]));
    println!("   Round: {}", block1.header.round);
    println!("   Validator: {}", hex::encode(&block1.header.validator_id[..8]));
    println!("   Transactions: {}", block1.transactions.len());
    println!("   IPPAN Time: {}", block1.header.hashtimer.ippan_time_ns);
    
    // Validate and add block 1
    let block1_valid = consensus.validate_block(&block1)?;
    println!("✅ Block 1 validation: {}", if block1_valid { "VALID" } else { "INVALID" });
    
    if block1_valid {
        consensus.add_block(block1.clone()).await?;
        println!("✅ Block 1 added to consensus");
    }
    
    // Small delay to simulate network propagation
    sleep(Duration::from_millis(100)).await;
    
    // Round 2: Validator 2 creates block with transaction 2
    let block2 = consensus.create_block(vec![tx2.clone()], validator2_id).await?;
    println!("🔨 Block 2 created by Validator 2");
    println!("   Hash: {}", hex::encode(&block2.header.hash[..8]));
    println!("   Round: {}", block2.header.round);
    println!("   Validator: {}", hex::encode(&block2.header.validator_id[..8]));
    println!("   Transactions: {}", block2.transactions.len());
    println!("   IPPAN Time: {}", block2.header.hashtimer.ippan_time_ns);
    
    // Validate and add block 2
    let block2_valid = consensus.validate_block(&block2)?;
    println!("✅ Block 2 validation: {}", if block2_valid { "VALID" } else { "INVALID" });
    
    if block2_valid {
        consensus.add_block(block2.clone()).await?;
        println!("✅ Block 2 added to consensus");
    }
    
    // Step 6: Create a block with multiple transactions
    println!("\n📋 Step 6: Creating Multi-Transaction Block");
    
    // Create additional transactions
    let tx3 = create_signed_transaction(
        &alice_ed25519,
        alice_pubkey,
        bob_pubkey,
        50,
        3,
        consensus.get_ippan_time(),
    ).await?;
    
    let tx4 = create_signed_transaction(
        &bob_ed25519,
        bob_pubkey,
        alice_pubkey,
        10,
        1,
        consensus.get_ippan_time(),
    ).await?;
    
    // Round 3: Validator 3 creates block with multiple transactions
    let block3 = consensus.create_block(vec![tx3.clone(), tx4.clone()], validator3_id).await?;
    println!("🔨 Block 3 created by Validator 3");
    println!("   Hash: {}", hex::encode(&block3.header.hash[..8]));
    println!("   Round: {}", block3.header.round);
    println!("   Validator: {}", hex::encode(&block3.header.validator_id[..8]));
    println!("   Transactions: {}", block3.transactions.len());
    println!("   IPPAN Time: {}", block3.header.hashtimer.ippan_time_ns);
    
    // Validate and add block 3
    let block3_valid = consensus.validate_block(&block3)?;
    println!("✅ Block 3 validation: {}", if block3_valid { "VALID" } else { "INVALID" });
    
    if block3_valid {
        consensus.add_block(block3.clone()).await?;
        println!("✅ Block 3 added to consensus");
    }
    
    // Step 7: Show final state
    println!("\n📊 Final State");
    println!("===============");
    
    println!("🔗 Consensus State:");
    println!("   Current Round: {}", consensus.current_round());
    println!("   Validators: {}", consensus.get_validators().len());
    println!("   IPPAN Time: {} ns", consensus.get_ippan_time());
    
    let time_stats = consensus.get_time_stats();
    println!("   Time Samples: {}", time_stats.sample_count);
    println!("   Time Deviation: {} ns", time_stats.deviation_ns);
    
    // Show BlockDAG structure
    println!("\n🔗 BlockDAG Structure:");
    let blockdag = consensus.blockdag();
    let tips = blockdag.get_tips().await;
    println!("   DAG Tips: {}", tips.len());
    for tip in tips {
        println!("   └─ Tip: {}", hex::encode(&tip[..8]));
    }
    
    println!("\n📋 Transaction Summary:");
    println!("   └─ Block 1: 1 transaction (Alice → Bob, 100 IPN)");
    println!("   └─ Block 2: 1 transaction (Bob → Alice, 25 IPN)"); 
    println!("   └─ Block 3: 2 transactions (Alice → Bob, 50 IPN & Bob → Alice, 10 IPN)");
    println!("   └─ Total: 4 transactions across 3 blocks in 3 rounds");
    
    // Verify signature validation
    println!("\n🔐 Signature Verification:");
    for (i, tx) in [&tx1, &tx2, &tx3, &tx4].iter().enumerate() {
        if let Some(signature) = &tx.signature {
            let tx_data = create_transaction_data(tx);
            let is_valid = if tx.from == alice_pubkey {
                alice_ed25519.verify_with_key(&tx_data, signature, &alice_pubkey).await?
            } else {
                bob_ed25519.verify_with_key(&tx_data, signature, &bob_pubkey).await?
            };
            println!("   └─ Transaction {}: {}", i + 1, if is_valid { "VALID SIGNATURE" } else { "INVALID SIGNATURE" });
        }
    }
    
    println!("\n✅ Demo completed successfully!");
    println!("All transactions were cryptographically signed, validated, and accepted in blocks through consensus rounds.");
    println!("The BlockDAG structure maintains the ordered history of all transactions.");
    
    Ok(())
}

/// Create a signed transaction
async fn create_signed_transaction(
    ed25519_wallet: &Ed25519Wallet,
    from: NodeId,
    to: NodeId,
    amount: u64,
    fee: u64,
    ippan_time: u64,
) -> Result<Transaction, Box<dyn std::error::Error>> {
    // Create HashTimer for the transaction
    let hashtimer = HashTimer::with_ippan_time(
        [0u8; 32], // Will be calculated from content
        from,
        ippan_time,
    );
    
    // Create transaction with nonce
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_nanos() as u64;
    
    let mut transaction = Transaction::new(
        from,
        to,
        amount,
        fee,
        nonce,
        hashtimer,
    );
    
    // Sign the transaction
    let tx_data = create_transaction_data(&transaction);
    let signature = ed25519_wallet.sign(&tx_data).await?;
    transaction.signature = Some(signature);
    
    Ok(transaction)
}

/// Create transaction data for signing
fn create_transaction_data(tx: &Transaction) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&tx.from);
    data.extend_from_slice(&tx.to);
    data.extend_from_slice(&tx.amount.to_le_bytes());
    data.extend_from_slice(&tx.fee.to_le_bytes());
    data.extend_from_slice(&tx.nonce.to_le_bytes());
    data.extend_from_slice(&tx.hashtimer.ippan_time_ns.to_le_bytes());
    data
} 