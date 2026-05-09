# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-05-09

### Added

- **C API** (`numaperf-c` crate): Stable C FFI bindings for NUMA topology discovery, thread affinity, and memory placement. The C API is built as both a shared library (`cdylib`) and a static library (`staticlib`), with C headers generated via `cbindgen`.
  - Topology: `npa_topology_discover()`, `npa_topology_free()`, `npa_topology_node_count()`, `npa_topology_cpu_count()`, `npa_topology_node_id()`, `npa_topology_node_cpu_count()`, `npa_topology_node_cpus()`, `npa_topology_node_distance()`, `npa_topology_node_memory_bytes()`, `npa_topology_node_for_cpu()`.
  - CPU set helpers: `npa_cpuset_new()`, `npa_cpuset_add()`, `npa_cpuset_remove()`, `npa_cpuset_contains()`, `npa_cpuset_count()`, `npa_cpuset_single()`.
  - Node mask helpers: `npa_nodemask_new()`, `npa_nodemask_add()`, `npa_nodemask_remove()`, `npa_nodemask_contains()`, `npa_nodemask_count()`, `npa_nodemask_single()`.
  - Affinity: `npa_pin_thread()`, `npa_get_affinity()`, `npa_set_affinity()`.
  - Memory: `npa_region_alloc()`, `npa_region_free()`, `npa_region_ptr()`, `npa_region_len()`, `npa_region_policy_name()`.
  - Error handling: `npa_error_string()`, `npa_last_error()`.
- Core Rust API: Topology discovery, thread pinning (`ScopedPin`), NUMA memory placement (`NumaRegion`), work scheduling (`NumaExecutor`), sharded data structures (`NumaSharded`, `ShardedCounter`), device locality (`DeviceMap`), and observability (`StatsCollector`, `LocalityReport`).
- Hard mode enforcement: strict vs soft mode for guaranteed NUMA placement.
- Platform support: full support on Linux x86_64 and aarch64; graceful degradation on single-socket systems and non-Linux platforms.
