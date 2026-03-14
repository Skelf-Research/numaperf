# Buffer Pool Example

Per-node buffer pools for NUMA-local data access.

## Overview

This example demonstrates:

- Creating per-node buffer pools
- NUMA-local memory allocation
- Simple bump allocator pattern
- Using `NumaSharded` for per-node state
- Cache padding to prevent false sharing

## Running the Example

```bash
cargo run -p numaperf --example buffer_pool
```

## Full Source Code

```rust
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
            Prefault::Touch
        )?;

        Ok(Self {
            node_id,
            region,
            offset: AtomicUsize::new(0),
        })
    }

    /// Allocate bytes from this pool (simple bump allocator).
    fn alloc(&self, size: usize) -> Option<&mut [u8]> {
        let offset = self.offset.fetch_add(size, Ordering::SeqCst);
        if offset + size <= self.region.len() {
            // Safety: we have exclusive access via atomic offset
            let slice = unsafe {
                std::slice::from_raw_parts_mut(
                    self.region.as_ptr().add(offset) as *mut u8,
                    size,
                )
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

    // Create per-node buffer pools
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

    println!();
    println!("Buffer pool example complete.");

    Ok(())
}
```

## Key Concepts

### Per-Node Buffer Pools

Each NUMA node gets its own buffer pool with memory bound to that node:

```rust
fn new(node_id: NodeId, size: usize) -> Result<Self, NumaError> {
    let nodes = NodeMask::single(node_id);
    let region = NumaRegion::anon(
        size,
        MemPolicy::Bind(nodes),  // Strictly on this node
        Default::default(),
        Prefault::Touch          // Fault pages immediately
    )?;
    // ...
}
```

### Bump Allocator

A simple lock-free bump allocator for fast allocation:

```rust
fn alloc(&self, size: usize) -> Option<&mut [u8]> {
    let offset = self.offset.fetch_add(size, Ordering::SeqCst);
    if offset + size <= self.region.len() {
        // Return slice at offset
    } else {
        None  // Out of space
    }
}
```

Benefits:

- Lock-free (atomic offset)
- O(1) allocation
- No fragmentation (bump only)
- Bulk reset for reuse

### Cache Padding

Prevent false sharing between pools:

```rust
node_pools.push(CachePadded::new(pool));
```

`CachePadded<T>` ensures each pool occupies its own cache line (128 bytes), preventing cache line bouncing when different threads access different pools.

### NumaSharded for Per-Node State

```rust
let counters = NumaSharded::new(&topo, || AtomicUsize::new(0));

// Access the local node's counter (fast path)
counters.local(|counter| {
    counter.fetch_add(1, Ordering::Relaxed);
});

// Access a specific node's counter
counters.get(node_id).map(|counter| {
    counter.fetch_add(1, Ordering::SeqCst);
});
```

## Sample Output

```
=== numaperf: Buffer Pool Example ===

Creating buffer pools for 2 NUMA nodes

  Node 0: 1048576 bytes allocated
  Node 1: 1048576 bytes allocated

Allocating from pools:
  Node 0: allocated 1024 bytes, 1047552 remaining
  Node 1: allocated 1024 bytes, 1047552 remaining

Using sharded counter for per-node statistics:
  Node 0: 5 operations
  Node 1: 5 operations
  Total: 10 operations

Resetting pools for reuse...
  Node 0: 1048576 bytes available
  Node 1: 1048576 bytes available

Buffer pool example complete.
```

## Patterns

### Thread-Local Pool Access

```rust
// In executor workers, access the local pool
let pool = &node_pools[current_node_index()];
let buf = pool.alloc(size)?;
```

### Pool with Stats Tracking

```rust
struct TrackedPool {
    pool: NodeBufferPool,
    allocations: AtomicUsize,
    bytes_allocated: AtomicUsize,
}

impl TrackedPool {
    fn alloc(&self, size: usize) -> Option<&mut [u8]> {
        let result = self.pool.alloc(size);
        if result.is_some() {
            self.allocations.fetch_add(1, Ordering::Relaxed);
            self.bytes_allocated.fetch_add(size, Ordering::Relaxed);
        }
        result
    }
}
```

### Epoch-Based Reset

```rust
// Instead of individual allocations, use epochs
struct EpochPool {
    pools: [NodeBufferPool; 2],
    current_epoch: AtomicUsize,
}

impl EpochPool {
    fn flip(&self) {
        // Reset old pool and switch epochs
        let old = self.current_epoch.fetch_xor(1, Ordering::SeqCst);
        self.pools[old].reset();
    }
}
```

## When to Use This Pattern

- **High-frequency allocations**: Bump allocation is faster than malloc
- **Deterministic latency**: No heap fragmentation or allocation jitter
- **NUMA-aware data processing**: Data stays on the node where it's processed
- **Network packet processing**: Allocate buffers on the NIC's local node

## Next Steps

- [Diagnostics Example](diagnostics.md) - Monitor locality effectiveness
- [Memory Placement Example](memory-placement.md) - Understand memory policies
