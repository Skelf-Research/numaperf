# numaperf

[![Crates.io](https://img.shields.io/crates/v/numaperf.svg)](https://crates.io/crates/numaperf)
[![Documentation](https://img.shields.io/badge/docs-skelfresearch.com-blue)](https://docs.skelfresearch.com/numaperf)
[![License](https://img.shields.io/crates/l/numaperf.svg)](https://github.com/Skelf-Research/numaperf#license)
[![CI](https://github.com/Skelf-Research/numaperf/actions/workflows/ci.yml/badge.svg)](https://github.com/Skelf-Research/numaperf/actions)
[![MSRV](https://img.shields.io/badge/MSRV-1.70-blue.svg)](https://github.com/Skelf-Research/numaperf)

**NUMA-first runtime for latency-critical Rust applications.**

numaperf gives you explicit control over memory placement, thread pinning, and work scheduling on NUMA systems. Stop guessing where your data lives and start guaranteeing it.

## Why numaperf?

On multi-socket servers, memory access latency varies by 2-3x depending on which CPU accesses which memory. Most applications ignore this, leading to unpredictable performance. numaperf makes NUMA a first-class concern:

| Approach | Limitation |
|----------|------------|
| **First-touch policy** | Fragile. Initialization order determines placement. Refactoring breaks locality. |
| **numactl / libnuma** | Process-level only. No per-region control, C API, no runtime observability. |
| **NUMA-aware allocators** | Good for small objects, but doesn't address large buffers, scheduling, or cross-node traffic. |
| **numaperf** | Explicit per-region placement, topology-aware scheduling, cross-node observability, hard-mode enforcement. |

## Quick Start

```bash
cargo add numaperf
```

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

    // region.as_mut_slice() is now guaranteed local to node 0
    println!("Allocated {} bytes on node {}", region.len(), node0);
    Ok(())
}
```

## Features

- **Topology Discovery** - Query NUMA nodes, CPUs, and inter-node distances at runtime
- **Thread Pinning** - RAII-based CPU affinity with `ScopedPin`
- **Memory Placement** - Explicit policies: Bind, Preferred, Interleave, Local
- **Work Scheduling** - `NumaExecutor` with per-node worker pools and configurable work stealing
- **Sharded Data** - `NumaSharded<T>` for per-node data structures, `ShardedCounter` for lock-free counting
- **Device Locality** - Map NICs and NVMe devices to their NUMA nodes
- **Observability** - Track locality ratios, generate health reports, identify cross-node traffic
- **Hard Mode** - Strict enforcement when you need guarantees, graceful degradation when you don't

## Use Cases

**Database Engines** - Pin buffer pools to specific nodes, schedule queries on data-local workers

**Network Processing** - Allocate packet buffers on the NIC's local node, process without cross-node copies

**Scientific Computing** - Partition large arrays across nodes, compute with guaranteed locality

**Trading Systems** - Eliminate latency variance from NUMA effects with strict pinning and placement

## Documentation

- [**Getting Started**](https://docs.skelfresearch.com/numaperf/getting-started/quickstart/) - 5-minute tutorial
- [**Guides**](https://docs.skelfresearch.com/numaperf/guides/topology-discovery/) - How-to guides for common tasks
- [**API Reference**](https://docs.skelfresearch.com/numaperf/api/overview/) - Complete API documentation
- [**Examples**](https://docs.skelfresearch.com/numaperf/examples/basic-topology/) - Annotated code examples

## Crate Structure

numaperf is organized as a workspace. Use the `numaperf` facade crate for everything, or pick individual crates:

| Crate | Purpose |
|-------|---------|
| `numaperf` | Facade - re-exports all public APIs |
| `numaperf-topo` | Topology discovery |
| `numaperf-affinity` | Thread pinning |
| `numaperf-mem` | Memory placement |
| `numaperf-sched` | Work scheduling |
| `numaperf-sharded` | Per-node data structures |
| `numaperf-io` | Device locality |
| `numaperf-perf` | Observability |

## Platform Support

| Platform | Support |
|----------|---------|
| Linux x86_64 | Full |
| Linux aarch64 | Full |
| macOS | Graceful degradation (no NUMA hardware) |

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

Contributions are welcome! Please see our [GitHub repository](https://github.com/Skelf-Research/numaperf) for:

- [Issue tracker](https://github.com/Skelf-Research/numaperf/issues)
- [Contributing guidelines](https://github.com/Skelf-Research/numaperf/blob/main/CONTRIBUTING.md)

For support, contact [support@skelfresearch.com](mailto:support@skelfresearch.com).

---

Built with care by [Skelf Research](https://github.com/Skelf-Research).
