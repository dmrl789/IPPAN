# IPPAN AI Registry

Production-grade AI model registry and governance system for the IPPAN blockchain.

## Overview

The AI Registry provides a comprehensive system for managing AI models on the IPPAN L1 blockchain, including:

- **Model Registration**: Secure registration of AI models with cryptographic verification
- **Governance**: Decentralized proposal and voting system for model approvals
- **Security**: Authentication, rate limiting, and input validation
- **Fee Management**: Configurable fee structures for various operations
- **Activation Management**: Round-based model activation scheduling
- **Storage**: Persistent storage with Sled database backend

## Features

### Core Features

- ✅ **Deterministic Model Registry**: On-chain verifiable model management
- ✅ **Cryptographic Verification**: Ed25519 signature verification for all models
- ✅ **Governance System**: Proposal creation, voting, and execution
- ✅ **Security Framework**: Rate limiting, authentication, and validation
- ✅ **Fee Management**: Multiple fee calculation methods (fixed, linear, logarithmic, step)
- ✅ **Activation Scheduling**: Round-based model lifecycle management
- ✅ **Persistent Storage**: Sled-based storage with in-memory caching
- ✅ **REST API**: Optional Axum-based API (enable `api` feature)

### Production Features

- 🔒 **Security**:
  - Token-based authentication
  - Rate limiting (configurable per user)
  - Input validation and sanitization
  - IP whitelisting support
  - Audit logging

- 📊 **Monitoring**:
  - Usage statistics per model
  - Fee collection tracking
  - Registry statistics

- 🔄 **Governance**:
  - Proposal lifecycle management
  - Weighted voting system
  - Execution deadline enforcement

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
ippan-ai-registry = { path = "../ai_registry" }

# For API support
ippan-ai-registry = { path = "../ai_registry", features = ["api"] }
```

## Usage

### Basic Usage

```rust
use ippan_ai_registry::{ModelRegistry, RegistryStorage, RegistryConfig, ModelCategory};
use ai_core::types::{ModelId, ModelMetadata};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize storage
    let storage = RegistryStorage::new(Some("./registry.db"))?;
    
    // Create registry with default config
    let config = RegistryConfig::default();
    let mut registry = ModelRegistry::new(storage, config);
    
    // Create model ID
    let model_id = ModelId {
        name: "my_model".to_string(),
        version: "1.0.0".to_string(),
        hash: "abc123".to_string(),
    };
    
    // Create metadata
    let metadata = ModelMetadata {
        id: model_id.clone(),
        architecture: "GBDT".to_string(),
        input_shape: vec![10],
        output_shape: vec![1],
        parameter_count: 1000,
        size_bytes: 50000,
        created_at: chrono::Utc::now().timestamp() as u64,
        description: Some("Test model".to_string()),
    };
    
    // Register model
    let registration = registry.register_model(
        model_id,
        metadata,
        "user_address".to_string(),
        ModelCategory::Other,
        Some("My test model".to_string()),
        Some("MIT".to_string()),
        Some("https://example.com/model".to_string()),
        vec!["test".to_string(), "demo".to_string()],
    ).await?;
    
    println!("Model registered: {:?}", registration);
    
    Ok(())
}
```

### Governance Example

```rust
use ippan_ai_registry::{GovernanceManager, RegistryStorage, RegistryConfig, ProposalType, ProposalData, VoteChoice};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let storage = RegistryStorage::new(None)?;
    let config = RegistryConfig::default();
    let mut governance = GovernanceManager::new(storage, config);
    
    // Set voting power
    governance.set_voting_power("user1".to_string(), 1000);
    governance.set_voting_power("user2".to_string(), 2000);
    
    // Create proposal
    let proposal = governance.create_proposal(
        ProposalType::ModelApproval,
        "Approve Model v1".to_string(),
        "Approve the new validator model".to_string(),
        "proposer_address".to_string(),
        ProposalData::ModelApproval {
            model_id: ModelId {
                name: "validator_model".to_string(),
                version: "1.0.0".to_string(),
                hash: "xyz789".to_string(),
            },
            approval_reason: "Tested and validated".to_string(),
        },
    ).await?;
    
    // Vote on proposal
    governance.vote_on_proposal(
        &proposal.id,
        "user1".to_string(),
        VoteChoice::For,
        Some("I support this model".to_string()),
    ).await?;
    
    Ok(())
}
```

### Security Example

```rust
use ippan_ai_registry::{SecurityManager, SecurityConfig, UserPermissions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = SecurityConfig::default();
    let mut security = SecurityManager::new(config);
    
    // Generate authentication token
    let token = security.generate_token(
        "user1".to_string(),
        vec!["read".to_string(), "write".to_string()],
    )?;
    
    // Set user permissions
    let permissions = UserPermissions {
        can_register: true,
        can_update: false,
        can_delete: false,
        can_vote: true,
        can_propose: true,
        can_admin: false,
        rate_limit: 100,
    };
    security.set_user_permissions("user1".to_string(), permissions);
    
    // Check authentication
    if let Some(user_id) = security.authenticate(&token.token).await? {
        println!("Authenticated as: {}", user_id);
        
        // Check rate limit
        if security.check_rate_limit(&user_id).await? {
            println!("Request allowed");
        } else {
            println!("Rate limit exceeded");
        }
    }
    
    Ok(())
}
```

### Fee Management Example

```rust
use ippan_ai_registry::{FeeManager, RegistryStorage, RegistryConfig, FeeType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let storage = RegistryStorage::new(None)?;
    let config = RegistryConfig::default();
    let fees = FeeManager::new(storage, config);
    
    // Calculate registration fee
    let calculation = fees.calculate_fee(
        FeeType::Registration,
        Some(&metadata),
        None,
        None,
    )?;
    
    println!("Registration fee: {} (base: {}, unit: {}, units: {})",
        calculation.amount,
        calculation.base_fee,
        calculation.unit_fee,
        calculation.units
    );
    
    Ok(())
}
```

### REST API Example (requires `api` feature)

```rust
use ippan_ai_registry::api::{ApiState, create_router};
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let storage = RegistryStorage::new(None)?;
    let config = RegistryConfig::default();
    
    let state = ApiState {
        registry: Arc::new(RwLock::new(ModelRegistry::new(storage.clone(), config.clone()))),
        governance: Arc::new(RwLock::new(GovernanceManager::new(storage.clone(), config.clone()))),
        fees: Arc::new(RwLock::new(FeeManager::new(storage, config))),
    };
    
    let app = create_router(state);
    
    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}
