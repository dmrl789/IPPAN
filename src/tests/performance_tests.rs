//! Performance and stress tests for IPPAN
//! 
//! Tests system performance under various load conditions

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::task;

use crate::{
    crypto::real_implementations::*,
    consensus::{bft_engine::*, consensus_manager::*},
    storage::real_storage::*,
    network::real_p2p::*,
    wallet::real_wallet::*,
    database::real_database::*,
    api::real_rest_api::*,
    mining::{block_creator::*, block_validator::*, mining_manager::*},
    genesis::{genesis_creator::*, genesis_manager::*},
    cli::{cli_manager::*, node_commands::*, wallet_commands::*},
    logging::{structured_logger::*, log_aggregator::*, log_analyzer::*},
};

/// Performance test results
#[derive(Debug, Clone)]
pub struct PerformanceTestResults {
    pub test_name: String,
    pub operations_performed: u64,
    pub total_duration: Duration,
    pub operations_per_second: f64,
    pub average_operation_time: Duration,
    pub min_operation_time: Duration,
    pub max_operation_time: Duration,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub success_rate: f64,
    pub errors: Vec<String>,
}

/// Stress test results
#[derive(Debug, Clone)]
pub struct StressTestResults {
    pub test_name: String,
    pub concurrent_operations: u32,
    pub total_operations: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,
    pub total_duration: Duration,
    pub operations_per_second: f64,
    pub average_response_time: Duration,
    pub p95_response_time: Duration,
    pub p99_response_time: Duration,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub error_rate: f64,
    pub errors: Vec<String>,
}

/// Performance test suite
pub struct PerformanceTestSuite;

impl PerformanceTestSuite {
    /// Run crypto performance tests
    pub async fn run_crypto_performance_tests() -> Vec<PerformanceTestResults> {
        let mut results = Vec::new();

        // SHA-256 performance test
        results.push(Self::test_sha256_performance().await);
        
        // SHA-512 performance test
        results.push(Self::test_sha512_performance().await);
        
        // Blake3 performance test
        results.push(Self::test_blake3_performance().await);
        
        // Ed25519 performance test
        results.push(Self::test_ed25519_performance().await);
        
        // AES-256-GCM performance test
        results.push(Self::test_aes256_gcm_performance().await);

        results
    }

    /// Test SHA-256 performance
    async fn test_sha256_performance() -> PerformanceTestResults {
        let test_name = "SHA-256 Performance".to_string();
        let operations = 10000u64;
        let test_data = vec![0u8; 1024]; // 1KB test data
        
        let start_time = Instant::now();
        let mut min_time = Duration::MAX;
        let mut max_time = Duration::ZERO;
        let mut total_time = Duration::ZERO;
        let mut errors = Vec::new();

        for i in 0..operations {
            let op_start = Instant::now();
            let hash = sha256_hash(&test_data);
            let op_duration = op_start.elapsed();
            
            if hash.len() != 32 {
                errors.push(format!("Invalid hash length at operation {}", i));
            }
            
            min_time = min_time.min(op_duration);
            max_time = max_time.max(op_duration);
            total_time += op_duration;
        }

        let total_duration = start_time.elapsed();
        let operations_per_second = operations as f64 / total_duration.as_secs_f64();
        let average_operation_time = total_time / operations;
        let success_rate = (operations - errors.len() as u64) as f64 / operations as f64;

        PerformanceTestResults {
            test_name,
            operations_performed: operations,
            total_duration,
            operations_per_second,
            average_operation_time,
            min_operation_time: min_time,
            max_operation_time: max_time,
            memory_usage_mb: 0.0, // TODO: Implement memory monitoring
            cpu_usage_percent: 0.0, // TODO: Implement CPU monitoring
            success_rate,
            errors,
        }
    }

