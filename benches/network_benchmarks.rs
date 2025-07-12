#![feature(test)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ippan::{
    network::{NetworkManager, NetworkConfig},
    dht::{DHTManager, DHTConfig},
    utils::crypto,
};
use std::time::Instant;

/// Benchmark network manager operations
fn benchmark_network_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("Network Operations");
    
    group.bench_function("create_network_manager", |b| {
        b.iter(|| {
            let config = NetworkConfig::default();
            black_box(NetworkManager::new(config).unwrap())
        });
    });
    
    group.bench_function("network_start_stop", |b| {
        b.iter(|| {
            let config = NetworkConfig::default();
            let mut network = NetworkManager::new(config).unwrap();
            network.start().unwrap();
            network.stop().unwrap();
        });
    });
    
    group.finish();
}

/// Benchmark P2P operations
fn benchmark_p2p_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("P2P Operations");
    
    group.bench_function("discover_peers", |b| {
        let config = NetworkConfig::default();
        let mut network = NetworkManager::new(config).unwrap();
        network.start().unwrap();
        
        b.iter(|| {
            black_box(network.discover_peers().unwrap())
        });
        
        network.stop().unwrap();
    });
    
    group.bench_function("connect_to_peer", |b| {
        let config = NetworkConfig::default();
        let mut network = NetworkManager::new(config).unwrap();
        network.start().unwrap();
        
        let peer_id = crypto::generate_node_id();
        
        b.iter(|| {
            black_box(network.connect_to_peer(&peer_id).unwrap())
        });
        
        network.stop().unwrap();
    });
    
    group.bench_function("send_message", |b| {
        let config = NetworkConfig::default();
        let mut network = NetworkManager::new(config).unwrap();
        network.start().unwrap();
        
        let peer_id = crypto::generate_node_id();
        let message = b"Test message for P2P benchmarking.";
        
        b.iter(|| {
            black_box(network.send_message(&peer_id, message).unwrap())
        });
        
        network.stop().unwrap();
    });
    
    group.bench_function("broadcast_message", |b| {
        let config = NetworkConfig::default();
        let mut network = NetworkManager::new(config).unwrap();
        network.start().unwrap();
        
        let message = b"Test broadcast message for benchmarking.";
        
        b.iter(|| {
            black_box(network.broadcast_message(message).unwrap())
        });
        
        network.stop().unwrap();
    });
    
    group.finish();
}

/// Benchmark DHT operations
fn benchmark_dht_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("DHT Operations");
    
    group.bench_function("create_dht_manager", |b| {
        b.iter(|| {
            let config = DHTConfig::default();
            black_box(DHTManager::new(config).unwrap())
        });
    });
    
    group.bench_function("dht_start_stop", |b| {
        b.iter(|| {
            let config = DHTConfig::default();
            let mut dht = DHTManager::new(config).unwrap();
            dht.start().unwrap();
            dht.stop().unwrap();
        });
    });
    
    group.bench_function("store_key_value", |b| {
        let config = DHTConfig::default();
        let mut dht = DHTManager::new(config).unwrap();
        dht.start().unwrap();
        
        b.iter(|| {
            let key = crypto::generate_node_id();
            let value = b"Test value for DHT benchmarking.";
            black_box(dht.store(&key, value).unwrap())
        });
        
        dht.stop().unwrap();
    });
    
    group.bench_function("lookup_key", |b| {
        let config = DHTConfig::default();
        let mut dht = DHTManager::new(config).unwrap();
        dht.start().unwrap();
        
        let key = crypto::generate_node_id();
        let value = b"Test value for DHT lookup benchmarking.";
        dht.store(&key, value).unwrap();
        
        b.iter(|| {
            black_box(dht.lookup(&key).unwrap())
        });
        
        dht.stop().unwrap();
    });
    
    group.bench_function("find_closest_nodes", |b| {
        let config = DHTConfig::default();
        let mut dht = DHTManager::new(config).unwrap();
        dht.start().unwrap();
        
        let target_key = crypto::generate_node_id();
        
        b.iter(|| {
            black_box(dht.find_closest_nodes(&target_key, 8).unwrap())
        });
        
        dht.stop().unwrap();
    });
    
    group.finish();
}

/// Benchmark network discovery
fn benchmark_network_discovery(c: &mut Criterion) {
    let mut group = c.benchmark_group("Network Discovery");
    
    group.bench_function("discover_nodes", |b| {
        let config = NetworkConfig::default();
        let mut network = NetworkManager::new(config).unwrap();
        network.start().unwrap();
        
        b.iter(|| {
            black_box(network.discover_nodes().unwrap())
        });
        
        network.stop().unwrap();
    });
    
    group.bench_function("get_connected_peers", |b| {
        let config = NetworkConfig::default();
        let mut network = NetworkManager::new(config).unwrap();
        network.start().unwrap();
        
        b.iter(|| {
            black_box(network.get_connected_peers())
        });
        
        network.stop().unwrap();
    });
    
    group.bench_function("get_network_stats", |b| {
        let config = NetworkConfig::default();
        let mut network = NetworkManager::new(config).unwrap();
        network.start().unwrap();
        
        b.iter(|| {
            black_box(network.get_network_stats().unwrap())
        });
        
        network.stop().unwrap();
    });
    
    group.finish();
}

