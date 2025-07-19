use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

// Import IPPAN modules
use ippan::{
    consensus::{
        ConsensusEngine, ConsensusConfig,
        blockdag::{Block, Transaction, BlockDAG},
        hashtimer::HashTimer,
        ippan_time::IppanTimeManager,
        round::RoundManager,
    },
    wallet::{
        ed25519::Ed25519Wallet,
        payments::PaymentWallet,
    },
    network::p2p::{P2pManager, P2pConfig, P2pMessage, TransactionAnnouncement, TransactionType},
    utils::crypto,
    NodeId, TransactionHash,
};

/// Demo node structure
#[derive(Clone)]
struct DemoNode {
    node_id: NodeId,
    consensus: Arc<RwLock<ConsensusEngine>>,
    wallet: Arc<RwLock<PaymentWallet>>,
    ed25519: Arc<RwLock<Ed25519Wallet>>,
    network: Arc<RwLock<P2pManager>>,
    name: String,
    is_validator: bool,
    stake: u64,
}

impl DemoNode {
    /// Create a new demo node
    async fn new(name: String, is_validator: bool, stake: u64) -> Self {
        let node_id = crypto::generate_node_id();
        
        // Create consensus engine
        let consensus_config = ConsensusConfig {
            max_validators: 3,
            min_stake: 10,
            block_time: 10,
            max_time_drift: 30,
            min_nodes_for_time: 2,
        };
        let consensus = Arc::new(RwLock::new(ConsensusEngine::new(consensus_config)));
        
        // Create wallet and Ed25519 key management
        let ed25519 = Arc::new(RwLock::new(Ed25519Wallet::new().await.unwrap()));
        let wallet = Arc::new(RwLock::new(PaymentWallet::new(&ed25519.read().await)));
        
        // Create network manager
        let network_config = P2pConfig::default();
        let network = Arc::new(RwLock::new(P2pManager::new(node_id, network_config)));
        
        // Give the node some initial funds for testing
        wallet.write().await.add_funds(1000).await.unwrap();
        
        Self {
            node_id,
            consensus,
            wallet,
            ed25519,
            network,
            name,
            is_validator,
            stake,
        }
    }
    
    /// Initialize the node as a validator
    async fn initialize_validator(&mut self) {
        if self.is_validator {
            println!("🔧 {} initializing as validator with stake: {}", self.name, self.stake);
            
            // Add this node as a validator in the consensus engine
            self.consensus.write().await
                .add_validator(self.node_id, self.stake)
                .unwrap();
            
            // Add current time sample to IPPAN Time
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64;
            
            self.consensus.write().await
                .add_node_time(self.node_id, current_time);
            
            println!("✅ {} validator initialized successfully", self.name);
        }
    }
    
    /// Create and sign a transaction
    async fn create_transaction(&self, to: NodeId, amount: u64, fee: u64) -> Transaction {
        let from = self.ed25519.read().await.get_public_key();
        
        // Get current IPPAN Time
        let ippan_time = self.consensus.read().await.get_ippan_time();
        
        // Create HashTimer for the transaction
        let hashtimer = HashTimer::with_ippan_time(
            [0u8; 32], // Will be calculated from content
            from,
            ippan_time,
        );
        
        // Create transaction with nonce
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
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
        let tx_data = self.create_transaction_data(&transaction);
        let signature = self.ed25519.read().await.sign(&tx_data).await.unwrap();
        transaction.signature = Some(signature);
        
        println!("💰 {} created transaction: {} IPN to {} (fee: {})", 
            self.name, amount, hex::encode(&to[..8]), fee);
        
        transaction
    }
    
