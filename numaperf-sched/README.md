# numaperf-sched

[![Crates.io](https://img.shields.io/crates/v/numaperf-sched.svg)](https://crates.io/crates/numaperf-sched)
[![Documentation](https://img.shields.io/badge/docs-skelfresearch.com-blue)](https://docs.skelfresearch.com/numaperf/api/scheduler/)

**Topology-aware work scheduling with locality-preserving stealing.**

## Overview

numaperf-sched provides a NUMA-aware task executor with per-node worker pools. Workers are pinned to their respective NUMA nodes, and work stealing can be configured to preserve locality.

## Usage

```toml
[dependencies]
numaperf-sched = "0.1"
```

Most users should use the `numaperf` facade crate instead.

## Example

```rust
use numaperf_sched::{NumaExecutor, StealPolicy};
use numaperf_topo::Topology;
use std::sync::Arc;

fn main() -> Result<(), numaperf_core::NumaError> {
    let topo = Arc::new(Topology::discover()?);

    let exec = NumaExecutor::builder(Arc::clone(&topo))
        .steal_policy(StealPolicy::LocalThenSocketThenRemote)
        .workers_per_node(2)
        .build()?;

    // Submit work to a specific node
    let node0 = topo.numa_nodes()[0].id();
    exec.submit_to_node(node0, || {
        println!("Running on node 0!");
    });

    exec.shutdown();
    Ok(())
}
```

## Steal Policies

| Policy | Behavior |
|--------|----------|
| `LocalOnly` | Never steal from other nodes |
| `LocalThenSocketThenRemote` | Prefer nearby nodes |
| `Any` | Steal from any node |

## Features

- **Per-node worker pools** - Workers pinned to NUMA nodes
- **Configurable stealing** - Balance locality vs throughput
- **Builder pattern** - Fluent configuration
- **Graceful shutdown** - Wait for pending work

## Types

- **`NumaExecutor`** - NUMA-aware task executor
- **`NumaExecutorBuilder`** - Builder for configuration
- **`StealPolicy`** - Work stealing strategy

## Part of numaperf

This crate is part of the [numaperf](https://github.com/Skelf-Research/numaperf) workspace.

- [Documentation](https://docs.skelfresearch.com/numaperf)
- [GitHub](https://github.com/Skelf-Research/numaperf)

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