    /// Test SHA-512 performance
    async fn test_sha512_performance() -> PerformanceTestResults {
        let test_name = "SHA-512 Performance".to_string();
        let operations = 10000u64;
        let test_data = vec![0u8; 1024]; // 1KB test data
        
        let start_time = Instant::now();
        let mut min_time = Duration::MAX;
        let mut max_time = Duration::ZERO;
        let mut total_time = Duration::ZERO;
        let mut errors = Vec::new();

        for i in 0..operations {
            let op_start = Instant::now();
            let hash = sha512_hash(&test_data);
            let op_duration = op_start.elapsed();
            
            if hash.len() != 64 {
                errors.push(format!("Invalid hash length at operation {}", i));
            }
            
            min_time = min_time.min(op_duration);
            max_time = max_time.max(op_duration);
            total_time += op_duration;
        }

        let total_duration = start_time.elapsed();
        let operations_per_second = operations as f64 / total_duration.as_secs_f64();
        let average_operation_time = total_time / operations;
        let success_rate = (operations - errors.len() as u64) as f64 / operations as f64;

        PerformanceTestResults {
            test_name,
            operations_performed: operations,
            total_duration,
            operations_per_second,
            average_operation_time,
            min_operation_time: min_time,
            max_operation_time: max_time,
            memory_usage_mb: 0.0,
            cpu_usage_percent: 0.0,
            success_rate,
            errors,
        }
    }

    /// Test Blake3 performance
    async fn test_blake3_performance() -> PerformanceTestResults {
        let test_name = "Blake3 Performance".to_string();
        let operations = 10000u64;
        let test_data = vec![0u8; 1024]; // 1KB test data
        
        let start_time = Instant::now();
        let mut min_time = Duration::MAX;
        let mut max_time = Duration::ZERO;
        let mut total_time = Duration::ZERO;
        let mut errors = Vec::new();

        for i in 0..operations {
            let op_start = Instant::now();
            let hash = blake3_hash(&test_data);
            let op_duration = op_start.elapsed();
            
            if hash.len() != 32 {
                errors.push(format!("Invalid hash length at operation {}", i));
            }
            
            min_time = min_time.min(op_duration);
            max_time = max_time.max(op_duration);
            total_time += op_duration;
        }

        let total_duration = start_time.elapsed();
        let operations_per_second = operations as f64 / total_duration.as_secs_f64();
        let average_operation_time = total_time / operations;
        let success_rate = (operations - errors.len() as u64) as f64 / operations as f64;

        PerformanceTestResults {
            test_name,
            operations_performed: operations,
            total_duration,
            operations_per_second,
            average_operation_time,
            min_operation_time: min_time,
            max_operation_time: max_time,
            memory_usage_mb: 0.0,
            cpu_usage_percent: 0.0,
            success_rate,
            errors,
        }
    }

    /// Test Ed25519 performance
    async fn test_ed25519_performance() -> PerformanceTestResults {
        let test_name = "Ed25519 Performance".to_string();
        let operations = 1000u64; // Fewer operations due to complexity
        let test_data = b"Performance test data for Ed25519 signing and verification";
        
        let start_time = Instant::now();
        let mut min_time = Duration::MAX;
        let mut max_time = Duration::ZERO;
        let mut total_time = Duration::ZERO;
        let mut errors = Vec::new();

        // Generate key pair once
        let key_pair = generate_ed25519_keypair();

        for i in 0..operations {
            let op_start = Instant::now();
            let signature = sign_ed25519(&key_pair.private_key, test_data);
            let is_valid = verify_ed25519(&key_pair.public_key, test_data, &signature);
            let op_duration = op_start.elapsed();
            
            if !is_valid {
                errors.push(format!("Signature verification failed at operation {}", i));
            }
            
            min_time = min_time.min(op_duration);
            max_time = max_time.max(op_duration);
            total_time += op_duration;
        }

        let total_duration = start_time.elapsed();
        let operations_per_second = operations as f64 / total_duration.as_secs_f64();
        let average_operation_time = total_time / operations;
        let success_rate = (operations - errors.len() as u64) as f64 / operations as f64;

        PerformanceTestResults {
            test_name,
            operations_performed: operations,
            total_duration,
            operations_per_second,
            average_operation_time,
            min_operation_time: min_time,
            max_operation_time: max_time,
            memory_usage_mb: 0.0,
            cpu_usage_percent: 0.0,
            success_rate,
            errors,
        }
    }

