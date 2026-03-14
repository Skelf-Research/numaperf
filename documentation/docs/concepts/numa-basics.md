# NUMA Basics

## What is NUMA?

**NUMA (Non-Uniform Memory Access)** is a computer memory architecture where memory access time depends on the memory location relative to the processor. In a NUMA system:

- Each CPU (or group of CPUs) has its own **local memory**
- CPUs can access memory attached to other CPUs (**remote memory**)
- **Local access is faster** than remote access (typically 1.5-3x)

```
┌─────────────────┐     ┌─────────────────┐
│    NUMA Node 0  │     │    NUMA Node 1  │
│  ┌───────────┐  │     │  ┌───────────┐  │
│  │  CPU 0-7  │  │     │  │ CPU 8-15  │  │
│  └─────┬─────┘  │     │  └─────┬─────┘  │
│        │        │     │        │        │
│  ┌─────▼─────┐  │     │  ┌─────▼─────┐  │
│  │  Memory   │◄─┼─────┼─►│  Memory   │  │
│  │  (32 GB)  │  │     │  │  (32 GB)  │  │
│  └───────────┘  │     │  └───────────┘  │
└─────────────────┘     └─────────────────┘
       LOCAL              REMOTE (slower)
```

## Why NUMA Matters

### The Performance Impact

On a typical 2-socket server:

| Access Type | Latency | Bandwidth |
|-------------|---------|-----------|
| Local | ~80ns | 100% |
| Remote | ~140ns | 60-70% |

This means:

- **Memory-intensive workloads** can see 30-50% performance loss with poor locality
- **Latency-sensitive applications** can have inconsistent response times
- **High-throughput systems** may bottleneck on the interconnect

### When NUMA Matters Most

NUMA effects are significant when:

1. **Large working sets** - Data doesn't fit in CPU caches
2. **Memory bandwidth bound** - Streaming large amounts of data
3. **Low latency requirements** - Every nanosecond counts
4. **Multi-threaded workloads** - Threads access shared data

NUMA effects are less important when:

1. **CPU-bound computation** - Data fits in caches
2. **I/O bound workloads** - Waiting on disk or network
3. **Small data sets** - Everything fits in cache

## NUMA Concepts

### NUMA Node

A **NUMA node** is a group of CPUs with their local memory. On most systems:

- One node per CPU socket
- All CPUs in a node have equal access to that node's memory
- Each node has a unique ID (0, 1, 2, ...)

### NUMA Distance

**Distance** measures the relative cost of accessing memory from one node to another:

```
Distance Matrix:
       Node 0  Node 1
Node 0     10      21
Node 1     21      10
```

- Distance 10 = local access (baseline)
- Distance 21 = remote access (2.1x the "cost")

### Memory Locality

**Memory locality** refers to how often a thread accesses memory on its local node:

- **100% local** = All memory accesses are to local memory
- **50% local** = Half local, half remote
- **Goal**: Maximize local accesses

## NUMA Strategies

### 1. Thread Pinning

**Pin threads** to specific CPUs to prevent migration:

```rust
use numaperf::{ScopedPin, CpuSet};

// Pin to CPUs on node 0
let _pin = ScopedPin::pin_current(CpuSet::parse("0-7")?)?;
// Thread will stay on these CPUs
```

### 2. Memory Placement

**Allocate memory** on specific nodes:

```rust
use numaperf::{NumaRegion, MemPolicy, NodeMask, NodeId, Prefault};

// Bind memory to node 0
let region = NumaRegion::anon(
    size,
    MemPolicy::Bind(NodeMask::single(NodeId::new(0))),
    Default::default(),
    Prefault::Touch,
)?;
```

### 3. Work Distribution

**Submit work** to the node where its data lives:

```rust
use numaperf::NumaExecutor;

// Submit to the node that owns the data
exec.submit_to_node(data_node_id, || {
    process(data);
});
```

### 4. Data Partitioning

**Partition data** by node with sharded structures:

```rust
use numaperf::NumaSharded;

// One shard per NUMA node
let data = NumaSharded::new(&topo, || Vec::new());

// Access local shard (fast)
data.local(|shard| shard.push(item));
```

## The Pin-Then-Allocate Pattern

The most common NUMA optimization pattern:

```rust
use numaperf::{ScopedPin, Topology, CpuSet};

fn numa_aware_init(topo: &Topology) {
    for node in topo.numa_nodes() {
        // 1. Pin to this node's CPUs
        let _pin = ScopedPin::pin_current(node.cpus().clone())?;

        // 2. Allocate (will be local to this node)
        let data = vec![0u8; 1024 * 1024];

        // 3. Use data while pinned
        process(&data);
    }
}
```

Why this works:

1. Linux allocates memory on the **current thread's node** by default
2. Pinning ensures we're on the **desired node**
3. Subsequent allocations are **automatically local**

## Common Pitfalls

### 1. First-Touch Allocation

Memory is allocated on **first access**, not at `malloc()` time:

```rust
// Memory is NOT allocated yet
let mut data = Vec::with_capacity(1_000_000);

// First touch happens here - on current node!
data.resize(1_000_000, 0);
```

**Solution**: Use `Prefault::Touch` or write to memory immediately after allocation.

### 2. Thread Migration

Without pinning, the OS can **migrate threads** between CPUs:

```rust
// Bad: Thread might move between accesses
loop {
    process(&local_data);  // Might be remote now!
}

// Good: Pin first
let _pin = ScopedPin::pin_current(cpus)?;
loop {
    process(&local_data);  // Always local
}
```

### 3. False Sharing

Multiple threads writing to the **same cache line**:

```rust
// Bad: All counters on same cache line
struct Counters {
    thread_0: AtomicU64,
    thread_1: AtomicU64,  // 8 bytes apart
}

// Good: Pad to cache line size
use numaperf::CachePadded;
struct Counters {
    thread_0: CachePadded<AtomicU64>,
    thread_1: CachePadded<AtomicU64>,
}
```

### 4. Shared Data Structures

Global data structures cause **remote accesses**:

```rust
// Bad: Single global counter
static COUNTER: AtomicU64 = AtomicU64::new(0);

// Good: Per-node counter
let counter = ShardedCounter::new(&topo);
counter.increment();  // Uses local shard
```

## Measuring NUMA Effects

### Using numaperf

```rust
use numaperf::{StatsCollector, LocalityReport};

let collector = StatsCollector::new(&topo);

// Your workload here...
collector.record_local_execution();

let stats = collector.snapshot();
println!("Locality: {:.1}%", stats.locality_ratio() * 100.0);
```

### Using System Tools

```bash
# Watch NUMA statistics
numastat -p <pid>

# Memory placement
numactl --hardware

# Per-node memory info
cat /sys/devices/system/node/node0/meminfo
```

## Next Steps

- [Architecture](architecture.md) - How numaperf is organized
- [Memory Policies](memory-policies.md) - Detailed memory placement options
- [Thread Pinning Guide](../guides/thread-pinning.md) - Practical pinning techniques
