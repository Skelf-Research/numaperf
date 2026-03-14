//! NUMA-aware sharded counter.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use numaperf_core::NodeId;
use numaperf_topo::Topology;

use crate::sharded::NumaSharded;

/// A NUMA-aware atomic counter with per-node shards.
///
/// `ShardedCounter` maintains a separate atomic counter for each NUMA node,
/// reducing contention when multiple threads increment the counter concurrently.
/// The total count is the sum of all per-node counters.
///
/// # Example
///
/// ```no_run
/// use std::sync::Arc;
/// use numaperf_sharded::ShardedCounter;
/// use numaperf_topo::Topology;
///
/// let topo = Arc::new(Topology::discover()?);
/// let counter = ShardedCounter::new(&topo);
///
/// // Increment from any thread - uses local shard
/// counter.increment();
/// counter.add(10);
///
/// // Get total count across all shards
/// println!("Total: {}", counter.sum());
/// # Ok::<(), numaperf_core::NumaError>(())
/// ```
///
/// # Performance
///
/// The per-node sharding reduces cache-line bouncing when multiple threads
/// on different NUMA nodes increment the counter simultaneously. Each node's
/// counter stays in that node's local memory.
pub struct ShardedCounter {
    shards: NumaSharded<AtomicU64>,
}

impl ShardedCounter {
    /// Create a new sharded counter initialized to zero.
    pub fn new(topo: &Arc<Topology>) -> Self {
        Self {
            shards: NumaSharded::new(topo, || AtomicU64::new(0)),
        }
    }

    /// Increment the counter by 1.
    ///
    /// Uses the local NUMA node's shard for the increment.
    #[inline]
    pub fn increment(&self) {
        self.add(1);
    }

    /// Add a value to the counter.
    ///
    /// Uses the local NUMA node's shard for the addition.
    #[inline]
    pub fn add(&self, n: u64) {
        self.shards.local(|counter| {
            counter.fetch_add(n, Ordering::Relaxed);
        });
    }

    /// Subtract a value from the counter.
    ///
    /// Uses the local NUMA node's shard for the subtraction.
    #[inline]
    pub fn sub(&self, n: u64) {
        self.shards.local(|counter| {
            counter.fetch_sub(n, Ordering::Relaxed);
        });
    }

    /// Get the sum of all shards.
    ///
    /// This performs a relaxed read of each shard, so the result may not
    /// reflect concurrent modifications. For a consistent snapshot, external
    /// synchronization is required.
    pub fn sum(&self) -> u64 {
        self.shards
            .iter()
            .map(|(_, counter)| counter.load(Ordering::Relaxed))
            .sum()
    }

    /// Get the count for a specific node.
    pub fn node_count(&self, node: NodeId) -> Option<u64> {
        self.shards.get(node).map(|c| c.load(Ordering::Relaxed))
    }

    /// Reset all shards to zero.
    pub fn reset(&self) {
        for (_, counter) in self.shards.iter() {
            counter.store(0, Ordering::Relaxed);
        }
    }

    /// Get the number of shards (equals node count).
    #[inline]
    pub fn num_shards(&self) -> usize {
        self.shards.len()
    }

    /// Iterate over per-node counts.
    pub fn iter(&self) -> impl Iterator<Item = (NodeId, u64)> + '_ {
        self.shards
            .iter()
            .map(|(node, counter)| (node, counter.load(Ordering::Relaxed)))
    }
}

impl std::fmt::Debug for ShardedCounter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShardedCounter")
            .field("sum", &self.sum())
            .field("num_shards", &self.num_shards())
            .finish()
    }
}

impl std::fmt::Display for ShardedCounter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.sum())
    }
}

// ShardedCounter is always Send + Sync because AtomicU64 is Send + Sync
unsafe impl Send for ShardedCounter {}
unsafe impl Sync for ShardedCounter {}

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
    fn test_counter_creation() {
        let topo = test_topology();
        let counter = ShardedCounter::new(&topo);

        assert_eq!(counter.sum(), 0);
        assert_eq!(counter.num_shards(), topo.node_count());
    }

    #[test]
    fn test_counter_increment() {
        let topo = test_topology();
        let counter = ShardedCounter::new(&topo);

        counter.increment();
        assert_eq!(counter.sum(), 1);

        counter.increment();
        counter.increment();
        assert_eq!(counter.sum(), 3);
    }

    #[test]
    fn test_counter_add() {
        let topo = test_topology();
        let counter = ShardedCounter::new(&topo);

        counter.add(10);
        assert_eq!(counter.sum(), 10);

        counter.add(5);
        assert_eq!(counter.sum(), 15);
    }

    #[test]
    fn test_counter_sub() {
        let topo = test_topology();
        let counter = ShardedCounter::new(&topo);

        counter.add(100);
        counter.sub(30);
        assert_eq!(counter.sum(), 70);
    }

    #[test]
    fn test_counter_reset() {
        let topo = test_topology();
        let counter = ShardedCounter::new(&topo);

        counter.add(100);
        assert_eq!(counter.sum(), 100);

        counter.reset();
        assert_eq!(counter.sum(), 0);
    }

    #[test]
    fn test_counter_node_count() {
        let topo = test_topology();
        let counter = ShardedCounter::new(&topo);

        // Increment local shard
        counter.add(42);

        // At least node 0 should exist
        let node0_count = counter.node_count(NodeId::new(0));
        assert!(node0_count.is_some());

        // Non-existent node
        assert_eq!(counter.node_count(NodeId::new(100)), None);
    }

    #[test]
    fn test_counter_iter() {
        let topo = test_topology();
        let counter = ShardedCounter::new(&topo);

        counter.add(10);

        let counts: Vec<_> = counter.iter().collect();
        assert_eq!(counts.len(), topo.node_count());

        // Sum via iter should match sum()
        let iter_sum: u64 = counts.iter().map(|(_, c)| c).sum();
        assert_eq!(iter_sum, counter.sum());
    }

    #[test]
    fn test_counter_concurrent() {
        let topo = test_topology();
        let counter = Arc::new(ShardedCounter::new(&topo));

        let mut handles = vec![];
        for _ in 0..4 {
            let c = Arc::clone(&counter);
            handles.push(thread::spawn(move || {
                for _ in 0..1000 {
                    c.increment();
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(counter.sum(), 4000);
    }

    #[test]
    fn test_counter_display() {
        let topo = test_topology();
        let counter = ShardedCounter::new(&topo);

        counter.add(42);
        assert_eq!(format!("{}", counter), "42");
    }
}
