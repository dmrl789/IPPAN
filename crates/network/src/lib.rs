pub mod parallel_gossip;
pub mod peers;

pub use parallel_gossip::{GossipMessage, ParallelGossip};
pub use peers::{Peer, PeerDirectory};
