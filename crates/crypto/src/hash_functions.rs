//! Hash functions for IPPAN
//!
//! Provides various cryptographic hash functions including
//! Blake3, SHA256, Keccak256, and other hash algorithms.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Trait for hash functions
pub trait HashFunction {
    /// Hash input data and return the result
    fn hash(&self, data: &[u8]) -> Vec<u8>;
    
    /// Hash input data and return a fixed-size array
    fn hash_fixed(&self, data: &[u8]) -> [u8; 32];
    
    /// Get the output size of the hash function
    fn output_size(&self) -> usize;
    
    /// Get the name of the hash function
    fn name(&self) -> &'static str;
}

/// Blake3 hash implementation
pub struct Blake3 {
    output_size: usize,
}

impl Blake3 {
    /// Create a new Blake3 instance
    pub fn new() -> Self {
        Self { output_size: 32 }
    }
    
    /// Create a new Blake3 instance with custom output size
    pub fn with_output_size(output_size: usize) -> Self {
        Self { output_size }
    }
}

impl HashFunction for Blake3 {
    fn hash(&self, data: &[u8]) -> Vec<u8> {
        use blake3::Hasher;
        let mut hasher = Hasher::new();
        hasher.update(data);
        let hash = hasher.finalize();
        hash.as_bytes()[..self.output_size].to_vec()
    }
    
    fn hash_fixed(&self, data: &[u8]) -> [u8; 32] {
        use blake3::Hasher;
        let mut hasher = Hasher::new();
        hasher.update(data);
        let hash = hasher.finalize();
        let mut result = [0u8; 32];
        result.copy_from_slice(&hash.as_bytes()[0..32]);
        result
    }
    
    fn output_size(&self) -> usize {
        self.output_size
    }
    
    fn name(&self) -> &'static str {
        "Blake3"
    }
}

/// SHA256 hash implementation
pub struct SHA256;

impl SHA256 {
    /// Create a new SHA256 instance
    pub fn new() -> Self {
        Self
    }
}

impl HashFunction for SHA256 {
    fn hash(&self, data: &[u8]) -> Vec<u8> {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().to_vec()
    }
    
    fn hash_fixed(&self, data: &[u8]) -> [u8; 32] {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = hasher.finalize();
        let mut result = [0u8; 32];
        result.copy_from_slice(&hash);
        result
    }
    
    fn output_size(&self) -> usize {
        32
    }
    
    fn name(&self) -> &'static str {
        "SHA256"
    }
}

/// Keccak256 hash implementation
pub struct Keccak256;

impl Keccak256 {
    /// Create a new Keccak256 instance
    pub fn new() -> Self {
        Self
    }
}

impl HashFunction for Keccak256 {
    fn hash(&self, data: &[u8]) -> Vec<u8> {
        use sha3::{Keccak256, Digest};
        let mut hasher = Keccak256::new();
        hasher.update(data);
        hasher.finalize().to_vec()
    }
    
    fn hash_fixed(&self, data: &[u8]) -> [u8; 32] {
        use sha3::{Keccak256, Digest};
        let mut hasher = Keccak256::new();
        hasher.update(data);
        let hash = hasher.finalize();
        let mut result = [0u8; 32];
        result.copy_from_slice(&hash);
        result
    }
    
    fn output_size(&self) -> usize {
        32
    }
    
    fn name(&self) -> &'static str {
        "Keccak256"
    }
}

/// SHA3-256 hash implementation
pub struct SHA3_256;

impl SHA3_256 {
    /// Create a new SHA3-256 instance
    pub fn new() -> Self {
        Self
    }
}

impl HashFunction for SHA3_256 {
    fn hash(&self, data: &[u8]) -> Vec<u8> {
        use sha3::{Sha3_256, Digest};
        let mut hasher = Sha3_256::new();
        hasher.update(data);
        hasher.finalize().to_vec()
    }
    
