#![feature(test)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ippan::{
    storage::{StorageOrchestrator, StorageConfig, StorageUsage},
    utils::crypto,
};
use std::time::Instant;

/// Benchmark storage orchestrator operations
fn benchmark_storage_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("Storage Operations");
    
    group.bench_function("create_storage_orchestrator", |b| {
        b.iter(|| {
            let config = StorageConfig::default();
            black_box(StorageOrchestrator::new(config).unwrap())
        });
    });
    
    group.bench_function("get_storage_usage", |b| {
        let config = StorageConfig::default();
        let storage = StorageOrchestrator::new(config).unwrap();
        
        b.iter(|| {
            black_box(storage.get_usage())
        });
    });
    
    group.finish();
}

/// Benchmark file upload and download operations
fn benchmark_file_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("File Operations");
    
    group.bench_function("upload_small_file", |b| {
        let config = StorageConfig::default();
        let mut storage = StorageOrchestrator::new(config).unwrap();
        storage.start().unwrap();
        
        let test_data = b"Small test file for benchmarking.";
        
        b.iter(|| {
            black_box(storage.upload_file("test_small.txt", test_data).unwrap())
        });
        
        storage.stop().unwrap();
    });
    
    group.bench_function("upload_medium_file", |b| {
        let config = StorageConfig::default();
        let mut storage = StorageOrchestrator::new(config).unwrap();
        storage.start().unwrap();
        
        let test_data = vec![0u8; 1024 * 1024]; // 1MB
        
        b.iter(|| {
            black_box(storage.upload_file("test_medium.txt", &test_data).unwrap())
        });
        
        storage.stop().unwrap();
    });
    
    group.bench_function("upload_large_file", |b| {
        let config = StorageConfig::default();
        let mut storage = StorageOrchestrator::new(config).unwrap();
        storage.start().unwrap();
        
        let test_data = vec![0u8; 10 * 1024 * 1024]; // 10MB
        
        b.iter(|| {
            black_box(storage.upload_file("test_large.txt", &test_data).unwrap())
        });
        
        storage.stop().unwrap();
    });
    
    group.bench_function("download_file", |b| {
        let config = StorageConfig::default();
        let mut storage = StorageOrchestrator::new(config).unwrap();
        storage.start().unwrap();
        
        let test_data = b"Test data for download benchmarking.";
        let file_hash = storage.upload_file("download_test.txt", test_data).unwrap();
        
        b.iter(|| {
            black_box(storage.download_file(&file_hash).unwrap())
        });
        
        storage.stop().unwrap();
    });
    
    group.finish();
}

/// Benchmark encryption operations
fn benchmark_encryption_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("Encryption Operations");
    
    group.bench_function("encrypt_small_data", |b| {
        let config = StorageConfig::default();
        let storage = StorageOrchestrator::new(config).unwrap();
        let test_data = b"Small data for encryption benchmarking.";
        
        b.iter(|| {
            black_box(storage.encrypt_file(test_data).unwrap())
        });
    });
    
    group.bench_function("encrypt_medium_data", |b| {
        let config = StorageConfig::default();
        let storage = StorageOrchestrator::new(config).unwrap();
        let test_data = vec![0u8; 1024 * 1024]; // 1MB
        
        b.iter(|| {
            black_box(storage.encrypt_file(&test_data).unwrap())
        });
    });
    
    group.bench_function("encrypt_large_data", |b| {
        let config = StorageConfig::default();
        let storage = StorageOrchestrator::new(config).unwrap();
        let test_data = vec![0u8; 10 * 1024 * 1024]; // 10MB
        
        b.iter(|| {
            black_box(storage.encrypt_file(&test_data).unwrap())
        });
    });
    
    group.finish();
}

/// Benchmark storage proof operations
fn benchmark_storage_proofs(c: &mut Criterion) {
    let mut group = c.benchmark_group("Storage Proofs");
    
    group.bench_function("generate_storage_proof", |b| {
        let config = StorageConfig::default();
        let mut storage = StorageOrchestrator::new(config).unwrap();
        storage.start().unwrap();
        
        let test_data = b"Data for storage proof benchmarking.";
        let file_hash = storage.upload_file("proof_test.txt", test_data).unwrap();
        
        b.iter(|| {
            black_box(storage.generate_storage_proof(&file_hash).unwrap())
        });
        
        storage.stop().unwrap();
    });
    
    group.bench_function("verify_storage_proof", |b| {
        let config = StorageConfig::default();
        let mut storage = StorageOrchestrator::new(config).unwrap();
        storage.start().unwrap();
        
        let test_data = b"Data for proof verification benchmarking.";
        let file_hash = storage.upload_file("verify_test.txt", test_data).unwrap();
        let proof = storage.generate_storage_proof(&file_hash).unwrap();
        
        b.iter(|| {
            black_box(storage.verify_storage_proof(&file_hash, &proof).unwrap())
        });
        
        storage.stop().unwrap();
    });
    
    group.finish();
}

