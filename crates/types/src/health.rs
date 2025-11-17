use serde::Serialize;

/// Public observability payload returned by `/health`.
///
/// All fields intentionally use integers, booleans, or strings to keep the
/// structure serialization-friendly and deterministic across runtimes.
#[derive(Debug, Clone, Serialize)]
pub struct HealthStatus {
    pub consensus_mode: String,
    pub consensus_healthy: bool,
    pub ai_enabled: bool,
    pub dht_file_mode: String,
    pub dht_handle_mode: String,
    pub dht_healthy: bool,
    pub rpc_healthy: bool,
    pub storage_healthy: bool,
    pub last_finalized_round: Option<u64>,
    pub last_consensus_round: Option<u64>,
    pub peer_count: u64,
    pub mempool_size: u64,
    pub uptime_seconds: u64,
    pub requests_served: u64,
    pub node_id: String,
    pub version: String,
    pub dev_mode: bool,
}

/// Builder-style context that can be created from various node runtimes.
///
/// The context stays integer/boolean/string-only so it can be shared between
/// crates (RPC, node, CLI, etc.) without pulling in heavy dependencies.
#[derive(Debug, Clone, Default)]
pub struct NodeHealthContext {
    pub consensus_mode: String,
    pub consensus_healthy: bool,
    pub ai_enabled: bool,
    pub dht_file_mode: String,
    pub dht_handle_mode: String,
    pub dht_healthy: bool,
    pub rpc_healthy: bool,
    pub storage_healthy: bool,
    pub last_finalized_round: Option<u64>,
    pub last_consensus_round: Option<u64>,
    pub peer_count: u64,
    pub mempool_size: u64,
    pub uptime_seconds: u64,
    pub requests_served: u64,
    pub node_id: String,
    pub version: String,
    pub dev_mode: bool,
}

impl NodeHealthContext {
    /// Convenience helper for callers that only know the consensus mode at
    /// construction time.
    pub fn with_consensus_mode(consensus_mode: impl Into<String>) -> Self {
        Self {
            consensus_mode: consensus_mode.into(),
            ..Self::default()
        }
    }
}

/// Stateless helper that converts a context into a serializable status.
pub struct NodeHealth;

impl NodeHealth {
    pub fn snapshot(context: NodeHealthContext) -> HealthStatus {
        HealthStatus {
            consensus_mode: context.consensus_mode,
            consensus_healthy: context.consensus_healthy,
            ai_enabled: context.ai_enabled,
            dht_file_mode: context.dht_file_mode,
            dht_handle_mode: context.dht_handle_mode,
            dht_healthy: context.dht_healthy,
            rpc_healthy: context.rpc_healthy,
            storage_healthy: context.storage_healthy,
            last_finalized_round: context.last_finalized_round,
            last_consensus_round: context.last_consensus_round,
            peer_count: context.peer_count,
            mempool_size: context.mempool_size,
            uptime_seconds: context.uptime_seconds,
            requests_served: context.requests_served,
            node_id: context.node_id,
            version: context.version,
            dev_mode: context.dev_mode,
        }
    }
}
