# Scheduler API

Types for NUMA-aware work scheduling.

## NumaExecutor

A NUMA-aware work executor with per-node worker pools.

```rust
pub struct NumaExecutor { /* internal */ }
```

### Construction

```rust
use numaperf::{NumaExecutor, Topology, StealPolicy};
use std::sync::Arc;

let topo = Arc::new(Topology::discover()?);

// Using builder (recommended)
let exec = NumaExecutor::builder(Arc::clone(&topo))
    .steal_policy(StealPolicy::LocalThenSocketThenRemote)
    .workers_per_node(2)
    .build()?;

// Quick construction
let exec = NumaExecutor::new(topo, StealPolicy::LocalThenSocketThenRemote)?;
```

### Methods

| Method | Description |
|--------|-------------|
| `new(topo, policy) -> Result<Self, NumaError>` | Create with default workers |
| `builder(topo) -> NumaExecutorBuilder` | Create builder |
| `submit_to_node(&self, node: NodeId, f: F)` | Submit work to node |
| `shutdown(&self)` | Graceful shutdown |
| `worker_count(&self) -> usize` | Total workers |
| `steal_policy(&self) -> StealPolicy` | Current policy |

### Submitting Work

```rust
exec.submit_to_node(NodeId::new(0), || {
    // This runs on a worker pinned to node 0
    process_data();
});

// With captured data
let data = Arc::new(vec![1, 2, 3]);
let d = Arc::clone(&data);
exec.submit_to_node(node_id, move || {
    println!("Data: {:?}", d);
});
```

### Shutdown

```rust
// Waits for all pending work to complete
exec.shutdown();
```

---

## NumaExecutorBuilder

Builder for configuring `NumaExecutor`.

```rust
pub struct NumaExecutorBuilder { /* internal */ }
```

### Methods

| Method | Description |
|--------|-------------|
| `steal_policy(self, policy: StealPolicy) -> Self` | Set steal policy |
| `workers_per_node(self, count: usize) -> Self` | Set workers per node |
| `hard_mode(self, mode: HardMode) -> Self` | Set enforcement mode |
| `build(self) -> Result<NumaExecutor, NumaError>` | Build executor |

### Example

```rust
let exec = NumaExecutor::builder(topo)
    .steal_policy(StealPolicy::LocalOnly)
    .workers_per_node(4)
    .hard_mode(HardMode::Strict)
    .build()?;
```

---

## StealPolicy

Work stealing policy for load balancing.

```rust
pub enum StealPolicy {
    /// Never steal from other nodes
    LocalOnly,
    /// Steal from nearby nodes first
    LocalThenSocketThenRemote,
    /// Steal from any node
    Any,
}
```

### Comparison

| Policy | Locality | Throughput | Use Case |
|--------|----------|------------|----------|
| `LocalOnly` | Best | Lower | Strict locality requirements |
| `LocalThenSocketThenRemote` | Good | Good | General purpose (default) |
| `Any` | Worst | Best | Maximum throughput |

### Display

```rust
let policy = StealPolicy::LocalThenSocketThenRemote;
println!("{}", policy); // "LocalThenSocketThenRemote"
```

---

## Thread Safety

- `NumaExecutor` is `Send + Sync`
- Can submit work from any thread
- Workers are pinned to their respective nodes
- Safe to share via `Arc<NumaExecutor>`