```

## Configuration

### RegistryConfig

```rust
pub struct RegistryConfig {
    pub min_registration_fee: u64,        // Default: 1,000
    pub max_registration_fee: u64,        // Default: 1,000,000
    pub default_execution_fee: u64,       // Default: 100
    pub storage_fee_per_byte_per_day: u64, // Default: 1
    pub proposal_fee: u64,                // Default: 10,000
    pub voting_period_seconds: u64,       // Default: 604,800 (7 days)
    pub execution_period_seconds: u64,    // Default: 1,209,600 (14 days)
    pub min_voting_power: u64,            // Default: 1,000
    pub max_model_size: u64,              // Default: 104,857,600 (100MB)
    pub max_parameter_count: u64,         // Default: 1,000,000,000
}
```

### SecurityConfig

```rust
pub struct SecurityConfig {
    pub enable_auth: bool,                // Default: true
    pub enable_rate_limiting: bool,       // Default: true
    pub rate_limit_window: u64,           // Default: 60 seconds
    pub max_requests_per_window: u64,     // Default: 100
    pub token_expiration: u64,            // Default: 3,600 seconds
    pub enable_ip_whitelist: bool,        // Default: false
    pub allowed_ips: Vec<String>,         // Default: empty
    pub enable_audit_logging: bool,       // Default: true
}
```

## API Endpoints

When the `api` feature is enabled, the following endpoints are available:

### Models
- `POST /models` - Register a new model
- `GET /models/:name` - Get model by name
- `GET /models/search?q=query` - Search models
- `POST /models/:name/status` - Update model status
- `GET /models/:name/stats` - Get model statistics

### Governance
- `POST /proposals` - Create a proposal
- `GET /proposals/:id` - Get proposal by ID
- `POST /proposals/:id/vote` - Vote on a proposal
- `POST /proposals/:id/execute` - Execute a proposal
- `GET /proposals` - List active proposals

### Fees
- `POST /fees/calculate` - Calculate fee
- `GET /fees/stats` - Get fee statistics

### Statistics
- `GET /stats` - Get registry statistics

## Testing

Run tests:

```bash
cargo test --package ippan-ai-registry
```

Run tests with API feature:

```bash
cargo test --package ippan-ai-registry --features api
```

## Architecture

```
ai_registry/
├── src/
│   ├── lib.rs           # Main library entry point
│   ├── errors.rs        # Error types
│   ├── types.rs         # Core type definitions
│   ├── storage.rs       # Persistent storage layer
│   ├── registry.rs      # Model registry implementation
│   ├── governance.rs    # Governance and voting
│   ├── security.rs      # Security and authentication
│   ├── fees.rs          # Fee management
│   ├── activation.rs    # Model activation scheduling
│   ├── proposal.rs      # Proposal management
│   └── api.rs           # REST API (optional)
└── Cargo.toml
```

## Production Considerations

### Performance
- In-memory caching for frequently accessed models
- Efficient database queries with Sled
- Async/await throughout for non-blocking I/O

### Security
- All inputs are validated and sanitized
- Rate limiting prevents abuse
- Token-based authentication with expiration
- Audit logging for security events

### Scalability
- Storage layer abstraction allows for easy backend swapping
- Stateless API design
- Configurable limits and thresholds

### Reliability
- Comprehensive error handling
- Transaction-like guarantees for critical operations
- Graceful degradation when optional features fail

## License

See the main IPPAN project LICENSE file.

## Contributing

Contributions are welcome! Please follow the IPPAN contribution guidelines.

## Maintainers

- Agent-Zeta (AI Registry Module Owner)
- MetaAgent (Infrastructure & Integration)

See `AGENTS.md` in the project root for the complete agent registry.
