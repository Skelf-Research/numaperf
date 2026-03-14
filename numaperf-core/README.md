# numaperf-core

[![Crates.io](https://img.shields.io/crates/v/numaperf-core.svg)](https://crates.io/crates/numaperf-core)
[![Documentation](https://img.shields.io/badge/docs-skelfresearch.com-blue)](https://docs.skelfresearch.com/numaperf/api/core-types/)

**Core types and error handling for the numaperf NUMA runtime.**

## Overview

numaperf-core provides the foundational types used throughout the numaperf workspace: type-safe identifiers for NUMA nodes and CPUs, efficient bitset implementations for CPU and node masks, and a unified error type.

## Usage

```toml
[dependencies]
numaperf-core = "0.1"
```

Most users should use the `numaperf` facade crate instead.

## Example

```rust
use numaperf_core::{NodeId, CpuSet, NodeMask};

// Type-safe node identifier
let node = NodeId::new(0);

// CPU set with bitmap operations
let mut cpus = CpuSet::new();
cpus.add(0);
cpus.add(1);
assert!(cpus.contains(0));

// Node mask for memory policies
let mask = NodeMask::single(node);
```

## Types

- **`NodeId`** - Type-safe NUMA node identifier
- **`CpuSet`** - Efficient CPU set with bitmap operations
- **`NodeMask`** - NUMA node set for memory policies
- **`NumaError`** - Unified error type for all numaperf operations
- **`HardMode`** - Soft vs strict enforcement mode
- **`Capabilities`** - System capability detection

## Part of numaperf

This crate is part of the [numaperf](https://github.com/Skelf-Research/numaperf) workspace.

- [Documentation](https://docs.skelfresearch.com/numaperf)
- [GitHub](https://github.com/Skelf-Research/numaperf)

## License

Licensed under the [MIT License](../LICENSE).
