//! IPPAN API Usage Examples
//! 
//! This example demonstrates how to interact with the IPPAN HTTP API
//! using the reqwest HTTP client.

use reqwest;
use serde_json::{json, Value};
use std::error::Error;

const API_BASE_URL: &str = "http://localhost:8080";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("🚀 IPPAN API Usage Examples");
    println!("==========================\n");

    // Create HTTP client
    let client = reqwest::Client::new();

    // Example 1: Health Check
    println!("1. Health Check");
    println!("---------------");
    let health_response = client.get(&format!("{}/health", API_BASE_URL))
        .send()
        .await?;
    
    if health_response.status().is_success() {
        let health_data: Value = health_response.json().await?;
        println!("✅ Health check passed: {}", health_data["data"]);
    } else {
        println!("❌ Health check failed");
    }
    println!();

    // Example 2: Get Node Status
    println!("2. Node Status");
    println!("--------------");
    let status_response = client.get(&format!("{}/status", API_BASE_URL))
        .send()
        .await?;
    
    if status_response.status().is_success() {
        let status_data: Value = status_response.json().await?;
        let data = &status_data["data"];
        println!("✅ Node Status:");
        println!("   Version: {}", data["version"]);
        println!("   Uptime: {} seconds", data["uptime_seconds"]);
        println!("   Consensus Round: {}", data["consensus_round"]);
        println!("   Network Peers: {}", data["network_peers"]);
        println!("   Wallet Balance: {}", data["wallet_balance"]);
    } else {
        println!("❌ Failed to get node status");
    }
    println!();

    // Example 3: Store a File
    println!("3. Store File");
    println!("-------------");
    let file_data = "Hello, IPPAN! This is a test file.";
    let store_request = json!({
        "file_id": "test_file_001",
        "name": "test.txt",
        "data": base64::encode(file_data.as_bytes()),
        "mime_type": "text/plain",
        "replication_factor": 3,
        "encryption_enabled": true
    });

    let store_response = client.post(&format!("{}/storage/files", API_BASE_URL))
        .json(&store_request)
        .send()
        .await?;

    if store_response.status().is_success() {
        let store_data: Value = store_response.json().await?;
        let data = &store_data["data"];
        println!("✅ File stored successfully:");
        println!("   File ID: {}", data["file_id"]);
        println!("   Size: {} bytes", data["size"]);
        println!("   Shard Count: {}", data["shard_count"]);
    } else {
        println!("❌ Failed to store file");
    }
    println!();

    // Example 4: Retrieve a File
    println!("4. Retrieve File");
    println!("----------------");
    let retrieve_response = client.get(&format!("{}/storage/files/test_file_001", API_BASE_URL))
        .send()
        .await?;

    if retrieve_response.status().is_success() {
        let retrieve_data: Value = retrieve_response.json().await?;
        let data = &retrieve_data["data"];
        println!("✅ File retrieved successfully:");
        println!("   File ID: {}", data["file_id"]);
        println!("   Name: {}", data["name"]);
        println!("   Size: {} bytes", data["size"]);
        println!("   MIME Type: {}", data["mime_type"]);
        
        // Decode the data
        if let Some(encoded_data) = data["data"].as_str() {
            if let Ok(decoded_data) = base64::decode(encoded_data) {
                if let Ok(decoded_string) = String::from_utf8(decoded_data) {
                    println!("   Content: {}", decoded_string);
                }
            }
        }
    } else {
        println!("❌ Failed to retrieve file");
    }
    println!();

    // Example 5: Get Storage Statistics
    println!("5. Storage Statistics");
    println!("---------------------");
    let stats_response = client.get(&format!("{}/storage/stats", API_BASE_URL))
        .send()
        .await?;

    if stats_response.status().is_success() {
        let stats_data: Value = stats_response.json().await?;
        let data = &stats_data["data"];
        println!("✅ Storage Statistics:");
        println!("   Total Files: {}", data["total_files"]);
        println!("   Total Shards: {}", data["total_shards"]);
        println!("   Total Nodes: {}", data["total_nodes"]);
        println!("   Online Nodes: {}", data["online_nodes"]);
        println!("   Used Bytes: {}", data["used_bytes"]);
        println!("   Total Bytes: {}", data["total_bytes"]);
        println!("   Replication Factor: {}", data["replication_factor"]);
    } else {
        println!("❌ Failed to get storage statistics");
    }
    println!();

    // Example 6: Get Network Peers
    println!("6. Network Peers");
    println!("----------------");
    let peers_response = client.get(&format!("{}/network/peers", API_BASE_URL))
        .send()
        .await?;

    if peers_response.status().is_success() {
        let peers_data: Value = peers_response.json().await?;
        let peers = &peers_data["data"];
        println!("✅ Network Peers:");
        if let Some(peer_list) = peers.as_array() {
            for (i, peer) in peer_list.iter().enumerate() {
                println!("   Peer {}: {}", i + 1, peer);
            }
        }
    } else {
        println!("❌ Failed to get network peers");
    }
    println!();

    // Example 7: Get Consensus Information
    println!("7. Consensus Information");
    println!("------------------------");
    let consensus_response = client.get(&format!("{}/consensus/round", API_BASE_URL))
        .send()
        .await?;

    if consensus_response.status().is_success() {
        let consensus_data: Value = consensus_response.json().await?;
        println!("✅ Consensus Round: {}", consensus_data["data"]);
    } else {
        println!("❌ Failed to get consensus information");
    }
    println!();

    // Example 8: Get Wallet Information
    println!("8. Wallet Information");
    println!("---------------------");
    
    // Get wallet address
    let address_response = client.get(&format!("{}/wallet/address", API_BASE_URL))
        .send()
        .await?;

    if address_response.status().is_success() {
        let address_data: Value = address_response.json().await?;
        println!("✅ Wallet Address: {}", address_data["data"]);
    } else {
        println!("❌ Failed to get wallet address");
    }

    // Get wallet balance
    let balance_response = client.get(&format!("{}/wallet/balance", API_BASE_URL))
        .send()
        .await?;

    if balance_response.status().is_success() {
        let balance_data: Value = balance_response.json().await?;
        println!("✅ Wallet Balance: {}", balance_data["data"]);
    } else {
        println!("❌ Failed to get wallet balance");
    }
    println!();

    // Example 9: Send a Transaction
    println!("9. Send Transaction");
    println!("-------------------");
    let transaction_request = json!({
        "to_address": "i1recipientaddress123456789",
        "amount": 100000,
        "fee": 1000,
        "memo": "Test transaction from API example"
    });

    let transaction_response = client.post(&format!("{}/wallet/send", API_BASE_URL))
        .json(&transaction_request)
        .send()
        .await?;

    if transaction_response.status().is_success() {
        let transaction_data: Value = transaction_response.json().await?;
        println!("✅ Transaction sent successfully:");
        println!("   Transaction Hash: {}", transaction_data["data"]);
    } else {
        println!("❌ Failed to send transaction");
    }
    println!();

    // Example 10: DHT Operations
    println!("10. DHT Operations");
    println!("------------------");
    
    // Store a DHT value
    let dht_store_request = json!({
        "value": "test_dht_value",
        "ttl": 3600
    });

    let dht_store_response = client.post(&format!("{}/dht/keys/test_key", API_BASE_URL))
        .json(&dht_store_request)
        .send()
        .await?;

    if dht_store_response.status().is_success() {
        let dht_store_data: Value = dht_store_response.json().await?;
        println!("✅ DHT value stored: {}", dht_store_data["data"]);
    } else {
        println!("❌ Failed to store DHT value");
    }

    // Get DHT keys
    let dht_keys_response = client.get(&format!("{}/dht/keys", API_BASE_URL))
        .send()
        .await?;

    if dht_keys_response.status().is_success() {
        let dht_keys_data: Value = dht_keys_response.json().await?;
        let keys = &dht_keys_data["data"];
        println!("✅ DHT Keys:");
        if let Some(key_list) = keys.as_array() {
            for (i, key) in key_list.iter().enumerate() {
                println!("   Key {}: {}", i + 1, key);
            }
        }
    } else {
        println!("❌ Failed to get DHT keys");
    }
    println!();

    // Example 11: Error Handling
    println!("11. Error Handling");
    println!("------------------");
    
    // Try to get a non-existent file
    let error_response = client.get(&format!("{}/storage/files/non_existent_file", API_BASE_URL))
        .send()
        .await?;

    if error_response.status().is_client_error() {
        let error_data: Value = error_response.json().await?;
        println!("✅ Error handling works:");
        println!("   Success: {}", error_data["success"]);
        println!("   Error: {}", error_data["error"]);
        println!("   Message: {}", error_data["message"]);
    } else {
        println!("❌ Expected error but got success");
    }
    println!();

    println!("🎉 API Examples Completed Successfully!");
    println!("\nFor more information, see the API documentation at:");
    println!("docs/api_reference.md");

    Ok(())
}

// Helper function to format bytes
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    match bytes {
        0..KB => format!("{} B", bytes),
        KB..MB => format!("{:.1} KB", bytes as f64 / KB as f64),
        MB..GB => format!("{:.1} MB", bytes as f64 / MB as f64),
        _ => format!("{:.1} GB", bytes as f64 / GB as f64),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1048576), "1.0 MB");
        assert_eq!(format_bytes(1073741824), "1.0 GB");
    }
} 