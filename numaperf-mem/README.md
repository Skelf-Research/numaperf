# numaperf-mem

[![Crates.io](https://img.shields.io/crates/v/numaperf-mem.svg)](https://crates.io/crates/numaperf-mem)
[![Documentation](https://img.shields.io/badge/docs-skelfresearch.com-blue)](https://docs.skelfresearch.com/numaperf/api/memory/)

**NUMA-aware memory allocation and placement policies.**

## Overview

numaperf-mem provides explicit control over memory placement on NUMA systems. Allocate memory regions with specific placement policies: bind to nodes, prefer nodes, or interleave across nodes.

## Usage

```toml
[dependencies]
numaperf-mem = "0.1"
```

Most users should use the `numaperf` facade crate instead.

## Example

```rust
use numaperf_mem::{NumaRegion, MemPolicy, Prefault};
use numaperf_core::{NodeId, NodeMask};

fn main() -> Result<(), numaperf_core::NumaError> {
    let node0 = NodeId::new(0);

    // Allocate 1 GB strictly bound to node 0
    let region = NumaRegion::anon(
        1024 * 1024 * 1024,
        MemPolicy::Bind(NodeMask::single(node0)),
        Default::default(),
        Prefault::Touch,
    )?;

    // Use as a byte slice
    let slice = region.as_mut_slice();
    slice[0] = 42;

    Ok(())
} // Memory automatically unmapped on drop
```

## Memory Policies

| Policy | Behavior |
|--------|----------|
| `Local` | Allocate on current thread's node |
| `Bind(nodes)` | Strictly allocate on specified nodes |
| `Preferred(node)` | Prefer node, but allow fallback |
| `Interleave(nodes)` | Round-robin pages across nodes |

## Features

- **`NumaRegion`** - RAII memory region with NUMA placement
- **Prefaulting** - Touch pages immediately or lazily
- **Huge pages** - Transparent or explicit huge page support
- **Hard mode** - Strict enforcement with `MPOL_BIND`

## Part of numaperf

This crate is part of the [numaperf](https://github.com/Skelf-Research/numaperf) workspace.

- [Documentation](https://docs.skelfresearch.com/numaperf)
- [GitHub](https://github.com/Skelf-Research/numaperf)

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
