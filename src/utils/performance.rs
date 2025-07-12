use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

/// Performance metrics for different subsystems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub timestamp: u64,
    pub subsystem: String,
    pub operation: String,
    pub duration_ns: u64,
    pub memory_usage_bytes: u64,
    pub cpu_usage_percent: f64,
    pub throughput_ops_per_sec: f64,
    pub error_count: u64,
    pub success_count: u64,
}

/// Performance profiler for measuring and tracking metrics
pub struct PerformanceProfiler {
    metrics: Arc<Mutex<Vec<PerformanceMetrics>>>,
    active_timers: Arc<Mutex<HashMap<String, Instant>>>,
    config: ProfilerConfig,
}

#[derive(Debug, Clone)]
pub struct ProfilerConfig {
    pub max_metrics_history: usize,
    pub enable_memory_tracking: bool,
    pub enable_cpu_tracking: bool,
    pub sampling_interval_ms: u64,
}

impl Default for ProfilerConfig {
    fn default() -> Self {
        Self {
            max_metrics_history: 10000,
            enable_memory_tracking: true,
            enable_cpu_tracking: true,
            sampling_interval_ms: 1000,
        }
    }
}

impl PerformanceProfiler {
    /// Create a new performance profiler
    pub fn new(config: ProfilerConfig) -> Self {
        Self {
            metrics: Arc::new(Mutex::new(Vec::new())),
            active_timers: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }

    /// Start timing an operation
    pub fn start_timer(&self, operation_id: String) {
        let mut timers = self.active_timers.lock().unwrap();
        timers.insert(operation_id, Instant::now());
    }

    /// Stop timing an operation and record metrics
    pub fn stop_timer(&self, operation_id: String, subsystem: String, operation: String) {
        let mut timers = self.active_timers.lock().unwrap();
        if let Some(start_time) = timers.remove(&operation_id) {
            let duration = start_time.elapsed();
            self.record_metric(subsystem, operation, duration);
        }
    }

    /// Record a performance metric
    pub fn record_metric(&self, subsystem: String, operation: String, duration: Duration) {
        let mut metrics = self.metrics.lock().unwrap();
        
        // Get current memory usage if enabled
        let memory_usage = if self.config.enable_memory_tracking {
            self.get_memory_usage()
        } else {
            0
        };

        // Get current CPU usage if enabled
        let cpu_usage = if self.config.enable_cpu_tracking {
            self.get_cpu_usage()
        } else {
            0.0
        };

        let metric = PerformanceMetrics {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            subsystem,
            operation,
            duration_ns: duration.as_nanos() as u64,
            memory_usage_bytes: memory_usage,
            cpu_usage_percent: cpu_usage,
            throughput_ops_per_sec: 0.0, // Will be calculated later
            error_count: 0,
            success_count: 1,
        };

        metrics.push(metric);

        // Trim old metrics if we exceed the limit
        if metrics.len() > self.config.max_metrics_history {
            metrics.drain(0..metrics.len() - self.config.max_metrics_history);
        }
    }

    /// Get memory usage in bytes
    fn get_memory_usage(&self) -> u64 {
        // This is a simplified implementation
        // In a real implementation, you'd use platform-specific APIs
        // For now, we'll return a placeholder value
        1024 * 1024 * 100 // 100MB placeholder
    }

    /// Get CPU usage percentage
    fn get_cpu_usage(&self) -> f64 {
        // This is a simplified implementation
        // In a real implementation, you'd use platform-specific APIs
        // For now, we'll return a placeholder value
        25.0 // 25% placeholder
    }

    /// Get all recorded metrics
    pub fn get_metrics(&self) -> Vec<PerformanceMetrics> {
        self.metrics.lock().unwrap().clone()
    }

    /// Get metrics for a specific subsystem
    pub fn get_subsystem_metrics(&self, subsystem: &str) -> Vec<PerformanceMetrics> {
        self.metrics
            .lock()
            .unwrap()
            .iter()
            .filter(|m| m.subsystem == subsystem)
            .cloned()
            .collect()
    }

    /// Get average duration for an operation
    pub fn get_average_duration(&self, subsystem: &str, operation: &str) -> Option<Duration> {
        let metrics = self.get_subsystem_metrics(subsystem);
        let relevant_metrics: Vec<_> = metrics
            .iter()
            .filter(|m| m.operation == operation)
            .collect();

        if relevant_metrics.is_empty() {
            return None;
        }

        let total_ns: u64 = relevant_metrics.iter().map(|m| m.duration_ns).sum();
        let count = relevant_metrics.len() as u64;
        Some(Duration::from_nanos(total_ns / count))
    }

    /// Get throughput statistics
    pub fn get_throughput_stats(&self, subsystem: &str, operation: &str) -> ThroughputStats {
        let metrics = self.get_subsystem_metrics(subsystem);
        let relevant_metrics: Vec<_> = metrics
            .iter()
            .filter(|m| m.operation == operation)
            .collect();

        if relevant_metrics.is_empty() {
            return ThroughputStats::default();
        }

        let total_ops = relevant_metrics.len() as u64;
        let total_duration_ns: u64 = relevant_metrics.iter().map(|m| m.duration_ns).sum();
        let avg_duration_ns = total_duration_ns / total_ops;
        let ops_per_sec = if avg_duration_ns > 0 {
            1_000_000_000.0 / avg_duration_ns as f64
        } else {
            0.0
        };

        ThroughputStats {
            total_operations: total_ops,
            average_duration_ns,
            operations_per_second: ops_per_sec,
            min_duration_ns: relevant_metrics.iter().map(|m| m.duration_ns).min().unwrap_or(0),
            max_duration_ns: relevant_metrics.iter().map(|m| m.duration_ns).max().unwrap_or(0),
        }
    }

    /// Clear all metrics
    pub fn clear_metrics(&self) {
        self.metrics.lock().unwrap().clear();
    }

    /// Export metrics to JSON
    pub fn export_metrics_json(&self) -> String {
        let metrics = self.get_metrics();
        serde_json::to_string_pretty(&metrics).unwrap_or_default()
    }

    /// Generate performance report
    pub fn generate_report(&self) -> PerformanceReport {
        let metrics = self.get_metrics();
        let subsystems: std::collections::HashSet<_> = metrics.iter().map(|m| m.subsystem.clone()).collect();
        
        let mut subsystem_stats = HashMap::new();
        for subsystem in subsystems {
            let subsystem_metrics = self.get_subsystem_metrics(&subsystem);
            let operations: std::collections::HashSet<_> = subsystem_metrics.iter().map(|m| m.operation.clone()).collect();
            
            let mut operation_stats = HashMap::new();
            for operation in operations {
                operation_stats.insert(operation, self.get_throughput_stats(&subsystem, &operation));
            }
            
            subsystem_stats.insert(subsystem, operation_stats);
        }

        PerformanceReport {
            total_metrics: metrics.len(),
            subsystem_stats,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}

/// Throughput statistics for operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThroughputStats {
    pub total_operations: u64,
    pub average_duration_ns: u64,
    pub operations_per_second: f64,
    pub min_duration_ns: u64,
    pub max_duration_ns: u64,
}

impl Default for ThroughputStats {
    fn default() -> Self {
        Self {
            total_operations: 0,
            average_duration_ns: 0,
            operations_per_second: 0.0,
            min_duration_ns: 0,
            max_duration_ns: 0,
        }
    }
}

/// Performance report containing aggregated statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    pub total_metrics: usize,
    pub subsystem_stats: HashMap<String, HashMap<String, ThroughputStats>>,
    pub timestamp: u64,
}

/// Performance monitoring for real-time tracking
pub struct PerformanceMonitor {
    profiler: Arc<PerformanceProfiler>,
    monitoring_thread: Option<std::thread::JoinHandle<()>>,
    stop_monitoring: Arc<Mutex<bool>>,
}

impl PerformanceMonitor {
    /// Create a new performance monitor
    pub fn new(profiler: Arc<PerformanceProfiler>) -> Self {
        Self {
            profiler,
            monitoring_thread: None,
            stop_monitoring: Arc::new(Mutex::new(false)),
        }
    }

