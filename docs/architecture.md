# Architecture

This document describes the core design of `numaperf` and the invariants it aims to enforce. It is intended to be implementation-agnostic and stable across crate versions.

**Design principles**
- Pin first, allocate second, touch third.
- Use explicit memory policies for large regions and long-lived structures.
- Shard everything by NUMA node to avoid global contention.
- Prefer locality-aware scheduling over global queues.
- Make cross-node traffic observable and limitable.
- Degrade gracefully in soft mode when kernel features or privileges are unavailable.
- In hard mode, refuse to claim enforcement if a policy cannot be guaranteed.

---

## Crate dependency graph

```
                         ┌─────────────────┐
                         │  numaperf-core  │
                         │ errors, config, │
                         │ HardMode, types │
                         │ NodeId, CpuSet  │
                         └────────┬────────┘
                                  │
                                  ▼
                         ┌─────────────────┐
                         │  numaperf-topo  │
                         │    Topology     │
                         │  NumaNode, etc  │
                         └────────┬────────┘
                                  │
                 ┌────────────────┼────────────────┐
                 │                │                │
                 ▼                ▼                ▼
        ┌──────────────┐  ┌─────────────┐  ┌─────────────┐
        │numaperf-     │  │numaperf-io  │  │numaperf-    │
        │affinity      │  │ PCI locality│  │sharded      │
        │ ScopedPin    │  └─────────────┘  │ NumaSharded │
        └───────┬──────┘                   └──────┬──────┘
                │                                 │
                ▼                                 │
        ┌──────────────┐                          │
        │ numaperf-mem │                          │
        │ NumaRegion   │                          │
        │ MemPolicy    │                          │
        └───────┬──────┘                          │
                │                                 │
                ▼                                 │
        ┌──────────────┐                          │
        │numaperf-sched│◄─────────────────────────┘
        │ NumaExecutor │
        │ StealPolicy  │
        └───────┬──────┘
                │
                ▼
        ┌──────────────┐
        │numaperf-perf │
        │LocalityStats │
        └──────────────┘

        ┌──────────────────────────────────────────┐
        │              numaperf (facade)           │
        │  Re-exports all public APIs from above   │
        └──────────────────────────────────────────┘
```

**Dependency table**

| Crate | Direct dependencies |
|-------|---------------------|
| `numaperf-core` | (none) |
| `numaperf-topo` | core |
| `numaperf-affinity` | core, topo |
| `numaperf-io` | core, topo |
| `numaperf-sharded` | core, topo |
| `numaperf-mem` | core, topo, affinity |
| `numaperf-sched` | core, topo, affinity, mem, sharded |
| `numaperf-perf` | core, topo, sched |
| `numaperf` | all crates (facade) |

**Design rationale**

- `numaperf-core` contains primitive types (`NodeId`, `CpuSet`, `NodeMask`, `MemPolicy`) so that crates can depend on types without pulling in implementations.
- `numaperf-mem` depends on `numaperf-affinity` to pin prefault worker threads to the correct nodes.
- `numaperf-sched` depends on `numaperf-mem` to allocate per-node queues with explicit NUMA placement.
- `numaperf-sched` depends on `numaperf-sharded` for per-node data structures.
- The `numaperf` facade crate re-exports all public APIs for convenience; production code can import individual crates for faster compilation.

**Topology sharing**

`Topology` is created once at startup and shared via `Arc<Topology>`:

```rust
use std::sync::Arc;
use numaperf_topo::Topology;
use numaperf_sched::NumaExecutor;

let topo = Arc::new(Topology::discover()?);
let executor = NumaExecutor::new(Arc::clone(&topo), StealPolicy::default())?;
```

---

**Topology and locality**
`numaperf-topo` provides a canonical view of the machine topology. It maps NUMA nodes, sockets, cores, SMT threads, and last-level cache groups into a compact model.

Key capabilities:
- Discover the hardware topology once and expose stable identifiers.
- Provide CPU sets per node and cache group.
- Provide PCI locality for devices when available.

**Affinity and pinning**
`numaperf-affinity` owns pinning and affinity guards.

Key capabilities:
- Pin current thread to a CPU set or node.
- Spawn pinned workers tied to a node.
- Provide scoped pinning that restores prior affinity.
- Optionally enforce “no migration” policies.

**Memory placement**
`numaperf-mem` provides explicit placement for memory regions that dominate performance.

