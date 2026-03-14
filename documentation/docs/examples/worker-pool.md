# Worker Pool Example

NUMA-aware parallel task execution with per-node worker pools.

## Overview

This example demonstrates:

- Creating a NUMA-aware executor
- Configuring work stealing policies
- Submitting tasks to specific nodes
- Measuring throughput

## Running the Example

```bash
cargo run -p numaperf --example worker_pool
```

## Full Source Code

```rust
use numaperf::{Capabilities, NumaExecutor, StealPolicy, Topology};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

fn main() -> Result<(), numaperf::NumaError> {
    println!("=== numaperf: Worker Pool Example ===\n");

    // Check capabilities
    let caps = Capabilities::detect();
    println!("NUMA nodes detected: {}", caps.numa_node_count);
    println!("Hard mode supported: {}", caps.supports_hard_mode());
    println!();

    // Discover topology
    let topo = Arc::new(Topology::discover()?);
    println!("Creating executor with {} nodes", topo.node_count());

    // Create executor with 2 workers per node
    let exec = NumaExecutor::builder(Arc::clone(&topo))
        .steal_policy(StealPolicy::LocalThenSocketThenRemote)
        .workers_per_node(2)
        .build()?;

    println!("Executor created:");
    println!("  Total workers: {}", exec.worker_count());
    println!("  Steal policy: {}", exec.steal_policy());
    println!();

    // Track work completion per node
    let total_tasks = 100;
    let completed = Arc::new(AtomicUsize::new(0));

    println!("Submitting {} tasks...", total_tasks);
    let start = Instant::now();

    // Distribute tasks across nodes
    for i in 0..total_tasks {
        let node_idx = i % topo.node_count();
        let node_id = topo.numa_nodes()[node_idx].id();
        let c = Arc::clone(&completed);

        exec.submit_to_node(node_id, move || {
            // Simulate some work
            let sum: u64 = (0..1000).sum();
            std::hint::black_box(sum);

            c.fetch_add(1, Ordering::SeqCst);
        });
    }

    println!("All tasks submitted, waiting for completion...");

    // Graceful shutdown waits for all tasks
    exec.shutdown();

    let elapsed = start.elapsed();
    let completed_count = completed.load(Ordering::SeqCst);

    println!();
    println!("Results:");
    println!("  Completed: {} tasks", completed_count);
    println!("  Time: {:?}", elapsed);
    println!(
        "  Throughput: {:.0} tasks/sec",
        completed_count as f64 / elapsed.as_secs_f64()
    );

    Ok(())
}
```

## Step-by-Step Walkthrough

### 1. Create the Executor

```rust
let topo = Arc::new(Topology::discover()?);

let exec = NumaExecutor::builder(Arc::clone(&topo))
    .steal_policy(StealPolicy::LocalThenSocketThenRemote)
    .workers_per_node(2)
    .build()?;
```

The builder configures:

| Option | Description |
|--------|-------------|
| `steal_policy()` | How workers steal from other nodes |
| `workers_per_node()` | Number of workers per NUMA node |
| `hard_mode()` | Whether to enforce strict pinning |

### 2. Choose a Steal Policy

```rust
.steal_policy(StealPolicy::LocalThenSocketThenRemote)
```

Available policies:

| Policy | Behavior | Use Case |
|--------|----------|----------|
| `LocalOnly` | Never steal from other nodes | Strict locality |
| `LocalThenSocketThenRemote` | Steal nearby first | General purpose |
| `Any` | Steal from any node | Maximum throughput |

### 3. Submit Work to Nodes

```rust
exec.submit_to_node(node_id, move || {
    // Work runs on a worker pinned to this node
    process_data();
});
```

The closure:

- Runs on a worker thread pinned to the specified node
- Has access to NUMA-local memory on that node
- May be stolen by another node if policy allows

### 4. Distribute Tasks

```rust
for i in 0..total_tasks {
    let node_idx = i % topo.node_count();
    let node_id = topo.numa_nodes()[node_idx].id();

    exec.submit_to_node(node_id, move || {
        // ...
    });
}
```

Round-robin distribution ensures balanced load across nodes.

### 5. Graceful Shutdown

```rust
exec.shutdown();
```

Shutdown:

- Stops accepting new work
- Waits for all pending work to complete
- Returns when all workers have stopped

## Sample Output

```
=== numaperf: Worker Pool Example ===

NUMA nodes detected: 2
Hard mode supported: true

Creating executor with 2 nodes
Executor created:
  Total workers: 4
  Steal policy: LocalThenSocketThenRemote

Submitting 100 tasks...
All tasks submitted, waiting for completion...

Results:
  Completed: 100 tasks
  Time: 1.234ms
  Throughput: 81037 tasks/sec
```

## Patterns

### Data-Local Processing

```rust
// Process data on the node where it's allocated
let data = Arc::new(NumaRegion::anon(
    size,
    MemPolicy::Bind(NodeMask::single(node_id)),
    ...
)?);

exec.submit_to_node(node_id, move || {
    // Access data with minimal latency
    process(data.as_slice());
});
```

### Capture Shared State

```rust
let shared_counter = Arc::new(AtomicUsize::new(0));

for i in 0..tasks {
    let counter = Arc::clone(&shared_counter);

    exec.submit_to_node(node_id, move || {
        counter.fetch_add(1, Ordering::SeqCst);
    });
}
```

### Device-Affine Processing

```rust
let devices = DeviceMap::discover(topo)?;
let nic_node = devices.device_node("eth0").unwrap_or(NodeId::new(0));

// Process packets on the NIC's local node
exec.submit_to_node(nic_node, || {
    process_packet(packet);
});
```

## Tuning Tips

1. **Workers per node**: Start with 1 per core, tune based on workload
2. **Steal policy**: Use `LocalThenSocketThenRemote` unless you have specific needs
3. **Task granularity**: Make tasks large enough to amortize scheduling overhead
4. **Memory locality**: Allocate data on the same node where it will be processed

## Next Steps

- [Memory Placement Example](memory-placement.md) - Allocate NUMA-local buffers
- [Buffer Pool Example](buffer-pool.md) - Per-node buffer management
- [Diagnostics Example](diagnostics.md) - Monitor locality effectiveness