    fn hash_fixed(&self, data: &[u8]) -> [u8; 32] {
        use sha3::{Sha3_256, Digest};
        let mut hasher = Sha3_256::new();
        hasher.update(data);
        let hash = hasher.finalize();
        let mut result = [0u8; 32];
        result.copy_from_slice(&hash);
        result
    }
    
    fn output_size(&self) -> usize {
        32
    }
    
    fn name(&self) -> &'static str {
        "SHA3-256"
    }
}

/// BLAKE2b hash implementation
pub struct BLAKE2b {
    output_size: usize,
}

impl BLAKE2b {
    /// Create a new BLAKE2b instance
    pub fn new() -> Self {
        Self { output_size: 32 }
    }
    
    /// Create a new BLAKE2b instance with custom output size
    pub fn with_output_size(output_size: usize) -> Self {
        Self { output_size }
    }
}

impl HashFunction for BLAKE2b {
    fn hash(&self, data: &[u8]) -> Vec<u8> {
        use blake2::{Blake2b, Digest};
        let mut hasher = Blake2b::new();
        hasher.update(data);
        let hash = hasher.finalize();
        hash.as_bytes()[..self.output_size].to_vec()
    }
    
    fn hash_fixed(&self, data: &[u8]) -> [u8; 32] {
        use blake2::{Blake2b, Digest};
        let mut hasher = Blake2b::new();
        hasher.update(data);
        let hash = hasher.finalize();
        let mut result = [0u8; 32];
        result.copy_from_slice(&hash.as_bytes()[0..32]);
        result
    }
    
    fn output_size(&self) -> usize {
        self.output_size
    }
    
    fn name(&self) -> &'static str {
        "BLAKE2b"
    }
}

/// Hash function type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HashType {
    Blake3,
    SHA256,
    Keccak256,
    SHA3_256,
    BLAKE2b,
}

impl HashType {
    /// Get the default output size for the hash type
    pub fn default_output_size(&self) -> usize {
        match self {
            HashType::Blake3 => 32,
            HashType::SHA256 => 32,
            HashType::Keccak256 => 32,
            HashType::SHA3_256 => 32,
            HashType::BLAKE2b => 32,
        }
    }
    
    /// Create a hash function instance
    pub fn create_instance(&self) -> Box<dyn HashFunction> {
        match self {
            HashType::Blake3 => Box::new(Blake3::new()),
            HashType::SHA256 => Box::new(SHA256::new()),
            HashType::Keccak256 => Box::new(Keccak256::new()),
            HashType::SHA3_256 => Box::new(SHA3_256::new()),
            HashType::BLAKE2b => Box::new(BLAKE2b::new()),
        }
    }
}

/// Hash function factory
pub struct HashFactory;

impl HashFactory {
    /// Create a hash function by type
    pub fn create(hash_type: HashType) -> Box<dyn HashFunction> {
        hash_type.create_instance()
    }
    
    /// Create a hash function by name
    pub fn create_by_name(name: &str) -> Result<Box<dyn HashFunction>> {
        match name.to_lowercase().as_str() {
            "blake3" => Ok(Box::new(Blake3::new())),
            "sha256" => Ok(Box::new(SHA256::new())),
            "keccak256" => Ok(Box::new(Keccak256::new())),
            "sha3-256" => Ok(Box::new(SHA3_256::new())),
            "blake2b" => Ok(Box::new(BLAKE2b::new())),
            _ => Err(anyhow::anyhow!("Unknown hash function: {}", name)),
        }
    }
}

/// Hash result wrapper
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HashResult {
    pub hash: Vec<u8>,
    pub algorithm: String,
    pub input_size: usize,
    pub output_size: usize,
}

impl HashResult {
    /// Create a new hash result
    pub fn new(hash: Vec<u8>, algorithm: String, input_size: usize, output_size: usize) -> Self {
        Self {
            hash,
            algorithm,
            input_size,
            output_size,
        }
    }
    
