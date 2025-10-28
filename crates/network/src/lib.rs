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
pub mod discovery;
pub mod metrics;
pub mod parallel_gossip;
pub mod peers;
pub mod deduplication;
pub mod reputation;
pub mod health;
pub mod protocol;

// ------------------------------------------------------------
// Re-exports for workspace-wide use
// ------------------------------------------------------------

// Connection & discovery
pub use connection::{ConnectionConfig, ConnectionManager, ConnectionState};
pub use discovery::{DiscoveryConfig, DiscoveryService, PeerDiscovery};

// Gossip & messaging
pub use parallel_gossip::{GossipMessage, ParallelGossip};
pub use deduplication::MessageDeduplicator;
pub use protocol::{MessageHandler, NetworkProtocol, ProtocolError};

// Peer management & reputation
pub use peers::{Peer, PeerDirectory};
pub use reputation::{ReputationManager, ReputationScore, PeerReputationStats};

// Metrics & health
pub use metrics::{MetricsCollector, NetworkMetrics, NetworkMetricsSnapshot};
pub use health::{HealthMonitor, PeerHealth, HealthCheckConfig, PeerHealthStats};