    /// Start continuous monitoring
    pub fn start_monitoring(&mut self) {
        let profiler = Arc::clone(&self.profiler);
        let stop_monitoring = Arc::clone(&self.stop_monitoring);
        
        self.monitoring_thread = Some(std::thread::spawn(move || {
            while !*stop_monitoring.lock().unwrap() {
                // Record system-wide metrics
                profiler.record_metric(
                    "system".to_string(),
                    "monitoring_tick".to_string(),
                    Duration::from_millis(100),
                );
                
                std::thread::sleep(Duration::from_millis(1000));
            }
        }));
    }

    /// Stop continuous monitoring
    pub fn stop_monitoring(&mut self) {
        if let Some(thread) = self.monitoring_thread.take() {
            *self.stop_monitoring.lock().unwrap() = true;
            let _ = thread.join();
        }
    }
}

/// Performance benchmarking utilities
pub struct BenchmarkRunner {
    profiler: Arc<PerformanceProfiler>,
}

impl BenchmarkRunner {
    /// Create a new benchmark runner
    pub fn new(profiler: Arc<PerformanceProfiler>) -> Self {
        Self { profiler }
    }

    /// Run a benchmark with multiple iterations
    pub fn run_benchmark<F>(
        &self,
        name: &str,
        subsystem: &str,
        iterations: usize,
        benchmark_fn: F,
    ) where
        F: Fn() + Send + Sync,
    {
        println!("Running benchmark: {} ({} iterations)", name, iterations);
        
        let start_time = Instant::now();
        
        for i in 0..iterations {
            let operation_start = Instant::now();
            benchmark_fn();
            let duration = operation_start.elapsed();
            
            self.profiler.record_metric(
                subsystem.to_string(),
                name.to_string(),
                duration,
            );
            
            if (i + 1) % 100 == 0 {
                println!("Completed {} iterations", i + 1);
            }
        }
        
        let total_time = start_time.elapsed();
        println!(
            "Benchmark '{}' completed in {:?} ({:.2} ops/sec)",
            name,
            total_time,
            iterations as f64 / total_time.as_secs_f64()
        );
    }

