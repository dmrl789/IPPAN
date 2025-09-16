//! IPPAN Address validation and management
//! 
//! Handles address format validation, generation, and verification

use serde::{Deserialize, Serialize};
use std::fmt;
use regex::Regex;
use crate::{Result, IppanError};

/// IPPAN Address type
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Address(String);

impl Address {
    /// Create a new address from string
    pub fn new(address: &str) -> Result<Self> {
        let addr = Self(address.to_string());
        addr.validate()?;
        Ok(addr)
    }
    
    /// Validate the address format
    pub fn validate(&self) -> Result<()> {
        // IPPAN address format: starts with 'i' followed by base58 characters, 64 characters total
        let address_pattern = Regex::new(r"^i[1-9A-HJ-NP-Za-km-z]{63}$")
            .map_err(|_| IppanError::Validation("Invalid address regex pattern".to_string()))?;
        
        if self.0.len() != 64 {
            return Err(IppanError::Validation(
                format!("Address must be 64 characters long, got {}", self.0.len())
            ));
        }
        
        if !self.0.starts_with('i') {
            return Err(IppanError::Validation(
                "Address must start with 'i'".to_string()
            ));
        }
        
        if !address_pattern.is_match(&self.0) {
            return Err(IppanError::Validation(
                "Address contains invalid characters (must be base58)".to_string()
            ));
        }
        
        Ok(())
    }
    
    /// Get the address as string
    pub fn as_str(&self) -> &str {
        &self.0
    }
    
    /// Check if address is valid format
    pub fn is_valid_format(address: &str) -> bool {
        Self::new(address).is_ok()
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Address> for String {
    fn from(addr: Address) -> Self {
        addr.0
    }
}

impl AsRef<str> for Address {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_valid_address() {
        let valid_addr = "iRbDqSo0H4NxPGC0q55ohG36JrvlcYGvM3DpS4Q";
        let addr = Address::new(valid_addr);
        assert!(addr.is_ok());
    }
    
    #[test]
    fn test_invalid_address_length() {
        let invalid_addr = "iRbDqSo0H4NxPGC0q55ohG36JrvlcYGvM3DpS4"; // Too short
        let addr = Address::new(invalid_addr);
        assert!(addr.is_err());
    }
    
    #[test]
    fn test_invalid_address_prefix() {
        let invalid_addr = "aRbDqSo0H4NxPGC0q55ohG36JrvlcYGvM3DpS4Q"; // Wrong prefix
        let addr = Address::new(invalid_addr);
        assert!(addr.is_err());
    }
    
    #[test]
    fn test_invalid_address_characters() {
        let invalid_addr = "iRbDqSo0H4NxPGC0q55ohG36JrvlcYGvM3DpS4O"; // Contains 'O' (invalid base58)
        let addr = Address::new(invalid_addr);
        assert!(addr.is_err());
    }
}
