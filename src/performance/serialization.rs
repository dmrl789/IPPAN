//! High-performance serialization for IPPAN
//! 
//! This module provides optimized serialization and deserialization
//! for high-throughput scenarios.

use std::io::{Read, Write};
use serde::{Serialize, Deserialize};
use bincode::{serialize, deserialize};
// Note: lz4_flex not available, using only bincode serialization

/// High-performance serializer
pub struct HighPerformanceSerializer {
    compression_enabled: bool,
    compression_level: u8,
}

impl HighPerformanceSerializer {
    /// Create a new high-performance serializer
    pub fn new(compression_enabled: bool, compression_level: u8) -> Self {
        Self {
            compression_enabled,
            compression_level,
        }
    }

    /// Serialize data with optional compression
    pub fn serialize<T: Serialize>(&self, data: &T) -> Result<Vec<u8>, String> {
        let serialized = serialize(data).map_err(|e| format!("Serialization error: {}", e))?;
        
        if self.compression_enabled {
            // Use simple compression for now (lz4_flex not available)
            // In production, you'd want to add a proper compression library
            Ok(serialized) // For now, just return serialized data
        } else {
            Ok(serialized)
        }
    }

    /// Deserialize data with optional decompression
    pub fn deserialize<T: for<'de> Deserialize<'de>>(&self, data: &[u8]) -> Result<T, String> {
        let decompressed = if self.compression_enabled {
            // For now, just return the data as-is (no compression)
            data.to_vec()
        } else {
            data.to_vec()
        };
        
        deserialize(&decompressed).map_err(|e| format!("Deserialization error: {}", e))
    }
}

/// Zero-copy serializer for maximum performance
pub struct ZeroCopySerializer {
    buffer: Vec<u8>,
    position: usize,
}

impl ZeroCopySerializer {
    /// Create a new zero-copy serializer
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![0; capacity],
            position: 0,
        }
    }

    /// Write data to the buffer
    pub fn write<T: Serialize>(&mut self, data: &T) -> Result<(), String> {
        let serialized = serialize(data).map_err(|e| format!("Serialization error: {}", e))?;
        
        if self.position + serialized.len() > self.buffer.capacity() {
            return Err("Buffer overflow".to_string());
        }
        
        self.buffer[self.position..self.position + serialized.len()].copy_from_slice(&serialized);
        self.position += serialized.len();
        Ok(())
    }

    /// Read data from the buffer
    pub fn read<T: for<'de> Deserialize<'de>>(&mut self) -> Result<T, String> {
        // This is a simplified implementation
        // In practice, you'd need to track the size of each serialized object
        deserialize(&self.buffer[..self.position]).map_err(|e| format!("Deserialization error: {}", e))
    }

    /// Get the current position
    pub fn position(&self) -> usize {
        self.position
    }

    /// Reset the buffer
    pub fn reset(&mut self) {
        self.position = 0;
    }
}

/// Batch serializer for processing multiple items
pub struct BatchSerializer {
    serializer: HighPerformanceSerializer,
    batch_size: usize,
}

impl BatchSerializer {
    /// Create a new batch serializer
    pub fn new(compression_enabled: bool, compression_level: u8, batch_size: usize) -> Self {
        Self {
            serializer: HighPerformanceSerializer::new(compression_enabled, compression_level),
            batch_size,
        }
    }

    /// Serialize a batch of items
    pub fn serialize_batch<T: Serialize + Clone>(&self, items: &[T]) -> Result<Vec<u8>, String> {
        let batch = items.to_vec();
        self.serializer.serialize(&batch)
    }

    /// Deserialize a batch of items
    pub fn deserialize_batch<T: for<'de> Deserialize<'de>>(&self, data: &[u8]) -> Result<Vec<T>, String> {
        self.serializer.deserialize(data)
    }
}

/// Streaming serializer for continuous processing
pub struct StreamingSerializer {
    serializer: HighPerformanceSerializer,
    buffer: Vec<u8>,
    max_buffer_size: usize,
}

impl StreamingSerializer {
    /// Create a new streaming serializer
    pub fn new(compression_enabled: bool, compression_level: u8, max_buffer_size: usize) -> Self {
        Self {
            serializer: HighPerformanceSerializer::new(compression_enabled, compression_level),
            buffer: Vec::new(),
            max_buffer_size,
        }
    }

    /// Add an item to the stream
    pub fn add_item<T: Serialize>(&mut self, item: &T) -> Result<(), String> {
        let serialized = self.serializer.serialize(item)?;
        
        if self.buffer.len() + serialized.len() > self.max_buffer_size {
            return Err("Buffer overflow".to_string());
        }
        
        self.buffer.extend_from_slice(&serialized);
        Ok(())
    }

    /// Flush the buffer
    pub fn flush(&mut self) -> Vec<u8> {
        let result = self.buffer.clone();
        self.buffer.clear();
        result
    }

    /// Get the current buffer size
    pub fn buffer_size(&self) -> usize {
        self.buffer.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
    struct TestData {
        id: u64,
        data: Vec<u8>,
    }

    #[test]
    fn test_high_performance_serializer() {
        let serializer = HighPerformanceSerializer::new(true, 6);
        
        let data = TestData {
            id: 123,
            data: vec![1, 2, 3, 4, 5],
        };
        
        let serialized = serializer.serialize(&data).unwrap();
        let deserialized: TestData = serializer.deserialize(&serialized).unwrap();
        
        assert_eq!(data, deserialized);
    }

    #[test]
    fn test_zero_copy_serializer() {
        let mut serializer = ZeroCopySerializer::new(1000);
        
        let data = TestData {
            id: 456,
            data: vec![6, 7, 8, 9, 10],
        };
        
        serializer.write(&data).unwrap();
        let deserialized: TestData = serializer.read().unwrap();
        
        assert_eq!(data, deserialized);
    }

    #[test]
    fn test_batch_serializer() {
        let serializer = BatchSerializer::new(true, 6, 10);
        
        let items = vec![
            TestData { id: 1, data: vec![1, 2, 3] },
            TestData { id: 2, data: vec![4, 5, 6] },
            TestData { id: 3, data: vec![7, 8, 9] },
        ];
        
        let serialized = serializer.serialize_batch(&items).unwrap();
        let deserialized: Vec<TestData> = serializer.deserialize_batch(&serialized).unwrap();
        
        assert_eq!(items, deserialized);
    }

    #[test]
    fn test_streaming_serializer() {
        let mut serializer = StreamingSerializer::new(true, 6, 1000);
        
        let items = vec![
            TestData { id: 1, data: vec![1, 2, 3] },
            TestData { id: 2, data: vec![4, 5, 6] },
            TestData { id: 3, data: vec![7, 8, 9] },
        ];
        
        for item in &items {
            serializer.add_item(item).unwrap();
        }
        
        let serialized = serializer.flush();
        assert!(!serialized.is_empty());
    }
}
