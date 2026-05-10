//! Per-node buffer pool pattern.
//!
//! This example demonstrates a common pattern: allocating per-node
//! buffer pools for NUMA-local data access.
//!
//! Run with: cargo run -p numaperf --example buffer_pool

use numaperf::{
    CachePadded, MemPolicy, NodeId, NodeMask, NumaRegion, NumaSharded, Prefault, Topology,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// A per-node buffer pool with NUMA-local allocation.
struct NodeBufferPool {
    /// The NUMA node this pool belongs to.
    node_id: NodeId,
    /// The allocated memory region.
    region: NumaRegion,
    /// Allocation offset (for simple bump allocation).
    offset: AtomicUsize,
}

impl NodeBufferPool {
    /// Create a new buffer pool for a specific node.
    fn new(node_id: NodeId, size: usize) -> Result<Self, numaperf::NumaError> {
        let nodes = NodeMask::single(node_id);
        let region = NumaRegion::anon(
            size,
            MemPolicy::Bind(nodes),
            Default::default(),
            Prefault::Touch,
        )?;

        Ok(Self {
            node_id,
            region,
            offset: AtomicUsize::new(0),
        })
    }

    /// Allocate bytes from this pool (simple bump allocator).
    #[allow(clippy::mut_from_ref)]
    fn alloc(&self, size: usize) -> Option<&mut [u8]> {
        let offset = self.offset.fetch_add(size, Ordering::SeqCst);
        if offset + size <= self.region.len() {
            // Safety: we have exclusive access via atomic offset
            let slice = unsafe {
                std::slice::from_raw_parts_mut(self.region.as_ptr().add(offset) as *mut u8, size)
            };
            Some(slice)
        } else {
            None
        }
    }

    /// Get remaining capacity.
    fn remaining(&self) -> usize {
        let offset = self.offset.load(Ordering::SeqCst);
        self.region.len().saturating_sub(offset)
    }

    /// Reset the pool (reuse memory).
    fn reset(&self) {
        self.offset.store(0, Ordering::SeqCst);
    }
}

fn main() -> Result<(), numaperf::NumaError> {
    println!("=== numaperf: Buffer Pool Example ===\n");

    // Discover topology
    let topo = Arc::new(Topology::discover()?);
    println!("Creating buffer pools for {} NUMA nodes", topo.node_count());
    println!();

    let pool_size = 1024 * 1024; // 1 MB per node

    // Create per-node buffer pools using NumaSharded
    let pools = NumaSharded::new(&topo, || {
        // This closure runs for each node, but we need the node ID
        // For this pattern, we'll create pools manually
        AtomicUsize::new(0) // Placeholder
    });

    // Actually create node-local pools
    let mut node_pools = Vec::new();
    for node in topo.numa_nodes() {
        let pool = NodeBufferPool::new(node.id(), pool_size)?;
        println!(
            "  Node {}: {} bytes allocated",
            node.id(),
            pool.region.len()
        );
        node_pools.push(CachePadded::new(pool));
    }
    println!();

    // Demonstrate allocation from pools
    println!("Allocating from pools:");
    for (i, pool) in node_pools.iter().enumerate() {
        // Allocate some data
        if let Some(buf) = pool.alloc(1024) {
            buf.fill(i as u8);
            println!(
                "  Node {}: allocated 1024 bytes, {} remaining",
                pool.node_id,
                pool.remaining()
            );
        }
    }
    println!();

    // Demonstrate sharded counter for tracking
    println!("Using sharded counter for per-node statistics:");
    let counters = NumaSharded::new(&topo, || AtomicUsize::new(0));

    // Simulate work that updates node-local counters
    for i in 0..10 {
        let node_idx = i % topo.node_count();
        if let Some(counter) = counters.get(topo.numa_nodes()[node_idx].id()) {
            counter.fetch_add(1, Ordering::SeqCst);
        }
    }

    // Read counters
    let mut total = 0;
    for (node_id, counter) in counters.iter() {
        let count = counter.load(Ordering::SeqCst);
        println!("  Node {}: {} operations", node_id, count);
        total += count;
    }
    println!("  Total: {} operations", total);
    println!();

    // Reset and reuse pools
    println!("Resetting pools for reuse...");
    for pool in &node_pools {
        pool.reset();
        println!(
            "  Node {}: {} bytes available",
            pool.node_id,
            pool.remaining()
        );
    }

    // Drop pools to use for statistics
    drop(pools);

    println!();
    println!("Buffer pool example complete.");

    Ok(())
}
