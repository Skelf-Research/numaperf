# numaperf-sharded

[![Crates.io](https://img.shields.io/crates/v/numaperf-sharded.svg)](https://crates.io/crates/numaperf-sharded)
[![Documentation](https://img.shields.io/badge/docs-skelfresearch.com-blue)](https://docs.skelfresearch.com/numaperf/api/sharded/)

**Per-node sharded data structures for NUMA systems.**

## Overview

numaperf-sharded provides data structures that maintain one shard per NUMA node, enabling lock-free local access patterns. Ideal for counters, caches, and other per-node state.

## Usage

```toml
[dependencies]
numaperf-sharded = "0.1"
```

Most users should use the `numaperf` facade crate instead.

## Example

```rust
use numaperf_sharded::{NumaSharded, ShardedCounter, CachePadded};
use numaperf_topo::Topology;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

fn main() -> Result<(), numaperf_core::NumaError> {
    let topo = Arc::new(Topology::discover()?);

    // Per-node counter (lock-free)
    let counter = ShardedCounter::new(&topo);
    counter.increment();  // Updates local shard
    println!("Total: {}", counter.sum());

    // Custom per-node data
    let data = NumaSharded::new(&topo, || AtomicUsize::new(0));
    data.local(|shard| {
        shard.fetch_add(1, Ordering::Relaxed);
    });

    Ok(())
}
```

## Features

- **`NumaSharded<T>`** - Per-node sharded container
- **`ShardedCounter`** - Lock-free distributed counter
- **`CachePadded<T>`** - Prevent false sharing (128-byte alignment)
- **Local access** - Fast path for current node's shard

## Types

- **`NumaSharded<T>`** - One T per NUMA node
- **`ShardedCounter`** - Specialized counter with `increment()` and `sum()`
- **`CachePadded<T>`** - Cache-line padded wrapper

## Part of numaperf

This crate is part of the [numaperf](https://github.com/Skelf-Research/numaperf) workspace.

- [Documentation](https://docs.skelfresearch.com/numaperf)
- [GitHub](https://github.com/Skelf-Research/numaperf)

## License

Licensed under the [MIT License](../LICENSE).
