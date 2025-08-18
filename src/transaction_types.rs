//! User-facing transaction types for IPPAN
//! 
//! This module contains all 23 canonical transaction types that users can submit
//! through the CLI or API. Each transaction type includes validation rules,
//! fee calculation, and proper serialization.

use crate::{Result, IppanError};
use serde::{Deserialize, Serialize};

// ============================================================================
// FEE CALCULATION HELPERS (PRD-aligned)
// ============================================================================

/// Calculate 1% fee on amount in smallest units (u64)
/// Returns fee in units, minimum 1 unit (dust guard)
pub fn calc_fee_1pct(amount_units: u64) -> u64 {
    let one_pct = amount_units.saturating_mul(1) / 100; // floor division
    one_pct.max(1) // dust guard: minimum 1 unit
}

/// Parse amount string to smallest units (1 IPN = 1e8 units)
pub fn parse_amount_to_units(amount_str: &str) -> u64 {
    if let Ok(amount) = amount_str.parse::<f64>() {
        (amount * 100_000_000.0) as u64 // 1 IPN = 100,000,000 units
    } else {
        0
    }
}

/// Format units back to IPN string
pub fn format_units_to_ipn(units: u64) -> String {
    let ipn = units as f64 / 100_000_000.0;
    format!("{:.8}", ipn)
}

// ============================================================================
// PAYMENTS
// ============================================================================

/// Standard payment transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PayTransaction {
    /// Sender address or handle
    pub from: String,
    /// Recipient address or handle
    pub to: String,
    /// Amount in IPN (decimal string or nano integer)
    pub amount_ipn: String,
    /// Optional memo (≤128 bytes)
    pub memo: Option<String>,
    /// Transaction fee in IPN
    pub fee: String,
    /// Transaction signature
    pub sig: String,
}

/// Batch payment transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PayBatchTransaction {
    /// Sender address
    pub from: String,
    /// Payment items
    pub items: Vec<PaymentItem>,
    /// Transaction fee in IPN
    pub fee: String,
    /// Transaction signature
    pub sig: String,
}

/// Payment item for batch transactions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PaymentItem {
    /// Recipient address or handle
    pub to: String,
    /// Amount in IPN
    pub amount_ipn: String,
    /// Optional memo
    pub memo: Option<String>,
}

/// Invoice creation transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct InvoiceCreateTransaction {
    /// Recipient address or handle
    pub to: String,
    /// Amount in IPN
    pub amount_ipn: String,
    /// Optional reference
    pub reference: Option<String>,
    /// Optional expiration timestamp in microseconds
    pub expires_at_us: Option<u64>,
    /// Transaction signature
    pub sig: String,
}

// ============================================================================
// IDENTITY: HANDLES
// ============================================================================

/// Handle registration transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct HandleRegisterTransaction {
    /// Handle name (e.g., "@desiree.ipn")
    pub handle: String,
    /// Owner public key
    pub owner_pk: String,
    /// Registration years
    pub years: u32,
    /// Transaction fee (includes registration price)
    pub fee: String,
    /// Transaction signature
    pub sig: String,
}

/// Handle renewal transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct HandleRenewTransaction {
    /// Handle name
    pub handle: String,
    /// Renewal years
    pub years: u32,
    /// Transaction fee
    pub fee: String,
    /// Transaction signature
    pub sig: String,
}

/// Handle transfer transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct HandleTransferTransaction {
    /// Handle name
    pub handle: String,
    /// New owner public key
    pub new_owner_pk: String,
    /// Transaction fee
    pub fee: String,
    /// Transaction signature
    pub sig: String,
}

/// Handle update transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct HandleUpdateTransaction {
    /// Handle name
    pub handle: String,
    /// Nonce for replay protection
    pub nonce: u64,
    /// Update operations
    pub ops: Vec<HandleUpdateOp>,
    /// TTL in milliseconds
    pub ttl_ms: u64,
    /// Transaction fee
    pub fee: String,
    /// Transaction signature
    pub sig: String,
}

