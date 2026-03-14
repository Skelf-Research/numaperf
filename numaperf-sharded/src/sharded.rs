//! Per-NUMA-node sharded data structure.

use std::sync::Arc;

use numaperf_core::NodeId;
use numaperf_topo::Topology;

use crate::padded::CachePadded;

/// A data structure with one shard per NUMA node.
///
/// `NumaSharded<T>` maintains a separate instance of `T` for each NUMA node,
/// allowing threads to access their local shard without cross-node contention.
/// Each shard is cache-padded to prevent false sharing.
///
/// # Thread Safety
///
/// `NumaSharded<T>` is `Send + Sync` if `T` is `Send + Sync`. For mutable access,
/// use a type with interior mutability like `Mutex<T>`, `RwLock<T>`, or atomic types.
///
/// # Example
///
/// ```no_run
/// use std::sync::Arc;
/// use std::sync::atomic::{AtomicU64, Ordering};
/// use numaperf_sharded::NumaSharded;
/// use numaperf_topo::Topology;
///
/// let topo = Arc::new(Topology::discover()?);
/// let shards = NumaSharded::new(&topo, || AtomicU64::new(0));
///
/// // Increment the local shard
/// shards.local(|counter| {
///     counter.fetch_add(1, Ordering::Relaxed);
/// });
///
/// // Sum across all shards
/// let total: u64 = shards.iter()
///     .map(|(_, counter)| counter.load(Ordering::Relaxed))
///     .sum();
/// # Ok::<(), numaperf_core::NumaError>(())
/// ```
pub struct NumaSharded<T> {
    /// Per-node shards, indexed by node ID.
    shards: Vec<CachePadded<T>>,
    /// Topology for CPU-to-node mapping.
    topo: Arc<Topology>,
}

impl<T> NumaSharded<T> {
    /// Create a new sharded structure with one shard per NUMA node.
    ///
    /// The `factory` function is called once per node to create each shard.
    pub fn new<F>(topo: &Arc<Topology>, factory: F) -> Self
    where
        F: Fn() -> T,
    {
        let num_nodes = topo.node_count();
        let shards = (0..num_nodes)
            .map(|_| CachePadded::new(factory()))
            .collect();

        Self {
            shards,
            topo: Arc::clone(topo),
        }
    }

    /// Access the shard for the current thread's NUMA node.
    ///
    /// The shard is determined by calling `sched_getcpu()` to get the current
    /// CPU and mapping it to a NUMA node. If the CPU cannot be determined,
    /// falls back to node 0.
    #[inline]
    pub fn local<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        let node = self.current_node();
        let shard = self.get(node).unwrap_or_else(|| {
            // Fallback to first shard if node not found
            self.shards.first().expect("shards cannot be empty").get()
        });
        f(shard)
    }

    /// Access a specific node's shard by node ID.
    ///
    /// Returns `None` if the node ID is out of range.
    #[inline]
    pub fn get(&self, node: NodeId) -> Option<&T> {
        let idx = node.as_u32() as usize;
        self.shards.get(idx).map(|p| p.get())
    }

    /// Get a mutable reference to a specific node's shard.
    ///
    /// This requires `&mut self`, so it's only usable when you have exclusive
    /// access to the entire `NumaSharded`. For concurrent mutable access,
    /// use interior mutability in `T`.
    #[inline]
    pub fn get_mut(&mut self, node: NodeId) -> Option<&mut T> {
        let idx = node.as_u32() as usize;
        self.shards.get_mut(idx).map(|p| p.get_mut())
    }

    /// Iterate over all shards with their node IDs.
    pub fn iter(&self) -> impl Iterator<Item = (NodeId, &T)> {
        self.shards
            .iter()
            .enumerate()
            .map(|(i, shard)| (NodeId::new(i as u32), shard.get()))
    }

    /// Iterate over all shards mutably with their node IDs.
    ///
    /// Requires exclusive access to the `NumaSharded`.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (NodeId, &mut T)> {
        self.shards
            .iter_mut()
            .enumerate()
            .map(|(i, shard)| (NodeId::new(i as u32), shard.get_mut()))
    }

    /// Get the number of shards (equals the number of NUMA nodes).
    #[inline]
    pub fn len(&self) -> usize {
        self.shards.len()
    }

    /// Check if there are no shards (always false for valid topologies).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.shards.is_empty()
    }

    /// Get the topology used by this sharded structure.
    #[inline]
    pub fn topology(&self) -> &Arc<Topology> {
        &self.topo
    }

    /// Determine the current thread's NUMA node.
    fn current_node(&self) -> NodeId {
        // Get current CPU using sched_getcpu()
        let cpu = unsafe { libc::sched_getcpu() };
        if cpu < 0 {
            // Fallback to node 0 if we can't determine the CPU
            return NodeId::new(0);
        }

        // Map CPU to NUMA node
        self.topo.node_for_cpu(cpu as u32).unwrap_or(NodeId::new(0))
    }
}

