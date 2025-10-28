pub mod connection;
pub mod discovery;
pub mod metrics;
pub mod parallel_gossip;
pub mod peers;
pub mod protocol;

pub use connection::{ConnectionConfig, ConnectionManager, ConnectionState};
pub use discovery::{DiscoveryConfig, DiscoveryService, PeerDiscovery};
pub use metrics::{MetricsCollector, NetworkMetrics};
pub use parallel_gossip::{GossipMessage, ParallelGossip};
pub use peers::{Peer, PeerDirectory};
pub use protocol::{MessageHandler, NetworkProtocol, ProtocolError};
