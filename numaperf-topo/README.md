# numaperf-topo

[![Crates.io](https://img.shields.io/crates/v/numaperf-topo.svg)](https://crates.io/crates/numaperf-topo)
[![Documentation](https://img.shields.io/badge/docs-skelfresearch.com-blue)](https://docs.skelfresearch.com/numaperf/api/topology/)

**NUMA topology discovery and CPU locality mapping.**

## Overview

numaperf-topo discovers your system's NUMA topology at runtime by reading from Linux sysfs. It provides information about NUMA nodes, their CPUs, and inter-node distances.

## Usage

```toml
[dependencies]
numaperf-topo = "0.1"
```

Most users should use the `numaperf` facade crate instead.

## Example

```rust
use numaperf_topo::Topology;

fn main() -> Result<(), numaperf_core::NumaError> {
    let topo = Topology::discover()?;

    println!("NUMA nodes: {}", topo.node_count());

    for node in topo.numa_nodes() {
        println!("Node {}: {} CPUs", node.id(), node.cpu_count());

        // Check distance to other nodes
        for other in topo.numa_nodes() {
            if let Some(dist) = node.distance_to(other.id()) {
                println!("  Distance to {}: {}", other.id(), dist);
            }
        }
    }

    Ok(())
}
```

## Features

- **Topology discovery** from `/sys/devices/system/node/`
- **Per-node information**: CPU count, CPU set, memory info
- **Distance matrix**: Inter-node access latencies
- **CPU-to-node mapping**: Find which node owns a CPU

## Types

- **`Topology`** - System NUMA topology
- **`NumaNode`** - Information about a single NUMA node

## Part of numaperf

This crate is part of the [numaperf](https://github.com/Skelf-Research/numaperf) workspace.

- [Documentation](https://docs.skelfresearch.com/numaperf)
- [GitHub](https://github.com/Skelf-Research/numaperf)

## License

Licensed under the [MIT License](../LICENSE).
