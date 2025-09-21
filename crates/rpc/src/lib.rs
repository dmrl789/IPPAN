use anyhow::Result;
use serde::{Deserialize, Serialize};
use ippan_types::{Transaction, Block, ippan_time_now, HashTimer, IppanTimeMicros};

/// RPC request types
#[derive(Debug, Deserialize)]
#[serde(tag = "method")]
pub enum RpcRequest {
    #[serde(rename = "submit_transaction")]
    SubmitTransaction { params: SubmitTransactionParams },
    #[serde(rename = "get_block")]
    GetBlock { params: GetBlockParams },
    #[serde(rename = "get_account")]
    GetAccount { params: GetAccountParams },
    #[serde(rename = "get_time")]
    GetTime,
}

/// RPC response types
#[derive(Debug, Serialize)]
#[serde(tag = "result")]
pub enum RpcResponse {
    #[serde(rename = "submit_transaction")]
    SubmitTransaction { tx_hash: String },
    #[serde(rename = "get_block")]
    GetBlock { block: Option<Block> },
    #[serde(rename = "get_account")]
    GetAccount { balance: u64, nonce: u64 },
    #[serde(rename = "get_time")]
    GetTime { time_us: u64 },
}

/// Submit transaction parameters
#[derive(Debug, Deserialize)]
pub struct SubmitTransactionParams {
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub nonce: u64,
    pub signature: String,
}

/// Get block parameters
#[derive(Debug, Deserialize)]
pub struct GetBlockParams {
    pub hash: Option<String>,
    pub height: Option<u64>,
}

/// Get account parameters
#[derive(Debug, Deserialize)]
pub struct GetAccountParams {
    pub address: String,
}

/// RPC server implementation
pub struct RpcServer {
    // In a real implementation, these would be proper storage and mempool
    // For now, we'll use in-memory storage
    transactions: std::collections::HashMap<String, Transaction>,
    blocks: std::collections::HashMap<String, Block>,
    accounts: std::collections::HashMap<String, (u64, u64)>, // (balance, nonce)
}

impl RpcServer {
    pub fn new() -> Self {
        Self {
            transactions: std::collections::HashMap::new(),
            blocks: std::collections::HashMap::new(),
            accounts: std::collections::HashMap::new(),
        }
    }

    /// Handle RPC request
    pub async fn handle_request(&mut self, request: RpcRequest) -> Result<RpcResponse> {
        match request {
            RpcRequest::SubmitTransaction { params } => {
                self.handle_submit_transaction(params).await
            }
            RpcRequest::GetBlock { params } => {
                self.handle_get_block(params).await
            }
            RpcRequest::GetAccount { params } => {
                self.handle_get_account(params).await
            }
            RpcRequest::GetTime => {
                self.handle_get_time().await
            }
        }
    }

    /// Submit a transaction
    async fn handle_submit_transaction(&mut self, params: SubmitTransactionParams) -> Result<RpcResponse> {
        // Parse addresses (in real implementation, these would be proper address types)
        let from = hex::decode(&params.from)?;
        let to = hex::decode(&params.to)?;
        let signature = hex::decode(&params.signature)?;
        
        if from.len() != 32 || to.len() != 32 || signature.len() != 64 {
            return Err(anyhow::anyhow!("Invalid address or signature length"));
        }
        
        let mut from_bytes = [0u8; 32];
        let mut to_bytes = [0u8; 32];
        let mut signature_bytes = [0u8; 64];
        
        from_bytes.copy_from_slice(&from);
        to_bytes.copy_from_slice(&to);
        signature_bytes.copy_from_slice(&signature);
        
        // Create transaction
        let mut tx = Transaction::new(from_bytes, to_bytes, params.amount, params.nonce);
        tx.signature = signature_bytes;
        
        // Validate transaction
        if !tx.is_valid() {
            return Err(anyhow::anyhow!("Invalid transaction"));
        }
        
        // Store transaction
        let tx_hash = hex::encode(tx.hash());
        self.transactions.insert(tx_hash.clone(), tx);
        
        Ok(RpcResponse::SubmitTransaction { tx_hash })
    }

    /// Get block by hash or height
    async fn handle_get_block(&self, params: GetBlockParams) -> Result<RpcResponse> {
        let block = if let Some(hash) = params.hash {
            self.blocks.get(&hash).cloned()
        } else if let Some(height) = params.height {
            // In real implementation, we'd have a height index
            self.blocks.values().find(|b| b.header.round_id == height).cloned()
        } else {
            None
        };
        
        Ok(RpcResponse::GetBlock { block })
    }

    /// Get account information
    async fn handle_get_account(&self, params: GetAccountParams) -> Result<RpcResponse> {
        let (balance, nonce) = self.accounts.get(&params.address)
            .copied()
            .unwrap_or((0, 0));
        
        Ok(RpcResponse::GetAccount { balance, nonce })
    }

    /// Get current IPPAN time
    async fn handle_get_time(&self) -> Result<RpcResponse> {
        let time_us = ippan_time_now();
        Ok(RpcResponse::GetTime { time_us })
    }
}

/// HTTP server for RPC endpoints
pub struct HttpServer {
    rpc_server: RpcServer,
}

impl HttpServer {
    pub fn new() -> Self {
        Self {
            rpc_server: RpcServer::new(),
        }
    }

    /// Start the HTTP server
    pub async fn start(&mut self, addr: &str) -> Result<()> {
        // In a real implementation, this would use axum or similar
        // For now, we'll just log that the server would start
        tracing::info!("RPC server would start on {}", addr);
        tracing::info!("Available endpoints:");
        tracing::info!("  POST /tx - Submit transaction");
        tracing::info!("  GET /block/{{hash|height}} - Get block");
        tracing::info!("  GET /account/{{address}} - Get account info");
        tracing::info!("  GET /time - Get current IPPAN time");
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_submit_transaction() {
        let mut server = RpcServer::new();
        
        let params = SubmitTransactionParams {
            from: hex::encode([1u8; 32]),
            to: hex::encode([2u8; 32]),
            amount: 1000,
            nonce: 1,
            signature: hex::encode([3u8; 64]),
        };
        
        let request = RpcRequest::SubmitTransaction { params };
        let response = server.handle_request(request).await.unwrap();
        
        match response {
            RpcResponse::SubmitTransaction { tx_hash } => {
                assert!(!tx_hash.is_empty());
            }
            _ => panic!("Expected SubmitTransaction response"),
        }
    }

    #[tokio::test]
    async fn test_get_time() {
        let mut server = RpcServer::new();
        
        let request = RpcRequest::GetTime;
        let response = server.handle_request(request).await.unwrap();
        
        match response {
            RpcResponse::GetTime { time_us } => {
                assert!(time_us > 0);
            }
            _ => panic!("Expected GetTime response"),
        }
    }
}
