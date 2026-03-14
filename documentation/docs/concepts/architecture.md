# Architecture

## Design Philosophy

numaperf is built on three core principles:

1. **Locality by default** - APIs guide you toward NUMA-aware patterns
2. **Transparency** - Know what enforcement you actually got
3. **Graceful degradation** - Works on any system, optimizes when possible

## Crate Organization

numaperf is organized as a workspace of specialized crates:

```
numaperf (facade)
    │
    ├── numaperf-core      # Shared types and errors
    │
    ├── numaperf-topo      # Topology discovery
    │       │
    │       └── depends on: core
    │
    ├── numaperf-affinity  # Thread pinning
    │       │
    │       └── depends on: core
    │
    ├── numaperf-mem       # Memory placement
    │       │
    │       └── depends on: core
    │
    ├── numaperf-sched     # Work scheduling
    │       │
    │       └── depends on: core, topo, affinity
    │
    ├── numaperf-sharded   # Sharded data structures
    │       │
    │       └── depends on: core, topo
    │
    ├── numaperf-io        # Device locality
    │       │
    │       └── depends on: core, topo
    │
    └── numaperf-perf      # Observability
            │
            └── depends on: core, topo, sharded
```

### Crate Responsibilities

| Crate | Responsibility |
|-------|----------------|
| `numaperf-core` | `NodeId`, `CpuSet`, `NodeMask`, `NumaError`, `HardMode`, `Capabilities` |
| `numaperf-topo` | `Topology`, `NumaNode`, discovery from `/sys` |
| `numaperf-affinity` | `ScopedPin`, `get_affinity()`, `set_affinity()` |
| `numaperf-mem` | `NumaRegion`, `MemPolicy`, `mbind()` wrapper |
| `numaperf-sched` | `NumaExecutor`, per-node worker pools, work stealing |
| `numaperf-sharded` | `NumaSharded<T>`, `ShardedCounter`, `CachePadded<T>` |
| `numaperf-io` | `DeviceMap`, device-to-node mapping |
| `numaperf-perf` | `StatsCollector`, `LocalityReport`, metrics |

## Key Patterns

### Arc<Topology>

Topology discovery is expensive. Create once, share everywhere:

```rust
use numaperf::Topology;
use std::sync::Arc;

// Create once at startup
let topo = Arc::new(Topology::discover()?);

// Share across threads
let topo_clone = Arc::clone(&topo);
std::thread::spawn(move || {
    // Use topo_clone
});
```

### RAII Guards

Resources are managed with RAII patterns:

```rust
use numaperf::{ScopedPin, NumaRegion};

{
    // Pin is active
    let _pin = ScopedPin::pin_current(cpus)?;
    // ...
} // Pin automatically restored

{
    // Memory is mapped
    let region = NumaRegion::anon(...)?;
    // ...
} // Memory automatically unmapped
```

### Builder Pattern

Complex types use builders:

```rust
use numaperf::{NumaExecutor, StealPolicy, HardMode};

let exec = NumaExecutor::builder(topo)
    .steal_policy(StealPolicy::LocalThenSocketThenRemote)
    .workers_per_node(4)
    .hard_mode(HardMode::Strict)
    .build()?;
```

### Enforcement Transparency

Operations report what enforcement they achieved:

```rust
use numaperf::{NumaRegion, EnforcementLevel};

let region = NumaRegion::anon(...)?;

match region.enforcement() {
    EnforcementLevel::Strict => println!("Guaranteed placement"),
    EnforcementLevel::BestEffort { reason } => println!("Best effort: {}", reason),
    EnforcementLevel::None { reason } => println!("No enforcement: {}", reason),
}
```

## Thread Safety

