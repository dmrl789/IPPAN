//! DHT routing module
//! 
//! Handles routing table management and finding optimal paths to nodes.

use crate::{dht::{DhtNode, RoutingEntry}, error::IppanError, Result};
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Routing manager
pub struct RoutingManager {
    /// Routing table
    routing_table: HashMap<[u8; 32], Vec<RoutingEntry>>,
    /// Local node ID
    local_node_id: [u8; 32],
    /// Routing statistics
    stats: RoutingStats,
}

/// Routing statistics
#[derive(Debug, Clone)]
pub struct RoutingStats {
    /// Total routing table entries
    pub total_entries: usize,
    /// Number of buckets
    pub bucket_count: usize,
    /// Average bucket size
    pub avg_bucket_size: f64,
    /// Routing table size
    pub routing_table_size: usize,
}

impl RoutingManager {
    /// Create a new routing manager
    pub fn new(local_node_id: [u8; 32]) -> Self {
        Self {
            routing_table: HashMap::new(),
            local_node_id,
            stats: RoutingStats {
                total_entries: 0,
                bucket_count: 0,
                avg_bucket_size: 0.0,
                routing_table_size: 0,
            },
        }
    }
    
    /// Add a routing entry
    pub fn add_entry(&mut self, entry: RoutingEntry) {
        let bucket_key = self.get_bucket_key(&entry.node.node_id);
        let bucket = self.routing_table.entry(bucket_key).or_insert_with(Vec::new);
        
        // Check if node already exists
        if let Some(existing) = bucket.iter_mut().find(|e| e.node.node_id == entry.node.node_id) {
            *existing = entry;
        } else {
            bucket.push(entry);
        }
        
        // Limit bucket size
        if bucket.len() > 20 {
            bucket.sort_by_key(|e| e.distance);
            bucket.truncate(20);
        }
        
        self.update_stats();
    }
    
    /// Remove a routing entry
    pub fn remove_entry(&mut self, node_id: &[u8; 32]) {
        let bucket_key = self.get_bucket_key(node_id);
        
        if let Some(bucket) = self.routing_table.get_mut(&bucket_key) {
            bucket.retain(|entry| entry.node.node_id != *node_id);
            
            if bucket.is_empty() {
                self.routing_table.remove(&bucket_key);
            }
        }
        
        self.update_stats();
    }
    
    /// Find closest nodes to a key
    pub fn find_closest_nodes(&self, key: &[u8; 32], count: usize) -> Vec<&RoutingEntry> {
        let mut candidates: Vec<_> = self.routing_table.values()
            .flatten()
            .collect();
        
        // Calculate distances
        for entry in &mut candidates {
            entry.distance = self.calculate_distance(&entry.node.node_id, key);
        }
        
        // Sort by distance
        candidates.sort_by_key(|entry| entry.distance);
        
        // Return closest nodes
        candidates.into_iter().take(count).collect()
    }
    
    /// Get bucket key for routing
    fn get_bucket_key(&self, node_id: &[u8; 32]) -> [u8; 32] {
        // Use first 4 bytes as bucket key
        let mut bucket_key = [0u8; 32];
        bucket_key[..4].copy_from_slice(&node_id[..4]);
        bucket_key
    }
    
    /// Calculate distance between two keys
    fn calculate_distance(&self, node_id: &[u8; 32], key: &[u8; 32]) -> u32 {
        // XOR distance for Kademlia-like routing
        let mut distance = 0u32;
        for i in 0..32 {
            distance += (node_id[i] ^ key[i]).count_ones();
        }
        distance
    }
    
    /// Update routing statistics
    fn update_stats(&mut self) {
        let total_entries: usize = self.routing_table.values().map(|bucket| bucket.len()).sum();
        let bucket_count = self.routing_table.len();
        let avg_bucket_size = if bucket_count > 0 {
            total_entries as f64 / bucket_count as f64
        } else {
            0.0
        };
        
        self.stats = RoutingStats {
            total_entries,
            bucket_count,
            avg_bucket_size,
            routing_table_size: self.routing_table.len(),
        };
    }
    
    /// Get routing statistics
    pub fn get_stats(&self) -> RoutingStats {
        self.stats.clone()
    }
    
    /// Get routing table
    pub fn get_routing_table(&self) -> &HashMap<[u8; 32], Vec<RoutingEntry>> {
        &self.routing_table
    }
    
    /// Get bucket for a key
    pub fn get_bucket(&self, key: &[u8; 32]) -> Option<&Vec<RoutingEntry>> {
        let bucket_key = self.get_bucket_key(key);
        self.routing_table.get(&bucket_key)
    }
    
    /// Check if a node is in routing table
    pub fn contains_node(&self, node_id: &[u8; 32]) -> bool {
        let bucket_key = self.get_bucket_key(node_id);
        
        if let Some(bucket) = self.routing_table.get(&bucket_key) {
            bucket.iter().any(|entry| entry.node.node_id == *node_id)
        } else {
            false
        }
    }
    
    /// Get routing entry for a node
    pub fn get_entry(&self, node_id: &[u8; 32]) -> Option<&RoutingEntry> {
        let bucket_key = self.get_bucket_key(node_id);
        
        if let Some(bucket) = self.routing_table.get(&bucket_key) {
            bucket.iter().find(|entry| entry.node.node_id == *node_id)
        } else {
            None
        }
    }
    
    /// Update node connection status
    pub fn update_connection_status(&mut self, node_id: &[u8; 32], connected: bool) {
        let bucket_key = self.get_bucket_key(node_id);
        
        if let Some(bucket) = self.routing_table.get_mut(&bucket_key) {
            if let Some(entry) = bucket.iter_mut().find(|e| e.node.node_id == *node_id) {
                entry.connected = connected;
            }
        }
    }
    
    /// Get connected nodes
    pub fn get_connected_nodes(&self) -> Vec<&RoutingEntry> {
        self.routing_table.values()
            .flatten()
            .filter(|entry| entry.connected)
            .collect()
    }
    
    /// Get disconnected nodes
    pub fn get_disconnected_nodes(&self) -> Vec<&RoutingEntry> {
        self.routing_table.values()
            .flatten()
            .filter(|entry| !entry.connected)
            .collect()
    }
    
    /// Clean up old entries
    pub fn cleanup_old_entries(&mut self, max_age: u64) {
        let now = chrono::Utc::now().timestamp() as u64;
        
        for bucket in self.routing_table.values_mut() {
            bucket.retain(|entry| {
                now - entry.last_pong < max_age
            });
        }
        
        // Remove empty buckets
        self.routing_table.retain(|_, bucket| !bucket.is_empty());
        
        self.update_stats();
    }
}
