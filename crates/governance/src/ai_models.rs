use anyhow::Result;
use ippan_ai_registry::AiModelProposal;
use ippan_types::{ratio_from_parts, RatioMicros, RATIO_SCALE};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Model registry entry for governance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRegistryEntry {
    pub model_id: String,
    pub model_hash: [u8; 32],
    pub version: u32,
    pub activation_round: u64,
    #[serde(with = "serde_bytes")]
    pub signature: [u8; 64],
    pub registered_at_round: u64,
    pub model_url: String,
}

impl ModelRegistryEntry {
    pub fn new(
        model_id: String,
        model_hash: [u8; 32],
        version: u32,
        activation_round: u64,
        signature: [u8; 64],
        registered_at_round: u64,
        model_url: String,
    ) -> Self {
        Self {
            model_id,
            model_hash,
            version,
            activation_round,
            signature,
            registered_at_round,
            model_url,
        }
    }
}

// -----------------------------------------------------------------------------
// 🧩 Proposal Manager — handles voting and stake-based approval
// -----------------------------------------------------------------------------
pub struct ProposalManager {
    threshold_micros: RatioMicros,
    minimum_stake: u64,
    proposals: HashMap<String, ProposalState>,
}

#[derive(Debug, Clone)]
struct ProposalState {
    proposal: AiModelProposal,
    total_stake_for: u64,
    total_stake_against: u64,
    voters: HashMap<[u8; 32], Vote>,
    status: ProposalStatus,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct Vote {
    stake: u64,
    approve: bool,
}

#[derive(Debug, Clone, PartialEq)]
enum ProposalStatus {
    Pending,
    Voting,
    Approved,
    Rejected,
    Executed,
}

impl ProposalManager {
    pub fn new(threshold_micros: RatioMicros, minimum_stake: u64) -> Self {
        Self {
            threshold_micros: threshold_micros.min(RATIO_SCALE),
            minimum_stake,
            proposals: HashMap::new(),
        }
    }

    pub fn submit_proposal(&mut self, proposal: AiModelProposal, stake: u64) -> Result<()> {
        if stake < self.minimum_stake {
            return Err(anyhow::anyhow!(
                "Insufficient stake: {} < {}",
                stake,
                self.minimum_stake
            ));
        }

        if self.proposals.contains_key(&proposal.model_id) {
            return Err(anyhow::anyhow!(
                "Proposal already exists for model {}",
                proposal.model_id
            ));
        }

        self.proposals.insert(
            proposal.model_id.clone(),
            ProposalState {
                proposal,
                total_stake_for: 0,
                total_stake_against: 0,
                voters: HashMap::new(),
                status: ProposalStatus::Pending,
            },
        );
        Ok(())
    }

    pub fn start_voting(&mut self, id: &str) -> Result<()> {
        let state = self
            .proposals
            .get_mut(id)
            .ok_or_else(|| anyhow::anyhow!("Proposal not found: {id}"))?;

        if state.status != ProposalStatus::Pending {
            return Err(anyhow::anyhow!("Proposal is not in pending state"));
        }

        state.status = ProposalStatus::Voting;
        Ok(())
    }

    pub fn vote(&mut self, id: &str, voter: [u8; 32], stake: u64, approve: bool) -> Result<()> {
        let state = self
            .proposals
            .get_mut(id)
            .ok_or_else(|| anyhow::anyhow!("Proposal not found: {id}"))?;

        if state.status != ProposalStatus::Voting {
            return Err(anyhow::anyhow!("Proposal is not accepting votes"));
        }

        if state.voters.contains_key(&voter) {
            return Err(anyhow::anyhow!("Voter has already voted"));
        }

        if approve {
            state.total_stake_for += stake;
        } else {
            state.total_stake_against += stake;
        }

        state.voters.insert(voter, Vote { stake, approve });

        // Check threshold
        let total_stake = state.total_stake_for + state.total_stake_against;
        if total_stake > 0 {
            let approval_ratio =
                ratio_from_parts(state.total_stake_for as u128, total_stake as u128);
            if approval_ratio >= self.threshold_micros {
                state.status = ProposalStatus::Approved;
            } else if approval_ratio < RATIO_SCALE.saturating_sub(self.threshold_micros) {
                state.status = ProposalStatus::Rejected;
            }
        }

        Ok(())
    }

