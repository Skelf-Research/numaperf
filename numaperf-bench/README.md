# numaperf-bench

[![Crates.io](https://img.shields.io/crates/v/numaperf-bench.svg)](https://crates.io/crates/numaperf-bench)
[![Documentation](https://img.shields.io/badge/docs-skelfresearch.com-blue)](https://docs.skelfresearch.com/numaperf/reference/cli/)

**Benchmark suite and CLI tools for numaperf.**

## Overview

numaperf-bench provides a command-line tool for benchmarking NUMA operations and displaying system topology information. Use it to validate your system's NUMA configuration and measure locality effectiveness.

## Installation

```bash
cargo install numaperf-bench
```

## Usage

### System Information

```bash
# Show all system info
numaperf-bench info

# Show specific section
numaperf-bench info topology
numaperf-bench info capabilities
numaperf-bench info distances

# JSON output
numaperf-bench info --format json
```

### Benchmarks

```bash
# Run all benchmarks
numaperf-bench bench

# Run specific category
numaperf-bench bench --category memory
numaperf-bench bench --category scheduler
numaperf-bench bench --category sharded
numaperf-bench bench --category affinity

# Custom iterations
numaperf-bench bench --iterations 10000

# JSON output for CI
numaperf-bench bench --format json
```

## Sample Output

```
=== numaperf System Information ===

Topology
--------
NUMA Nodes: 2

  Node 0:
    CPUs: 0-7 (8 total)

  Node 1:
    CPUs: 8-15 (8 total)

Capabilities
------------
Hard mode supported: yes
```

## Benchmark Categories

| Category | What it measures |
|----------|------------------|
| `sharded` | Per-node counter operations |
| `memory` | NUMA region allocation |
| `scheduler` | Executor task submission |
| `affinity` | Thread pinning operations |

## Part of numaperf

This crate is part of the [numaperf](https://github.com/Skelf-Research/numaperf) workspace.

- [Documentation](https://docs.skelfresearch.com/numaperf)
- [GitHub](https://github.com/Skelf-Research/numaperf)

## License

Licensed under the [MIT License](../LICENSE).