/// Handle update operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct HandleUpdateOp {
    /// Operation type
    pub op: String, // "SET", "PATCH", "UNSET"
    /// Field path
    pub path: String,
    /// Value (for SET/PATCH)
    pub value: Option<serde_json::Value>,
}

// ============================================================================
// DOMAINS & DNS
// ============================================================================

/// Domain registration transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DomainRegisterTransaction {
    /// Domain name (e.g., "example.ipn")
    pub domain: String,
    /// Owner public key
    pub owner_pk: String,
    /// Registration years
    pub years: u32,
    /// Plan type
    pub plan: String, // "standard" or "premium"
    /// Transaction fee (includes registration price)
    pub fee: String,
    /// Transaction signature
    pub sig: String,
}

/// Domain renewal transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DomainRenewTransaction {
    /// Domain name
    pub domain: String,
    /// Renewal years
    pub years: u32,
    /// Transaction fee
    pub fee: String,
    /// Transaction signature
    pub sig: String,
}

/// Domain transfer transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DomainTransferTransaction {
    /// Domain name
    pub domain: String,
    /// New owner public key
    pub new_owner_pk: String,
    /// Transaction fee
    pub fee: String,
    /// Transaction signature
    pub sig: String,
}

/// Zone update transaction (DNS records)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ZoneUpdateTransaction {
    /// Domain name
    pub domain: String,
    /// Nonce for replay protection
    pub nonce: u64,
    /// Zone operations
    pub ops: Vec<ZoneOp>,
    /// Update timestamp in microseconds
    pub updated_at_us: u64,
    /// Transaction fee in nano IPN
    pub fee_nano: u64,
    /// Transaction signature
    pub sig: String,
}

/// Zone operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ZoneOp {
    /// Operation type
    pub op: String, // "UPSERT_RRSET", "DELETE_RRSET", "PATCH_RECORDS"
    /// Record name (optional for some ops)
    pub name: Option<String>,
    /// Record type (optional for some ops)
    pub rtype: Option<String>,
    /// TTL in seconds (optional for some ops)
    pub ttl: Option<u32>,
    /// Record values (optional for some ops)
    pub records: Option<Vec<String>>,
}

// ============================================================================
// STORAGE / CDN & DHT
// ============================================================================

/// File publish transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FilePublishTransaction {
    /// Publisher address
    pub publisher: String,
    /// HashTimer content ID
    pub hash_timer: String,
    /// File size in bytes
    pub size_bytes: u64,
    /// MIME type
    pub mime: String,
    /// Target number of replicas
    pub replicas: u32,
    /// Storage plan
    pub storage_plan: String, // "free" or "paid"
    /// Transaction fee
    pub fee: String,
    /// Transaction signature
    pub sig: String,
}

/// File metadata update transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FileUpdateMetadataTransaction {
    /// HashTimer content ID
    pub hash_timer: String,
    /// Update operations
    pub ops: Vec<MetadataUpdateOp>,
    /// Transaction fee
    pub fee: String,
    /// Transaction signature
    pub sig: String,
}

/// Metadata update operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MetadataUpdateOp {
    /// Operation type
    pub op: String, // "SET", "PATCH", "UNSET"
    /// Field path
    pub path: String,
    /// Value (for SET/PATCH)
    pub value: Option<serde_json::Value>,
}

/// Storage rent topup transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct StorageRentTopupTransaction {
    /// HashTimer content ID
    pub hash_timer: String,
    /// Amount in IPN
    pub amount_ipn: String,
    /// Transaction fee
    pub fee: String,
    /// Transaction signature
    pub sig: String,
}

/// Pin request transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PinRequestTransaction {
    /// HashTimer content ID
    pub hash_timer: String,
    /// Number of replicas
    pub replicas: u32,
    /// Maximum price in IPN
    pub max_price_ipn: String,
    /// Transaction fee
    pub fee: String,
    /// Transaction signature
    pub sig: String,
}

// ============================================================================
// STAKING / NODE OPS
// ============================================================================

