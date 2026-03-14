# numaperf

**NUMA-first runtime for latency-critical Rust applications**

numaperf is a comprehensive toolkit for building NUMA-aware applications in Rust. It provides topology discovery, thread pinning, memory placement, work scheduling, and observability—all designed to maximize data locality and minimize cross-node traffic.

## Why numaperf?

Modern servers have Non-Uniform Memory Access (NUMA) architectures where memory access latency depends on which CPU accesses which memory. Accessing "remote" memory (on a different NUMA node) can be 2-3x slower than "local" memory.

numaperf helps you:

- **Discover** your system's NUMA topology
- **Pin** threads to specific CPUs for consistent locality
- **Allocate** memory on specific NUMA nodes
- **Schedule** work to maintain data locality
- **Monitor** locality effectiveness

## Quick Example

```rust
use numaperf::{Topology, NumaExecutor, StealPolicy};
use std::sync::Arc;

fn main() -> Result<(), numaperf::NumaError> {
    // Discover NUMA topology
    let topo = Arc::new(Topology::discover()?);
    println!("System has {} NUMA nodes", topo.node_count());

    // Create NUMA-aware executor
    let exec = NumaExecutor::builder(Arc::clone(&topo))
        .steal_policy(StealPolicy::LocalThenSocketThenRemote)
        .workers_per_node(2)
        .build()?;

    // Submit work to specific nodes
    for node in topo.numa_nodes() {
        exec.submit_to_node(node.id(), || {
            println!("Running on node {}!", node.id().as_u32());
        });
    }

    exec.shutdown();
    Ok(())
}
```

## Features

| Feature | Description |
|---------|-------------|
| **Topology Discovery** | Automatic detection of NUMA nodes, CPUs, and distances |
| **Thread Pinning** | RAII-based CPU affinity with automatic restoration |
| **Memory Placement** | Explicit memory allocation policies (Bind, Preferred, Interleave) |
| **Work Scheduling** | Per-node worker pools with configurable work stealing |
| **Sharded Data** | Lock-free per-node data structures |
| **Device Locality** | Map network and block devices to NUMA nodes |
| **Observability** | Locality metrics and diagnostic reports |

## Getting Started

<div class="grid cards" markdown>

-   :material-download:{ .lg .middle } **Installation**

    ---

    Add numaperf to your project and check system requirements

    [:octicons-arrow-right-24: Installation](getting-started/installation.md)

-   :material-rocket-launch:{ .lg .middle } **Quickstart**

    ---

    Build your first NUMA-aware application in 5 minutes

    [:octicons-arrow-right-24: Quickstart](getting-started/quickstart.md)

-   :material-book-open-variant:{ .lg .middle } **Concepts**

    ---

    Understand NUMA basics and numaperf's architecture

    [:octicons-arrow-right-24: NUMA Basics](concepts/numa-basics.md)

-   :material-api:{ .lg .middle } **API Reference**

    ---

    Complete API documentation for all types and functions

    [:octicons-arrow-right-24: API Reference](api/overview.md)

</div>

## Platform Support

| Platform | Support Level |
|----------|--------------|
| Linux (NUMA) | Full support |
| Linux (single socket) | Graceful fallback |
| macOS | Topology only |
| Windows | Limited |

## License

numaperf is dual-licensed under MIT and Apache 2.0.