    /// Test AES-256-GCM performance
    async fn test_aes256_gcm_performance() -> PerformanceTestResults {
        let test_name = "AES-256-GCM Performance".to_string();
        let operations = 5000u64;
        let test_data = vec![0u8; 1024]; // 1KB test data
        let key = [0u8; 32];
        let nonce = [0u8; 12];
        
        let start_time = Instant::now();
        let mut min_time = Duration::MAX;
        let mut max_time = Duration::ZERO;
        let mut total_time = Duration::ZERO;
        let mut errors = Vec::new();

        for i in 0..operations {
            let op_start = Instant::now();
            let encrypted = aes256_gcm_encrypt(&test_data, &key, &nonce);
            let decrypted = aes256_gcm_decrypt(&encrypted, &key, &nonce);
            let op_duration = op_start.elapsed();
            
            if decrypted != test_data {
                errors.push(format!("Encryption/decryption failed at operation {}", i));
            }
            
            min_time = min_time.min(op_duration);
            max_time = max_time.max(op_duration);
            total_time += op_duration;
        }

        let total_duration = start_time.elapsed();
        let operations_per_second = operations as f64 / total_duration.as_secs_f64();
        let average_operation_time = total_time / operations;
        let success_rate = (operations - errors.len() as u64) as f64 / operations as f64;

        PerformanceTestResults {
            test_name,
            operations_performed: operations,
            total_duration,
            operations_per_second,
            average_operation_time,
            min_operation_time: min_time,
            max_operation_time: max_time,
            memory_usage_mb: 0.0,
            cpu_usage_percent: 0.0,
            success_rate,
            errors,
        }
    }

    /// Run consensus performance tests
    pub async fn run_consensus_performance_tests() -> Vec<PerformanceTestResults> {
        let mut results = Vec::new();

        // BFT engine performance test
        results.push(Self::test_bft_engine_performance().await);
        
        // Consensus manager performance test
        results.push(Self::test_consensus_manager_performance().await);

        results
    }

    /// Test BFT engine performance
    async fn test_bft_engine_performance() -> PerformanceTestResults {
        let test_name = "BFT Engine Performance".to_string();
        let operations = 1000u64;
        
        let start_time = Instant::now();
        let mut min_time = Duration::MAX;
        let mut max_time = Duration::ZERO;
        let mut total_time = Duration::ZERO;
        let mut errors = Vec::new();

        let bft_engine = BFTEngine::new();

        for i in 0..operations {
            let op_start = Instant::now();
            let result = bft_engine.process_pre_prepare_message(
                format!("performance_test_block_{}", i), 
                i, 
                0
            ).await;
            let op_duration = op_start.elapsed();
            
            if result.is_err() {
                errors.push(format!("BFT operation failed at operation {}: {:?}", i, result.err()));
            }
            
            min_time = min_time.min(op_duration);
            max_time = max_time.max(op_duration);
            total_time += op_duration;
        }

        let total_duration = start_time.elapsed();
        let operations_per_second = operations as f64 / total_duration.as_secs_f64();
        let average_operation_time = total_time / operations;
        let success_rate = (operations - errors.len() as u64) as f64 / operations as f64;

        PerformanceTestResults {
            test_name,
            operations_performed: operations,
            total_duration,
            operations_per_second,
            average_operation_time,
            min_operation_time: min_time,
            max_operation_time: max_time,
            memory_usage_mb: 0.0,
            cpu_usage_percent: 0.0,
            success_rate,
            errors,
        }
    }

    /// Test consensus manager performance
    async fn test_consensus_manager_performance() -> PerformanceTestResults {
        let test_name = "Consensus Manager Performance".to_string();
        let operations = 1000u64;
        
        let start_time = Instant::now();
        let mut min_time = Duration::MAX;
        let mut max_time = Duration::ZERO;
        let mut total_time = Duration::ZERO;
        let mut errors = Vec::new();

        let consensus_manager = ConsensusManager::new();

        for i in 0..operations {
            let op_start = Instant::now();
            let result = consensus_manager.create_block().await;
            let op_duration = op_start.elapsed();
            
            if result.is_err() {
                errors.push(format!("Consensus manager operation failed at operation {}: {:?}", i, result.err()));
            }
            
            min_time = min_time.min(op_duration);
            max_time = max_time.max(op_duration);
            total_time += op_duration;
        }

        let total_duration = start_time.elapsed();
        let operations_per_second = operations as f64 / total_duration.as_secs_f64();
        let average_operation_time = total_time / operations;
        let success_rate = (operations - errors.len() as u64) as f64 / operations as f64;

        PerformanceTestResults {
            test_name,
            operations_performed: operations,
            total_duration,
            operations_per_second,
            average_operation_time,
            min_operation_time: min_time,
            max_operation_time: max_time,
            memory_usage_mb: 0.0,
            cpu_usage_percent: 0.0,
            success_rate,
            errors,
        }
    }

