pub mod parallel_gossip;
pub mod peers;
pub mod deduplication;
pub mod reputation;
pub mod metrics;
pub mod health;

pub use parallel_gossip::{GossipMessage, ParallelGossip};
pub use peers::{Peer, PeerDirectory};
pub use deduplication::MessageDeduplicator;
pub use reputation::{ReputationManager, ReputationScore, PeerReputationStats};
pub use metrics::{NetworkMetrics, NetworkMetricsSnapshot};
pub use health::{HealthMonitor, PeerHealth, HealthCheckConfig, PeerHealthStats};
