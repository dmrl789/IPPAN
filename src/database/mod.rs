//! Database subsystem for IPPAN
//!
//! Handles persistent storage of blockchain state, transactions, blocks,
//! accounts, and all other blockchain data with real database operations.

use crate::{Result, IppanError, TransactionHash};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod real_database; // NEW - Real persistent database implementation
pub mod blockchain_state;
pub mod transaction_db;
pub mod account_db;
pub mod block_db;
pub mod index_db;

pub use real_database::RealDatabase;
pub use blockchain_state::BlockchainState;
pub use transaction_db::{TransactionDatabase, StoredTransaction};
pub use account_db::{AccountDatabase, StoredAccount};
pub use block_db::{BlockDatabase, StoredBlock};
pub use index_db::IndexDatabase;

/// Database manager that coordinates all database operations
pub struct DatabaseManager {
    /// Real database instance
    pub real_database: Arc<RealDatabase>,
    /// Blockchain state manager
    pub blockchain_state: Arc<BlockchainState>,
    /// Transaction database
    pub transaction_db: Arc<TransactionDatabase>,
    /// Account database
    pub account_db: Arc<AccountDatabase>,
    /// Block database
    pub block_db: Arc<BlockDatabase>,
    /// Index database
    pub index_db: Arc<IndexDatabase>,
    /// Is running
    is_running: bool,
}

impl DatabaseManager {
    /// Create a new database manager
    pub async fn new(database_path: &str) -> Result<Self> {
        let real_database = Arc::new(RealDatabase::new(database_path).await?);
        let blockchain_state = Arc::new(BlockchainState::new(real_database.clone()).await?);
        let transaction_db = Arc::new(TransactionDatabase::new(real_database.clone()).await?);
        let account_db = Arc::new(AccountDatabase::new(real_database.clone()).await?);
        let block_db = Arc::new(BlockDatabase::new(real_database.clone()).await?);
        let index_db = Arc::new(IndexDatabase::new(real_database.clone()).await?);

        Ok(Self {
            real_database,
            blockchain_state,
            transaction_db,
            account_db,
            block_db,
            index_db,
            is_running: false,
        })
    }

    /// Start the database manager
    pub async fn start(&mut self) -> Result<()> {
        log::info!("Starting database manager...");
        
        // Initialize all database components
        self.real_database.start().await?;
        self.blockchain_state.start().await?;
        self.transaction_db.start().await?;
        self.account_db.start().await?;
        self.block_db.start().await?;
        self.index_db.start().await?;
        
        self.is_running = true;
        log::info!("Database manager started");
        Ok(())
    }

    /// Stop the database manager
    pub async fn stop(&mut self) -> Result<()> {
        log::info!("Stopping database manager...");
        
        // Stop all database components
        self.real_database.stop().await?;
        self.blockchain_state.stop().await?;
        self.transaction_db.stop().await?;
        self.account_db.stop().await?;
        self.block_db.stop().await?;
        self.index_db.stop().await?;
        
        self.is_running = false;
        log::info!("Database manager stopped");
        Ok(())
    }

    /// Get database statistics
    pub async fn get_stats(&self) -> Result<DatabaseStats> {
        let real_stats = self.real_database.get_stats().await?;
        let blockchain_stats = self.blockchain_state.get_stats().await?;
        let transaction_stats = self.transaction_db.get_stats().await?;
        let account_stats = self.account_db.get_stats().await?;
        let block_stats = self.block_db.get_stats().await?;
        let index_stats = self.index_db.get_stats().await?;

        Ok(DatabaseStats {
            real_database: real_stats,
            blockchain_state: blockchain_stats,
            transactions: transaction_stats,
            accounts: account_stats,
            blocks: block_stats,
            indexes: index_stats,
        })
    }
}

/// Database statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStats {
    pub real_database: real_database::DatabaseStats,
    pub blockchain_state: blockchain_state::BlockchainStateStats,
    pub transactions: transaction_db::TransactionStats,
    pub accounts: account_db::AccountStats,
    pub blocks: block_db::BlockStats,
    pub indexes: index_db::IndexStats,
}