    /// Run stress tests
    pub async fn run_stress_tests() -> Vec<StressTestResults> {
        let mut results = Vec::new();

        // Crypto stress test
        results.push(Self::run_crypto_stress_test().await);
        
        // Consensus stress test
        results.push(Self::run_consensus_stress_test().await);
        
        // Storage stress test
        results.push(Self::run_storage_stress_test().await);

        results
    }

    /// Run crypto stress test
    async fn run_crypto_stress_test() -> StressTestResults {
        let test_name = "Crypto Stress Test".to_string();
        let concurrent_operations = 100u32;
        let operations_per_task = 100u64;
        let total_operations = concurrent_operations as u64 * operations_per_task;
        
        let start_time = Instant::now();
        let mut successful_operations = 0u64;
        let mut failed_operations = 0u64;
        let mut response_times = Vec::new();
        let mut errors = Vec::new();

        let tasks: Vec<_> = (0..concurrent_operations)
            .map(|_| {
                task::spawn(async move {
                    let mut task_successful = 0u64;
                    let mut task_failed = 0u64;
                    let mut task_response_times = Vec::new();
                    let mut task_errors = Vec::new();

                    for i in 0..operations_per_task {
                        let op_start = Instant::now();
                        let key_pair = generate_ed25519_keypair();
                        let test_data = b"Stress test data";
                        let signature = sign_ed25519(&key_pair.private_key, test_data);
                        let is_valid = verify_ed25519(&key_pair.public_key, test_data, &signature);
                        let op_duration = op_start.elapsed();
                        
                        task_response_times.push(op_duration);
                        
                        if is_valid {
                            task_successful += 1;
                        } else {
                            task_failed += 1;
                            task_errors.push(format!("Signature verification failed at operation {}", i));
                        }
                    }

                    (task_successful, task_failed, task_response_times, task_errors)
                })
            })
            .collect();

        for task in tasks {
            match task.await {
                Ok((successful, failed, response_times_task, errors_task)) => {
                    successful_operations += successful;
                    failed_operations += failed;
                    response_times.extend(response_times_task);
                    errors.extend(errors_task);
                }
                Err(e) => {
                    failed_operations += operations_per_task;
                    errors.push(format!("Task failed: {}", e));
                }
            }
        }

        let total_duration = start_time.elapsed();
        let operations_per_second = total_operations as f64 / total_duration.as_secs_f64();
        
        // Calculate response time percentiles
        response_times.sort();
        let average_response_time = response_times.iter().sum::<Duration>() / response_times.len() as u32;
        let p95_index = (response_times.len() as f64 * 0.95) as usize;
        let p99_index = (response_times.len() as f64 * 0.99) as usize;
        let p95_response_time = response_times.get(p95_index).copied().unwrap_or(Duration::ZERO);
        let p99_response_time = response_times.get(p99_index).copied().unwrap_or(Duration::ZERO);
        
        let error_rate = failed_operations as f64 / total_operations as f64;

        StressTestResults {
            test_name,
            concurrent_operations,
            total_operations,
            successful_operations,
            failed_operations,
            total_duration,
            operations_per_second,
            average_response_time,
            p95_response_time,
            p99_response_time,
            memory_usage_mb: 0.0,
            cpu_usage_percent: 0.0,
            error_rate,
            errors,
        }
    }

