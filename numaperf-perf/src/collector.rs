//! Statistics collector using per-node sharded counters.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use numaperf_core::NodeId;
use numaperf_sharded::{NumaSharded, ShardedCounter};
use numaperf_topo::Topology;

use crate::stats::{LocalityStats, NodeStats};

/// Collects locality statistics using per-node sharded counters.
///
/// This is the live metrics collector that tracks local executions and
/// cross-node steals. Use `snapshot()` to get a point-in-time view of
/// the collected statistics.
///
/// # Thread Safety
///
/// `StatsCollector` is `Send + Sync` and designed for concurrent use.
/// All counter updates are lock-free atomic operations.
///
/// # Example
///
/// ```no_run
/// use std::sync::Arc;
/// use numaperf_perf::StatsCollector;
/// use numaperf_topo::Topology;
///
/// let topo = Arc::new(Topology::discover()?);
/// let collector = StatsCollector::new(&topo);
///
/// // Record metrics
/// collector.record_local_execution();
/// collector.record_steal(numaperf_core::NodeId::new(1));
///
/// // Get snapshot
/// let stats = collector.snapshot();
/// println!("Locality ratio: {:.1}%", stats.locality_ratio() * 100.0);
/// # Ok::<(), numaperf_core::NumaError>(())
/// ```
pub struct StatsCollector {
    /// Topology reference.
    topo: Arc<Topology>,
    /// Per-node: tasks executed locally (from own queue).
    local_executions: ShardedCounter,
    /// Per-node: tasks stolen from this node by others.
    /// Indexed by the victim node.
    tasks_stolen: NumaSharded<AtomicU64>,
    /// Per-node: steals this node performed (tasks taken from others).
    steals_performed: ShardedCounter,
}

impl StatsCollector {
    /// Create a new statistics collector.
    pub fn new(topo: &Arc<Topology>) -> Self {
        Self {
            topo: Arc::clone(topo),
            local_executions: ShardedCounter::new(topo),
            tasks_stolen: NumaSharded::new(topo, || AtomicU64::new(0)),
            steals_performed: ShardedCounter::new(topo),
        }
    }

    /// Record a local execution on the current node.
    ///
    /// Call this when a task is executed on its home node without
    /// being stolen.
    #[inline]
    pub fn record_local_execution(&self) {
        self.local_executions.increment();
    }

    /// Record multiple local executions on the current node.
    #[inline]
    pub fn record_local_executions(&self, count: u64) {
        self.local_executions.add(count);
    }

