//! Advanced synchronization manager for IPPAN
//!
//! Provides sophisticated synchronization capabilities including
//! conflict resolution, state reconciliation, and performance optimization.

use anyhow::{anyhow, Result};
use ippan_types::{format_ratio, RatioMicros, RATIO_SCALE};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, RwLock};
use tokio::time::interval;
use tracing::{debug, error, info, warn};

const PERFORMANCE_SCORE_SCALE: u32 = 1_000;
const PERFORMANCE_SCORE_MAX: u32 = 1_500;
const PERFORMANCE_SCORE_MIN: u32 = 100;
const PERFORMANCE_SCORE_INCREMENT: u32 = 50;
const PERFORMANCE_SCORE_DECAY_NUM: u32 = 9;
const PERFORMANCE_SCORE_DECAY_DEN: u32 = 10;

use crate::block::Block;
use crate::dag::BlockDAG;
use crate::dag_operations::{DAGOperations, DAGOptimizationConfig};

/// Synchronization state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncState {
    Idle,
    Discovering,
    Syncing,
    CatchingUp,
    UpToDate,
    Error(String),
}

/// Synchronization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    pub max_peers: usize,
    pub sync_interval: Duration,
    pub batch_size: usize,
    pub timeout: Duration,
    pub retry_attempts: u32,
    pub enable_optimization: bool,
    pub conflict_resolution_strategy: ConflictResolutionStrategy,
    pub performance_monitoring: bool,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            max_peers: 50,
            sync_interval: Duration::from_secs(30),
            batch_size: 100,
            timeout: Duration::from_secs(10),
            retry_attempts: 3,
            enable_optimization: true,
            conflict_resolution_strategy: ConflictResolutionStrategy::LongestChain,
            performance_monitoring: true,
        }
    }
}

/// Conflict resolution strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictResolutionStrategy {
    LongestChain,
    MostRecent,
    WeightedVoting,
}

/// Trait for custom conflict resolution
pub trait ConflictResolver: Send + Sync {
    fn resolve_conflict(&self, blocks: &[Block]) -> Result<Block>;
}

/// Synchronization event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncEvent {
    StateChanged(SyncState),
    BlockReceived(Block),
    ConflictDetected(Vec<Block>),
    SyncCompleted,
    Error(String),
    PerformanceUpdate(SyncPerformance),
}

/// Synchronization performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPerformance {
    pub blocks_per_second: u64,
    pub average_latency_micros: u64,
    pub success_rate_micros: RatioMicros,
    pub total_blocks_synced: usize,
    pub sync_duration: Duration,
    pub memory_usage_bytes: u64,
}

/// Advanced synchronization manager
pub struct SyncManager {
    dag: Arc<RwLock<BlockDAG>>,
    dag_ops: Arc<RwLock<DAGOperations>>,
    config: SyncConfig,
    state: Arc<RwLock<SyncState>>,
    performance: Arc<RwLock<SyncPerformance>>,
    event_sender: mpsc::UnboundedSender<SyncEvent>,
    peer_connections: Arc<RwLock<HashMap<String, PeerConnection>>>,
    sync_queue: Arc<RwLock<VecDeque<SyncTask>>>,
    last_sync_time: Arc<RwLock<Option<Instant>>>,
    is_running: Arc<RwLock<bool>>,
}

/// Peer connection information
#[derive(Debug, Clone)]
struct PeerConnection {
    peer_id: String,
    last_seen: Instant,
    sync_capability: SyncCapability,
    performance_score: u32,
    is_active: bool,
}

/// Peer synchronization capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncCapability {
    max_batch_size: usize,
    supported_protocols: Vec<String>,
    compression_enabled: bool,
    encryption_enabled: bool,
}

/// Synchronization task
#[derive(Debug, Clone)]
struct SyncTask {
    task_id: String,
    task_type: SyncTaskType,
    priority: u8,
    created_at: Instant,
    retry_count: u32,
    data: Vec<u8>,
}