// Safety: NumaSharded<T> is Send if T is Send
unsafe impl<T: Send> Send for NumaSharded<T> {}

// Safety: NumaSharded<T> is Sync if T is Sync
unsafe impl<T: Sync> Sync for NumaSharded<T> {}

impl<T: std::fmt::Debug> std::fmt::Debug for NumaSharded<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NumaSharded")
            .field("num_shards", &self.shards.len())
            .field("shards", &self.shards.iter().map(|s| s.get()).collect::<Vec<_>>())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use numaperf_core::CpuSet;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn test_topology() -> Arc<Topology> {
        Arc::new(Topology::discover().unwrap_or_else(|_| {
            let cpus = CpuSet::parse("0-7").unwrap();
            Topology::single_node(cpus)
        }))
    }

    #[test]
    fn test_sharded_creation() {
        let topo = test_topology();
        let sharded = NumaSharded::new(&topo, || 42u64);

        assert_eq!(sharded.len(), topo.node_count());
        assert!(!sharded.is_empty());
    }

    #[test]
    fn test_sharded_get() {
        let topo = test_topology();
        let sharded = NumaSharded::new(&topo, || 42u64);

        // Node 0 should always exist
        assert_eq!(sharded.get(NodeId::new(0)), Some(&42u64));

        // Non-existent node
        assert_eq!(sharded.get(NodeId::new(100)), None);
    }

    #[test]
    fn test_sharded_local() {
        let topo = test_topology();
        let sharded = NumaSharded::new(&topo, || AtomicUsize::new(0));

        // Access local shard
        sharded.local(|counter| {
            counter.fetch_add(1, Ordering::SeqCst);
        });

        // At least one shard should have been incremented
        let sum: usize = sharded
            .iter()
            .map(|(_, c)| c.load(Ordering::SeqCst))
            .sum();
        assert_eq!(sum, 1);
    }

    #[test]
    fn test_sharded_iter() {
        let topo = test_topology();
        let sharded = NumaSharded::new(&topo, || 42u64);

        let items: Vec<_> = sharded.iter().collect();
        assert_eq!(items.len(), topo.node_count());

        for (node, &value) in items {
            assert!(node.as_u32() < topo.node_count() as u32);
            assert_eq!(value, 42);
        }
    }

    #[test]
    fn test_sharded_iter_mut() {
        let topo = test_topology();
        let mut sharded = NumaSharded::new(&topo, || 0u64);

        // Modify all shards
        for (_, value) in sharded.iter_mut() {
            *value = 100;
        }

        // Verify all were modified
        for (_, &value) in sharded.iter() {
            assert_eq!(value, 100);
        }
    }

    #[test]
    fn test_sharded_with_different_init_values() {
        let topo = test_topology();
        // Use AtomicUsize for thread-safe initialization with different values
        let counter = std::sync::atomic::AtomicUsize::new(0);
        let sharded = NumaSharded::new(&topo, || {
            counter.fetch_add(1, Ordering::SeqCst)
        });

        // Each shard should have a different value
        let values: Vec<_> = sharded.iter().map(|(_, &v)| v).collect();
        for (i, &v) in values.iter().enumerate() {
            assert_eq!(v, i);
        }
    }

    #[test]
    fn test_sharded_topology_access() {
        let topo = test_topology();
        let sharded = NumaSharded::new(&topo, || 0u64);

        assert_eq!(sharded.topology().node_count(), topo.node_count());
    }
}
