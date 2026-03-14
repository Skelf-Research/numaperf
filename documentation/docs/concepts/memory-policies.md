# Memory Policies

numaperf supports four memory placement policies that control where memory is physically allocated.

## Overview

| Policy | Behavior | Use Case |
|--------|----------|----------|
| `Local` | Allocate on current thread's node | Default, general purpose |
| `Bind` | Strictly allocate on specified nodes | Guaranteed placement |
| `Preferred` | Prefer specified node, allow fallback | Soft preference |
| `Interleave` | Round-robin pages across nodes | Bandwidth-bound workloads |

## Local Policy (Default)

Memory is allocated on the **current thread's NUMA node**:

```rust
use numaperf::{NumaRegion, MemPolicy, Prefault};

let region = NumaRegion::anon(
    1024 * 1024,
    MemPolicy::Local,  // Allocate on current node
    Default::default(),
    Prefault::Touch,
)?;
```

### When to Use Local

- General purpose allocations
- When combined with thread pinning
- When you don't know the optimal node at allocation time

### The Pin-Then-Allocate Pattern

Local policy works best with thread pinning:

```rust
use numaperf::{ScopedPin, Topology};

let topo = Topology::discover()?;

for node in topo.numa_nodes() {
    // 1. Pin to this node
    let _pin = ScopedPin::pin_current(node.cpus().clone())?;

    // 2. Allocate (will be local to this node)
    let data = NumaRegion::anon(size, MemPolicy::Local, ...)?;

    // 3. Use data - guaranteed local access
}
```

## Bind Policy

Memory is **strictly allocated** on the specified nodes:

```rust
use numaperf::{NumaRegion, MemPolicy, NodeMask, NodeId, Prefault};

// Bind to node 0 only
let node0 = NodeMask::single(NodeId::new(0));
let region = NumaRegion::anon(
    1024 * 1024,
    MemPolicy::Bind(node0),
    Default::default(),
    Prefault::Touch,
)?;
```

### Binding to Multiple Nodes

```rust
// Bind to nodes 0 and 1
let mut nodes = NodeMask::new();
nodes.add(NodeId::new(0));
nodes.add(NodeId::new(1));

let region = NumaRegion::anon(
    size,
    MemPolicy::Bind(nodes),
    Default::default(),
    Prefault::Touch,
)?;
```

### When to Use Bind

- Critical data that must be on specific nodes
- When you know the access pattern at allocation time
- Data accessed by pinned threads

### Bind Limitations

- Requires `CAP_SYS_ADMIN` for strict enforcement
- Allocation fails if nodes don't have enough memory
- In soft mode, may fall back to other nodes

## Preferred Policy

Memory is **preferentially allocated** on the specified node, but falls back to other nodes if needed:

```rust
use numaperf::{NumaRegion, MemPolicy, NodeId, Prefault};

let region = NumaRegion::anon(
    1024 * 1024,
    MemPolicy::Preferred(NodeId::new(0)),  // Prefer node 0
    Default::default(),
    Prefault::Touch,
)?;
```

### When to Use Preferred

- Soft locality preference
- When availability is more important than strict placement
- Large allocations that might exceed a single node's capacity

### Preferred vs Bind

| Aspect | Bind | Preferred |
|--------|------|-----------|
| Enforcement | Strict | Soft |
| Failure mode | Error or degraded | Always succeeds |
| Privileges | May need CAP_SYS_ADMIN | No special privileges |
| Use case | Critical data | Nice-to-have locality |

## Interleave Policy

Memory is **distributed round-robin** across specified nodes:

```rust
use numaperf::{NumaRegion, MemPolicy, NodeMask, NodeId, Prefault, Topology};

let topo = Topology::discover()?;

// Interleave across all nodes
let mut all_nodes = NodeMask::new();
for node in topo.numa_nodes() {
    all_nodes.add(node.id());
}

let region = NumaRegion::anon(
    1024 * 1024,
    MemPolicy::Interleave(all_nodes),
    Default::default(),
    Prefault::Touch,
)?;
```

