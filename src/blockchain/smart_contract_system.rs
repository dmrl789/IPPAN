//! Smart Contract System for IPPAN
//! 
//! This module provides a capability-based WASM execution environment for programs.
//! Currently disabled by default - enable with the "contracts" feature flag.

use crate::transaction_types::Transaction;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// VM Host for executing smart contract programs
#[cfg_attr(not(feature = "contracts"), allow(dead_code))]
pub struct VmHost;

#[cfg(feature = "contracts")]
impl VmHost {
    /// Execute a program call transaction
    pub fn execute(tx: &Transaction, state: &mut dyn StateView) -> Result<(), VmError> {
        match tx {
            Transaction::ProgramCall { .. } => {
                // TODO: load program by id, instantiate deterministic WASM, bind capability syscalls
                // For now, reject until programs are enabled.
                Err(VmError::Disabled)
            }
            _ => Ok(()), // non-program txs handled elsewhere
        }
    }
}

/// VM Error types
#[derive(thiserror::Error, Debug)]
pub enum VmError {
    #[error("programs disabled")]
    Disabled,
    #[error("unknown program")]
    UnknownProgram,
    #[error("execution error: {0}")]
    Exec(String),
    #[error("capability denied")]
    CapabilityDenied,
    #[error("out of gas")]
    OutOfGas,
    #[error("memory limit exceeded")]
    MemoryLimitExceeded,
}

/// Trait for state access without committing to a specific engine
pub trait StateView {
    fn read(&self, key: &[u8]) -> Option<Vec<u8>>;
    fn write(&mut self, key: &[u8], val: &[u8]) -> Result<(), String>;
    fn has_capability(&self, cap: &CapabilityRef) -> bool;
    fn consume_gas(&mut self, amount: u64) -> Result<(), VmError>;
}

/// Capability reference for controlling program permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityRef {
    pub kind: u16,
    pub target: [u8; 32],
    pub permissions: Vec<String>,
}

/// Program metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramMetadata {
    pub id: [u8; 32],
    pub name: String,
    pub version: String,
    pub capabilities: Vec<CapabilityRef>,
    pub memory_limit: u32,
    pub gas_limit: u64,
    pub wasm_bytes: Vec<u8>,
}

/// Program registry for managing deployed programs
#[cfg_attr(not(feature = "contracts"), allow(dead_code))]
pub struct ProgramRegistry {
    programs: HashMap<[u8; 32], ProgramMetadata>,
}

#[cfg(feature = "contracts")]
impl ProgramRegistry {
    pub fn new() -> Self {
        Self {
            programs: HashMap::new(),
        }
    }

    pub fn register_program(&mut self, metadata: ProgramMetadata) -> Result<(), VmError> {
        self.programs.insert(metadata.id, metadata);
        Ok(())
    }

    pub fn get_program(&self, id: &[u8; 32]) -> Option<&ProgramMetadata> {
        self.programs.get(id)
    }

    pub fn list_programs(&self) -> Vec<&ProgramMetadata> {
        self.programs.values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_error_display() {
        let error = VmError::Disabled;
        assert_eq!(error.to_string(), "programs disabled");
    }

    #[test]
    fn test_capability_ref_creation() {
        let cap = CapabilityRef {
            kind: 1,
            target: [0u8; 32],
            permissions: vec!["read".to_string(), "write".to_string()],
        };
        assert_eq!(cap.permissions.len(), 2);
    }

    #[cfg(feature = "contracts")]
    #[test]
    fn test_program_registry() {
        let mut registry = ProgramRegistry::new();
        let metadata = ProgramMetadata {
            id: [1u8; 32],
            name: "test_program".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec![],
            memory_limit: 1024,
            gas_limit: 1000,
            wasm_bytes: vec![],
        };
        
        registry.register_program(metadata).unwrap();
        assert_eq!(registry.list_programs().len(), 1);
    }

    #[test]
    fn test_feature_flag_compilation() {
        // This test ensures the code compiles with contracts disabled
        let _vm_host = VmHost;
        
        // Verify that VmError is available even when contracts are disabled
        let error = VmError::Disabled;
        assert_eq!(error.to_string(), "programs disabled");
        
        // Verify that CapabilityRef is available
        let cap = CapabilityRef {
            kind: 1,
            target: [0u8; 32],
            permissions: vec!["read".to_string()],
        };
        assert_eq!(cap.kind, 1);
    }
} 