    /// Record that a task was stolen from `from_node`.
    ///
    /// This increments:
    /// - `steals_performed` on the current (stealing) node
    /// - `tasks_stolen` on the victim (`from_node`)
    #[inline]
    pub fn record_steal(&self, from_node: NodeId) {
        // Increment steals_performed on the current node
        self.steals_performed.increment();

        // Increment tasks_stolen on the victim node
        if let Some(counter) = self.tasks_stolen.get(from_node) {
            counter.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Record multiple steals from a specific node.
    #[inline]
    pub fn record_steals(&self, from_node: NodeId, count: u64) {
        self.steals_performed.add(count);

        if let Some(counter) = self.tasks_stolen.get(from_node) {
            counter.fetch_add(count, Ordering::Relaxed);
        }
    }

    /// Take a snapshot of current statistics.
    ///
    /// This reads all counters and returns a consistent point-in-time
    /// view. The snapshot is independent of the collector and can be
    /// used after the collector is modified.
    pub fn snapshot(&self) -> LocalityStats {
        let mut node_stats = Vec::with_capacity(self.topo.node_count());

        for (node_id, local) in self.local_executions.iter() {
            let tasks_stolen = self
                .tasks_stolen
                .get(node_id)
                .map(|c| c.load(Ordering::Relaxed))
                .unwrap_or(0);

            let steals_performed = self.steals_performed.node_count(node_id).unwrap_or(0);

            let stats = NodeStats {
                node_id,
                local_executions: local,
                tasks_stolen,
                steals_performed,
                queue_depth: 0, // Not tracked by collector
            };

            node_stats.push(stats);
        }

        LocalityStats::new(node_stats)
    }

    /// Reset all counters to zero.
    pub fn reset(&self) {
        self.local_executions.reset();
        self.steals_performed.reset();

        for (_, counter) in self.tasks_stolen.iter() {
            counter.store(0, Ordering::Relaxed);
        }
    }

    /// Get the topology.
    pub fn topology(&self) -> &Arc<Topology> {
        &self.topo
    }

    /// Get total local executions across all nodes.
    pub fn total_local_executions(&self) -> u64 {
        self.local_executions.sum()
    }

    /// Get total steals performed across all nodes.
    pub fn total_steals(&self) -> u64 {
        self.steals_performed.sum()
    }
}

// Safety: StatsCollector is Send + Sync because all fields are
unsafe impl Send for StatsCollector {}
unsafe impl Sync for StatsCollector {}

impl std::fmt::Debug for StatsCollector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StatsCollector")
            .field("node_count", &self.topo.node_count())
            .field("local_executions", &self.local_executions.sum())
            .field("steals_performed", &self.steals_performed.sum())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use numaperf_core::CpuSet;
    use std::thread;

    fn test_topology() -> Arc<Topology> {
        Arc::new(Topology::discover().unwrap_or_else(|_| {
            let cpus = CpuSet::parse("0-7").unwrap();
            Topology::single_node(cpus)
        }))
    }

    #[test]
    fn test_collector_creation() {
        let topo = test_topology();
        let collector = StatsCollector::new(&topo);

        assert_eq!(collector.total_local_executions(), 0);
        assert_eq!(collector.total_steals(), 0);
    }

    #[test]
    fn test_record_local_execution() {
        let topo = test_topology();
        let collector = StatsCollector::new(&topo);

        collector.record_local_execution();
        collector.record_local_execution();

        let stats = collector.snapshot();
        assert_eq!(stats.local_executions(), 2);
        assert_eq!(stats.remote_steals(), 0);
    }

    #[test]
    fn test_record_local_executions_batch() {
        let topo = test_topology();
        let collector = StatsCollector::new(&topo);

        collector.record_local_executions(100);

        let stats = collector.snapshot();
        assert_eq!(stats.local_executions(), 100);
    }

    #[test]
    fn test_record_steal() {
        let topo = test_topology();
        let collector = StatsCollector::new(&topo);

        // Steal from node 0
        collector.record_steal(NodeId::new(0));

        let stats = collector.snapshot();
        assert_eq!(stats.remote_steals(), 1);

        // Node 0 should show tasks_stolen = 1
        if let Some(node_stats) = stats.node(NodeId::new(0)) {
            assert_eq!(node_stats.tasks_stolen, 1);
        }
    }

    #[test]
    fn test_reset() {
        let topo = test_topology();
        let collector = StatsCollector::new(&topo);

        collector.record_local_executions(100);
        collector.record_steal(NodeId::new(0));

        collector.reset();

        let stats = collector.snapshot();
        assert_eq!(stats.local_executions(), 0);
        assert_eq!(stats.remote_steals(), 0);
    }

    #[test]
    fn test_snapshot_independence() {
        let topo = test_topology();
        let collector = StatsCollector::new(&topo);

        collector.record_local_executions(50);
        let stats1 = collector.snapshot();

        collector.record_local_executions(50);
        let stats2 = collector.snapshot();

        // Snapshots should be independent
        assert_eq!(stats1.local_executions(), 50);
        assert_eq!(stats2.local_executions(), 100);
    }

    #[test]
    fn test_concurrent_recording() {
        let topo = test_topology();
        let collector = Arc::new(StatsCollector::new(&topo));

        let mut handles = vec![];
        for _ in 0..4 {
            let c = Arc::clone(&collector);
            handles.push(thread::spawn(move || {
                for _ in 0..1000 {
                    c.record_local_execution();
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        let stats = collector.snapshot();
        assert_eq!(stats.local_executions(), 4000);
    }

    #[test]
    fn test_locality_ratio() {
        let topo = test_topology();
        let collector = StatsCollector::new(&topo);

        collector.record_local_executions(90);
        for _ in 0..10 {
            collector.record_steal(NodeId::new(0));
        }

        let stats = collector.snapshot();
        let ratio = stats.locality_ratio();
        assert!((ratio - 0.9).abs() < 0.001, "Expected ~0.9, got {}", ratio);
    }
}