/// Stake bond transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct StakeBondTransaction {
    /// Validator public key
    pub validator_pk: String,
    /// Amount in IPN
    pub amount_ipn: String,
    /// Minimum lock period in days
    pub min_lock_days: u32,
    /// Transaction fee
    pub fee: String,
    /// Transaction signature
    pub sig: String,
}

/// Stake unbond transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct StakeUnbondTransaction {
    /// Validator public key
    pub validator_pk: String,
    /// Amount in IPN
    pub amount_ipn: String,
    /// Transaction fee
    pub fee: String,
    /// Transaction signature
    pub sig: String,
}

/// Stake withdraw transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct StakeWithdrawTransaction {
    /// Validator public key
    pub validator_pk: String,
    /// Transaction fee
    pub fee: String,
    /// Transaction signature
    pub sig: String,
}

// ============================================================================
// FAUCET / BOOTSTRAP
// ============================================================================

/// Faucet claim transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FaucetClaimTransaction {
    /// Recipient handle or address
    pub handle_or_addr: String,
    /// Uptime proof
    pub uptime_proof: String,
    /// Transaction fee (often zero)
    pub fee: String,
    /// Transaction signature
    pub sig: String,
}

// ============================================================================
// ACCOUNT MANAGEMENT
// ============================================================================

/// Key rotation transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct KeyRotateTransaction {
    /// Account address
    pub address: String,
    /// New owner public key
    pub new_owner_pk: String,
    /// Transaction fee
    pub fee: String,
    /// Transaction signature
    pub sig: String,
}

/// Set controllers transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SetControllersTransaction {
    /// Target type
    pub target_type: String, // "handle" or "domain"
    /// Target ID
    pub target_id: String,
    /// Controller public keys
    pub controllers: Vec<String>,
    /// Transaction fee
    pub fee: String,
    /// Transaction signature
    pub sig: String,
}

// ============================================================================
// GOVERNANCE / PROTOCOL
// ============================================================================

/// Governance vote transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct GovVoteTransaction {
    /// Proposal ID
    pub proposal_id: String,
    /// Vote choice
    pub choice: String, // "yes", "no", "abstain"
    /// Optional stake weight
    pub stake_weight: Option<String>,
    /// Transaction fee
    pub fee: String,
    /// Transaction signature
    pub sig: String,
}

// ============================================================================
// SERVICE PAYMENTS
// ============================================================================

/// Service payment transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ServicePayTransaction {
    /// Service ID
    pub service_id: String,
    /// Plan type
    pub plan: String,
    /// Amount in IPN
    pub amount_ipn: String,
    /// Optional period
    pub period: Option<String>,
    /// Transaction fee
    pub fee: String,
    /// Transaction signature
    pub sig: String,
}

// ============================================================================
// VALIDATION TRAITS AND IMPLEMENTATIONS
// ============================================================================

/// Trait for transaction validation
pub trait TransactionValidator {
    /// Validate the transaction
    fn validate(&self) -> Result<()>;
    
    /// Calculate the minimum fee for this transaction
    fn calculate_min_fee(&self) -> u64;
    
    /// Get the transaction type as a string
    fn transaction_type(&self) -> &'static str;
}

impl TransactionValidator for PayTransaction {
    fn validate(&self) -> Result<()> {
        // Validate from address
        if self.from.is_empty() {
            return Err(IppanError::Validation("From address cannot be empty".to_string()));
        }
        
        // Validate to address
        if self.to.is_empty() {
            return Err(IppanError::Validation("To address cannot be empty".to_string()));
        }
        
        // Validate amount
        if let Ok(amount) = self.amount_ipn.parse::<f64>() {
            if amount <= 0.0 {
                return Err(IppanError::Validation("Amount must be positive".to_string()));
            }
            if amount > 1_000_000_000.0 {
                return Err(IppanError::Validation("Amount cannot exceed 1 billion IPN".to_string()));
            }
        } else {
            return Err(IppanError::Validation("Invalid amount format".to_string()));
        }
        
        // Validate memo length
        if let Some(memo) = &self.memo {
            if memo.len() > 128 {
                return Err(IppanError::Validation("Memo cannot exceed 128 bytes".to_string()));
            }
        }
        
        // Validate signature format
        if !self.sig.starts_with("ed25519:") {
            return Err(IppanError::Validation("Signature must start with 'ed25519:'".to_string()));
        }
        
        Ok(())
    }
    