    pub fn execute_proposal(&mut self, id: &str) -> Result<ModelRegistryEntry> {
        let state = self
            .proposals
            .get_mut(id)
            .ok_or_else(|| anyhow::anyhow!("Proposal not found: {id}"))?;

        if state.status != ProposalStatus::Approved {
            return Err(anyhow::anyhow!("Proposal is not approved for execution"));
        }

        let entry = ModelRegistryEntry::new(
            state.proposal.model_id.clone(),
            state.proposal.model_hash,
            state.proposal.version,
            state.proposal.activation_round,
            state.proposal.signature,
            0,
            state.proposal.model_url.clone(),
        );

        state.status = ProposalStatus::Executed;
        Ok(entry)
    }
}

// -----------------------------------------------------------------------------
// 🗂 Model Registry — activated models and round scheduling
// -----------------------------------------------------------------------------
pub struct ModelRegistry {
    models: HashMap<String, RegistryState>,
    active_models: HashMap<u64, Vec<String>>,
}

#[derive(Debug, Clone)]
struct RegistryState {
    entry: ModelRegistryEntry,
    activated: bool,
    activation_round: Option<u64>,
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelRegistry {
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
            active_models: HashMap::new(),
        }
    }

    pub fn register_model(&mut self, entry: ModelRegistryEntry) -> Result<()> {
        if self.models.contains_key(&entry.model_id) {
            return Err(anyhow::anyhow!(
                "Model already registered: {}",
                entry.model_id
            ));
        }

        self.models.insert(
            entry.model_id.clone(),
            RegistryState {
                entry,
                activated: false,
                activation_round: None,
            },
        );
        Ok(())
    }

    pub fn activate_model(&mut self, id: &str, round: u64) -> Result<()> {
        let state = self
            .models
            .get_mut(id)
            .ok_or_else(|| anyhow::anyhow!("Model not found: {id}"))?;

        if state.activated {
            return Err(anyhow::anyhow!("Model already activated"));
        }

        state.activated = true;
        state.activation_round = Some(round);

        self.active_models
            .entry(round)
            .or_default()
            .push(id.to_string());

        Ok(())
    }

    pub fn get_model(&self, id: &str) -> Option<ModelRegistryEntry> {
        self.models.get(id).map(|s| s.entry.clone())
    }

    pub fn list_active_models(&self, round: u64) -> Vec<String> {
        self.active_models
            .iter()
            .filter(|(r, _)| **r <= round)
            .flat_map(|(_, m)| m.clone())
            .collect()
    }
}

// -----------------------------------------------------------------------------
// ⚙️ Activation Manager — schedules and triggers model activation by round
// -----------------------------------------------------------------------------
pub struct ActivationManager {
    pending_activations: HashMap<u64, Vec<String>>,
    activated_models: HashMap<String, u64>,
}

impl Default for ActivationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ActivationManager {
    pub fn new() -> Self {
        Self {
            pending_activations: HashMap::new(),
            activated_models: HashMap::new(),
        }
    }

    pub fn schedule_activation(&mut self, model_id: String, round: u64) -> Result<()> {
        if self.activated_models.contains_key(&model_id) {
            return Err(anyhow::anyhow!("Model already activated: {model_id}"));
        }

        self.pending_activations
            .entry(round)
            .or_default()
            .push(model_id);

        Ok(())
    }

    pub fn process_round(&mut self, round: u64) -> Result<Vec<String>> {
        let mut activated = Vec::new();
        let rounds_to_process: Vec<u64> = self
            .pending_activations
            .keys()
            .filter(|&&r| r <= round)
            .copied()
            .collect();

        for r in rounds_to_process {
            if let Some(models) = self.pending_activations.remove(&r) {
                for model_id in models {
                    self.activated_models.insert(model_id.clone(), r);
                    activated.push(model_id);
                }
            }
        }

        Ok(activated)
    }

    pub fn is_active(&self, model_id: &str) -> bool {
        self.activated_models.contains_key(model_id)
    }

    pub fn get_activation_round(&self, model_id: &str) -> Option<u64> {
        self.activated_models.get(model_id).copied()
    }
}

// -----------------------------------------------------------------------------
// 🧠 AiModelGovernance — foundation-level governance over AI models
// -----------------------------------------------------------------------------
pub struct AiModelGovernance {
    model_registry: HashMap<String, ModelRegistryEntry>,
    active_proposals: HashMap<String, AiModelProposal>,
}

impl AiModelGovernance {
    pub fn new() -> Self {
        Self {
            model_registry: HashMap::new(),
            active_proposals: HashMap::new(),
        }
    }

    pub fn submit_proposal(&mut self, proposal: AiModelProposal) -> Result<()> {
        if proposal.model_id.is_empty() {
            return Err(anyhow::anyhow!("Model ID cannot be empty"));
        }
        if proposal.description.is_empty() {
            return Err(anyhow::anyhow!("Rationale cannot be empty"));
        }
        if proposal.activation_round == 0 {
            return Err(anyhow::anyhow!("Activation round must be > 0"));
        }

        let model_id = proposal.model_id.clone();
        self.active_proposals.insert(model_id, proposal);
        Ok(())
    }

