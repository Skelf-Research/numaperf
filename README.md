# numaperf

NUMA-first runtime building blocks for bandwidth-bound systems.

**What it is**
`numaperf` is a Rust workspace that provides explicit control over NUMA placement, pinning, scheduling, and locality observability. It is designed for database engines and other systems that scale on large multi-socket machines and need predictable memory and CPU locality.

**Why numaperf?**

| Approach | Trade-off |
|----------|-----------|
| **First-touch policy** | Simple but fragile. Initialization patterns determine placement, often accidentally. Refactoring breaks locality. |
| **numactl / libnuma** | Process-level control. No per-region granularity, no runtime observability, C-only API. |
| **Allocator NUMA modes** (mimalloc, jemalloc) | Good for small objects but doesn't address large regions, scheduling, or cross-node traffic. |
| **numaperf** | Explicit per-region placement, topology-aware scheduling, cross-node observability, and hard-mode enforcement when you need guarantees. |

**Quick example**
```rust
use numaperf_topo::Topology;
use numaperf_affinity::ScopedPin;
use numaperf_mem::{NumaRegion, MemPolicy, HugePageMode, Prefault};

// 1. Discover topology
let topo = Topology::discover()?;
let node0 = topo.numa_nodes()[0].id();

// 2. Pin this thread to node 0's CPUs
let _pin = ScopedPin::pin_current(topo.cpu_set(node0))?;

// 3. Allocate 2 GB bound to node 0
let buffer = NumaRegion::anon(
    2 * 1024 * 1024 * 1024,
    MemPolicy::Bind([node0].into()),
    HugePageMode::TransparentOn,
    Prefault::ParallelTouch,
)?;

// buffer.as_mut_slice() is now guaranteed local to node 0
```

**Goals**
- Make NUMA a first-class runtime concern, not a best-effort optimization.
- Offer explicit memory placement for large regions where locality matters.
- Provide topology-aware scheduling with enforceable policies.
- Ship observability so cross-node traffic becomes visible.
- Keep crate boundaries clean so projects can adopt only what they need.

**Non-goals**
- Implement database algorithms or buffer manager policies.
- Provide a full async runtime or general-purpose task system.
- Guarantee identical behavior across non-Linux platforms.

**Crate layout**
- `numaperf`: Facade crate that re-exports all public APIs.
- `numaperf-core`: Shared types, error model, and configuration.
- `numaperf-topo`: Topology discovery and locality maps.
- `numaperf-affinity`: Thread pinning and CPU set management.
- `numaperf-mem`: NUMA-aware memory placement and policy enforcement.
- `numaperf-sched`: Topology-aware work scheduling and stealing.
- `numaperf-sharded`: NUMA-local shared structures and counters.
- `numaperf-io`: Device locality helpers for NVMe and NICs.
- `numaperf-perf`: Locality observability and metrics.

**Operating modes**
- **Soft mode**: Uses best-effort pinning and placement, degrades gracefully when privileges or kernel features are missing.
- **Hard mode**: Enforces strict pinning, explicit placement, prefaulting, and cross-node traffic limits.

**Platform support**
- Linux is the primary target for full functionality.
- Other platforms may support pinning and partial topology but will lack memory policy controls.

**MVP focus**
1. `numaperf-topo` with stable topology discovery and CPU set APIs.
2. `numaperf-affinity` with scoped pinning and pinned worker spawns.
3. `numaperf-mem` with explicit placement for large regions.
4. `numaperf-sched` with per-node queues and configurable steal order.

**Docs (current)**
- `docs/architecture.md`
- `docs/api.md`
- `docs/roadmap.md`
- `docs/kernel-requirements.md`

**Docs (planned)**
- `docs/getting-started.md`
- `docs/hard-mode.md`

**Status**
This repository is a design-first workspace. APIs will be implemented incrementally, starting with topology, affinity, memory placement, and scheduling.