    fn calculate_min_fee(&self) -> u64 {
        // PRD Rule: 1% of transferred amount, minimum 1 unit (dust guard)
        let amount_units = parse_amount_to_units(&self.amount_ipn);
        calc_fee_1pct(amount_units)
    }
    
    fn transaction_type(&self) -> &'static str {
        "pay"
    }
}

impl TransactionValidator for ZoneUpdateTransaction {
    fn validate(&self) -> Result<()> {
        // Validate domain
        if self.domain.is_empty() {
            return Err(IppanError::Validation("Domain cannot be empty".to_string()));
        }
        
        // Validate operations
        if self.ops.is_empty() {
            return Err(IppanError::Validation("At least one operation required".to_string()));
        }
        
        // Validate each operation
        for op in &self.ops {
            match op.op.as_str() {
                "UPSERT_RRSET" => {
                    if op.name.is_none() || op.rtype.is_none() || op.ttl.is_none() || op.records.is_none() {
                        return Err(IppanError::Validation("UPSERT_RRSET requires name, rtype, ttl, and records".to_string()));
                    }
                }
                "DELETE_RRSET" => {
                    if op.name.is_none() || op.rtype.is_none() {
                        return Err(IppanError::Validation("DELETE_RRSET requires name and rtype".to_string()));
                    }
                }
                "PATCH_RECORDS" => {
                    if op.name.is_none() || op.rtype.is_none() || op.records.is_none() {
                        return Err(IppanError::Validation("PATCH_RECORDS requires name, rtype, and records".to_string()));
                    }
                }
                _ => {
                    return Err(IppanError::Validation(format!("Unknown operation type: {}", op.op)));
                }
            }
        }
        
        // Validate timestamp skew (within 5 seconds)
        let now_us = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;
        
        let skew = if now_us > self.updated_at_us {
            now_us - self.updated_at_us
        } else {
            self.updated_at_us - now_us
        };
        
        if skew > 5_000_000 { // 5 seconds in microseconds
            return Err(IppanError::Validation("Timestamp skew too large (max 5 seconds)".to_string()));
        }
        
        // Validate signature
        if self.sig.is_empty() {
            return Err(IppanError::Validation("Signature cannot be empty".to_string()));
        }
        
        Ok(())
    }
    
    fn calculate_min_fee(&self) -> u64 {
        // Base fee for zone updates
        let base_fee = 100; // 0.0000001 IPN in nano units
        
        // Operation-based fee
        let op_fee = self.ops.len() as u64 * 50;
        
        // Size-based fee
        let size = self.domain.len() + self.ops.iter().map(|op| {
            op.name.as_ref().map(|n| n.len()).unwrap_or(0) +
            op.rtype.as_ref().map(|r| r.len()).unwrap_or(0) +
            op.records.as_ref().map(|r| r.iter().map(|rec| rec.len()).sum::<usize>()).unwrap_or(0)
        }).sum::<usize>();
        
        let size_fee = (size as u64) * 2;
        
        base_fee + op_fee + size_fee
    }
    
    fn transaction_type(&self) -> &'static str {
        "zone_update"
    }
}

// Add validation implementations for other transaction types as needed...

/// Helper function to validate address format
pub fn validate_address(addr: &str) -> Result<()> {
    if addr.is_empty() {
        return Err(IppanError::Validation("Address cannot be empty".to_string()));
    }
    
    // Basic format validation (can be enhanced)
    if !addr.starts_with("i1") {
        return Err(IppanError::Validation("Address must start with 'i1'".to_string()));
    }
    
    if addr.len() < 3 || addr.len() > 100 {
        return Err(IppanError::Validation("Invalid address length".to_string()));
    }
    
    Ok(())
}

