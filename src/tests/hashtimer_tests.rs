//! Tests for IPPAN HashTimer system

use ippan::consensus::hashtimer::{
    HashTimer, IppanTimeManager, DriftAnalysis, SyncStats
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[tokio::test]
async fn test_hashtimer_creation() {
    let hashtimer = HashTimer::new("test_node", 1, 1);
    
    assert_eq!(hashtimer.node_id, "test_node");
    assert_eq!(hashtimer.round, 1);
    assert_eq!(hashtimer.sequence, 1);
    assert_eq!(hashtimer.drift_ns, 0);
    assert_eq!(hashtimer.precision_ns, 100);
    assert!(!hashtimer.hash.is_empty());
    assert!(hashtimer.timestamp_ns > 0);
}

#[tokio::test]
async fn test_hashtimer_with_timestamp() {
    let timestamp_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    
    let hashtimer = HashTimer::with_timestamp(
        timestamp_ns,
        "test_node",
        2,
        3,
        50,
    );
    
    assert_eq!(hashtimer.timestamp_ns, timestamp_ns);
    assert_eq!(hashtimer.node_id, "test_node");
    assert_eq!(hashtimer.round, 2);
    assert_eq!(hashtimer.sequence, 3);
    assert_eq!(hashtimer.drift_ns, 50);
    assert_eq!(hashtimer.precision_ns, 100);
}

#[tokio::test]
async fn test_hashtimer_validation() {
    let hashtimer = HashTimer::new("test_node", 1, 1);
    
    // Valid HashTimer should pass validation
    assert!(hashtimer.validate());
    
    // Test with invalid hash
    let mut invalid_hashtimer = hashtimer.clone();
    invalid_hashtimer.hash = "invalid_hash".to_string();
    assert!(!invalid_hashtimer.validate());
}

#[tokio::test]
async fn test_hashtimer_timestamp_conversion() {
    let hashtimer = HashTimer::new("test_node", 1, 1);
    
    // Test Duration conversion
    let duration = hashtimer.as_duration();
    assert!(duration.as_nanos() > 0);
    
    // Test SystemTime conversion
    let system_time = hashtimer.as_system_time();
    assert!(system_time > UNIX_EPOCH);
    
    // Test precision conversions
    assert_eq!(hashtimer.precision_us(), 0); // 100ns = 0.1μs
    assert_eq!(hashtimer.precision_ms(), 0); // 100ns = 0.0001ms
}

#[tokio::test]
async fn test_ippan_time_manager_creation() {
    let manager = IppanTimeManager::new("test_node", 100);
    
    // Test initial state
    let stats = manager.get_sync_stats().await;
    assert_eq!(stats.node_count, 0);
    assert_eq!(stats.current_drift_ns, 0);
    assert!(!stats.has_drift);
    assert_eq!(stats.confidence, 0.0);
}

#[tokio::test]
async fn test_hashtimer_creation_with_manager() {
    let manager = IppanTimeManager::new("test_node", 100);
    
    let hashtimer = manager.create_hashtimer(1, 1).await;
    
    assert_eq!(hashtimer.node_id, "test_node");
    assert_eq!(hashtimer.round, 1);
    assert_eq!(hashtimer.sequence, 1);
    assert!(hashtimer.timestamp_ns > 0);
    assert!(hashtimer.validate());
}

#[tokio::test]
async fn test_network_time_synchronization() {
    let manager = IppanTimeManager::new("test_node", 100);
    
    // Add network times from other nodes
    let time1 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    
    let time2 = time1 + 1000; // 1μs later
    let time3 = time1 + 2000; // 2μs later
    
    manager.add_network_time("node1", time1, 50).await;
    manager.add_network_time("node2", time2, 75).await;
    manager.add_network_time("node3", time3, 100).await;
    
    // Check synchronization stats
    let stats = manager.get_sync_stats().await;
    assert_eq!(stats.node_count, 3);
    assert!(stats.avg_precision_ns > 0);
}

#[tokio::test]
async fn test_drift_detection() {
    let manager = IppanTimeManager::new("test_node", 100);
    
    // Add network times to simulate drift
    let base_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    
    // Add times with increasing drift
    for i in 0..20 {
        let drift = i * 1000; // 1μs per iteration
        manager.add_network_time(&format!("node{}", i), base_time + drift, 50).await;
    }
    
    // Check drift analysis
    let drift_analysis = manager.detect_drift().await;
    assert!(drift_analysis.has_drift);
    assert!(drift_analysis.drift_rate_ns_per_sec > 0.0);
    assert!(drift_analysis.confidence > 0.0);
}

#[tokio::test]
async fn test_timestamp_precision_validation() {
    let manager = IppanTimeManager::new("test_node", 100);
    
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    
    // Test valid precision
    assert!(manager.validate_timestamp_precision(current_time, 1_000_000)); // 1ms tolerance
    
    // Test invalid precision (too far in future)
    let future_time = current_time + 2_000_000_000; // 2 seconds in future
    assert!(!manager.validate_timestamp_precision(future_time, 1_000_000));
    
    // Test invalid precision (too far in past)
    let past_time = current_time - 2_000_000_000; // 2 seconds in past
    assert!(!manager.validate_timestamp_precision(past_time, 1_000_000));
}

#[tokio::test]
async fn test_cleanup_old_times() {
    let manager = IppanTimeManager::new("test_node", 100);
    
    // Add some network times
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    
    manager.add_network_time("node1", time, 50).await;
    manager.add_network_time("node2", time + 1000, 75).await;
    
    // Check initial count
    let stats = manager.get_sync_stats().await;
    assert_eq!(stats.node_count, 2);
    
    // Clean up old times (should keep recent ones)
    manager.cleanup_old_times(1).await; // Keep times less than 1 second old
    
    // Check count after cleanup
    let stats_after = manager.get_sync_stats().await;
    assert_eq!(stats_after.node_count, 2); // Should still have recent times
}

#[tokio::test]
async fn test_synchronized_time_without_network() {
    let manager = IppanTimeManager::new("test_node", 100);
    
    // When no network times are available, should use local time
    let sync_time = manager.get_synchronized_time().await;
    let local_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    
    // Times should be close (within 1ms)
    let time_diff = if sync_time > local_time {
        sync_time - local_time
    } else {
        local_time - sync_time
    };
    
    assert!(time_diff < 1_000_000); // 1ms tolerance
}

#[tokio::test]
async fn test_median_time_calculation() {
    let manager = IppanTimeManager::new("test_node", 100);
    
    let base_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    
    // Add times in random order
    manager.add_network_time("node1", base_time + 1000, 50).await;
    manager.add_network_time("node2", base_time + 500, 75).await;
    manager.add_network_time("node3", base_time + 1500, 100).await;
    
    // Median should be base_time + 1000
    let sync_time = manager.get_synchronized_time().await;
    let expected_median = base_time + 1000;
    
    // Should be close to expected median
    let time_diff = if sync_time > expected_median {
        sync_time - expected_median
    } else {
        expected_median - sync_time
    };
    
    assert!(time_diff < 1_000_000); // 1ms tolerance
}

#[tokio::test]
async fn test_drift_rate_calculation() {
    let manager = IppanTimeManager::new("test_node", 100);
    
    let base_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    
    // Add times with consistent drift
    for i in 0..10 {
        let drift = i * 100_000; // 0.1ms per iteration
        manager.add_network_time(&format!("node{}", i), base_time + drift, 50).await;
    }
    
    let drift_analysis = manager.detect_drift().await;
    
    // Should detect drift
    assert!(drift_analysis.has_drift);
    assert!(drift_analysis.drift_rate_ns_per_sec > 0.0);
    assert!(drift_analysis.confidence > 0.5); // Should have good confidence
}

#[tokio::test]
async fn test_precision_targets() {
    // Test different precision targets
    let manager_100ns = IppanTimeManager::new("test_node", 100);
    let manager_1000ns = IppanTimeManager::new("test_node", 1000);
    
    let hashtimer_100ns = manager_100ns.create_hashtimer(1, 1).await;
    let hashtimer_1000ns = manager_1000ns.create_hashtimer(1, 1).await;
    
    assert_eq!(hashtimer_100ns.precision_ns, 100);
    assert_eq!(hashtimer_1000ns.precision_ns, 100); // Default precision
    
    // Both should be valid
    assert!(hashtimer_100ns.validate());
    assert!(hashtimer_1000ns.validate());
}

#[tokio::test]
async fn test_hash_consistency() {
    let hashtimer1 = HashTimer::new("test_node", 1, 1);
    let hashtimer2 = HashTimer::new("test_node", 1, 1);
    
    // Same parameters should produce same hash
    assert_eq!(hashtimer1.hash, hashtimer2.hash);
    
    // Different parameters should produce different hashes
    let hashtimer3 = HashTimer::new("test_node", 2, 1);
    assert_ne!(hashtimer1.hash, hashtimer3.hash);
    
    let hashtimer4 = HashTimer::new("test_node", 1, 2);
    assert_ne!(hashtimer1.hash, hashtimer4.hash);
    
    let hashtimer5 = HashTimer::new("other_node", 1, 1);
    assert_ne!(hashtimer1.hash, hashtimer5.hash);
} 