| Type | Send | Sync | Notes |
|------|------|------|-------|
| `Topology` | Yes | Yes | Immutable after creation |
| `NumaNode` | Yes | Yes | Immutable |
| `ScopedPin` | **No** | **No** | Thread-local by design |
| `NumaRegion` | Yes | Yes | Memory can be shared |
| `NumaExecutor` | Yes | Yes | Submit from any thread |
| `NumaSharded<T>` | If T: Send | If T: Sync | Depends on T |
| `StatsCollector` | Yes | Yes | Lock-free internals |

### ScopedPin is !Send

`ScopedPin` intentionally cannot be sent between threads:

```rust
let pin = ScopedPin::pin_current(cpus)?;

// This won't compile - and that's correct!
std::thread::spawn(move || {
    drop(pin);  // Would restore wrong thread's affinity
});
```

## Data Flow

### Typical Application Flow

```
1. Startup
   ├── Capabilities::detect()  ─► Check system support
   └── Topology::discover()    ─► Learn NUMA layout

2. Initialization
   ├── NumaExecutor::builder() ─► Create worker pools
   ├── NumaSharded::new()      ─► Per-node data
   └── StatsCollector::new()   ─► Metrics collection

3. Runtime
   ├── exec.submit_to_node()   ─► Submit work
   ├── sharded.local()         ─► Access local data
   └── collector.record_*()    ─► Track locality

4. Shutdown
   ├── exec.shutdown()         ─► Wait for completion
   └── LocalityReport::generate() ─► Analyze results
```

### Memory Allocation Flow

```
NumaRegion::anon(size, policy, huge_pages, prefault)
    │
    ├── mmap(NULL, size, PROT_READ|PROT_WRITE, MAP_PRIVATE|MAP_ANONYMOUS, -1, 0)
    │
    ├── mbind(addr, size, policy, nodemask, maxnode, flags)
    │   │
    │   ├── Success ─► EnforcementLevel::Strict
    │   │
    │   └── EPERM ─► Soft mode: EnforcementLevel::BestEffort
    │               Hard mode: NumaError::HardModeUnavailable
    │
    └── prefault (if requested)
        └── Touch each page to force allocation
```

### Work Scheduling Flow

```
exec.submit_to_node(node_id, closure)
    │
    ├── Find queue for target node
    │
    └── Push to node's work queue
            │
            └── Worker on that node picks it up
                    │
                    ├── Execute closure
                    │
                    └── If queue empty, try stealing
                            │
                            ├── LocalOnly: Never steal
                            │
                            ├── LocalThenSocketThenRemote:
                            │   1. Try same-socket nodes
                            │   2. Try remote nodes
                            │
                            └── Any: Steal from any node
```

## Error Handling

All fallible operations return `Result<T, NumaError>`:

```rust
pub enum NumaError {
    // System errors
    IoError(std::io::Error),

    // Configuration errors
    InvalidNodeId(u32),
    InvalidCpuId(u32),
    EmptyCpuSet,
    EmptyNodeMask,

    // Capability errors
    NotSupported(String),
    HardModeUnavailable { operation: String, reason: String },

    // Runtime errors
    TopologyMismatch,
    WorkerPanic,
}
```

Errors include context for debugging:

```rust
match result {
    Err(NumaError::HardModeUnavailable { operation, reason }) => {
        eprintln!("Cannot enforce {} in hard mode: {}", operation, reason);
    }
    // ...
}
```

## Platform Abstraction

Linux-specific code is isolated:

```
numaperf-topo/src/
├── lib.rs
├── topology.rs      # Platform-agnostic API
├── node.rs
└── discovery/
    ├── mod.rs       # Platform selection
    ├── linux.rs     # Linux: reads /sys/devices/system/node/
    └── fallback.rs  # Other: single synthetic node
```

This allows:

- Full functionality on Linux
- Graceful degradation elsewhere
- Easy testing with synthetic topologies

## Next Steps

- [Soft vs Hard Mode](soft-vs-hard-mode.md) - Enforcement modes explained
- [Memory Policies](memory-policies.md) - Memory placement in detail
- [API Overview](../api/overview.md) - Complete API reference
