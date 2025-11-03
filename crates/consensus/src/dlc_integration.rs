//! DLC Integration Layer
//!
//! Bridges the existing PoAConsensus with DLC components

use anyhow::Result;
use std::sync::Arc;
use parking_lot::RwLock;

use crate::{
    DLCConfig, DLCConsensus, DGBDTEngine, ShadowVerifierSet, BondingManager,
    ValidatorMetrics, PoAConsensus,
};
use ippan_types::ValidatorId;

/// Extended PoA consensus with DLC capabilities
pub struct DLCIntegratedConsensus {
    /// Base PoA consensus engine
    pub poa: PoAConsensus,
    
    /// DLC consensus engine
    pub dlc: Arc<RwLock<DLCConsensus>>,
    
    /// Whether DLC mode is enabled
    pub dlc_enabled: bool,
}

impl DLCIntegratedConsensus {
    /// Create a new DLC-integrated consensus engine
    pub fn new(poa: PoAConsensus, dlc_config: DLCConfig, validator_id: ValidatorId) -> Self {
        let dlc = DLCConsensus::new(dlc_config, validator_id);
        
        Self {
            poa,
            dlc: Arc::new(RwLock::new(dlc)),
            dlc_enabled: true,
        }
    }

    /// Start the integrated consensus engine
    pub async fn start(&mut self) -> Result<()> {
        // Start base PoA consensus
        self.poa.start().await?;
        
        // Start DLC consensus if enabled
        if self.dlc_enabled {
            let mut dlc = self.dlc.write();
            dlc.start().await?;
        }
        
        Ok(())
    }

    /// Stop the integrated consensus engine
    pub async fn stop(&mut self) -> Result<()> {
        self.poa.stop().await
    }

    /// Get DLC consensus reference
    pub fn get_dlc(&self) -> Arc<RwLock<DLCConsensus>> {
        self.dlc.clone()
    }

    /// Update validator metrics for D-GBDT
    pub fn update_validator_metrics(&self, validator_id: ValidatorId, metrics: ValidatorMetrics) {
        if self.dlc_enabled {
            let dlc = self.dlc.read();
            dlc.update_validator_metrics(validator_id, metrics);
        }
    }

    /// Add validator bond
    pub fn add_validator_bond(&self, validator_id: ValidatorId, amount: u64) -> Result<()> {
        if self.dlc_enabled {
            let dlc = self.dlc.read();
            dlc.add_validator_bond(validator_id, amount)
        } else {
            Ok(())
        }
    }
}

/// Helper to create DLC config from PoA config
pub fn dlc_config_from_poa(enable_dlc: bool, finality_ms: u64) -> DLCConfig {
    DLCConfig {
        temporal_finality_ms: finality_ms.clamp(100, 250),
        hashtimer_precision_us: 1,
        shadow_verifier_count: 3,
        min_reputation_score: 5000,
        max_transactions_per_block: 1000,
        enable_dgbdt_fairness: enable_dlc,
        enable_shadow_verifiers: enable_dlc,
        require_validator_bond: enable_dlc,
        dag_config: Default::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PoAConfig;
    use ippan_storage::SledStorage;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_dlc_integration() {
        let temp_dir = tempdir().unwrap();
        let storage = Arc::new(SledStorage::new(temp_dir.path().to_str().unwrap()).unwrap());
        storage.initialize().unwrap();
        
        let validator_id = [1u8; 32];
        let poa_config = PoAConfig::default();
        let poa = PoAConsensus::new(poa_config, storage, validator_id);
        
        let dlc_config = dlc_config_from_poa(true, 250);
        let integrated = DLCIntegratedConsensus::new(poa, dlc_config, validator_id);
        
        assert!(integrated.dlc_enabled);
    }
}
