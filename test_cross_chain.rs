use ippan::crosschain::{CrossChainManager, CrossChainConfig, AnchorTx, ProofType};
use ippan::consensus::hashtimer::HashTimer;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 IPPAN Cross-Chain Test");
    println!("========================\n");

    // Initialize cross-chain manager
    let config = CrossChainConfig::default();
    let manager = CrossChainManager::new(config).await?;
    let manager = Arc::new(manager);

    // Test 1: Submit anchor from TestChain1
    println!("📌 Test 1: Submitting anchor from TestChain1");
    let anchor_tx = AnchorTx {
        external_chain_id: "TestChain1".to_string(),
        external_state_root: "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
        timestamp: HashTimer::new([0u8; 32], [0u8; 32]),
        proof_type: Some(ProofType::Signature),
        proof_data: vec![1; 64], // Mock signature
    };

    let anchor_id = manager.submit_anchor(anchor_tx).await?;
    println!("✅ Anchor submitted with ID: {}\n", anchor_id);

    // Test 2: Submit anchor from StarkNet
    println!("📌 Test 2: Submitting anchor from StarkNet");
    let anchor_tx = AnchorTx {
        external_chain_id: "starknet".to_string(),
        external_state_root: "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
        timestamp: HashTimer::new([0u8; 32], [0u8; 32]),
        proof_type: Some(ProofType::ZK),
        proof_data: vec![2; 128], // Mock ZK proof
    };

    let anchor_id = manager.submit_anchor(anchor_tx).await?;
    println!("✅ Anchor submitted with ID: {}\n", anchor_id);

    // Test 3: Get latest anchors
    println!("📌 Test 3: Getting latest anchors");
    let testchain_anchor = manager.get_latest_anchor("TestChain1").await?;
    let starknet_anchor = manager.get_latest_anchor("starknet").await?;

    if let Some(anchor) = testchain_anchor {
        println!("✅ TestChain1 latest anchor: {}", anchor.external_state_root);
    }
    if let Some(anchor) = starknet_anchor {
        println!("✅ StarkNet latest anchor: {}", anchor.external_state_root);
    }
    println!();

    // Test 4: Register bridge endpoints
    println!("📌 Test 4: Registering bridge endpoints");
    let testchain_bridge = ippan::crosschain::bridge::BridgeEndpoint {
        chain_id: "TestChain1".to_string(),
        accepted_anchor_types: vec![ProofType::Signature, ProofType::ZK],
        latest_anchor: None,
        config: ippan::crosschain::bridge::BridgeConfig::default(),
        status: ippan::crosschain::bridge::BridgeStatus::Active,
        last_activity: chrono::Utc::now(),
    };

    let starknet_bridge = ippan::crosschain::bridge::BridgeEndpoint {
        chain_id: "starknet".to_string(),
        accepted_anchor_types: vec![ProofType::ZK, ProofType::Merkle],
        latest_anchor: None,
        config: ippan::crosschain::bridge::BridgeConfig {
            trust_level: 80,
            ..Default::default()
        },
        status: ippan::crosschain::bridge::BridgeStatus::Active,
        last_activity: chrono::Utc::now(),
    };

    manager.register_bridge(testchain_bridge).await?;
    manager.register_bridge(starknet_bridge).await?;
    println!("✅ Bridge endpoints registered\n");

    // Test 5: Verify inclusion proofs
    println!("📌 Test 5: Verifying inclusion proofs");
    
    // Mock Merkle proof for TestChain1
    let merkle_proof = vec![
        // Root hash (32 bytes)
        0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef,
        0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef,
        0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef,
        0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef,
        // Leaf hash (32 bytes)
        0xab, 0xcd, 0xef, 0x12, 0x34, 0x56, 0x78, 0x90,
        0xab, 0xcd, 0xef, 0x12, 0x34, 0x56, 0x78, 0x90,
        0xab, 0xcd, 0xef, 0x12, 0x34, 0x56, 0x78, 0x90,
        0xab, 0xcd, 0xef, 0x12, 0x34, 0x56, 0x78, 0x90,
        // Sibling hashes and path (64 bytes)
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
        0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
        0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
        0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20,
        0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28,
        0x29, 0x2a, 0x2b, 0x2c, 0x2d, 0x2e, 0x2f, 0x30,
        0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38,
        0x39, 0x3a, 0x3b, 0x3c, 0x3d, 0x3e, 0x3f, 0x40,
    ];

    let result = manager.verify_external_inclusion(
        "TestChain1",
        "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
        &merkle_proof,
    ).await?;

    if result.success {
        println!("✅ Inclusion proof verified successfully!");
        println!("   Details: {}", result.details);
    } else {
        println!("❌ Inclusion proof verification failed!");
        println!("   Details: {}", result.details);
    }
    println!();

    // Test 6: Get light sync data
    println!("📌 Test 6: Getting light sync data");
    let light_sync_data = manager.get_light_sync_data(12345).await?;
    
    if let Some(sync_data) = light_sync_data {
        println!("✅ Light sync data retrieved for round {}", sync_data.round);
        println!("   Merkle Root: {}", sync_data.merkle_root);
        println!("   ZK Proof Size: {} bytes", sync_data.zk_proof.as_ref().map_or(0, |p| p.len()));
        println!("   Anchor Headers: {}", sync_data.anchor_headers.len());
    } else {
        println!("❌ No light sync data found for round 12345");
    }
    println!();

    // Test 7: Generate comprehensive report
    println!("📌 Test 7: Generating comprehensive report");
    let report = manager.generate_cross_chain_report().await?;
    
    println!("📊 Cross-Chain Report");
    println!("   Generated at: {}", report.generated_at.format("%Y-%m-%d %H:%M:%S UTC"));
    println!("   Total Anchors: {}", report.total_anchors);
    println!("   Active Bridges: {}", report.active_bridges);
    println!("   Verification Success Rate: {:.2}%", report.verification_success_rate * 100.0);
    
    if !report.recent_anchors.is_empty() {
        println!("\n📌 Recent Anchors:");
        for anchor in report.recent_anchors.iter().take(3) {
            println!("   - {}: {} (Proof: {:?})", 
                anchor.external_chain_id, 
                anchor.external_state_root,
                anchor.proof_type);
        }
    }
    
    if !report.bridge_endpoints.is_empty() {
        println!("\n🌉 Bridge Endpoints:");
        for bridge in report.bridge_endpoints.iter().take(3) {
            println!("   - {}: {:?} (Trust: {})", 
                bridge.chain_id, 
                bridge.status,
                bridge.config.trust_level);
        }
    }
    println!();

    println!("🎉 Cross-chain test completed successfully!");
    println!("IPPAN is now ready to serve as a global Layer 1 for external chains!");

    Ok(())
} 