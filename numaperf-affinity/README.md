# numaperf-affinity

[![Crates.io](https://img.shields.io/crates/v/numaperf-affinity.svg)](https://crates.io/crates/numaperf-affinity)
[![Documentation](https://img.shields.io/badge/docs-skelfresearch.com-blue)](https://docs.skelfresearch.com/numaperf/api/affinity/)

**Thread pinning and CPU affinity management.**

## Overview

numaperf-affinity provides RAII-based thread pinning with `ScopedPin`, plus low-level affinity control functions. Pin threads to specific CPUs or NUMA nodes to ensure predictable memory access patterns.

## Usage

```toml
[dependencies]
numaperf-affinity = "0.1"
```

Most users should use the `numaperf` facade crate instead.

## Example

```rust
use numaperf_affinity::{ScopedPin, get_affinity};
use numaperf_core::CpuSet;

fn main() -> Result<(), numaperf_core::NumaError> {
    // Check current affinity
    let current = get_affinity()?;
    println!("Currently allowed on {} CPUs", current.count());

    // Pin to specific CPUs (RAII - unpins on drop)
    let cpus = CpuSet::from_range(0, 4);
    let _pin = ScopedPin::to_cpus(cpus)?;

    // Thread is now pinned to CPUs 0-3
    do_work();

    Ok(())
} // Automatically unpinned here
```

## Features

- **`ScopedPin`** - RAII thread pinning, restores affinity on drop
- **`get_affinity()`** - Query current thread's allowed CPUs
- **`set_affinity()`** - Set thread's CPU affinity
- **Node pinning** - Pin to all CPUs on a NUMA node
- **Hard mode** - Strict enforcement with verification

## Types

- **`ScopedPin`** - RAII guard for thread pinning

## Part of numaperf

This crate is part of the [numaperf](https://github.com/Skelf-Research/numaperf) workspace.

- [Documentation](https://docs.skelfresearch.com/numaperf)
- [GitHub](https://github.com/Skelf-Research/numaperf)

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
