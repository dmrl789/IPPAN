# AI Model Governance

This document describes the governance system for AI models in the IPPAN blockchain, including proposal submission, voting, and model activation.

## Overview

The AI Model Governance system allows validators and stakeholders to propose, vote on, and activate new AI models for use in consensus. This ensures that model updates are transparent, democratic, and secure.

## Key Components

### 1. Model Registry

The `ModelRegistry` maintains a record of all AI models, their status, and activation schedules:

- **Proposed**: Model is submitted but not yet activated
- **Active**: Model is currently being used in consensus
- **Deprecated**: Model is marked for deactivation
- **Deactivated**: Model is no longer used

### 2. Proposal System

The `ProposalManager` handles AI model proposals:

- Validates proposal signatures
- Manages voting sessions
- Executes approved proposals
- Tracks proposal status

### 3. Activation Manager

The `ActivationManager` schedules model activations:

- Manages activation rounds
- Processes model activations/deactivations
- Maintains activation schedule

## Proposal Process

### 1. Submit Proposal

Create a proposal in JSON or YAML format:

```json
{
  "proposal_id": "unique_proposal_id",
  "model_id": "model_identifier",
  "version": 1,
  "model_url": "https://example.com/model.json",
  "model_hash": "sha256_hash_here",
  "signature": "ed25519_signature_here",
  "signer_pubkey": "ed25519_public_key_here",
  "activation_round": 1000,
  "description": "Description of the model and its purpose",
  "proposer": "proposer_address_here",
  "created_at": 1234567890,
  "metadata": {
    "category": "reputation_scoring",
    "performance_metrics": "accuracy: 0.95, precision: 0.93",
    "training_data": "historical_validator_data"
  }
}
```

### 2. Validation

The system validates:
- Proposal signature
- Model hash integrity
- Activation round is in the future
- Required fields are present

### 3. Voting

Once validated, the proposal enters voting:
- Voting duration: 7 days (configurable)
- Voting threshold: 67% of stake (configurable)
- Stake-weighted voting

### 4. Execution

If approved, the proposal is executed:
- Model is added to registry
- Activation is scheduled
- Model becomes active at specified round

## Model Requirements

### 1. Format

Models must be in JSON format with the following structure:

```json
{
  "version": 1,
  "feature_count": 8,
  "bias": 100,
  "scale": 10000,
  "trees": [...],
  "metadata": {...}
}
```

### 2. Validation

Models must pass validation:
- Valid tree structure
- Correct feature count
- No invalid references
- Deterministic evaluation

### 3. Signing

Models must be signed by authorized signers:
- Ed25519 signature
- Signature covers model hash
- Public key must be authorized

## Security Considerations

### 1. Model Integrity

- SHA-256 hash verification
- Signature validation
- Deterministic evaluation

### 2. Access Control

- Only authorized signers can propose models
- Minimum stake requirements
- Voting power based on stake

### 3. Activation Safety

- Models activate at specified rounds
- Old models remain valid until deactivation
- Gradual rollout possible

## Configuration

### Default Parameters

- Minimum proposal stake: 1,000,000 tokens
- Voting threshold: 67%
- Voting duration: 7 days
- Maximum active proposals: 10

### Updating Parameters

Parameters can be updated through governance proposals:
- Submit parameter change proposal
- Vote on the change
- Execute if approved

## Examples

### Submitting a Model Proposal

```rust
use ippan_governance::ai_models::*;

let proposal = AiModelProposal {
    proposal_id: "reputation_v2".to_string(),
    model_id: "reputation_v2".to_string(),
    version: 2,
    model_url: "https://models.ippan.org/reputation_v2.json".to_string(),
    model_hash: [0u8; 32], // Actual hash
    signature: [0u8; 64], // Actual signature
    signer_pubkey: [0u8; 32], // Actual public key
    activation_round: 1000,
    description: "Improved reputation scoring model".to_string(),
    proposer: [0u8; 32], // Proposer address
    created_at: 1234567890,
    metadata: HashMap::new(),
};

governance.submit_model_proposal(proposal, 2000000)?;
```

### Processing Rounds

```rust
// Process model activations for current round
let activated_models = governance.process_round(current_round)?;

for model_id in activated_models {
    println!("Activated model: {}", model_id);
}
```

## Best Practices

### 1. Model Development

- Test models thoroughly before submission
- Use deterministic evaluation
- Document performance metrics
- Include comprehensive metadata

### 2. Proposal Submission

- Use descriptive proposal IDs
- Provide clear justifications
- Set appropriate activation rounds
- Include all required metadata

### 3. Voting

- Review model details before voting
- Consider security implications
- Vote based on technical merit
- Participate in governance discussions

## Troubleshooting

### Common Issues

1. **Invalid Signature**: Ensure model is properly signed
2. **Hash Mismatch**: Verify model file integrity
3. **Invalid Format**: Check JSON structure and validation
4. **Insufficient Stake**: Ensure minimum stake requirements

### Getting Help

- Check logs for detailed error messages
- Review model validation output
- Consult governance documentation
- Contact validator community