//! IPPAN Network Core
//!
//! Provides deterministic networking primitives for node discovery,
//! peer management, gossip propagation, and reputation tracking.
//!
//! ## Modules
//! - `connection`: Async peer connection management
//! - `discovery`: Peer discovery and DHT integration
//! - `parallel_gossip`: Deterministic gossip with deduplication
//! - `peers`: Peer directory and state registry
//! - `deduplication`: Prevents duplicate message processing
//! - `reputation`: Tracks validator and peer reputation
//! - `protocol`: Defines wire protocol, message formats, and handlers
//! - `metrics`: Aggregates real-time network statistics
//! - `health`: Monitors peer and network health

pub mod connection;
pub mod deduplication;
pub mod discovery;
pub mod health;
pub mod identity_store;
pub mod metrics;
pub mod parallel_gossip;
pub mod peers;
pub mod protocol;
pub mod reputation;

// ------------------------------------------------------------
// Re-exports for workspace-wide use
// ------------------------------------------------------------

// Connection & discovery
pub use connection::{ConnectionConfig, ConnectionManager, ConnectionState};
pub use discovery::{DiscoveryConfig, DiscoveryService, PeerDiscovery};
pub use identity_store::{load_identity_with_fallback, load_or_generate_identity_keypair};

// Gossip & messaging
pub use deduplication::MessageDeduplicator;
pub use parallel_gossip::{GossipMessage, ParallelGossip};
pub use protocol::{MessageHandler, NetworkProtocol, ProtocolError};

// Peer management & reputation
pub use peers::{Peer, PeerDirectory};
pub use reputation::{PeerReputationStats, ReputationManager, ReputationScore};

// Metrics & health
pub use health::{HealthCheckConfig, HealthMonitor, PeerHealth, PeerHealthStats};
pub use metrics::{MetricsCollector, NetworkMetrics, NetworkMetricsSnapshot};
