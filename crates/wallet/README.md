# IPPAN Multi-Address Wallet

A comprehensive wallet implementation for managing multiple IPPAN addresses, private keys, and transactions with secure encryption and storage.

## Features

### 🔐 Security
- **Password Protection**: Optional password encryption for private keys
- **AES-GCM Encryption**: Industry-standard encryption for sensitive data
- **Argon2 Key Derivation**: Secure password hashing
- **Secure Storage**: Encrypted wallet files with atomic writes

### 📍 Address Management
- **Multiple Addresses**: Generate and manage unlimited addresses
- **HD Wallet Support**: Hierarchical deterministic address generation
- **Address Labels**: Organize addresses with custom labels
- **Address Validation**: Built-in IPPAN address format validation

### 💰 Transaction Management
- **Send Transactions**: Send funds between addresses
- **Transaction History**: Track all wallet transactions
- **Balance Tracking**: Real-time balance updates
- **Nonce Management**: Automatic transaction nonce handling

### 🔄 Backup & Recovery
- **Automatic Backups**: Scheduled wallet backups
- **Export/Import**: Full wallet data export and import
- **Checksum Verification**: Data integrity validation
- **Multiple Backup Formats**: JSON-based backup system

### 🖥️ CLI Interface
- **Command-Line Tool**: Full-featured CLI for wallet management
- **Interactive Prompts**: User-friendly password and input prompts
- **Verbose Output**: Detailed operation logging
- **Help System**: Comprehensive command documentation

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
ippan_wallet = { path = "crates/wallet" }
```

Or install the CLI tool:

```bash
cargo install --path crates/wallet
```

## Quick Start

### 1. Initialize a New Wallet

```rust
use ippan_wallet::*;
use std::sync::Arc;

let storage = Arc::new(storage::WalletStorage::new("./wallet"));
let wallet = operations::WalletManager::new(storage, None);

// Create wallet with password protection
wallet.create_wallet("My IPPAN Wallet".to_string(), Some("secure_password"))?;
```

### 2. Generate Addresses

```rust
// Generate a single address
let address = wallet.generate_address(
    Some("My First Address".to_string()), 
    Some("secure_password")
)?;

// Generate multiple addresses
let addresses = wallet.generate_addresses(
    5, 
    Some("Batch".to_string()), 
    Some("secure_password")
)?;
```

### 3. Send Transactions

```rust
// Send funds between addresses
let tx_hash = wallet.send_transaction(
    &from_address,
    &to_address,
    1000, // amount in atomic units
    Some("secure_password")
)?;
```

### 4. Check Balances

```rust
// Get specific address balance
let balance = wallet.get_address_balance(&address)?;

// Get total wallet balance
let total_balance = wallet.get_total_balance()?;
```

## CLI Usage

The revamped `ippan-wallet` CLI focuses on four core tasks: key generation,
address inspection, signing, and payments.

### 1. Generate a password-protected key

```bash
# Create an encrypted key file for devnet and store it under ./keys
ippan-wallet \
  --network devnet \
  generate-key \
  --out ./keys/devnet.key \
  --prompt-password
```

To generate a plaintext key (not recommended outside automated tests), pass
`--insecure-plaintext`.

### 2. Show the derived address

```bash
ippan-wallet show-address --key ./keys/devnet.key --prompt-password --json
```

### 3. Sign payloads or files

```bash
# Sign an inline message
ippan-wallet sign \
  --key ./keys/devnet.key \
  --prompt-password \
  --message "hello ipn" \
  --out signature.hex

# Sign raw bytes from a file
ippan-wallet sign \
  --key ./keys/devnet.key \
  --prompt-password \
  --file unsigned_tx.bin
```

### 4. Send a payment through the node RPC

```bash
ippan-wallet --rpc-url http://127.0.0.1:18080 send-payment \
  --key ./keys/devnet.key \
  --prompt-password \
  --to ippan1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx00d \
  --amount 0.5 \
  --memo "coffee"
```

The CLI automatically fetches the next nonce, signs the payment, and submits it
to `/tx/payment`. Use `--yes` to skip the confirmation prompt or `--fee` /
`--fee-atomic` to set a fee cap.

## API Reference

### WalletManager

The main wallet operations manager.

```rust
pub struct WalletManager {
    // ... internal fields
}

impl WalletManager {
    // Create new wallet
    pub fn create_wallet(&self, name: String, password: Option<&str>) -> Result<()>
    
    // Load existing wallet
    pub fn load_wallet(&self, password: Option<&str>) -> Result<()>
    
    // Generate new address
    pub fn generate_address(&self, label: Option<String>, password: Option<&str>) -> Result<String>
    
    // Generate multiple addresses
    pub fn generate_addresses(&self, count: usize, label_prefix: Option<String>, password: Option<&str>) -> Result<Vec<String>>
    
    // List all addresses
    pub fn list_addresses(&self) -> Result<Vec<&WalletAddress>>
    
    // Get address balance
    pub fn get_address_balance(&self, address: &str) -> Result<u64>
    
    // Send transaction
    pub fn send_transaction(&self, from: &str, to: &str, amount: u64, password: Option<&str>) -> Result<String>
    
    // Get transaction history
    pub fn get_address_transactions(&self, address: &str) -> Result<Vec<WalletTransaction>>
    
    // Create backup
    pub fn create_backup(&self) -> Result<PathBuf>
    
    // Restore from backup
    pub fn restore_from_backup(&self, backup_path: &Path, password: Option<&str>) -> Result<()>
}
```

### WalletAddress

Represents a single address in the wallet.

```rust
pub struct WalletAddress {
    pub id: Uuid,
    pub address: String,
    pub encrypted_private_key: EncryptedKey,
    pub label: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    pub balance: u64,
    pub nonce: u64,
}
```

### WalletTransaction

Represents a transaction in the wallet.

```rust
pub struct WalletTransaction {
    pub id: Uuid,
    pub tx_hash: String,
    pub from_address: Option<String>,
    pub to_address: Option<String>,
    pub amount: u64,
    pub fee: u64,
    pub timestamp: DateTime<Utc>,
    pub status: TransactionStatus,
    pub label: Option<String>,
}
```

## Security Considerations

### Password Security
- Use strong, unique passwords for wallet encryption
- Never share your wallet password
- Consider using a password manager

### Backup Security
- Store backups in secure, encrypted locations
- Test backup restoration regularly
- Keep multiple backup copies in different locations

### Private Key Security
- Private keys are encrypted with AES-GCM
- Keys are derived using Argon2 for additional security
- Never share private keys or seed phrases

## Error Handling

The wallet uses a comprehensive error handling system:

```rust
pub enum WalletError {
    AddressNotFound(String),
    InvalidAddress(String),
    InsufficientBalance { required: u64, available: u64 },
    InvalidPrivateKey(String),
    EncryptionError(String),
    DecryptionError(String),
    StorageError(String),
    TransactionError(String),
    WalletLocked(String),
    InvalidPassword,
    WalletNotInitialized,
    // ... more error types
}
```

## Testing

Run the test suite:

```bash
cargo test
```

Run integration tests:

```bash
cargo test --test integration_tests
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Submit a pull request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Support

For support and questions:
- Create an issue on GitHub
- Check the documentation
- Review the test cases for usage examples