/// Helper function to validate handle format
pub fn validate_handle(handle: &str) -> Result<()> {
    if handle.is_empty() {
        return Err(IppanError::Validation("Handle cannot be empty".to_string()));
    }
    
    if !handle.starts_with('@') {
        return Err(IppanError::Validation("Handle must start with '@'".to_string()));
    }
    
    if !handle.ends_with(".ipn") {
        return Err(IppanError::Validation("Handle must end with '.ipn'".to_string()));
    }
    
    let name = &handle[1..handle.len()-4]; // Remove @ and .ipn
    if name.is_empty() {
        return Err(IppanError::Validation("Handle name cannot be empty".to_string()));
    }
    
    if name.len() > 63 {
        return Err(IppanError::Validation("Handle name too long (max 63 characters)".to_string()));
    }
    
    // Validate characters
    if !name.chars().all(|c| c.is_alphanumeric() || c == '-') {
        return Err(IppanError::Validation("Handle contains invalid characters".to_string()));
    }
    
    if name.starts_with('-') || name.ends_with('-') {
        return Err(IppanError::Validation("Handle cannot start or end with hyphen".to_string()));
    }
    
    Ok(())
}

/// Helper function to validate domain format
pub fn validate_domain(domain: &str) -> Result<()> {
    if domain.is_empty() {
        return Err(IppanError::Validation("Domain cannot be empty".to_string()));
    }
    
    if !domain.ends_with(".ipn") {
        return Err(IppanError::Validation("Domain must end with '.ipn'".to_string()));
    }
    
    let name = &domain[..domain.len()-4]; // Remove .ipn
    if name.is_empty() {
        return Err(IppanError::Validation("Domain name cannot be empty".to_string()));
    }
    
    if name.len() > 253 {
        return Err(IppanError::Validation("Domain too long (max 253 characters)".to_string()));
    }
    
    // Validate labels
    for label in name.split('.') {
        if label.is_empty() {
            return Err(IppanError::Validation("Empty label in domain".to_string()));
        }
        if label.len() > 63 {
            return Err(IppanError::Validation("Label too long (max 63 characters)".to_string()));
        }
        if !label.chars().all(|c| c.is_alphanumeric() || c == '-') {
            return Err(IppanError::Validation("Domain contains invalid characters".to_string()));
        }
        if label.starts_with('-') || label.ends_with('-') {
            return Err(IppanError::Validation("Label cannot start or end with hyphen".to_string()));
        }
    }
    
    Ok(())
}

/// Helper function to validate amount format
pub fn validate_amount(amount: &str) -> Result<f64> {
    if amount.is_empty() {
        return Err(IppanError::Validation("Amount cannot be empty".to_string()));
    }
    
    let amount = amount.parse::<f64>()
        .map_err(|_| IppanError::Validation("Invalid amount format".to_string()))?;
    
    if amount <= 0.0 {
        return Err(IppanError::Validation("Amount must be positive".to_string()));
    }
    
    if amount > 1_000_000_000.0 { // 1 billion IPN limit
        return Err(IppanError::Validation("Amount too large".to_string()));
    }
    
    Ok(amount)
}

/// Helper function to validate signature format
pub fn validate_signature(sig: &str) -> Result<()> {
    if sig.is_empty() {
        return Err(IppanError::Validation("Signature cannot be empty".to_string()));
    }
    
    if !sig.starts_with("ed25519:") {
        return Err(IppanError::Validation("Signature must start with 'ed25519:'".to_string()));
    }
    
    let sig_data = &sig[8..]; // Remove "ed25519:" prefix
    if sig_data.len() != 128 { // 64 bytes = 128 hex chars
        return Err(IppanError::Validation("Invalid signature length".to_string()));
    }
    
    // Validate hex format
    if !sig_data.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(IppanError::Validation("Invalid signature format".to_string()));
    }
    
    Ok(())
}