    pub fn get_active_proposals(&self) -> &HashMap<String, AiModelProposal> {
        &self.active_proposals
    }

    pub fn get_proposal(&self, model_id: &str) -> Option<&AiModelProposal> {
        self.active_proposals.get(model_id)
    }

    pub fn approve_proposal(&mut self, model_id: &str, round: u64) -> Result<()> {
        if let Some(proposal) = self.active_proposals.remove(model_id) {
            let entry = ModelRegistryEntry::new(
                proposal.model_id.clone(),
                proposal.model_hash,
                proposal.version,
                proposal.activation_round,
                proposal.signature,
                round,
                proposal.model_url,
            );
            self.model_registry.insert(model_id.to_string(), entry);
        }
        Ok(())
    }

    pub fn reject_proposal(&mut self, model_id: &str) -> Result<()> {
        self.active_proposals.remove(model_id);
        Ok(())
    }

    pub fn get_registered_models(&self) -> &HashMap<String, ModelRegistryEntry> {
        &self.model_registry
    }

    pub fn get_registered_model(&self, model_id: &str) -> Option<&ModelRegistryEntry> {
        self.model_registry.get(model_id)
    }

    pub fn is_model_registered(&self, model_id: &str) -> bool {
        self.model_registry.contains_key(model_id)
    }

    pub fn active_proposal_count(&self) -> usize {
        self.active_proposals.len()
    }

    pub fn registered_model_count(&self) -> usize {
        self.model_registry.len()
    }
}

impl Default for AiModelGovernance {
    fn default() -> Self {
        Self::new()
    }
}

// -----------------------------------------------------------------------------
// 📜 Templates & Parsers
// -----------------------------------------------------------------------------
pub const AI_MODEL_PROPOSAL_TEMPLATE: &str = r#"{
  "proposal_id": "proposal-001",
  "model_id": "model_identifier",
  "version": 1,
  "model_url": "https://example.com/model.json",
  "model_hash": [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
  "signature": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
  "signer_pubkey": [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
  "activation_round": 1000,
  "description": "Description of the model and its purpose",
  "proposer": [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
  "created_at": 1700000000,
  "metadata": {}
}"#;

pub const AI_MODEL_PROPOSAL_YAML_TEMPLATE: &str = r#"---
proposal_id: "proposal-001"
model_id: "model_identifier"
version: 1
model_url: "https://example.com/model.json"
model_hash:
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
signature: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
signer_pubkey:
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
activation_round: 1000
description: "Description of the model and its purpose"
proposer:
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
  - 0
created_at: 1700000000
metadata: {}
"#;

pub fn parse_json_proposal(json: &str) -> Result<AiModelProposal> {
    let proposal: AiModelProposal = serde_json::from_str(json)?;
    Ok(proposal)
}

pub fn parse_yaml_proposal(yaml: &str) -> Result<AiModelProposal> {
    let proposal: AiModelProposal = serde_yaml::from_str(yaml)?;
    Ok(proposal)
}

pub fn validate_proposal_format(proposal: &AiModelProposal) -> Result<()> {
    if proposal.model_id.is_empty() {
        return Err(anyhow::anyhow!("Model ID cannot be empty"));
    }
    if proposal.model_url.is_empty() {
        return Err(anyhow::anyhow!("Model URL cannot be empty"));
    }
    if proposal.description.is_empty() {
        return Err(anyhow::anyhow!("Rationale cannot be empty"));
    }
    if proposal.activation_round == 0 {
        return Err(anyhow::anyhow!("Activation round must be greater than 0"));
    }
    Ok(())
}

// -----------------------------------------------------------------------------
// ✅ Tests
// -----------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_proposal(model_id: &str) -> AiModelProposal {
        AiModelProposal {
            proposal_id: format!("proposal-{}", model_id),
            model_id: model_id.to_string(),
            version: 1,
            model_url: "https://example.com/model".to_string(),
            model_hash: [0u8; 32],
            signature: [0u8; 64],
            signer_pubkey: [1u8; 32],
            activation_round: 100,
            description: "Test proposal".to_string(),
            proposer: [2u8; 32],
            created_at: 1_234_567_890,
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn test_governance_flow() {
        let mut gov = AiModelGovernance::new();
        let proposal = create_test_proposal("m1");

        gov.submit_proposal(proposal).unwrap();
        assert_eq!(gov.active_proposal_count(), 1);

        gov.approve_proposal("m1", 1000).unwrap();
        assert_eq!(gov.registered_model_count(), 1);
        assert!(gov.is_model_registered("m1"));
    }
}
