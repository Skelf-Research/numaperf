# Quickstart

This guide will help you build your first NUMA-aware application in 5 minutes.

## Step 1: Check Your System

First, let's see what NUMA capabilities your system has:

```rust
use numaperf::Capabilities;

fn main() {
    let caps = Capabilities::detect();
    println!("{}", caps.summary());
}
```

Output on a NUMA system:

```
NUMA System Capabilities
========================
NUMA nodes detected: 2
CAP_SYS_ADMIN (strict memory binding): no
CAP_SYS_NICE (strict CPU affinity): yes
CAP_IPC_LOCK (memory locking): no
NUMA balancing disabled: no

Hard mode supported: NO
```

## Step 2: Discover Topology

Discover your system's NUMA layout:

```rust
use numaperf::Topology;

fn main() -> Result<(), numaperf::NumaError> {
    let topo = Topology::discover()?;

    println!("System has {} NUMA nodes", topo.node_count());

    for node in topo.numa_nodes() {
        println!("  Node {}: {} CPUs, {:?} MB memory",
            node.id().as_u32(),
            node.cpu_count(),
            node.memory_mb());
    }

    Ok(())
}
```

## Step 3: Create a NUMA-Aware Worker Pool

The executor provides per-node worker pools with configurable work stealing:

```rust
use numaperf::{NumaExecutor, Topology, StealPolicy};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

fn main() -> Result<(), numaperf::NumaError> {
    let topo = Arc::new(Topology::discover()?);

    // Create executor with 2 workers per node
    let exec = NumaExecutor::builder(Arc::clone(&topo))
        .steal_policy(StealPolicy::LocalThenSocketThenRemote)
        .workers_per_node(2)
        .build()?;

    // Track work completion
    let counter = Arc::new(AtomicUsize::new(0));

    // Submit work to each node
    for node in topo.numa_nodes() {
        let c = Arc::clone(&counter);
        exec.submit_to_node(node.id(), move || {
            // This runs on a worker pinned to the target node
            c.fetch_add(1, Ordering::SeqCst);
        });
    }

    exec.shutdown();
    println!("Completed {} tasks", counter.load(Ordering::SeqCst));
    Ok(())
}
```

## Step 4: Allocate NUMA-Local Memory

Allocate memory with explicit placement policies:

```rust
use numaperf::{NumaRegion, MemPolicy, NodeMask, NodeId, Prefault};

fn main() -> Result<(), numaperf::NumaError> {
    // Allocate 1MB bound to node 0
    let nodes = NodeMask::single(NodeId::new(0));
    let mut region = NumaRegion::anon(
        1024 * 1024,                    // Size in bytes
        MemPolicy::Bind(nodes),         // Strict binding to node 0
        Default::default(),             // Default huge page settings
        Prefault::Touch,                // Fault in pages immediately
    )?;

    // Use the memory
    let slice = region.as_mut_slice();
    slice[0] = 42;

    println!("Allocated {} bytes on node 0", slice.len());
    Ok(())
}
```

## Step 5: Pin Threads for Locality

Pin threads to specific CPUs before allocating memory:

```rust
use numaperf::{ScopedPin, CpuSet};

fn main() -> Result<(), numaperf::NumaError> {
    let cpus = CpuSet::parse("0-3").expect("valid CPU set");

    {
        // Pin to CPUs 0-3
        let _pin = ScopedPin::pin_current(cpus)?;

        // Memory allocated here will be local to this node
        let data: Vec<u8> = vec![0; 1024 * 1024];
        println!("Allocated {} bytes while pinned", data.len());

        // Do work with the data...
    }
    // Previous affinity is automatically restored

    Ok(())
}
```

## Step 6: Monitor Locality

Track how well your application maintains locality:

```rust
use numaperf::{StatsCollector, LocalityReport, Topology, NodeId};
use std::sync::Arc;

fn main() -> Result<(), numaperf::NumaError> {
    let topo = Arc::new(Topology::discover()?);
    let collector = StatsCollector::new(&topo);

    // Simulate some work
    collector.record_local_execution();
    collector.record_local_execution();
    collector.record_steal(NodeId::new(0));

    // Check metrics
    let stats = collector.snapshot();
    println!("Locality ratio: {:.1}%", stats.locality_ratio() * 100.0);

    // Generate diagnostic report
    let report = LocalityReport::generate(&stats);
    println!("{}", report);

    Ok(())
}
```

## Complete Example

Here's a complete example combining multiple features:

```rust
use numaperf::{
    Topology, NumaExecutor, StealPolicy, StatsCollector,
    LocalityReport, NumaRegion, MemPolicy, Prefault,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

fn main() -> Result<(), numaperf::NumaError> {
    // 1. Discover topology
    let topo = Arc::new(Topology::discover()?);
    println!("Running on {} NUMA nodes", topo.node_count());

    // 2. Create executor
    let exec = NumaExecutor::builder(Arc::clone(&topo))
        .steal_policy(StealPolicy::LocalThenSocketThenRemote)
        .workers_per_node(2)
        .build()?;

    // 3. Create stats collector
    let stats = Arc::new(StatsCollector::new(&topo));

    // 4. Submit work
    let counter = Arc::new(AtomicU64::new(0));

    for _ in 0..100 {
        for node in topo.numa_nodes() {
            let c = Arc::clone(&counter);
            let s = Arc::clone(&stats);

            exec.submit_to_node(node.id(), move || {
                s.record_local_execution();
                c.fetch_add(1, Ordering::Relaxed);
            });
        }
    }

    exec.shutdown();

    // 5. Report results
    let snapshot = stats.snapshot();
    println!("\nCompleted {} tasks", counter.load(Ordering::Relaxed));
    println!("Locality ratio: {:.1}%", snapshot.locality_ratio() * 100.0);

    let report = LocalityReport::generate(&snapshot);
    println!("\n{}", report);

    Ok(())
}
```

## Next Steps

- [System Requirements](system-requirements.md) - Detailed platform requirements
- [NUMA Basics](../concepts/numa-basics.md) - Understand NUMA fundamentals
- [Thread Pinning Guide](../guides/thread-pinning.md) - Deep dive into affinity
- [Memory Allocation Guide](../guides/memory-allocation.md) - Memory placement strategies