Key capabilities:
- Allocate large anonymous regions with a fixed policy.
- Bind file-backed mappings to a policy and prefault locally.
- Expose prefault strategies to force first-touch locality.
- Support optional page migration and interleave policies.
- Provide optional huge page controls.

Memory policy model:
- `Bind(nodes)` for strict locality.
- `Preferred(node)` for preferred locality with fallback.
- `Interleave(nodes)` for scan-heavy read-mostly data.
- `Local` for “use current thread’s node.”

**Scheduling**
`numaperf-sched` enforces topology-aware execution.

Key capabilities:
- One worker pool per node with local queues.
- A configurable steal order that prefers local, then same socket, then remote.
- Optional “home node” tagging for data-local work.
- Cross-node steal budgets to prevent remote storms.

**Sharded structures**
`numaperf-sharded` provides locality-friendly shared structures.

Key capabilities:
- Per-node shards for maps, counters, and registries.
- Cache-padded wrappers for hot structures.
- Safe aggregation patterns for global views.

**Device locality**
`numaperf-io` maps storage and network devices to NUMA nodes when available.

Key capabilities:
- Determine the closest node for a block or network device.
- Provide IO worker pools pinned to device-local nodes.

**Observability**
`numaperf-perf` makes locality visible.

Key capabilities:
- Track per-node allocation and fault counters.
- Track queue depth and cross-node steal counts.
- Provide a “cheap mode” without privileged counters.
- Provide an “advanced mode” with perf events where available.

**Degradation strategy**
When a feature is unavailable, `numaperf` must do one of the following:
- Fall back to best-effort behavior while reporting loss of strictness.
- Refuse to enable hard mode and return a structured error.

The library must never silently pretend a hard-mode policy is being enforced when it is not.

---

## Thread safety guarantees

All public types follow Rust's ownership model. The table below summarizes thread safety:

| Type | `Send` | `Sync` | Notes |
|------|--------|--------|-------|
| `Topology` | Yes | Yes | Immutable after creation; share via `Arc` |
| `NodeId`, `CpuSet` | Yes | Yes | Copy types |
| `ScopedPin` | No | No | Must stay on the thread it pins |
| `NumaRegion` | Yes | No | Owned buffer; use external sync for shared access |
| `NumaSharded<T>` | Yes | Yes | Safe concurrent access to per-node shards |
| `NumaExecutor` | Yes | Yes | Submit work from any thread |
| `LocalityStats` | Yes | Yes | Snapshot is immutable |

**Guidelines:**
- Create `Topology` once at startup and share via `Arc<Topology>`.
- `ScopedPin` is `!Send` because pinning is thread-local; do not move across threads.
- `NumaRegion` can be sent to another thread but requires external synchronization for concurrent access (e.g., wrap in `Mutex` or use atomic operations).

---

## Single-socket behavior

On single-socket machines (one NUMA node), `numaperf` degrades gracefully:

| Feature | Behavior |
|---------|----------|
| Topology discovery | Returns one node with all CPUs |
| Memory policies | Accepted but have no locality effect |
| Thread pinning | Works normally; useful for cache locality |
| Scheduling | Per-node queues collapse to one queue |
| Sharded structures | Single shard; no overhead |
| Observability | Reports zero cross-node traffic |

This allows development and CI on commodity hardware while production runs on multi-socket machines.

---

## Platform support matrix

| Platform | Topology | Affinity | Memory policy | IO locality | Observability |
|----------|----------|----------|---------------|-------------|---------------|
| Linux 5.4+ | Full | Full | Full | Full | Full |
| Linux 4.x | Full | Full | Partial | Full | Partial |
| macOS | Partial | Partial | None | None | None |
| Windows | Partial | Partial | None | None | None |
| FreeBSD | Partial | Partial | None | None | None |

**Legend:**
- **Full**: Feature works as documented
- **Partial**: Basic functionality; some APIs return `NotSupported`
- **None**: Feature unavailable; APIs return `NotSupported` or no-op

**Linux-specific features:**
- `mbind()` / `set_mempolicy()` for memory placement
- `/sys/devices/system/node/` for topology
- `perf_event_open()` for hardware counters

**Non-Linux platforms:**
- Topology discovery uses platform-specific APIs where available
- Affinity uses `pthread` APIs or platform equivalents
- Memory policy APIs return `NumaError::PolicyNotSupported`
- Hard mode is unavailable; only soft mode is supported
