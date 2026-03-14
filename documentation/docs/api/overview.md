# API Overview

This section provides complete API reference documentation for all numaperf types and functions.

## Crate Organization

```
numaperf (facade - use this)
├── Core Types      → NodeId, CpuSet, NodeMask, NumaError
├── Configuration   → HardMode, EnforcementLevel, Capabilities
├── Topology        → Topology, NumaNode
├── Affinity        → ScopedPin, get_affinity, set_affinity
├── Memory          → NumaRegion, MemPolicy, HugePageMode, Prefault
├── Scheduling      → NumaExecutor, NumaExecutorBuilder, StealPolicy
├── Sharding        → NumaSharded, ShardedCounter, CachePadded
├── I/O             → DeviceMap, DeviceLocality, DeviceType
└── Observability   → StatsCollector, LocalityStats, LocalityReport
```

## Quick Reference

### Core Types

| Type | Description | See |
|------|-------------|-----|
| `NodeId` | NUMA node identifier | [Core Types](core-types.md#nodeid) |
| `CpuSet` | Set of CPU IDs | [Core Types](core-types.md#cpuset) |
| `NodeMask` | Set of NUMA nodes | [Core Types](core-types.md#nodemask) |
| `NumaError` | Error type | [Errors](errors.md) |

### Configuration

| Type | Description | See |
|------|-------------|-----|
| `HardMode` | Soft vs strict enforcement | [Core Types](core-types.md#hardmode) |
| `EnforcementLevel` | Actual enforcement achieved | [Core Types](core-types.md#enforcementlevel) |
| `Capabilities` | System capability detection | [Core Types](core-types.md#capabilities) |

### Topology

| Type | Description | See |
|------|-------------|-----|
| `Topology` | System NUMA topology | [Topology](topology.md#topology) |
| `NumaNode` | Single NUMA node info | [Topology](topology.md#numanode) |

### Affinity

| Type | Description | See |
|------|-------------|-----|
| `ScopedPin` | RAII thread pinning | [Affinity](affinity.md#scopedpin) |
| `get_affinity()` | Query current affinity | [Affinity](affinity.md#get_affinity) |

### Memory

| Type | Description | See |
|------|-------------|-----|
| `NumaRegion` | NUMA-aware memory | [Memory](memory.md#numaregion) |
| `MemPolicy` | Memory placement policy | [Memory](memory.md#mempolicy) |
| `HugePageMode` | Huge page settings | [Memory](memory.md#hugepagemode) |
| `Prefault` | Page fault strategy | [Memory](memory.md#prefault) |

### Scheduling

| Type | Description | See |
|------|-------------|-----|
| `NumaExecutor` | Work executor | [Scheduler](scheduler.md#numaexecutor) |
| `NumaExecutorBuilder` | Executor configuration | [Scheduler](scheduler.md#numaexecutorbuilder) |
| `StealPolicy` | Work stealing policy | [Scheduler](scheduler.md#stealpolicy) |

### Sharding

| Type | Description | See |
|------|-------------|-----|
| `NumaSharded<T>` | Per-node sharded data | [Sharded](sharded.md#numasharded) |
| `ShardedCounter` | Sharded atomic counter | [Sharded](sharded.md#shardedcounter) |
| `CachePadded<T>` | Cache-line padding | [Sharded](sharded.md#cachepadded) |

### I/O

| Type | Description | See |
|------|-------------|-----|
| `DeviceMap` | Device-to-node mapping | [I/O](io.md#devicemap) |
| `DeviceLocality` | Device locality info | [I/O](io.md#devicelocality) |
| `DeviceType` | Device type enum | [I/O](io.md#devicetype) |

### Observability

| Type | Description | See |
|------|-------------|-----|
| `StatsCollector` | Locality metrics | [Observability](observability.md#statscollector) |
| `LocalityStats` | Metrics snapshot | [Observability](observability.md#localitystats) |
| `LocalityReport` | Diagnostic report | [Observability](observability.md#localityreport) |
| `LocalityHealth` | Health classification | [Observability](observability.md#localityhealth) |

## Common Patterns

### Create Once, Share Everywhere

```rust
use numaperf::Topology;
use std::sync::Arc;

let topo = Arc::new(Topology::discover()?);
// Share topo across threads
```

### RAII Resource Management

```rust
use numaperf::{ScopedPin, NumaRegion};

{
    let _pin = ScopedPin::pin_current(cpus)?;
    // Pinned here
}
// Automatically restored

{
    let region = NumaRegion::anon(...)?;
    // Memory mapped
}
// Automatically unmapped
```

### Check Enforcement

```rust
use numaperf::EnforcementLevel;

let region = NumaRegion::anon(...)?;
match region.enforcement() {
    EnforcementLevel::Strict => { /* guaranteed */ }
    EnforcementLevel::BestEffort { reason } => { /* check reason */ }
    EnforcementLevel::None { reason } => { /* no NUMA */ }
}
```

## Thread Safety Summary

| Type | Send | Sync | Notes |
|------|------|------|-------|
| `Topology` | ✓ | ✓ | Immutable |
| `NumaNode` | ✓ | ✓ | Immutable |
| `ScopedPin` | ✗ | ✗ | Thread-local |
| `NumaRegion` | ✓ | ✓ | Memory shareable |
| `NumaExecutor` | ✓ | ✓ | Submit from anywhere |
| `NumaSharded<T>` | If T | If T | Depends on T |
| `StatsCollector` | ✓ | ✓ | Lock-free |
