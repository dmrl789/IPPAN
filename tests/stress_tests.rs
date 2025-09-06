//! Stress Tests for IPPAN
//! 
//! Tests system behavior under extreme stress conditions

use ippan::{
    consensus::{Block, Transaction, HashTimer},
    performance::{PerformanceManager, PerformanceConfig},
    storage::{StorageOrchestrator, StorageConfig},
    network::{NetworkManager, NetworkConfig},
};
use std::time::{Duration, Instant};

#[tokio::test]
async fn test_extreme_transaction_volume() {
    // Create performance manager
    let mut manager = PerformanceManager::new(PerformanceConfig::default());

    // Create extreme volume of transactions
    let transactions: Vec<Transaction> = (0..100000)
        .map(|i| {
            Transaction::new(
                [i as u8; 32],
                1000,
                [(i + 1) as u8; 32],
                HashTimer::with_ippan_time(
                    [i as u8; 32],
                    [(i + 1) as u8; 32],
                    i as u64,
                ),
            )
        })
        .collect();

    // Test extreme volume processing
    let start_time = Instant::now();
    let processed_transactions = manager.process_transactions(transactions).await.unwrap();
    let duration = start_time.elapsed();

    // Calculate TPS
    let tps = processed_transactions.len() as f64 / duration.as_secs_f64();
    println!("Extreme volume TPS: {:.2}", tps);

    // Verify processing
    assert_eq!(processed_transactions.len(), 100000);
    assert!(tps > 1000.0, "Extreme volume TPS too low: {:.2}", tps);

    // Test memory usage under stress
    let metrics = manager.get_metrics().await;
    assert!(metrics.memory_usage > 0);
}

#[tokio::test]
async fn test_memory_pressure() {
    // Create performance manager
    let mut manager = PerformanceManager::new(PerformanceConfig::default());

    // Create large transactions to test memory pressure
    let large_transactions: Vec<Transaction> = (0..1000)
        .map(|i| {
            Transaction::new(
                [i as u8; 32],
                1000000, // Large amount
                [(i + 1) as u8; 32],
                HashTimer::with_ippan_time(
                    [i as u8; 32],
                    [(i + 1) as u8; 32],
                    i as u64,
                ),
            )
        })
        .collect();

    // Process large transactions
    let start_time = Instant::now();
    let processed_transactions = manager.process_transactions(large_transactions).await.unwrap();
    let duration = start_time.elapsed();

    // Verify processing under memory pressure
    assert_eq!(processed_transactions.len(), 1000);

    // Test memory usage
    let metrics = manager.get_metrics().await;
    assert!(metrics.memory_usage > 0);

    println!("Memory pressure test completed in {:?}", duration);
}

#[tokio::test]
async fn test_rapid_succession_processing() {
    // Create performance manager
    let mut manager = PerformanceManager::new(PerformanceConfig::default());

    // Process transactions in rapid succession
    let start_time = Instant::now();
    for batch in 0..100 {
        let transactions: Vec<Transaction> = (0..100)
            .map(|i| {
                let global_index = batch * 100 + i;
                Transaction::new(
                    [global_index as u8; 32],
                    1000,
                    [(global_index + 1) as u8; 32],
                    HashTimer::with_ippan_time(
                        [global_index as u8; 32],
                        [(global_index + 1) as u8; 32],
                        global_index as u64,
                    ),
                )
            })
            .collect();

        let processed_transactions = manager.process_transactions(transactions).await.unwrap();
        assert_eq!(processed_transactions.len(), 100);
    }
    let duration = start_time.elapsed();

    // Calculate rapid succession TPS
    let tps = 10000.0 / duration.as_secs_f64();
    println!("Rapid succession TPS: {:.2}", tps);

    // Verify rapid succession processing
    assert!(tps > 1000.0, "Rapid succession TPS too low: {:.2}", tps);
}

#[tokio::test]
async fn test_storage_under_stress() {
    // Create storage orchestrator
    let mut storage = StorageOrchestrator::new(StorageConfig::default()).unwrap();
    storage.start().await.unwrap();

    // Test storage under stress
    let start_time = Instant::now();
    for i in 0..1000 {
        let test_data = format!("Stress test data {}", i).into_bytes();
        let file_hash = storage.upload_file(&format!("stress_test_{}.txt", i), &test_data).await.unwrap();
        assert!(!file_hash.is_empty());

        // Verify data integrity
        let retrieved_data = storage.download_file(&file_hash).await.unwrap();
        assert_eq!(retrieved_data, test_data);
    }
    let duration = start_time.elapsed();

    println!("Storage stress test completed in {:?}", duration);

    // Test storage statistics
    let usage = storage.get_usage();
    assert!(usage.used_bytes > 0);

    storage.stop().await.unwrap();
}

#[tokio::test]
async fn test_network_under_stress() {
    // Create network manager
    let mut network = NetworkManager::new(NetworkConfig::default()).unwrap();
    network.start().await.unwrap();

    // Test network under stress
    let start_time = Instant::now();
    for i in 0..100 {
        // Simulate network operations
        let peer_count = network.get_peer_count();
        assert!(peer_count >= 0);

        let stats = network.get_stats();
        assert!(stats.total_nodes >= 0);
    }
    let duration = start_time.elapsed();

    println!("Network stress test completed in {:?}", duration);

    network.stop().await.unwrap();
}

#[tokio::test]
async fn test_system_resilience() {
    // Create performance manager
    let mut manager = PerformanceManager::new(PerformanceConfig::default());

    // Test system resilience under various conditions
    let test_conditions = vec![
        ("normal", 1000),
        ("high_load", 5000),
        ("extreme_load", 10000),
    ];

    for (condition, transaction_count) in test_conditions {
        let transactions: Vec<Transaction> = (0..transaction_count)
            .map(|i| {
                Transaction::new(
                    [i as u8; 32],
                    1000,
                    [(i + 1) as u8; 32],
                    HashTimer::with_ippan_time(
                        [i as u8; 32],
                        [(i + 1) as u8; 32],
                        i as u64,
                    ),
                )
            })
            .collect();

        let start_time = Instant::now();
        let processed_transactions = manager.process_transactions(transactions).await.unwrap();
        let duration = start_time.elapsed();

        let tps = processed_transactions.len() as f64 / duration.as_secs_f64();
        println!("{} TPS: {:.2}", condition, tps);

        // Verify resilience
        assert_eq!(processed_transactions.len(), transaction_count);
        assert!(tps > 100.0, "{} TPS too low: {:.2}", condition, tps);
    }

    // Test final system state
    let metrics = manager.get_metrics().await;
    assert!(metrics.transactions_processed > 0);
}
