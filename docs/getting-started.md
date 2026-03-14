# Getting Started with numaperf

This guide will help you get started with numaperf, a NUMA-first runtime for building high-performance Rust applications.

## Prerequisites

- **Rust 1.70+** (check with `rustc --version`)
- **Linux kernel 5.4+** with CONFIG_NUMA enabled (for full functionality)
- For testing on single-socket systems, numaperf provides graceful fallbacks

## Installation

Add numaperf to your `Cargo.toml`:

```toml
[dependencies]
numaperf = "0.1"
```

Or use cargo add:

```bash
cargo add numaperf
```

## Quick Start

### 1. Check System Capabilities

Before using NUMA features, check what your system supports:

```rust
use numaperf::Capabilities;

fn main() {
    let caps = Capabilities::detect();
    println!("{}", caps.summary());
}
```

This will output something like:

```
NUMA System Capabilities
========================
NUMA nodes detected: 2
CAP_SYS_ADMIN (strict memory binding): no
CAP_SYS_NICE (strict CPU affinity): yes
CAP_IPC_LOCK (memory locking): no
NUMA balancing disabled: no

Hard mode supported: NO

Missing for hard mode:
  - CAP_SYS_ADMIN (for strict memory binding)
  - kernel.numa_balancing=0 (to prevent automatic migration)
```

### 2. Discover NUMA Topology

Discover your system's NUMA layout:

```rust
use numaperf::Topology;

fn main() -> Result<(), numaperf::NumaError> {
    let topo = Topology::discover()?;

    println!("System has {} NUMA nodes", topo.node_count());

    for node in topo.numa_nodes() {
        println!("  Node {}: {} CPUs", node.id(), node.cpu_count());
    }

    Ok(())
}
```

### 3. Create a NUMA-Aware Worker Pool

The scheduler provides per-node worker pools with configurable work stealing:

```rust
use numaperf::{NumaExecutor, Topology, StealPolicy, NodeId};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

fn main() -> Result<(), numaperf::NumaError> {
    // Discover topology
    let topo = Arc::new(Topology::discover()?);

    // Create executor with 2 workers per node
    let exec = NumaExecutor::builder(Arc::clone(&topo))
        .steal_policy(StealPolicy::LocalThenSocketThenRemote)
        .workers_per_node(2)
        .build()?;

    // Track work completion
    let counter = Arc::new(AtomicUsize::new(0));

    // Submit work to each node
    for node in topo.numa_nodes() {
        let c = Arc::clone(&counter);
        exec.submit_to_node(node.id(), move || {
            // This runs on a worker pinned to the target node
            c.fetch_add(1, Ordering::SeqCst);
        });
    }

    // Graceful shutdown
    exec.shutdown();

    println!("Completed {} tasks", counter.load(Ordering::SeqCst));
    Ok(())
}
```

### 4. Allocate NUMA-Local Memory

Allocate memory with explicit placement policies:

```rust
use numaperf::{NumaRegion, MemPolicy, NodeMask, NodeId, Prefault};

fn main() -> Result<(), numaperf::NumaError> {
    // Allocate 1MB bound to node 0
    let nodes = NodeMask::single(NodeId::new(0));
    let mut region = NumaRegion::anon(
        1024 * 1024,                    // Size in bytes
        MemPolicy::Bind(nodes),         // Strict binding to node 0
        Default::default(),             // Default huge page settings
        Prefault::Touch,                // Fault in pages immediately
    )?;

    // Use the memory
    let slice = region.as_mut_slice();
    slice[0] = 42;

    println!("Allocated {} bytes on node 0", slice.len());
    Ok(())
}
```

### 5. Pin Threads for Data Locality

Pin threads to specific CPUs before allocating memory:

```rust
use numaperf::{ScopedPin, CpuSet};

fn main() -> Result<(), numaperf::NumaError> {
    let cpus = CpuSet::parse("0-3").expect("valid CPU set");

    {
        // Pin to CPUs 0-3 (presumably on one NUMA node)
        let _pin = ScopedPin::pin_current(cpus)?;

        // Memory allocated here will be local to this node
        let data: Vec<u8> = vec![0; 1024 * 1024];

        // Do work with the data...
        println!("Allocated {} bytes while pinned", data.len());
    }
    // Previous affinity is automatically restored

    Ok(())
}
```

## Memory Placement Policies

numaperf supports four memory placement policies:

| Policy | Description | Use Case |
|--------|-------------|----------|
| `Local` | Allocate on current thread's node (default) | General purpose |
| `Bind(nodes)` | Strict allocation on specified nodes | Guaranteed placement |
| `Preferred(node)` | Prefer one node, fallback allowed | Soft preference |
| `Interleave(nodes)` | Round-robin across nodes | Bandwidth-bound workloads |

Example with different policies:

```rust
use numaperf::{NumaRegion, MemPolicy, NodeMask, NodeId, Prefault};

fn main() -> Result<(), numaperf::NumaError> {
    // Local policy (default) - allocate on current node
    let _local = NumaRegion::anon(4096, MemPolicy::Local, Default::default(), Prefault::Touch)?;

    // Bind - strict allocation on node 0
    let nodes = NodeMask::single(NodeId::new(0));
    let _bound = NumaRegion::anon(4096, MemPolicy::Bind(nodes), Default::default(), Prefault::Touch)?;

    // Preferred - prefer node 0, but allow fallback
    let _preferred = NumaRegion::anon(4096, MemPolicy::Preferred(NodeId::new(0)), Default::default(), Prefault::Touch)?;

    // Interleave - round-robin across all nodes (for a 2-node system)
    let mut all_nodes = NodeMask::new();
    all_nodes.add(NodeId::new(0));
    all_nodes.add(NodeId::new(1));
    let _interleaved = NumaRegion::anon(4096, MemPolicy::Interleave(all_nodes), Default::default(), Prefault::Touch)?;

    Ok(())
}
```

## Work Stealing Policies

Control how workers steal work from other nodes:

| Policy | Description | Locality | Throughput |
|--------|-------------|----------|------------|
| `LocalOnly` | Never steal | Highest | Lower (may have idle workers) |
| `LocalThenSocketThenRemote` | Steal from nearby nodes first (default) | Balanced | Balanced |
| `Any` | Steal from any node | Lowest | Highest |

## Observability

Monitor NUMA locality effectiveness:

```rust
use numaperf::{StatsCollector, LocalityReport, Topology, NodeId};
use std::sync::Arc;

fn main() -> Result<(), numaperf::NumaError> {
    let topo = Arc::new(Topology::discover()?);
    let collector = StatsCollector::new(&topo);

    // Record metrics (in real usage, integrated with your executor)
    collector.record_local_execution();
    collector.record_local_execution();
    collector.record_steal(NodeId::new(1));

    // Generate diagnostic report
    let stats = collector.snapshot();
    let report = LocalityReport::generate(&stats);

    println!("Locality ratio: {:.1}%", stats.locality_ratio() * 100.0);
    println!("\n{}", report);

    Ok(())
}
```

## Soft Mode vs Hard Mode

numaperf supports two enforcement modes:

- **Soft mode** (default): Best-effort locality with graceful degradation
- **Hard mode**: Strict enforcement; operations fail if guarantees can't be met

```rust
use numaperf::{NumaExecutor, Topology, HardMode, Capabilities};
use std::sync::Arc;

fn main() -> Result<(), numaperf::NumaError> {
    let caps = Capabilities::detect();

    // Check if hard mode is supported
    if !caps.supports_hard_mode() {
        println!("Hard mode not available. Missing:");
        for cap in caps.missing_for_hard_mode() {
            println!("  - {}", cap);
        }
    }

    let topo = Arc::new(Topology::discover()?);

    // Use hard mode for strict enforcement
    let exec = NumaExecutor::builder(topo)
        .hard_mode(HardMode::Strict)
        .workers_per_node(2)
        .build()?;  // Will fail if pinning can't be guaranteed

    exec.shutdown();
    Ok(())
}
```

See the [Hard Mode Guide](hard-mode.md) for details.

## Next Steps

- [Architecture Overview](architecture.md) - Understand the design
- [API Reference](api.md) - Detailed API documentation
- [Hard Mode Guide](hard-mode.md) - Strict enforcement configuration
- [Kernel Requirements](kernel-requirements.md) - Platform support details

## Examples

Run the bundled examples:

```bash
# Discover topology
cargo run -p numaperf --example basic_topology

# NUMA-aware worker pool
cargo run -p numaperf --example worker_pool

# Memory placement
cargo run -p numaperf --example memory_placement

# Per-node buffer pool
cargo run -p numaperf --example buffer_pool

# Locality diagnostics
cargo run -p numaperf --example diagnostics
```
