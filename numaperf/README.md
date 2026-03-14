# numaperf

[![Crates.io](https://img.shields.io/crates/v/numaperf.svg)](https://crates.io/crates/numaperf)
[![Documentation](https://img.shields.io/badge/docs-skelfresearch.com-blue)](https://docs.skelfresearch.com/numaperf)

**NUMA-first runtime for latency-critical Rust applications.**

## Overview

numaperf is a facade crate that re-exports all public APIs from the numaperf workspace. It provides explicit control over memory placement, thread pinning, and work scheduling on NUMA systems.

## Usage

```toml
[dependencies]
numaperf = "0.1"
```

## Example

```rust
use numaperf::{Topology, ScopedPin, NumaRegion, MemPolicy, NodeMask, Prefault};

fn main() -> Result<(), numaperf::NumaError> {
    // Discover NUMA topology
    let topo = Topology::discover()?;
    let node0 = topo.numa_nodes()[0].id();

    // Pin this thread to node 0's CPUs
    let _pin = ScopedPin::to_node(&topo, node0)?;

    // Allocate 1 GB bound to node 0
    let region = NumaRegion::anon(
        1024 * 1024 * 1024,
        MemPolicy::Bind(NodeMask::single(node0)),
        Default::default(),
        Prefault::Touch,
    )?;

    println!("Allocated {} bytes on node {}", region.len(), node0);
    Ok(())
}
```

## Re-exported Crates

| Crate | Purpose |
|-------|---------|
| `numaperf-core` | Core types: NodeId, CpuSet, NodeMask, NumaError |
| `numaperf-topo` | Topology discovery |
| `numaperf-affinity` | Thread pinning |
| `numaperf-mem` | Memory placement |
| `numaperf-sched` | Work scheduling |
| `numaperf-sharded` | Per-node data structures |
| `numaperf-io` | Device locality |
| `numaperf-perf` | Observability |

## Part of numaperf

This is the main entry point for the [numaperf](https://github.com/Skelf-Research/numaperf) workspace.

- [Documentation](https://docs.skelfresearch.com/numaperf)
- [GitHub](https://github.com/Skelf-Research/numaperf)

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