/// Benchmark storage throughput
fn benchmark_storage_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("Storage Throughput");
    
    group.bench_function("upload_multiple_files", |b| {
        let config = StorageConfig::default();
        let mut storage = StorageOrchestrator::new(config).unwrap();
        storage.start().unwrap();
        
        b.iter(|| {
            for i in 0..10 {
                let test_data = format!("Test data for file {}", i).into_bytes();
                black_box(storage.upload_file(&format!("file_{}.txt", i), &test_data).unwrap());
            }
        });
        
        storage.stop().unwrap();
    });
    
    group.bench_function("download_multiple_files", |b| {
        let config = StorageConfig::default();
        let mut storage = StorageOrchestrator::new(config).unwrap();
        storage.start().unwrap();
        
        let mut file_hashes = Vec::new();
        for i in 0..10 {
            let test_data = format!("Test data for file {}", i).into_bytes();
            let hash = storage.upload_file(&format!("download_file_{}.txt", i), &test_data).unwrap();
            file_hashes.push(hash);
        }
        
        b.iter(|| {
            for hash in &file_hashes {
                black_box(storage.download_file(hash).unwrap());
            }
        });
        
        storage.stop().unwrap();
    });
    
    group.finish();
}

/// Benchmark memory usage
fn benchmark_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("Memory Usage");
    
    group.bench_function("memory_efficient_upload", |b| {
        let config = StorageConfig::default();
        let mut storage = StorageOrchestrator::new(config).unwrap();
        storage.start().unwrap();
        
        let test_data = vec![0u8; 1024 * 1024]; // 1MB
        
        b.iter(|| {
            // Simulate memory-efficient upload
            let chunks: Vec<&[u8]> = test_data.chunks(1024).collect();
            for chunk in chunks {
                black_box(storage.upload_file("memory_test.txt", chunk).unwrap());
            }
        });
        
        storage.stop().unwrap();
    });
    
    group.bench_function("memory_efficient_download", |b| {
        let config = StorageConfig::default();
        let mut storage = StorageOrchestrator::new(config).unwrap();
        storage.start().unwrap();
        
        let test_data = vec![0u8; 1024 * 1024]; // 1MB
        let file_hash = storage.upload_file("memory_download_test.txt", &test_data).unwrap();
        
        b.iter(|| {
            // Simulate memory-efficient download
            let data = storage.download_file(&file_hash).unwrap();
            let chunks: Vec<&[u8]> = data.chunks(1024).collect();
            for chunk in chunks {
                black_box(chunk);
            }
        });
        
        storage.stop().unwrap();
    });
    
    group.finish();
}

/// Benchmark storage statistics
fn benchmark_storage_statistics(c: &mut Criterion) {
    let mut group = c.benchmark_group("Storage Statistics");
    
    group.bench_function("calculate_storage_usage", |b| {
        let config = StorageConfig::default();
        let storage = StorageOrchestrator::new(config).unwrap();
        
        b.iter(|| {
            let usage = storage.get_usage();
            black_box(usage.total_bytes);
            black_box(usage.used_bytes);
            black_box(usage.available_bytes);
        });
    });
    
    group.bench_function("storage_metrics_calculation", |b| {
        let config = StorageConfig::default();
        let storage = StorageOrchestrator::new(config).unwrap();
        
        b.iter(|| {
            let usage = storage.get_usage();
            let utilization_percentage = (usage.used_bytes as f64 / usage.total_bytes as f64) * 100.0;
            let available_percentage = (usage.available_bytes as f64 / usage.total_bytes as f64) * 100.0;
            black_box(utilization_percentage);
            black_box(available_percentage);
        });
    });
    
    group.finish();
}

criterion_group!(
    storage_benches,
    benchmark_storage_operations,
    benchmark_file_operations,
    benchmark_encryption_operations,
    benchmark_storage_proofs,
    benchmark_storage_throughput,
    benchmark_memory_usage,
    benchmark_storage_statistics
);
criterion_main!(storage_benches); 