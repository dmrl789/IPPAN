use parking_lot::RwLock;
use serde_json;
use std::fs;
use std::path::{Path, PathBuf};

use crate::crypto::*;
use crate::errors::*;
use crate::types::*;

/// Secure wallet storage implementation
pub struct WalletStorage {
    wallet_path: PathBuf,
    backup_path: PathBuf,
    password_hash: RwLock<Option<String>>,
    wallet_state: RwLock<Option<WalletState>>,
}

impl WalletStorage {
    /// Create a new wallet storage instance
    pub fn new(wallet_dir: &Path) -> Self {
        let wallet_path = wallet_dir.join("wallet.json");
        let backup_path = wallet_dir.join("backups");

        // Ensure backup directory exists
        if !backup_path.exists() {
            let _ = fs::create_dir_all(&backup_path);
        }

        Self {
            wallet_path,
            backup_path,
            password_hash: RwLock::new(None),
            wallet_state: RwLock::new(None),
        }
    }

    /// Initialize wallet with password protection
    pub fn initialize(&self, name: String, password: Option<&str>) -> Result<()> {
        let mut state = WalletState::new(name);

        if let Some(pwd) = password {
            let hash = hash_password(pwd)?;
            state.config.encryption_enabled = true;
            *self.password_hash.write() = Some(hash);
        }

        *self.wallet_state.write() = Some(state);
        self.save_wallet()?;
        Ok(())
    }

    /// Load wallet from storage
    pub fn load_wallet(&self, password: Option<&str>) -> Result<()> {
        if !self.wallet_path.exists() {
            return Err(WalletError::WalletNotInitialized);
        }

        let data = fs::read_to_string(&self.wallet_path)
            .map_err(|e| WalletError::StorageError(format!("Failed to read wallet: {}", e)))?;

        let state: WalletState = serde_json::from_str(&data)
            .map_err(|e| WalletError::StorageError(format!("Failed to parse wallet: {}", e)))?;

        // Verify password if encryption is enabled
        if state.config.encryption_enabled {
            if let Some(pwd) = password {
                if let Some(hash) = self.password_hash.read().as_ref() {
                    if !verify_password(pwd, hash)? {
                        return Err(WalletError::InvalidPassword);
                    }
                }
            } else {
                return Err(WalletError::WalletLocked("Password required".to_string()));
            }
        }

        *self.wallet_state.write() = Some(state);
        Ok(())
    }

    /// Save wallet to storage
    pub fn save_wallet(&self) -> Result<()> {
        let state = self.wallet_state.read();
        if let Some(ref wallet_state) = *state {
            let data = serde_json::to_string_pretty(wallet_state).map_err(|e| {
                WalletError::StorageError(format!("Failed to serialize wallet: {}", e))
            })?;

            // Write to temporary file first, then rename for atomicity
            let temp_path = self.wallet_path.with_extension("tmp");
            fs::write(&temp_path, data)
                .map_err(|e| WalletError::StorageError(format!("Failed to write wallet: {}", e)))?;

            fs::rename(&temp_path, &self.wallet_path).map_err(|e| {
                WalletError::StorageError(format!("Failed to rename wallet file: {}", e))
            })?;
        }

        Ok(())
    }

    /// Get wallet state (read-only)
    pub fn get_wallet_state(&self) -> Result<WalletState> {
        let state = self.wallet_state.read();
        state.clone().ok_or(WalletError::WalletNotInitialized)
    }

    /// Get mutable wallet state
    /// Update wallet state
    pub fn update_wallet_state<F>(&self, updater: F) -> Result<()>
    where
        F: FnOnce(&mut WalletState) -> Result<()>,
    {
        let mut state = self.wallet_state.write();
        if let Some(ref mut wallet_state) = *state {
            updater(wallet_state)?;
            self.save_wallet()?;
        }
        Ok(())
    }

    /// Create a backup of the wallet
    pub fn create_backup(&self) -> Result<PathBuf> {
        let state = self.get_wallet_state()?;
        let backup = WalletBackup::new(state);

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let backup_file = self
            .backup_path
            .join(format!("wallet_backup_{}.json", timestamp));

        let data = serde_json::to_string_pretty(&backup)
            .map_err(|e| WalletError::StorageError(format!("Failed to serialize backup: {}", e)))?;

        fs::write(&backup_file, data)
            .map_err(|e| WalletError::StorageError(format!("Failed to write backup: {}", e)))?;

        // Update last backup time
        self.update_wallet_state(|state| {
            state.config.last_backup = Some(chrono::Utc::now());
            Ok(())
        })?;

        Ok(backup_file)
    }