/// Types of synchronization tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
enum SyncTaskType {
    BlockSync,
    StateSync,
    ConflictResolution,
    Optimization,
    HealthCheck,
}

impl SyncManager {
    /// Create a new synchronization manager
    pub fn new(
        dag: BlockDAG,
        config: SyncConfig,
    ) -> Result<(Self, mpsc::UnboundedReceiver<SyncEvent>)> {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();

        let dag_arc = Arc::new(RwLock::new(dag));
        let dag_ops = Arc::new(RwLock::new(DAGOperations::new(
            dag_arc.clone(),
            DAGOptimizationConfig::default(),
        )));

        let manager = Self {
            dag: dag_arc.clone(),
            dag_ops,
            config,
            state: Arc::new(RwLock::new(SyncState::Idle)),
            performance: Arc::new(RwLock::new(SyncPerformance::default())),
            event_sender,
            peer_connections: Arc::new(RwLock::new(HashMap::new())),
            sync_queue: Arc::new(RwLock::new(VecDeque::new())),
            last_sync_time: Arc::new(RwLock::new(None)),
            is_running: Arc::new(RwLock::new(false)),
        };

        Ok((manager, event_receiver))
    }

    /// Start the synchronization manager
    pub async fn start(&self) -> Result<()> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return Err(anyhow!("Sync manager is already running"));
        }
        *is_running = true;
        drop(is_running);

        info!("Starting synchronization manager");

        // Start background tasks
        let sync_loop = self.start_sync_loop();
        let performance_monitor = self.start_performance_monitor();
        let peer_manager = self.start_peer_manager();

        // Wait for all tasks
        tokio::try_join!(sync_loop, performance_monitor, peer_manager)?;

        Ok(())
    }

    /// Stop the synchronization manager
    pub async fn stop(&self) -> Result<()> {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        drop(is_running);

        info!("Stopping synchronization manager");
        Ok(())
    }

    /// Get current synchronization state
    pub async fn get_state(&self) -> SyncState {
        self.state.read().await.clone()
    }

    /// Get synchronization performance metrics
    pub async fn get_performance(&self) -> SyncPerformance {
        self.performance.read().await.clone()
    }

    /// Add a peer connection
    pub async fn add_peer(&self, peer_id: String, capability: SyncCapability) -> Result<()> {
        let mut connections = self.peer_connections.write().await;
        let connection = PeerConnection {
            peer_id: peer_id.clone(),
            last_seen: Instant::now(),
            sync_capability: capability,
            performance_score: PERFORMANCE_SCORE_SCALE,
            is_active: true,
        };
        connections.insert(peer_id.clone(), connection);
        info!("Added peer connection: {}", peer_id);
        Ok(())
    }

    /// Remove a peer connection
    pub async fn remove_peer(&self, peer_id: &str) -> Result<()> {
        let mut connections = self.peer_connections.write().await;
        if connections.remove(peer_id).is_some() {
            info!("Removed peer connection: {}", peer_id);
        }
        Ok(())
    }

    /// Trigger manual synchronization
    pub async fn trigger_sync(&self) -> Result<()> {
        let mut queue = self.sync_queue.write().await;
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?;
        let task = SyncTask {
            task_id: format!("manual_sync_{}", timestamp.as_secs()),
            task_type: SyncTaskType::BlockSync,
            priority: 10, // High priority
            created_at: Instant::now(),
            retry_count: 0,
            data: format!("manual sync issued at {}", timestamp.as_millis()).into_bytes(),
        };
        queue.push_back(task);
        info!("Triggered manual synchronization");
        Ok(())
    }

    /// Start the main synchronization loop
    async fn start_sync_loop(&self) -> Result<()> {
        let mut interval = interval(self.config.sync_interval);

        while *self.is_running.read().await {
            interval.tick().await;

            if let Err(e) = self.perform_sync_cycle().await {
                error!("Sync cycle failed: {}", e);
                self.set_state(SyncState::Error(e.to_string())).await;
            }
        }

        Ok(())
    }

    /// Start the performance monitoring loop
    async fn start_performance_monitor(&self) -> Result<()> {
        let mut interval = interval(Duration::from_secs(5));

        while *self.is_running.read().await {
            interval.tick().await;

            if self.config.performance_monitoring {
                self.update_performance_metrics().await?;
            }
        }

        Ok(())
    }

    /// Start the peer management loop
    async fn start_peer_manager(&self) -> Result<()> {
        let mut interval = interval(Duration::from_secs(10));

        while *self.is_running.read().await {
            interval.tick().await;
            self.cleanup_inactive_peers().await?;
        }

        Ok(())
    }

    /// Perform a single synchronization cycle
    async fn perform_sync_cycle(&self) -> Result<()> {
        debug!("Starting sync cycle");

        // Check if we have peers
        let peer_count = self.peer_connections.read().await.len();
        if peer_count == 0 {
            self.set_state(SyncState::Discovering).await;
            return Ok(());
        }

        self.set_state(SyncState::Syncing).await;

        // Process sync queue
        self.process_sync_queue().await?;

        // Perform DAG optimization if enabled
        if self.config.enable_optimization {
            self.optimize_dag().await?;
        }

        // Update last sync time
        {
            let mut last_sync = self.last_sync_time.write().await;
            *last_sync = Some(Instant::now());
        }

        self.set_state(SyncState::UpToDate).await;
        self.send_event(SyncEvent::SyncCompleted).await?;

        debug!("Sync cycle completed");
        Ok(())
    }

    /// Process the synchronization queue
    async fn process_sync_queue(&self) -> Result<()> {
        let mut queue = self.sync_queue.write().await;
        let mut processed_tasks = Vec::new();

        if !queue.is_empty() {
            let slice = queue.make_contiguous();
            slice.sort_by(|a, b| {
                b.priority
                    .cmp(&a.priority)
                    .then_with(|| a.created_at.cmp(&b.created_at))
            });
        }

        while let Some(mut task) = queue.pop_front() {
            let payload_bytes = task.data.len();
            let task_age = task.created_at.elapsed();
            debug!(
                "Processing sync task {} (priority {}, retries {}, payload {} bytes, age {:?})",
                task.task_id, task.priority, task.retry_count, payload_bytes, task_age
            );

            match self.process_sync_task(&task).await {
                Ok(_) => {
                    processed_tasks.push(task.task_id);
                }
                Err(e) => {
                    error!("Failed to process sync task {}: {}", task.task_id, e);

                    task.retry_count += 1;
                    if task.priority > 0 {
                        task.priority -= 1;
                    }
                    if task.retry_count < self.config.retry_attempts {
                        queue.push_back(task);
                        let slice = queue.make_contiguous();
                        slice.sort_by(|a, b| {
                            b.priority
                                .cmp(&a.priority)
                                .then_with(|| a.created_at.cmp(&b.created_at))
                        });
                    } else {
                        warn!(
                            "Dropping sync task {} after {} retries (payload {} bytes)",
                            task.task_id, self.config.retry_attempts, payload_bytes
                        );
                    }
                }
            }
        }

        if !processed_tasks.is_empty() {
            info!("Processed {} sync tasks", processed_tasks.len());
        }

        Ok(())
    }

    /// Process a single synchronization task
    async fn process_sync_task(&self, task: &SyncTask) -> Result<()> {
        match task.task_type {
            SyncTaskType::BlockSync => {
                self.sync_blocks().await?;
            }
            SyncTaskType::StateSync => {
                self.sync_state().await?;
            }
            SyncTaskType::ConflictResolution => {
                self.resolve_conflicts().await?;
            }
            SyncTaskType::Optimization => {
                self.optimize_dag().await?;
            }
            SyncTaskType::HealthCheck => {
                self.perform_health_check().await?;
            }
        }
        Ok(())
    }

    /// Synchronize blocks with peers
    async fn sync_blocks(&self) -> Result<()> {
        let active_peers: Vec<PeerConnection> = {
            let connections = self.peer_connections.read().await;
            connections
                .values()
                .filter(|conn| conn.is_active && conn.sync_capability.max_batch_size > 0)
                .cloned()
                .collect()
        };

        if active_peers.is_empty() {
            return Ok(());
        }

        // Get our current tip
        let dag = self.dag.read().await;
        let our_tips = dag.get_tips()?;
        drop(dag);

        // Request blocks from peers
        for peer in active_peers {
            let batch_limit = peer
                .sync_capability
                .max_batch_size
                .min(self.config.batch_size);

            self.request_blocks_from_peer(
                &peer.peer_id,
                &our_tips,
                batch_limit,
                peer.sync_capability.supported_protocols.clone(),
                peer.performance_score,
            )
            .await?;

            self.update_peer_activity(&peer.peer_id, true).await;
        }

        Ok(())
    }

    /// Synchronize state with peers
    async fn sync_state(&self) -> Result<()> {
        // Implement state synchronization logic
        debug!("Performing state synchronization");
        Ok(())
    }

    /// Resolve conflicts between blocks
    async fn resolve_conflicts(&self) -> Result<()> {
        let mut dag_ops = self.dag_ops.write().await;
        let analysis = dag_ops.analyze_dag().await?;

        if analysis.convergence_ratio_micros > RATIO_SCALE / 2 {
            warn!(
                "High convergence ratio detected: {}",
                format_ratio(analysis.convergence_ratio_micros)
            );
            self.send_event(SyncEvent::ConflictDetected(vec![])).await?;
        }

        Ok(())
    }

    /// Optimize the DAG
    async fn optimize_dag(&self) -> Result<()> {
        let mut dag_ops = self.dag_ops.write().await;
        let pruned = dag_ops.optimize_dag().await?;
        let compacted = dag_ops.compact_dag()?;

        if pruned > 0 || compacted > 0 {
            info!(
                "DAG optimization: {} pruned, {} compacted",
                pruned, compacted
            );
        }

        Ok(())
    }

    /// Perform health check
    async fn perform_health_check(&self) -> Result<()> {
        let dag = self.dag.read().await;
        let tips = dag.get_tips()?;
        let tip_count = tips.len();
        drop(dag);

        if tip_count == 0 {
            warn!("No tips found in DAG");
        }

        debug!("Health check completed: {} tips", tip_count);
        Ok(())
    }

    /// Request blocks from a specific peer
    async fn request_blocks_from_peer(
        &self,
        peer_id: &str,
        our_tips: &[[u8; 32]],
        batch_limit: usize,
        protocols: Vec<String>,
        performance_score: u32,
    ) -> Result<()> {
        if protocols.is_empty() {
            warn!(
                "Peer {} has no supported protocols; skipping block request",
                peer_id
            );
            return Ok(());
        }

        debug!(
            "Requesting up to {} blocks from peer {} (score {}) using {:?} for {} tips",
            batch_limit,
            peer_id,
            format_performance_score(performance_score),
            protocols,
            our_tips.len()
        );

        // Implement peer communication logic
        Ok(())
    }

    async fn update_peer_activity(&self, peer_id: &str, success: bool) {
        let mut connections = self.peer_connections.write().await;
        if let Some(connection) = connections.get_mut(peer_id) {
            connection.last_seen = Instant::now();
            if success {
                connection.performance_score = connection
                    .performance_score
                    .saturating_add(PERFORMANCE_SCORE_INCREMENT)
                    .min(PERFORMANCE_SCORE_MAX);
            } else {
                let decayed = connection
                    .performance_score
                    .saturating_mul(PERFORMANCE_SCORE_DECAY_NUM)
                    / PERFORMANCE_SCORE_DECAY_DEN;
                connection.performance_score = decayed.max(PERFORMANCE_SCORE_MIN);
            }
        }
    }

    /// Update performance metrics
    async fn update_performance_metrics(&self) -> Result<()> {
        let mut performance = self.performance.write().await;

        // Calculate blocks per second
        let last_sync = self.last_sync_time.read().await;
        if let Some(last) = *last_sync {
            let elapsed = last.elapsed();
            if elapsed.as_secs() > 0 {
                performance.blocks_per_second =
                    performance.total_blocks_synced as u64 / elapsed.as_secs();
            } else {
                performance.blocks_per_second = 0;
            }
        }

        // Update other metrics
        performance.average_latency_micros = 50_000; // Placeholder
        performance.success_rate_micros = (RATIO_SCALE * 95) / 100; // Placeholder
        performance.memory_usage_bytes = 100 * 1024 * 1024; // Placeholder

        self.send_event(SyncEvent::PerformanceUpdate(performance.clone()))
            .await?;
        Ok(())
    }

    /// Clean up inactive peers
    async fn cleanup_inactive_peers(&self) -> Result<()> {
        let mut connections = self.peer_connections.write().await;
        let cutoff_time = Instant::now() - Duration::from_secs(300); // 5 minutes

        let inactive_peers: Vec<String> = connections
            .iter()
            .filter(|(_, conn)| conn.last_seen < cutoff_time)
            .map(|(peer_id, _)| peer_id.clone())
            .collect();

        for peer_id in inactive_peers {
            connections.remove(&peer_id);
            info!("Removed inactive peer: {}", peer_id);
        }

        Ok(())
    }

    /// Set the synchronization state
    async fn set_state(&self, new_state: SyncState) {
        let mut state = self.state.write().await;
        *state = new_state.clone();
        drop(state);

        self.send_event(SyncEvent::StateChanged(new_state))
            .await
            .ok();
    }

    /// Send a synchronization event
    async fn send_event(&self, event: SyncEvent) -> Result<()> {
        self.event_sender.send(event)?;
        Ok(())
    }
}

