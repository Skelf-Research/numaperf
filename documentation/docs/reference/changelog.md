# Changelog

All notable changes to numaperf.

## [Unreleased]

### Added

- Initial public release
- Comprehensive MkDocs documentation

---

## [0.1.0] - 2024-XX-XX

### Added

#### Core Types
- `NodeId` - Type-safe NUMA node identifier
- `CpuSet` - Efficient CPU set with bitmap operations
- `NodeMask` - NUMA node set for memory policies

#### Topology Discovery
- `Topology::discover()` - Automatic topology detection from sysfs
- `NumaNode` - Per-node information (CPUs, memory, distances)
- Distance matrix queries

#### Memory Management
- `NumaRegion` - RAII NUMA-aware memory regions
- `MemPolicy` - Memory placement policies (Local, Bind, Preferred, Interleave)
- `Prefault` - Page fault strategies (None, Touch, Populate)
- `HardMode` - Strict vs soft enforcement

#### Thread Affinity
- `ScopedPin` - RAII thread pinning
- `pin_to_node()` - Pin to all CPUs on a node
- `get_affinity()` / `set_affinity()` - Low-level affinity control

#### Work Scheduling
- `NumaExecutor` - NUMA-aware task executor
- `NumaExecutorBuilder` - Fluent configuration
- `StealPolicy` - Work stealing strategies (LocalOnly, LocalThenSocketThenRemote, Any)
- Per-node worker pools with configurable thread counts

#### Sharded Data Structures
- `NumaSharded<T>` - Per-node sharded data
- `ShardedCounter` - Lock-free distributed counter
- `CachePadded<T>` - Cache line padding to prevent false sharing

#### I/O Device Locality
- `DeviceMap` - Discover device-to-node mapping
- `DeviceLocality` - Device NUMA information
- Network and block device support

#### Observability
- `StatsCollector` - Lock-free locality metrics
- `LocalityStats` - Point-in-time snapshots
- `LocalityReport` - Diagnostic reports with health classification
- `LocalityHealth` - Health levels (Excellent, Good, Fair, Poor)

#### System Capabilities
- `Capabilities::detect()` - Runtime capability detection
- Hard mode support checking
- Missing capability reporting

#### Error Handling
- `NumaError` - Unified error type
- `HardModeUnavailable` - Detailed hard mode failures

#### CLI Tools
- `numaperf-bench` - Benchmarking and system information
- `info` subcommand - System topology display
- `bench` subcommand - Performance benchmarks
- JSON output support

### Platform Support
- Linux x86_64 - Full support
- Linux aarch64 - Full support
- macOS - Graceful degradation (no NUMA)

---

## Version History

| Version | Date | Highlights |
|---------|------|------------|
| 0.1.0 | TBD | Initial release |

---

## Versioning Policy

numaperf follows [Semantic Versioning](https://semver.org/):

- **MAJOR**: Breaking API changes
- **MINOR**: New features, backward compatible
- **PATCH**: Bug fixes, backward compatible

### Stability Guarantees

- **Stable API**: All public items in the `numaperf` facade crate
- **Unstable API**: Items marked with `#[doc(hidden)]` or in `-internal` crates

### Minimum Supported Rust Version (MSRV)

- Current MSRV: **1.70.0**
- MSRV bumps are considered minor version changes

---

## Migration Guides

### Upgrading to 0.2.0 (Future)

*No breaking changes planned yet.*

---

## Reporting Issues

Found a bug or have a feature request?

1. Check [existing issues](https://github.com/numaperf/numaperf/issues)
2. Open a new issue with:
   - numaperf version
   - Rust version
   - Operating system and kernel version
   - Minimal reproduction case

---

## Contributing

See [CONTRIBUTING.md](https://github.com/numaperf/numaperf/blob/main/CONTRIBUTING.md) for guidelines.
