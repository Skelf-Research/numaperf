# Sharded Data Structures

Learn how to use per-node sharded data for lock-free NUMA-local access.

## Why Sharding?

Global data structures cause contention and remote accesses:

```rust
// Bad: Global counter causes cache-line bouncing
static COUNTER: AtomicU64 = AtomicU64::new(0);

// Good: Per-node counter minimizes contention
let counter = ShardedCounter::new(&topo);
```

## ShardedCounter

A counter with one shard per NUMA node:

```rust
use numaperf::{ShardedCounter, Topology};
use std::sync::Arc;

let topo = Arc::new(Topology::discover()?);
let counter = ShardedCounter::new(&topo);

// Increment local shard (fast, no contention)
counter.increment();
counter.add(10);

// Read total (aggregates all shards)
let total = counter.sum();
```

## NumaSharded<T>

Generic per-node sharding for any type:

```rust
use numaperf::NumaSharded;

// One Vec per NUMA node
let buffers = NumaSharded::new(&topo, || Vec::new());

// Access local shard
buffers.local(|buf| {
    buf.push(42);
});

// Access specific node's shard
buffers.get(NodeId::new(0), |buf| {
    println!("Node 0 has {} items", buf.len());
});
```

## CachePadded<T>

Prevents false sharing by padding to cache line size (128 bytes):

```rust
use numaperf::CachePadded;
use std::sync::atomic::AtomicU64;

struct PerThreadState {
    counter: CachePadded<AtomicU64>,
    // Other fields won't share cache line with counter
}

let state = PerThreadState {
    counter: CachePadded::new(AtomicU64::new(0)),
};
```

## Iterating Shards

```rust
let counters = NumaSharded::new(&topo, || AtomicU64::new(0));

// Iterate all shards
for (node_id, counter) in counters.iter() {
    println!("Node {}: {}", node_id.as_u32(), counter.load(Ordering::Relaxed));
}
```

## Pattern: Per-Node Buffer Pool

```rust
use numaperf::{NumaSharded, CachePadded};
use std::sync::atomic::{AtomicUsize, Ordering};

struct BufferPool {
    buffers: NumaSharded<Vec<Vec<u8>>>,
    allocated: NumaSharded<CachePadded<AtomicUsize>>,
}

impl BufferPool {
    fn new(topo: &Arc<Topology>) -> Self {
        Self {
            buffers: NumaSharded::new(topo, || Vec::new()),
            allocated: NumaSharded::new(topo, || CachePadded::new(AtomicUsize::new(0))),
        }
    }

    fn allocate(&self, size: usize) -> Vec<u8> {
        self.buffers.local(|pool| {
            pool.pop().unwrap_or_else(|| vec![0; size])
        })
    }

    fn release(&self, buf: Vec<u8>) {
        self.buffers.local(|pool| pool.push(buf));
    }
}
```

## Pattern: Sharded Statistics

```rust
use numaperf::NumaSharded;
use std::sync::atomic::{AtomicU64, Ordering};

struct Stats {
    requests: NumaSharded<AtomicU64>,
    errors: NumaSharded<AtomicU64>,
    bytes: NumaSharded<AtomicU64>,
}

impl Stats {
    fn record_request(&self, bytes: u64) {
        self.requests.local(|c| c.fetch_add(1, Ordering::Relaxed));
        self.bytes.local(|c| c.fetch_add(bytes, Ordering::Relaxed));
    }

    fn record_error(&self) {
        self.errors.local(|c| c.fetch_add(1, Ordering::Relaxed));
    }

    fn snapshot(&self) -> StatsSnapshot {
        StatsSnapshot {
            requests: self.requests.iter()
                .map(|(_, c)| c.load(Ordering::Relaxed))
                .sum(),
            errors: self.errors.iter()
                .map(|(_, c)| c.load(Ordering::Relaxed))
                .sum(),
            bytes: self.bytes.iter()
                .map(|(_, c)| c.load(Ordering::Relaxed))
                .sum(),
        }
    }
}
```

## Thread Safety

| Type | Send | Sync |
|------|------|------|
| `NumaSharded<T>` | If T: Send | If T: Sync |
| `ShardedCounter` | Yes | Yes |
| `CachePadded<T>` | If T: Send | If T: Sync |

## Performance Characteristics

| Operation | NumaSharded | Global |
|-----------|-------------|--------|
| Local access | O(1), no contention | O(1), contention |
| Remote access | O(1), cache miss | O(1), cache miss |
| Aggregate | O(nodes) | O(1) |

## When to Use Sharding

**Use sharding when:**
- Many threads updating concurrently
- Data is naturally partitioned by thread
- Updates are frequent, reads are rare

**Avoid sharding when:**
- Single-threaded access
- Reads dominate writes
- Need atomic cross-node operations

## Best Practices

1. **Use ShardedCounter** for simple counting
2. **Use CachePadded** to prevent false sharing
3. **Access via local()** whenever possible
4. **Aggregate lazily** - only when needed
5. **Keep shard data small** for cache efficiency