    /// Helper to create transaction data for signing
    fn create_transaction_data(&self, tx: &Transaction) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&tx.from);
        data.extend_from_slice(&tx.to);
        data.extend_from_slice(&tx.amount.to_le_bytes());
        data.extend_from_slice(&tx.fee.to_le_bytes());
        data.extend_from_slice(&tx.nonce.to_le_bytes());
        data.extend_from_slice(&tx.hashtimer.ippan_time_ns.to_le_bytes());
        data
    }
    
    /// Broadcast transaction to network
    async fn broadcast_transaction(&self, transaction: &Transaction) {
        let announcement = TransactionAnnouncement {
            tx_hash: transaction.hash,
            tx_type: TransactionType::Payment,
            sender: self.node_id,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        
        let message = P2pMessage::TransactionAnnouncement(announcement);
        
        // Broadcast to all connected peers
        self.network.write().await.broadcast_message(message).await.unwrap();
        
        println!("📡 {} broadcasted transaction: {}", 
            self.name, hex::encode(&transaction.hash[..8]));
    }
    
    /// Process transactions into a block (for validators)
    async fn create_block(&self, transactions: Vec<Transaction>) -> Option<Block> {
        if !self.is_validator {
            return None;
        }
        
        println!("🔨 {} creating block with {} transactions", 
            self.name, transactions.len());
        
        // Create block using consensus engine
        let block = self.consensus.write().await
            .create_block(transactions.clone(), self.node_id)
            .await
            .unwrap();
        
        println!("✅ {} created block {} in round {} with {} transactions", 
            self.name, 
            hex::encode(&block.header.hash[..8]),
            block.header.round,
            transactions.len());
        
        Some(block)
    }
    
    /// Add a block to the consensus engine
    async fn add_block(&self, block: Block) -> bool {
        println!("📦 {} validating and adding block {}", 
            self.name, hex::encode(&block.header.hash[..8]));
        
        match self.consensus.write().await.add_block(block.clone()).await {
            Ok(_) => {
                println!("✅ {} accepted block {} in round {}", 
                    self.name, 
                    hex::encode(&block.header.hash[..8]),
                    block.header.round);
                true
            }
            Err(e) => {
                println!("❌ {} rejected block {}: {}", 
                    self.name, 
                    hex::encode(&block.header.hash[..8]), 
                    e);
                false
            }
        }
    }
    
    /// Get node status
    async fn get_status(&self) -> String {
        let balance = self.wallet.read().await.get_available_balance().await.unwrap();
        let round = self.consensus.read().await.current_round();
        let validator_count = self.consensus.read().await.get_validators().len();
        
        format!("{}[{}] Balance: {} IPN, Round: {}, Validators: {}", 
            self.name, 
            hex::encode(&self.node_id[..4]), 
            balance, 
            round, 
            validator_count)
    }
    
    /// Get wallet balance
    async fn get_balance(&self) -> u64 {
        self.wallet.read().await.get_available_balance().await.unwrap()
    }
}

/// Demo transaction pool for collecting transactions
struct TransactionPool {
    transactions: Arc<RwLock<Vec<Transaction>>>,
}

