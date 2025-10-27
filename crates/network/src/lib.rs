pub mod parallel_gossip;
pub mod peers;
pub mod connection;
pub mod protocol;
pub mod discovery;
pub mod metrics;

pub use parallel_gossip::{GossipMessage, ParallelGossip};
pub use peers::{Peer, PeerDirectory};
pub use connection::{ConnectionManager, ConnectionConfig, ConnectionState};
pub use protocol::{NetworkProtocol, MessageHandler, ProtocolError};
pub use discovery::{PeerDiscovery, DiscoveryConfig, DiscoveryService};
pub use metrics::{NetworkMetrics, MetricsCollector};