    /// Run a benchmark with custom setup and teardown
    pub fn run_benchmark_with_setup<F, S, T>(
        &self,
        name: &str,
        subsystem: &str,
        iterations: usize,
        setup: S,
        benchmark_fn: F,
        teardown: T,
    ) where
        F: Fn() + Send + Sync,
        S: Fn() + Send + Sync,
        T: Fn() + Send + Sync,
    {
        println!("Running benchmark with setup: {} ({} iterations)", name, iterations);
        
        let start_time = Instant::now();
        
        for i in 0..iterations {
            setup();
            
            let operation_start = Instant::now();
            benchmark_fn();
            let duration = operation_start.elapsed();
            
            teardown();
            
            self.profiler.record_metric(
                subsystem.to_string(),
                name.to_string(),
                duration,
            );
            
            if (i + 1) % 100 == 0 {
                println!("Completed {} iterations", i + 1);
            }
        }
        
        let total_time = start_time.elapsed();
        println!(
            "Benchmark '{}' completed in {:?} ({:.2} ops/sec)",
            name,
            total_time,
            iterations as f64 / total_time.as_secs_f64()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiler_creation() {
        let config = ProfilerConfig::default();
        let profiler = PerformanceProfiler::new(config);
        assert_eq!(profiler.get_metrics().len(), 0);
    }

    #[test]
    fn test_metric_recording() {
        let config = ProfilerConfig::default();
        let profiler = PerformanceProfiler::new(config);
        
        profiler.record_metric(
            "test".to_string(),
            "operation".to_string(),
            Duration::from_millis(100),
        );
        
        let metrics = profiler.get_metrics();
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].subsystem, "test");
        assert_eq!(metrics[0].operation, "operation");
    }

    #[test]
    fn test_timer_operations() {
        let config = ProfilerConfig::default();
        let profiler = PerformanceProfiler::new(config);
        
        profiler.start_timer("test_timer".to_string());
        std::thread::sleep(Duration::from_millis(10));
        profiler.stop_timer("test_timer".to_string(), "test".to_string(), "operation".to_string());
        
        let metrics = profiler.get_metrics();
        assert_eq!(metrics.len(), 1);
        assert!(metrics[0].duration_ns > 0);
    }

    #[test]
    fn test_throughput_stats() {
        let config = ProfilerConfig::default();
        let profiler = PerformanceProfiler::new(config);
        
        // Record multiple metrics
        for _ in 0..10 {
            profiler.record_metric(
                "test".to_string(),
                "operation".to_string(),
                Duration::from_millis(100),
            );
        }
        
        let stats = profiler.get_throughput_stats("test", "operation");
        assert_eq!(stats.total_operations, 10);
        assert!(stats.operations_per_second > 0.0);
    }
} 