    /// Restore wallet from backup
    pub fn restore_from_backup(&self, backup_path: &Path, password: Option<&str>) -> Result<()> {
        let data = fs::read_to_string(backup_path)
            .map_err(|e| WalletError::StorageError(format!("Failed to read backup: {}", e)))?;

        let backup: WalletBackup = serde_json::from_str(&data)
            .map_err(|e| WalletError::StorageError(format!("Failed to parse backup: {}", e)))?;

        if !backup.verify_checksum() {
            return Err(WalletError::StorageError(
                "Backup checksum verification failed".to_string(),
            ));
        }

        // Verify password if encryption is enabled
        if backup.wallet_state.config.encryption_enabled {
            if let Some(pwd) = password {
                if let Some(hash) = self.password_hash.read().as_ref() {
                    if !verify_password(pwd, hash)? {
                        return Err(WalletError::InvalidPassword);
                    }
                }
            } else {
                return Err(WalletError::WalletLocked("Password required".to_string()));
            }
        }

        *self.wallet_state.write() = Some(backup.wallet_state);
        self.save_wallet()?;
        Ok(())
    }

    /// List available backups
    pub fn list_backups(&self) -> Result<Vec<PathBuf>> {
        let mut backups = Vec::new();

        if self.backup_path.exists() {
            let entries = fs::read_dir(&self.backup_path).map_err(|e| {
                WalletError::StorageError(format!("Failed to read backup directory: {}", e))
            })?;

            for entry in entries {
                let entry = entry.map_err(|e| {
                    WalletError::StorageError(format!("Failed to read backup entry: {}", e))
                })?;
                let path = entry.path();

                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    backups.push(path);
                }
            }
        }

        // Sort by modification time (newest first)
        backups.sort_by(|a, b| {
            b.metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::UNIX_EPOCH)
                .cmp(
                    &a.metadata()
                        .and_then(|m| m.modified())
                        .unwrap_or(std::time::UNIX_EPOCH),
                )
        });

        Ok(backups)
    }

    /// Export wallet data (for migration or external use)
    pub fn export_wallet(&self) -> Result<WalletBackup> {
        let state = self.get_wallet_state()?;
        Ok(WalletBackup::new(state))
    }

    /// Import wallet data
    pub fn import_wallet(&self, backup: WalletBackup, password: Option<&str>) -> Result<()> {
        if !backup.verify_checksum() {
            return Err(WalletError::StorageError(
                "Import data checksum verification failed".to_string(),
            ));
        }

        // Verify password if encryption is enabled
        if backup.wallet_state.config.encryption_enabled {
            if let Some(pwd) = password {
                if let Some(hash) = self.password_hash.read().as_ref() {
                    if !verify_password(pwd, hash)? {
                        return Err(WalletError::InvalidPassword);
                    }
                }
            } else {
                return Err(WalletError::WalletLocked("Password required".to_string()));
            }
        }

        *self.wallet_state.write() = Some(backup.wallet_state);
        self.save_wallet()?;
        Ok(())
    }

    /// Check if wallet exists
    pub fn wallet_exists(&self) -> bool {
        self.wallet_path.exists()
    }

    /// Get wallet file path
    pub fn wallet_path(&self) -> &Path {
        &self.wallet_path
    }

    /// Get backup directory path
    pub fn backup_path(&self) -> &Path {
        &self.backup_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_wallet_storage_creation() {
        let temp_dir = tempdir().unwrap();
        let storage = WalletStorage::new(temp_dir.path());

        assert!(!storage.wallet_exists());
    }

    #[test]
    fn test_wallet_initialization() {
        let temp_dir = tempdir().unwrap();
        let storage = WalletStorage::new(temp_dir.path());

        storage
            .initialize("Test Wallet".to_string(), Some("password123"))
            .unwrap();
        assert!(storage.wallet_exists());
    }

    #[test]
    fn test_wallet_backup_restore() {
        let temp_dir = tempdir().unwrap();
        let storage = WalletStorage::new(temp_dir.path());

        // Initialize wallet
        storage
            .initialize("Test Wallet".to_string(), Some("password123"))
            .unwrap();

        // Create backup
        let backup_path = storage.create_backup().unwrap();
        assert!(backup_path.exists());

        // Restore from backup
        storage
            .restore_from_backup(&backup_path, Some("password123"))
            .unwrap();

        let state = storage.get_wallet_state().unwrap();
        assert_eq!(state.config.name, "Test Wallet");
    }
}
