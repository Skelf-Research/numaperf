# API Sketch

This document captures the intended public API shape. The exact names may change, but the roles and boundaries should remain stable.

**Topology**
```rust
use numaperf_topo::{Topology, NodeId, CpuSet};

let topo = Topology::discover()?;
let nodes = topo.numa_nodes();
let node0 = nodes[0].id();
let cpus = topo.cpu_set(node0);
```

Note: crate names use hyphens in Cargo (`numaperf-topo`) but underscores in Rust paths (`numaperf_topo`).

**Affinity**
```rust
use numaperf_affinity::ScopedPin;

let _pin = ScopedPin::pin_current(cpus)?;
// current thread is now pinned to the given CPU set
```

**Memory policies**
```rust
use numaperf_mem::{MemPolicy, NumaRegion, HugePageMode, Prefault};

let policy = MemPolicy::Bind([node0].into());
let region = NumaRegion::anon(
    2 * 1024 * 1024 * 1024,
    policy,
    HugePageMode::TransparentOn,
    Prefault::ParallelTouch,
)?;
```

**Memory policies model**
```rust
pub enum MemPolicy {
    Bind(NodeMask),
    Preferred(NodeId),
    Interleave(NodeMask),
    Local,
}
```

**Scheduling**
```rust
use std::sync::Arc;
use numaperf_sched::{NumaExecutor, StealPolicy};

let topo = Arc::new(Topology::discover()?);
let exec = NumaExecutor::new(Arc::clone(&topo), StealPolicy::LocalThenSocketThenRemote)?;
exec.submit(node0, || {
    // data-local work here
});
```

**Sharded structures**
```rust
use numaperf_sharded::NumaSharded;

let shards = NumaSharded::new(&topo, || MyShard::default());
shards.local(|shard| shard.record(1));
```

**Observability**
```rust
use numaperf_perf::LocalityStats;

let stats = LocalityStats::snapshot();
let remote_steals = stats.remote_steals();
```

**Hard mode toggles**
`numaperf` treats hard mode as explicit and opt-in. If a policy cannot be enforced, creation returns a structured error rather than silently degrading.

```rust
use numaperf_core::HardMode;

let hard = HardMode::Strict;
let exec = NumaExecutor::with_mode(Arc::clone(&topo), StealPolicy::LocalThenSocketThenRemote, hard)?;
```

---

## Error handling

All fallible operations return `Result<T, NumaError>`. Errors are structured to explain both what failed and why.

**Error types**
```rust
use numaperf_core::NumaError;

pub enum NumaError {
    /// The requested policy is not supported on this kernel.
    PolicyNotSupported { policy: MemPolicy, reason: String },

    /// Required capability is missing (e.g., CAP_SYS_NICE).
    CapabilityMissing { capability: &'static str },

    /// Hard mode was requested but cannot be enforced.
    HardModeUnavailable { feature: &'static str, reason: String },

    /// Topology discovery failed.
    TopologyError { source: std::io::Error },

    /// Memory allocation or mapping failed.
    AllocationFailed { size: usize, source: std::io::Error },

    /// Thread pinning failed.
    PinningFailed { cpus: CpuSet, source: std::io::Error },
}
```

**Handling errors**
```rust
use numaperf_mem::{NumaRegion, MemPolicy, HugePageMode, Prefault};
use numaperf_core::NumaError;

match NumaRegion::anon(size, policy, huge, prefault) {
    Ok(region) => {
        // Use the region
    }
    Err(NumaError::PolicyNotSupported { policy, reason }) => {
        eprintln!("Policy {:?} not supported: {}", policy, reason);
        // Fall back to standard allocation
    }
    Err(NumaError::CapabilityMissing { capability }) => {
        eprintln!("Missing capability: {}", capability);
        // Request elevation or use soft mode
    }
    Err(e) => return Err(e.into()),
}
```

---

## Soft mode fallbacks

Soft mode is the default. When a feature is unavailable, operations succeed with reduced guarantees and report the degradation.

**Checking enforcement level**
```rust
use numaperf_mem::{NumaRegion, EnforcementLevel};

let region = NumaRegion::anon(size, policy, huge, prefault)?;

match region.enforcement() {
    EnforcementLevel::Strict => {
        // Policy is fully enforced
    }
    EnforcementLevel::BestEffort { reason } => {
        // Policy applied but not guaranteed
        log::warn!("NUMA policy degraded: {}", reason);
    }
    EnforcementLevel::None { reason } => {
        // No NUMA policy applied
        log::warn!("NUMA policy not applied: {}", reason);
    }
}
```

**Requiring strict enforcement**
```rust
use numaperf_core::HardMode;
use numaperf_mem::NumaRegion;

// This returns Err if strict enforcement is impossible
let region = NumaRegion::anon_with_mode(
    size, policy, huge, prefault,
    HardMode::Strict,
)?;
```

---

## Ownership and lifetimes

**Topology** is `Send + Sync` and should be created once and shared.
```rust
use std::sync::Arc;
use numaperf_topo::Topology;

let topo = Arc::new(Topology::discover()?);
// Share `topo` across threads
```

**ScopedPin** restores the prior affinity when dropped.
```rust
{
    let _pin = ScopedPin::pin_current(cpus)?;
    // Thread is pinned here
} // Affinity restored on drop
```

**NumaRegion** owns its memory and unmaps on drop.
```rust
let region = NumaRegion::anon(size, policy, huge, prefault)?;
let slice: &mut [u8] = region.as_mut_slice();
// `slice` borrows from `region`
drop(region); // Memory is unmapped
```

---

## Common patterns

**Pin-then-allocate pattern**
```rust
// Pin first to ensure first-touch goes to the right node
let _pin = ScopedPin::pin_current(topo.cpu_set(node))?;
let region = NumaRegion::anon(size, MemPolicy::Local, huge, Prefault::Touch)?;
```

**Per-node worker pools**
```rust
let topo = Arc::new(Topology::discover()?);
let handles: Vec<_> = topo.numa_nodes().iter().map(|node| {
    let topo = Arc::clone(&topo);
    let cpus = topo.cpu_set(node.id());
    std::thread::spawn(move || {
        let _pin = ScopedPin::pin_current(cpus).unwrap();
        // This thread is now pinned to `node`
        worker_loop()
    })
}).collect();
```

**Graceful degradation**
```rust
let region = match NumaRegion::anon(size, MemPolicy::Bind([node].into()), huge, prefault) {
    Ok(r) => r,
    Err(NumaError::PolicyNotSupported { .. }) => {
        // Fall back to local policy
        NumaRegion::anon(size, MemPolicy::Local, huge, prefault)?
    }
    Err(e) => return Err(e),
};
```