### How Interleaving Works

Pages are distributed round-robin:

```
Page 0 → Node 0
Page 1 → Node 1
Page 2 → Node 0
Page 3 → Node 1
...
```

### When to Use Interleave

- **Bandwidth-bound workloads** - Aggregate bandwidth from all nodes
- **Streaming access patterns** - Sequential reads/writes
- **Shared data structures** - Accessed equally from all nodes

### Interleave Trade-offs

| Benefit | Cost |
|---------|------|
| Higher aggregate bandwidth | No locality guarantees |
| Even memory distribution | Some accesses are remote |
| Good for streaming | Worse for random access |

## Prefault Strategies

The `Prefault` parameter controls when memory is physically allocated:

```rust
pub enum Prefault {
    None,   // Allocate on first access
    Touch,  // Allocate immediately
}
```

### None (Lazy Allocation)

```rust
let region = NumaRegion::anon(
    size,
    policy,
    Default::default(),
    Prefault::None,  // Pages allocated on first access
)?;
```

- **Pros**: Fast allocation, only allocates what's used
- **Cons**: First access may be slow, allocation happens on accessing thread's node

### Touch (Eager Allocation)

```rust
let region = NumaRegion::anon(
    size,
    policy,
    Default::default(),
    Prefault::Touch,  // All pages allocated immediately
)?;
```

- **Pros**: Predictable performance, memory on intended nodes
- **Cons**: Slower allocation, allocates full size

### Recommendation

Use `Prefault::Touch` for:

- Critical hot data
- Bind policy (ensures placement before use)
- Performance-sensitive paths

Use `Prefault::None` for:

- Large sparse allocations
- Memory pools with gradual usage
- When allocation speed matters

## Choosing a Policy

```
┌─────────────────────────────────────────────────────────┐
│                    Start Here                           │
└────────────────────────┬────────────────────────────────┘
                         │
                         ▼
              ┌──────────────────────┐
              │ Is strict placement  │
              │ required?            │
              └──────────┬───────────┘
                    Yes  │  No
                    ┌────┴────┐
                    ▼         ▼
            ┌───────────┐  ┌───────────────────┐
            │   Bind    │  │ Is bandwidth more │
            └───────────┘  │ important than    │
                           │ latency?          │
                           └─────────┬─────────┘
                                Yes  │  No
                                ┌────┴────┐
                                ▼         ▼
                        ┌────────────┐  ┌─────────────────┐
                        │ Interleave │  │ Do you know the │
                        └────────────┘  │ target node?    │
                                        └────────┬────────┘
                                            Yes  │  No
                                            ┌────┴────┐
                                            ▼         ▼
                                    ┌───────────┐  ┌───────┐
                                    │ Preferred │  │ Local │
                                    └───────────┘  └───────┘
```

## Policy Comparison

| Scenario | Recommended Policy |
|----------|-------------------|
| Thread-private data | Local + pinning |
| Shared read-only data | Bind to single node |
| Write-heavy shared data | Interleave |
| Large lookup table | Interleave |
| Per-thread buffers | Local |
| Connection state | Bind to handler's node |
| Streaming I/O buffers | Interleave |

## Checking Enforcement

Always verify what you actually got:

```rust
let region = NumaRegion::anon(...)?;

println!("Enforcement: {:?}", region.enforcement());

if !region.enforcement().is_strict() {
    log::warn!(
        "Memory placement degraded: {:?}",
        region.enforcement().reason()
    );
}
```

## Next Steps

- [Memory Allocation Guide](../guides/memory-allocation.md) - Practical patterns
- [Soft vs Hard Mode](soft-vs-hard-mode.md) - Enforcement modes
- [API Reference: Memory](../api/memory.md) - Complete API