    /// Run consensus stress test
    async fn run_consensus_stress_test() -> StressTestResults {
        let test_name = "Consensus Stress Test".to_string();
        let concurrent_operations = 50u32;
        let operations_per_task = 50u64;
        let total_operations = concurrent_operations as u64 * operations_per_task;
        
        let start_time = Instant::now();
        let mut successful_operations = 0u64;
        let mut failed_operations = 0u64;
        let mut response_times = Vec::new();
        let mut errors = Vec::new();

        let tasks: Vec<_> = (0..concurrent_operations)
            .map(|_| {
                task::spawn(async move {
                    let mut task_successful = 0u64;
                    let mut task_failed = 0u64;
                    let mut task_response_times = Vec::new();
                    let mut task_errors = Vec::new();

                    let bft_engine = BFTEngine::new();

                    for i in 0..operations_per_task {
                        let op_start = Instant::now();
                        let result = bft_engine.process_pre_prepare_message(
                            format!("stress_test_block_{}", i), 
                            i, 
                            0
                        ).await;
                        let op_duration = op_start.elapsed();
                        
                        task_response_times.push(op_duration);
                        
                        if result.is_ok() {
                            task_successful += 1;
                        } else {
                            task_failed += 1;
                            task_errors.push(format!("BFT operation failed at operation {}: {:?}", i, result.err()));
                        }
                    }

                    (task_successful, task_failed, task_response_times, task_errors)
                })
            })
            .collect();

        for task in tasks {
            match task.await {
                Ok((successful, failed, response_times_task, errors_task)) => {
                    successful_operations += successful;
                    failed_operations += failed;
                    response_times.extend(response_times_task);
                    errors.extend(errors_task);
                }
                Err(e) => {
                    failed_operations += operations_per_task;
                    errors.push(format!("Task failed: {}", e));
                }
            }
        }

        let total_duration = start_time.elapsed();
        let operations_per_second = total_operations as f64 / total_duration.as_secs_f64();
        
        // Calculate response time percentiles
        response_times.sort();
        let average_response_time = response_times.iter().sum::<Duration>() / response_times.len() as u32;
        let p95_index = (response_times.len() as f64 * 0.95) as usize;
        let p99_index = (response_times.len() as f64 * 0.99) as usize;
        let p95_response_time = response_times.get(p95_index).copied().unwrap_or(Duration::ZERO);
        let p99_response_time = response_times.get(p99_index).copied().unwrap_or(Duration::ZERO);
        
        let error_rate = failed_operations as f64 / total_operations as f64;

        StressTestResults {
            test_name,
            concurrent_operations,
            total_operations,
            successful_operations,
            failed_operations,
            total_duration,
            operations_per_second,
            average_response_time,
            p95_response_time,
            p99_response_time,
            memory_usage_mb: 0.0,
            cpu_usage_percent: 0.0,
            error_rate,
            errors,
        }
    }

    /// Run storage stress test
    async fn run_storage_stress_test() -> StressTestResults {
        let test_name = "Storage Stress Test".to_string();
        let concurrent_operations = 50u32;
        let operations_per_task = 20u64;
        let total_operations = concurrent_operations as u64 * operations_per_task;
        
        let start_time = Instant::now();
        let mut successful_operations = 0u64;
        let mut failed_operations = 0u64;
        let mut response_times = Vec::new();
        let mut errors = Vec::new();

        let tasks: Vec<_> = (0..concurrent_operations)
            .map(|task_id| {
                task::spawn(async move {
                    let mut task_successful = 0u64;
                    let mut task_failed = 0u64;
                    let mut task_response_times = Vec::new();
                    let mut task_errors = Vec::new();

                    let storage_manager = RealStorageManager::new();

                    for i in 0..operations_per_task {
                        let op_start = Instant::now();
                        let filename = format!("stress_test_file_{}_{}", task_id, i);
                        let test_data = vec![0u8; 1024]; // 1KB test data
                        
                        let store_result = storage_manager.store_file(&filename, &test_data).await;
                        let retrieve_result = storage_manager.retrieve_file(&filename).await;
                        let delete_result = storage_manager.delete_file(&filename).await;
                        
                        let op_duration = op_start.elapsed();
                        task_response_times.push(op_duration);
                        
                        if store_result.is_ok() && retrieve_result.is_ok() && delete_result.is_ok() {
                            task_successful += 1;
                        } else {
                            task_failed += 1;
                            task_errors.push(format!("Storage operation failed at operation {}: store={:?}, retrieve={:?}, delete={:?}", 
                                i, store_result.err(), retrieve_result.err(), delete_result.err()));
                        }
                    }

                    (task_successful, task_failed, task_response_times, task_errors)
                })
            })
            .collect();

        for task in tasks {
            match task.await {
                Ok((successful, failed, response_times_task, errors_task)) => {
                    successful_operations += successful;
                    failed_operations += failed;
                    response_times.extend(response_times_task);
                    errors.extend(errors_task);
                }
                Err(e) => {
                    failed_operations += operations_per_task;
                    errors.push(format!("Task failed: {}", e));
                }
            }
        }

        let total_duration = start_time.elapsed();
        let operations_per_second = total_operations as f64 / total_duration.as_secs_f64();
        
        // Calculate response time percentiles
        response_times.sort();
        let average_response_time = response_times.iter().sum::<Duration>() / response_times.len() as u32;
        let p95_index = (response_times.len() as f64 * 0.95) as usize;
        let p99_index = (response_times.len() as f64 * 0.99) as usize;
        let p95_response_time = response_times.get(p95_index).copied().unwrap_or(Duration::ZERO);
        let p99_response_time = response_times.get(p99_index).copied().unwrap_or(Duration::ZERO);
        
        let error_rate = failed_operations as f64 / total_operations as f64;

        StressTestResults {
            test_name,
            concurrent_operations,
            total_operations,
            successful_operations,
            failed_operations,
            total_duration,
            operations_per_second,
            average_response_time,
            p95_response_time,
            p99_response_time,
            memory_usage_mb: 0.0,
            cpu_usage_percent: 0.0,
            error_rate,
            errors,
        }
    }