fn format_performance_score(score: u32) -> String {
    let whole = score / PERFORMANCE_SCORE_SCALE;
    let fractional = score % PERFORMANCE_SCORE_SCALE;
    if fractional == 0 {
        format!("{whole}")
    } else {
        let mut frac_str = format!("{fractional:03}");
        while frac_str.ends_with('0') {
            frac_str.pop();
        }
        format!("{whole}.{frac_str}")
    }
}

impl Default for SyncPerformance {
    fn default() -> Self {
        Self {
            blocks_per_second: 0,
            average_latency_micros: 0,
            success_rate_micros: RATIO_SCALE,
            total_blocks_synced: 0,
            sync_duration: Duration::from_secs(0),
            memory_usage_bytes: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    async fn create_test_sync_manager() -> (SyncManager, mpsc::UnboundedReceiver<SyncEvent>) {
        let dir = tempdir().unwrap();
        let dag = BlockDAG::open(dir.path()).unwrap();
        let config = SyncConfig::default();
        SyncManager::new(dag, config).unwrap()
    }

    #[tokio::test]
    async fn test_sync_manager_creation() {
        let (manager, _) = create_test_sync_manager().await;
        let state = manager.get_state().await;
        assert_eq!(state, SyncState::Idle);
    }

    #[tokio::test]
    async fn test_peer_management() {
        let (manager, _) = create_test_sync_manager().await;

        let capability = SyncCapability {
            max_batch_size: 100,
            supported_protocols: vec!["v1".to_string()],
            compression_enabled: true,
            encryption_enabled: true,
        };

        manager
            .add_peer("test_peer".to_string(), capability)
            .await
            .unwrap();
        manager.remove_peer("test_peer").await.unwrap();
    }

    #[tokio::test]
    async fn test_manual_sync_trigger() {
        let (manager, _) = create_test_sync_manager().await;
        manager.trigger_sync().await.unwrap();
    }
}
