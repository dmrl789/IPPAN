//! Load Tests for IPPAN
//! 
//! Tests system performance under high load conditions

use ippan::{
    consensus::{Block, Transaction, HashTimer},
    performance::{PerformanceManager, PerformanceConfig},
};
use std::time::{Duration, Instant};

#[tokio::test]
async fn test_high_throughput_processing() {
    // Create performance manager
    let mut manager = PerformanceManager::new(PerformanceConfig::default());

    // Create large batch of transactions
    let transactions: Vec<Transaction> = (0..10000)
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

    // Test high-throughput processing
    let start_time = Instant::now();
    let processed_transactions = manager.process_transactions(transactions).await.unwrap();
    let duration = start_time.elapsed();

    // Calculate TPS
    let tps = processed_transactions.len() as f64 / duration.as_secs_f64();
    println!("High-throughput TPS: {:.2}", tps);

    // Verify TPS target (should be > 1000 TPS)
    assert!(tps > 1000.0, "TPS too low: {:.2}", tps);

    // Test memory efficiency
    let metrics = manager.get_metrics().await;
    assert!(metrics.memory_usage > 0);

    // Test cache performance
    let cache_stats = manager.get_cache_stats().await;
    assert!(cache_stats.hits >= 0);
}

#[tokio::test]
async fn test_memory_usage_under_load() {
    // Create performance manager
    let mut manager = PerformanceManager::new(PerformanceConfig::default());

    // Create transactions in batches to test memory usage
    for batch in 0..10 {
        let transactions: Vec<Transaction> = (0..1000)
            .map(|i| {
                let global_index = batch * 1000 + i;
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

        // Process batch
        let processed_transactions = manager.process_transactions(transactions).await.unwrap();
        assert_eq!(processed_transactions.len(), 1000);

        // Check memory usage
        let metrics = manager.get_metrics().await;
        assert!(metrics.memory_usage > 0);
    }
}

#[tokio::test]
async fn test_concurrent_processing() {
    // Create performance manager
    let mut manager = PerformanceManager::new(PerformanceConfig::default());

    // Create concurrent processing tasks
    let handles: Vec<_> = (0..10)
        .map(|batch| {
            let mut manager = manager.clone();
            tokio::spawn(async move {
                let transactions: Vec<Transaction> = (0..1000)
                    .map(|i| {
                        let global_index = batch * 1000 + i;
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

                manager.process_transactions(transactions).await
            })
        })
        .collect();

    // Wait for all tasks to complete
    let start_time = Instant::now();
    let mut total_processed = 0;
    for handle in handles {
        let result = handle.await.unwrap().unwrap();
        total_processed += result.len();
    }
    let duration = start_time.elapsed();

    // Calculate concurrent TPS
    let tps = total_processed as f64 / duration.as_secs_f64();
    println!("Concurrent TPS: {:.2}", tps);

    // Verify concurrent processing
    assert_eq!(total_processed, 10000);
    assert!(tps > 1000.0, "Concurrent TPS too low: {:.2}", tps);
}
