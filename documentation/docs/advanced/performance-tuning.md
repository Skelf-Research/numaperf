# Performance Tuning

Guidelines for optimizing NUMA-aware applications with numaperf.

## Profiling First

Before optimizing, measure your current performance:

```bash
# Check NUMA memory stats
numastat -p $(pgrep your_app)

# Profile with perf
perf stat -e \
  node-loads,node-load-misses,\
  node-stores,node-store-misses \
  -- ./your_app

# Memory bandwidth
perf stat -e \
  uncore_imc/cas_count_read/,\
  uncore_imc/cas_count_write/ \
  -- ./your_app
```

## Key Metrics

| Metric | Target | Meaning |
|--------|--------|---------|
| Locality ratio | > 90% | Local vs remote work execution |
| Node load misses | < 5% | Remote memory reads |
| Node store misses | < 5% | Remote memory writes |
| Cross-node steals | < 10% | Work stolen across nodes |

## Executor Tuning

### Workers Per Node

Start with 1 worker per core, then tune based on workload:

```rust
let exec = NumaExecutor::builder(topo)
    .workers_per_node(cores_per_node)  // Start here
    .build()?;
```

**CPU-bound workloads**: 1 worker per core
**I/O-bound workloads**: 2-4 workers per core
**Mixed workloads**: Benchmark to find optimal

### Steal Policy Selection

| Policy | Locality | Throughput | Use When |
|--------|----------|------------|----------|
| `LocalOnly` | Best | Lowest | Strict locality requirements |
| `LocalThenSocketThenRemote` | Good | Good | General purpose |
| `Any` | Worst | Best | Maximum throughput needed |

```rust
// For latency-sensitive workloads
.steal_policy(StealPolicy::LocalOnly)

// For general purpose (default)
.steal_policy(StealPolicy::LocalThenSocketThenRemote)

// For throughput-focused workloads
.steal_policy(StealPolicy::Any)
```

### Task Granularity

Tasks should be large enough to amortize scheduling overhead:

```rust
// Bad: Too fine-grained
for item in items {
    exec.submit_to_node(node, move || process_one(item));
}

// Good: Batch processing
for chunk in items.chunks(100) {
    let chunk = chunk.to_vec();
    exec.submit_to_node(node, move || {
        for item in chunk {
            process_one(item);
        }
    });
}
```

**Rule of thumb**: Each task should take at least 10-100 microseconds.

## Memory Placement

### Data Locality

Allocate data where it will be processed:

```rust
// Allocate on specific node
let data = NumaRegion::anon(
    size,
    MemPolicy::Bind(NodeMask::single(processing_node)),
    Default::default(),
    Prefault::Touch,
)?;

// Process on the same node
exec.submit_to_node(processing_node, move || {
    process(data.as_slice());
});
```

### Prefaulting

Use prefaulting to avoid page faults during critical operations:

```rust
// Touch pages immediately (recommended for latency-sensitive)
NumaRegion::anon(size, policy, opts, Prefault::Touch)?;

// Lazy allocation (default, best for memory efficiency)
NumaRegion::anon(size, policy, opts, Prefault::None)?;

// Kernel populate (may be faster for large allocations)
NumaRegion::anon(size, policy, opts, Prefault::Populate)?;
```

### Huge Pages

Enable huge pages for large allocations:

```rust
use numaperf::MemOptions;

let opts = MemOptions {
    huge_pages: true,
    ..Default::default()
};

let region = NumaRegion::anon(size, policy, opts, Prefault::Touch)?;
```

Configure system for huge pages:

```bash
# Transparent huge pages (automatic)
echo madvise | sudo tee /sys/kernel/mm/transparent_hugepage/enabled

# Explicit huge pages (manual)
echo 1024 | sudo tee /sys/kernel/mm/hugepages/hugepages-2048kB/nr_hugepages
```

## Avoiding False Sharing

Use `CachePadded` to prevent cache line bouncing:

```rust
use numaperf::CachePadded;
use std::sync::atomic::AtomicU64;

// Bad: Adjacent counters share cache line
struct Bad {
    counter_a: AtomicU64,
    counter_b: AtomicU64,
}

// Good: Padded to separate cache lines
struct Good {
    counter_a: CachePadded<AtomicU64>,
    counter_b: CachePadded<AtomicU64>,
}
```

## Sharded Data Structures

Use `NumaSharded` for per-node state:

```rust
use numaperf::NumaSharded;

// Per-node counters
let counters = NumaSharded::new(&topo, || AtomicU64::new(0));

// Fast local access
counters.local(|counter| {
    counter.fetch_add(1, Ordering::Relaxed);
});
```

Or use `ShardedCounter` for counting:

```rust
use numaperf::ShardedCounter;

let counter = ShardedCounter::new(&topo);

// Fast increment (local shard)
counter.increment();

// Aggregate (reads all shards)
let total = counter.sum();
```

## Device Affinity

Process I/O on the device's local node:

```rust
let devices = DeviceMap::discover(topo)?;

// Find NIC's node
let nic_node = devices.device_node("eth0")
    .unwrap_or(NodeId::new(0));

// Allocate buffers on NIC's node
let buffers = NumaRegion::anon(
    size,
    MemPolicy::Bind(NodeMask::single(nic_node)),
    Default::default(),
    Prefault::Touch,
)?;

// Process packets on NIC's node
exec.submit_to_node(nic_node, || {
    process_packets(buffers);
});
```

## Monitoring in Production

### Continuous Locality Tracking

```rust
use numaperf::StatsCollector;

let collector = StatsCollector::new(&topo);

// In your monitoring loop
std::thread::spawn(move || {
    loop {
        std::thread::sleep(Duration::from_secs(60));

        let stats = collector.snapshot();
        let ratio = stats.locality_ratio();

        metrics::gauge!("numa.locality_ratio", ratio);

        if ratio < 0.8 {
            log::warn!("Low NUMA locality: {:.1}%", ratio * 100.0);
        }

        collector.reset();
    }
});
```

### Health Checks

```rust
use numaperf::{LocalityReport, LocalityHealth};

fn health_check(collector: &StatsCollector) -> bool {
    let stats = collector.snapshot();
    let report = LocalityReport::generate(&stats);

    report.health().is_acceptable()
}
```

## Common Anti-Patterns

### Random Node Assignment

```rust
// Bad: Random assignment ignores data locality
let node = random_node();
exec.submit_to_node(node, || process(data));

// Good: Submit to data's node
let data_node = get_data_node(&data);
exec.submit_to_node(data_node, || process(data));
```

### Global Allocations

```rust
// Bad: Global allocator ignores NUMA
let data = vec![0u8; size];

// Good: NUMA-aware allocation
let data = NumaRegion::anon(size, MemPolicy::Local, ...)?;
```

### Excessive Cross-Node Communication

```rust
// Bad: Shared state accessed from all nodes
let shared = Arc::new(Mutex::new(Vec::new()));

// Good: Per-node state, aggregate when needed
let sharded = NumaSharded::new(&topo, || Mutex::new(Vec::new()));
```

## Benchmarking

Use the included benchmark suite:

```bash
# Run all benchmarks
cargo run -p numaperf-bench -- bench

# Specific benchmarks
cargo run -p numaperf-bench -- bench --category memory
cargo run -p numaperf-bench -- bench --category scheduler

# Criterion benchmarks (detailed)
cargo bench -p numaperf-bench
```
