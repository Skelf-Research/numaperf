# Sharded API

Types for per-node sharded data structures.

## NumaSharded<T>

Per-node sharded data with one shard per NUMA node.

```rust
pub struct NumaSharded<T> { /* internal */ }
```

### Construction

```rust
use numaperf::{NumaSharded, Topology};
use std::sync::Arc;

let topo = Arc::new(Topology::discover()?);

// Create with initializer function
let data = NumaSharded::new(&topo, || Vec::new());

// Each node gets its own Vec
```

### Methods

| Method | Description |
|--------|-------------|
| `new(topo, init: F) -> Self` | Create with per-node initialization |
| `local<R>(&self, f: F) -> R` | Access local shard |
| `get<R>(&self, node: NodeId, f: F) -> R` | Access specific shard |
| `iter(&self) -> impl Iterator` | Iterate all shards |

### Local Access

```rust
// Fast path - accesses current thread's local shard
data.local(|shard| {
    shard.push(42);
});
```

### Specific Node Access

```rust
// Access a specific node's shard
data.get(NodeId::new(0), |shard| {
    println!("Node 0 has {} items", shard.len());
});
```

### Iteration

```rust
for (node_id, shard) in data.iter() {
    println!("Node {}: {:?}", node_id.as_u32(), shard);
}
```

---

## ShardedCounter

A specialized `NumaSharded<AtomicU64>` for counting.

```rust
pub struct ShardedCounter { /* internal */ }
```

### Construction

```rust
use numaperf::{ShardedCounter, Topology};

let topo = Arc::new(Topology::discover()?);
let counter = ShardedCounter::new(&topo);
```

### Methods

| Method | Description |
|--------|-------------|
| `new(topo) -> Self` | Create counter initialized to 0 |
| `increment(&self)` | Add 1 to local shard |
| `add(&self, n: u64)` | Add n to local shard |
| `sum(&self) -> u64` | Sum all shards |

### Example

```rust
let counter = ShardedCounter::new(&topo);

// Fast increment (local shard)
counter.increment();
counter.add(10);

// Aggregate (reads all shards)
let total = counter.sum();
println!("Total: {}", total);
```

---

## CachePadded<T>

Wrapper that pads T to cache line size (128 bytes).

```rust
pub struct CachePadded<T> { /* internal */ }
```

Prevents false sharing when multiple threads access adjacent data.

### Construction

```rust
use numaperf::CachePadded;
use std::sync::atomic::AtomicU64;

let counter = CachePadded::new(AtomicU64::new(0));
```

### Methods

| Method | Description |
|--------|-------------|
| `new(value: T) -> Self` | Create padded wrapper |
| `into_inner(self) -> T` | Unwrap the value |

### Deref

`CachePadded<T>` implements `Deref<Target = T>` and `DerefMut`:

```rust
let counter = CachePadded::new(AtomicU64::new(0));
counter.fetch_add(1, Ordering::Relaxed); // Deref to AtomicU64
```

### Use Case

```rust
// Without padding - false sharing!
struct Bad {
    counter_a: AtomicU64,  // Same cache line
    counter_b: AtomicU64,  // as counter_a
}

// With padding - no false sharing
struct Good {
    counter_a: CachePadded<AtomicU64>,
    counter_b: CachePadded<AtomicU64>,
}
```

---

## Thread Safety

| Type | Send | Sync |
|------|------|------|
| `NumaSharded<T>` | If T: Send | If T: Sync |
| `ShardedCounter` | Yes | Yes |
| `CachePadded<T>` | If T: Send | If T: Sync |
