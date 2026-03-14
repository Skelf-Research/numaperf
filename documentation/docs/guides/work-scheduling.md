# Work Scheduling

Learn how to use the NUMA-aware executor for work distribution.

## Basic Usage

```rust
use numaperf::{NumaExecutor, Topology, StealPolicy};
use std::sync::Arc;

let topo = Arc::new(Topology::discover()?);

// Create executor
let exec = NumaExecutor::builder(Arc::clone(&topo))
    .steal_policy(StealPolicy::LocalThenSocketThenRemote)
    .workers_per_node(2)
    .build()?;

// Submit work
exec.submit_to_node(NodeId::new(0), || {
    println!("Running on node 0!");
});

// Shutdown when done
exec.shutdown();
```

## Steal Policies

### LocalOnly

Workers never steal from other nodes:

```rust
let exec = NumaExecutor::builder(topo)
    .steal_policy(StealPolicy::LocalOnly)
    .build()?;
```

- **Best locality** - Work always runs on submitted node
- **Risk**: Idle workers if load is imbalanced

### LocalThenSocketThenRemote (Default)

Tiered stealing - prefer nearby nodes:

```rust
let exec = NumaExecutor::builder(topo)
    .steal_policy(StealPolicy::LocalThenSocketThenRemote)
    .build()?;
```

- **Balanced** - Good locality with load balancing
- **Recommended** for most workloads

### Any

Steal from any node:

```rust
let exec = NumaExecutor::builder(topo)
    .steal_policy(StealPolicy::Any)
    .build()?;
```

- **Best throughput** - No idle workers
- **Worst locality** - Work may run far from data

## Submitting Work

### To Specific Node

```rust
exec.submit_to_node(NodeId::new(0), || {
    // Runs on a worker pinned to node 0
});
```

### Distribute Across Nodes

```rust
for (i, node) in topo.numa_nodes().iter().enumerate() {
    let data = get_data_for_node(i);

    exec.submit_to_node(node.id(), move || {
        process(data);
    });
}
```

## Tracking Completion

### With Atomics

```rust
use std::sync::atomic::{AtomicUsize, Ordering};

let counter = Arc::new(AtomicUsize::new(0));

for _ in 0..100 {
    let c = Arc::clone(&counter);
    exec.submit_to_node(node_id, move || {
        // Do work...
        c.fetch_add(1, Ordering::SeqCst);
    });
}

exec.shutdown();
println!("Completed: {}", counter.load(Ordering::SeqCst));
```

### With Channels

```rust
use std::sync::mpsc;

let (tx, rx) = mpsc::channel();

for i in 0..100 {
    let tx = tx.clone();
    exec.submit_to_node(node_id, move || {
        let result = compute(i);
        tx.send(result).unwrap();
    });
}
drop(tx);  // Close sender

let results: Vec<_> = rx.iter().collect();
```

## Configuration Options

```rust
let exec = NumaExecutor::builder(topo)
    .steal_policy(StealPolicy::LocalThenSocketThenRemote)
    .workers_per_node(4)        // 4 workers per NUMA node
    .hard_mode(HardMode::Strict) // Fail if pinning fails
    .build()?;
```

## Inspection

```rust
println!("Workers: {}", exec.worker_count());
println!("Steal policy: {}", exec.steal_policy());
```

## Shutdown

```rust
// Graceful shutdown - waits for all tasks
exec.shutdown();
```

## Pattern: Data-Local Processing

Submit work to the node that owns the data:

```rust
// Data partitioned by node
let node_data: Vec<Arc<Data>> = partition_data(&topo);

for (node_idx, data) in node_data.iter().enumerate() {
    let node_id = topo.numa_nodes()[node_idx].id();
    let data = Arc::clone(data);

    exec.submit_to_node(node_id, move || {
        // Process data on its local node
        process(&data);
    });
}
```

## Pattern: Pipeline Processing

```rust
// Stage 1: Parse on any node
for chunk in input_chunks {
    exec.submit_to_node(NodeId::new(0), move || {
        let parsed = parse(chunk);
        stage2_queue.push(parsed);
    });
}

// Stage 2: Process on node 1
while let Some(item) = stage2_queue.pop() {
    exec.submit_to_node(NodeId::new(1), move || {
        process(item);
    });
}
```

## Best Practices

1. **Match work to data location** - Submit to node that owns data
2. **Use LocalThenSocketThenRemote** for most workloads
3. **Use LocalOnly** when locality is critical
4. **Set workers_per_node** based on CPU cores per node
5. **Track locality** with StatsCollector