    /// Get the hash as a hex string
    pub fn to_hex(&self) -> String {
        hex::encode(&self.hash)
    }
    
    /// Get the hash as a base64 string
    pub fn to_base64(&self) -> String {
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &self.hash)
    }
}

impl fmt::Display for HashResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.algorithm, self.to_hex())
    }
}

/// Hash comparison utility
pub struct HashComparator;

impl HashComparator {
    /// Compare two hash results
    pub fn compare(a: &HashResult, b: &HashResult) -> bool {
        a.hash == b.hash && a.algorithm == b.algorithm
    }
    
    /// Compare hash results from different algorithms
    pub fn compare_different_algorithms(a: &HashResult, b: &HashResult) -> bool {
        a.hash == b.hash
    }
}

/// Hash benchmarking utility
pub struct HashBenchmark {
    iterations: usize,
}

impl HashBenchmark {
    /// Create a new hash benchmark
    pub fn new(iterations: usize) -> Self {
        Self { iterations }
    }
    
    /// Benchmark a hash function
    pub fn benchmark<F>(&self, hash_func: F, data: &[u8]) -> HashBenchmarkResult
    where
        F: Fn(&[u8]) -> Vec<u8>,
    {
        let start = std::time::Instant::now();
        
        for _ in 0..self.iterations {
            let _ = hash_func(data);
        }
        
        let duration = start.elapsed();
        let avg_time = duration / self.iterations as u32;
        
        HashBenchmarkResult {
            iterations: self.iterations,
            total_time: duration,
            average_time: avg_time,
            throughput: (data.len() * self.iterations) as f64 / duration.as_secs_f64(),
        }
    }
}

/// Hash benchmark result
#[derive(Debug, Clone)]
pub struct HashBenchmarkResult {
    pub iterations: usize,
    pub total_time: std::time::Duration,
    pub average_time: std::time::Duration,
    pub throughput: f64, // bytes per second
}

impl fmt::Display for HashBenchmarkResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Hash Benchmark: {} iterations, {:.2?} total, {:.2?} avg, {:.2} MB/s",
            self.iterations,
            self.total_time,
            self.average_time,
            self.throughput / 1_000_000.0
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blake3() {
        let blake3 = Blake3::new();
        let data = b"test data";
        let hash = blake3.hash(data);
        assert_eq!(hash.len(), 32);
        assert_eq!(blake3.name(), "Blake3");
    }

    #[test]
    fn test_sha256() {
        let sha256 = SHA256::new();
        let data = b"test data";
        let hash = sha256.hash(data);
        assert_eq!(hash.len(), 32);
        assert_eq!(sha256.name(), "SHA256");
    }

    #[test]
    fn test_keccak256() {
        let keccak256 = Keccak256::new();
        let data = b"test data";
        let hash = keccak256.hash(data);
        assert_eq!(hash.len(), 32);
        assert_eq!(keccak256.name(), "Keccak256");
    }

    #[test]
    fn test_hash_factory() {
        let blake3 = HashFactory::create(HashType::Blake3);
        let data = b"test data";
        let hash = blake3.hash(data);
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn test_hash_result() {
        let blake3 = Blake3::new();
        let data = b"test data";
        let hash = blake3.hash(data);
        let result = HashResult::new(hash, "Blake3".to_string(), data.len(), 32);
        assert_eq!(result.algorithm, "Blake3");
        assert_eq!(result.input_size, data.len());
        assert_eq!(result.output_size, 32);
    }

    #[test]
    fn test_hash_benchmark() {
        let benchmark = HashBenchmark::new(1000);
        let blake3 = Blake3::new();
        let data = b"test data for benchmarking";
        
        let result = benchmark.benchmark(|d| blake3.hash(d), data);
        assert_eq!(result.iterations, 1000);
        assert!(result.throughput > 0.0);
    }
}