impl TransactionPool {
    fn new() -> Self {
        Self {
            transactions: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    async fn add_transaction(&self, tx: Transaction) {
        self.transactions.write().await.push(tx);
    }
    
    async fn get_transactions(&self) -> Vec<Transaction> {
        let mut txs = self.transactions.write().await;
        let result = txs.clone();
        txs.clear();
        result
    }
}

/// Main demo function
#[tokio::main]
async fn main() {
    println!("🚀 IPPAN Two-Node Transaction Demo");
    println!("=====================================");
    
    // Create transaction pool
    let tx_pool = Arc::new(TransactionPool::new());
    
    // Create two user nodes
    let mut alice = DemoNode::new("Alice".to_string(), false, 0).await;
    let mut bob = DemoNode::new("Bob".to_string(), false, 0).await;
    
    // Create validator nodes
    let mut validator1 = DemoNode::new("Validator-1".to_string(), true, 50).await;
    let mut validator2 = DemoNode::new("Validator-2".to_string(), true, 75).await;
    let mut validator3 = DemoNode::new("Validator-3".to_string(), true, 100).await;
    
    println!("\n📋 Initial Setup:");
    println!("▶ Alice: {}", alice.get_status().await);
    println!("▶ Bob: {}", bob.get_status().await);
    println!("▶ Validator-1: {}", validator1.get_status().await);
    println!("▶ Validator-2: {}", validator2.get_status().await);
    println!("▶ Validator-3: {}", validator3.get_status().await);
    
    // Initialize validators
    validator1.initialize_validator().await;
    validator2.initialize_validator().await;
    validator3.initialize_validator().await;
    
    // Share validator information between nodes (in a real network this would be automatic)
    for validator in [&validator1, &validator2, &validator3] {
        alice.consensus.write().await
            .add_validator(validator.node_id, validator.stake)
            .unwrap();
        bob.consensus.write().await
            .add_validator(validator.node_id, validator.stake)
            .unwrap();
        
        // Add time samples for IPPAN Time calculation
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        
        alice.consensus.write().await
            .add_node_time(validator.node_id, current_time);
        bob.consensus.write().await
            .add_node_time(validator.node_id, current_time);
    }
    
    println!("\n🏁 Starting Transaction Demo...");
    sleep(Duration::from_secs(1)).await;
    
    // Round 1: Alice sends money to Bob
    println!("\n🔄 ROUND 1: Alice → Bob Transaction");
    println!("=====================================");
    
    let alice_to_bob_tx = alice.create_transaction(bob.node_id, 100, 5).await;
    alice.broadcast_transaction(&alice_to_bob_tx).await;
    tx_pool.add_transaction(alice_to_bob_tx.clone()).await;
    
    // Validator 1 processes the transaction
    let pending_txs = tx_pool.get_transactions().await;
    if let Some(block1) = validator1.create_block(pending_txs).await {
        // All nodes validate and accept the block
        alice.add_block(block1.clone()).await;
        bob.add_block(block1.clone()).await;
        validator1.add_block(block1.clone()).await;
        validator2.add_block(block1.clone()).await;
        validator3.add_block(block1.clone()).await;
        
        // Update wallet balances (simplified - in real system this would be automatic)
        alice.wallet.write().await.add_funds(-105).await.unwrap(); // -100 amount -5 fee
        bob.wallet.write().await.add_funds(100).await.unwrap(); // +100 received
    }
    
    sleep(Duration::from_secs(2)).await;
    
    // Round 2: Bob sends money back to Alice
    println!("\n🔄 ROUND 2: Bob → Alice Transaction");
    println!("=====================================");
    
    let bob_to_alice_tx = bob.create_transaction(alice.node_id, 50, 3).await;
    bob.broadcast_transaction(&bob_to_alice_tx).await;
    tx_pool.add_transaction(bob_to_alice_tx.clone()).await;
    
    // Validator 2 processes the transaction
    let pending_txs = tx_pool.get_transactions().await;
    if let Some(block2) = validator2.create_block(pending_txs).await {
        // All nodes validate and accept the block
        alice.add_block(block2.clone()).await;
        bob.add_block(block2.clone()).await;
        validator1.add_block(block2.clone()).await;
        validator2.add_block(block2.clone()).await;
        validator3.add_block(block2.clone()).await;
        
        // Update wallet balances
        bob.wallet.write().await.add_funds(-53).await.unwrap(); // -50 amount -3 fee
        alice.wallet.write().await.add_funds(50).await.unwrap(); // +50 received
    }
    
    sleep(Duration::from_secs(2)).await;
    
    // Round 3: Multiple transactions in same block
    println!("\n🔄 ROUND 3: Multiple Transactions");
    println!("=====================================");
    
    let alice_to_bob_tx2 = alice.create_transaction(bob.node_id, 25, 2).await;
    let bob_to_alice_tx2 = bob.create_transaction(alice.node_id, 15, 1).await;
    
    alice.broadcast_transaction(&alice_to_bob_tx2).await;
    bob.broadcast_transaction(&bob_to_alice_tx2).await;
    
    tx_pool.add_transaction(alice_to_bob_tx2.clone()).await;
    tx_pool.add_transaction(bob_to_alice_tx2.clone()).await;
    
    // Validator 3 processes both transactions
    let pending_txs = tx_pool.get_transactions().await;
    if let Some(block3) = validator3.create_block(pending_txs).await {
        // All nodes validate and accept the block
        alice.add_block(block3.clone()).await;
        bob.add_block(block3.clone()).await;
        validator1.add_block(block3.clone()).await;
        validator2.add_block(block3.clone()).await;
        validator3.add_block(block3.clone()).await;
        
        // Update wallet balances
        alice.wallet.write().await.add_funds(-27).await.unwrap(); // -25 amount -2 fee
        alice.wallet.write().await.add_funds(15).await.unwrap(); // +15 received
        bob.wallet.write().await.add_funds(25).await.unwrap(); // +25 received
        bob.wallet.write().await.add_funds(-16).await.unwrap(); // -15 amount -1 fee
    }
    
    sleep(Duration::from_secs(2)).await;
    
    // Display final results
    println!("\n📊 FINAL RESULTS");
    println!("=================");
    println!("▶ Alice: {}", alice.get_status().await);
    println!("▶ Bob: {}", bob.get_status().await);
    println!("▶ Validator-1: {}", validator1.get_status().await);
    println!("▶ Validator-2: {}", validator2.get_status().await);
    println!("▶ Validator-3: {}", validator3.get_status().await);
    
    // Show consensus statistics
    println!("\n⚡ Consensus Statistics:");
    let alice_round = alice.consensus.read().await.current_round();
    let alice_ippan_time = alice.consensus.read().await.get_ippan_time();
    let time_stats = alice.consensus.read().await.get_time_stats();
    
    println!("Current Round: {}", alice_round);
    println!("IPPAN Time: {} ns", alice_ippan_time);
    println!("Time Samples: {}", time_stats.sample_count);
    println!("Time Deviation: {} ns", time_stats.deviation_ns);
    
    // Show BlockDAG structure
    println!("\n🔗 BlockDAG Structure:");
    let blockdag = alice.consensus.read().await.blockdag();
    let tips = blockdag.get_tips().await;
    println!("DAG Tips: {}", tips.len());
    for tip in tips {
        println!("  └─ Tip: {}", hex::encode(&tip[..8]));
    }
    
    println!("\n✅ Demo completed successfully!");
    println!("All transactions were signed, validated, and included in blocks through consensus rounds.");
} 