    /// Run all performance tests
    pub async fn run_all_performance_tests() -> (Vec<PerformanceTestResults>, Vec<StressTestResults>) {
        let performance_results = Self::run_crypto_performance_tests().await;
        let stress_results = Self::run_stress_tests().await;
        
        (performance_results, stress_results)
    }

    /// Print performance test results
    pub fn print_performance_results(results: &[PerformanceTestResults]) {
        println!("\n📊 Performance Test Results:");
        println!("=============================");
        
        for result in results {
            println!("\n🔧 {}", result.test_name);
            println!("  Operations: {}", result.operations_performed);
            println!("  Duration: {:?}", result.total_duration);
            println!("  Ops/sec: {:.2}", result.operations_per_second);
            println!("  Avg time: {:?}", result.average_operation_time);
            println!("  Min time: {:?}", result.min_operation_time);
            println!("  Max time: {:?}", result.max_operation_time);
            println!("  Success rate: {:.2}%", result.success_rate * 100.0);
            
            if !result.errors.is_empty() {
                println!("  Errors: {}", result.errors.len());
                for error in &result.errors[..3] { // Show first 3 errors
                    println!("    - {}", error);
                }
                if result.errors.len() > 3 {
                    println!("    ... and {} more errors", result.errors.len() - 3);
                }
            }
        }
    }

    /// Print stress test results
    pub fn print_stress_results(results: &[StressTestResults]) {
        println!("\n💪 Stress Test Results:");
        println!("=======================");
        
        for result in results {
            println!("\n🔥 {}", result.test_name);
            println!("  Concurrent ops: {}", result.concurrent_operations);
            println!("  Total ops: {}", result.total_operations);
            println!("  Successful: {}", result.successful_operations);
            println!("  Failed: {}", result.failed_operations);
            println!("  Duration: {:?}", result.total_duration);
            println!("  Ops/sec: {:.2}", result.operations_per_second);
            println!("  Avg response: {:?}", result.average_response_time);
            println!("  P95 response: {:?}", result.p95_response_time);
            println!("  P99 response: {:?}", result.p99_response_time);
            println!("  Error rate: {:.2}%", result.error_rate * 100.0);
            
            if !result.errors.is_empty() {
                println!("  Errors: {}", result.errors.len());
                for error in &result.errors[..3] { // Show first 3 errors
                    println!("    - {}", error);
                }
                if result.errors.len() > 3 {
                    println!("    ... and {} more errors", result.errors.len() - 3);
                }
            }
        }
    }
}

/// Run all performance tests
pub async fn run_all_performance_tests() -> (Vec<PerformanceTestResults>, Vec<StressTestResults>) {
    PerformanceTestSuite::run_all_performance_tests().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sha256_performance() {
        let result = PerformanceTestSuite::test_sha256_performance().await;
        
        assert_eq!(result.test_name, "SHA-256 Performance");
        assert_eq!(result.operations_performed, 10000);
        assert!(result.operations_per_second > 0.0);
        assert!(result.success_rate >= 0.99);
    }

    #[tokio::test]
    async fn test_ed25519_performance() {
        let result = PerformanceTestSuite::test_ed25519_performance().await;
        
        assert_eq!(result.test_name, "Ed25519 Performance");
        assert_eq!(result.operations_performed, 1000);
        assert!(result.operations_per_second > 0.0);
        assert!(result.success_rate >= 0.99);
    }

    #[tokio::test]
    async fn test_crypto_stress_test() {
        let result = PerformanceTestSuite::run_crypto_stress_test().await;
        
        assert_eq!(result.test_name, "Crypto Stress Test");
        assert_eq!(result.concurrent_operations, 100);
        assert!(result.total_operations > 0);
        assert!(result.operations_per_second > 0.0);
    }
}