/// Benchmark message routing
fn benchmark_message_routing(c: &mut Criterion) {
    let mut group = c.benchmark_group("Message Routing");
    
    group.bench_function("route_message", |b| {
        let config = NetworkConfig::default();
        let mut network = NetworkManager::new(config).unwrap();
        network.start().unwrap();
        
        let target_id = crypto::generate_node_id();
        let message = b"Test routed message for benchmarking.";
        
        b.iter(|| {
            black_box(network.route_message(&target_id, message).unwrap())
        });
        
        network.stop().unwrap();
    });
    
    group.bench_function("route_multiple_messages", |b| {
        let config = NetworkConfig::default();
        let mut network = NetworkManager::new(config).unwrap();
        network.start().unwrap();
        
        let target_ids: Vec<[u8; 32]> = (0..10).map(|_| crypto::generate_node_id()).collect();
        let message = b"Test routed message for multiple targets.";
        
        b.iter(|| {
            for target_id in &target_ids {
                black_box(network.route_message(target_id, message).unwrap());
            }
        });
        
        network.stop().unwrap();
    });
    
    group.finish();
}

/// Benchmark network throughput
fn benchmark_network_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("Network Throughput");
    
    group.bench_function("send_multiple_messages", |b| {
        let config = NetworkConfig::default();
        let mut network = NetworkManager::new(config).unwrap();
        network.start().unwrap();
        
        let peer_ids: Vec<[u8; 32]> = (0..10).map(|_| crypto::generate_node_id()).collect();
        
        b.iter(|| {
            for (i, peer_id) in peer_ids.iter().enumerate() {
                let message = format!("Test message {}", i).into_bytes();
                black_box(network.send_message(peer_id, &message).unwrap());
            }
        });
        
        network.stop().unwrap();
    });
    
    group.bench_function("broadcast_multiple_messages", |b| {
        let config = NetworkConfig::default();
        let mut network = NetworkManager::new(config).unwrap();
        network.start().unwrap();
        
        b.iter(|| {
            for i in 0..10 {
                let message = format!("Broadcast message {}", i).into_bytes();
                black_box(network.broadcast_message(&message).unwrap());
            }
        });
        
        network.stop().unwrap();
    });
    
    group.bench_function("dht_bulk_operations", |b| {
        let config = DHTConfig::default();
        let mut dht = DHTManager::new(config).unwrap();
        dht.start().unwrap();
        
        b.iter(|| {
            for i in 0..100 {
                let key = crypto::generate_node_id();
                let value = format!("Test value {}", i).into_bytes();
                black_box(dht.store(&key, &value).unwrap());
            }
        });
        
        dht.stop().unwrap();
    });
    
    group.finish();
}

/// Benchmark network latency simulation
fn benchmark_network_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("Network Latency");
    
    group.bench_function("simulate_low_latency", |b| {
        let config = NetworkConfig::default();
        let mut network = NetworkManager::new(config).unwrap();
        network.start().unwrap();
        
        let peer_id = crypto::generate_node_id();
        let message = b"Low latency test message.";
        
        b.iter(|| {
            // Simulate low latency network
            std::thread::sleep(std::time::Duration::from_millis(1));
            black_box(network.send_message(&peer_id, message).unwrap())
        });
        
        network.stop().unwrap();
    });
    
    group.bench_function("simulate_high_latency", |b| {
        let config = NetworkConfig::default();
        let mut network = NetworkManager::new(config).unwrap();
        network.start().unwrap();
        
        let peer_id = crypto::generate_node_id();
        let message = b"High latency test message.";
        
        b.iter(|| {
            // Simulate high latency network
            std::thread::sleep(std::time::Duration::from_millis(100));
            black_box(network.send_message(&peer_id, message).unwrap())
        });
        
        network.stop().unwrap();
    });
    
    group.finish();
}

/// Benchmark network reliability
fn benchmark_network_reliability(c: &mut Criterion) {
    let mut group = c.benchmark_group("Network Reliability");
    
    group.bench_function("reliable_message_delivery", |b| {
        let config = NetworkConfig::default();
        let mut network = NetworkManager::new(config).unwrap();
        network.start().unwrap();
        
        let peer_id = crypto::generate_node_id();
        let message = b"Reliable delivery test message.";
        
        b.iter(|| {
            // Simulate reliable message delivery with retries
            for _ in 0..3 {
                match network.send_message(&peer_id, message) {
                    Ok(_) => break,
                    Err(_) => std::thread::sleep(std::time::Duration::from_millis(10)),
                }
            }
        });
        
        network.stop().unwrap();
    });
    
    group.bench_function("fault_tolerant_routing", |b| {
        let config = NetworkConfig::default();
        let mut network = NetworkManager::new(config).unwrap();
        network.start().unwrap();
        
        let target_id = crypto::generate_node_id();
        let message = b"Fault tolerant routing test message.";
        
        b.iter(|| {
            // Simulate fault-tolerant routing with multiple paths
            for _ in 0..3 {
                match network.route_message(&target_id, message) {
                    Ok(_) => break,
                    Err(_) => std::thread::sleep(std::time::Duration::from_millis(10)),
                }
            }
        });
        
        network.stop().unwrap();
    });
    
    group.finish();
}

criterion_group!(
    network_benches,
    benchmark_network_operations,
    benchmark_p2p_operations,
    benchmark_dht_operations,
    benchmark_network_discovery,
    benchmark_message_routing,
    benchmark_network_throughput,
    benchmark_network_latency,
    benchmark_network_reliability
);
criterion_main!(network_benches); 