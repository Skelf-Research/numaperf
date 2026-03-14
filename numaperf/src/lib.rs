//! # numaperf - NUMA-First Runtime for Rust
//!
//! `numaperf` is a comprehensive toolkit for building NUMA-aware applications
//! in Rust. It provides topology discovery, thread pinning, memory placement,
//! work scheduling, and observability—all designed to maximize data locality
//! and minimize cross-node traffic.
//!
//! ## Quick Start
//!
//! ```no_run
//! use numaperf::{Topology, Capabilities, NumaExecutor, StealPolicy};
//! use std::sync::Arc;
//!
//! fn main() -> Result<(), numaperf::NumaError> {
//!     // Check system capabilities
//!     let caps = Capabilities::detect();
//!     println!("NUMA nodes: {}", caps.numa_node_count);
//!     println!("Hard mode supported: {}", caps.supports_hard_mode());
//!
//!     // Discover topology
//!     let topo = Arc::new(Topology::discover()?);
//!
//!     // Create NUMA-aware executor
//!     let exec = NumaExecutor::builder(Arc::clone(&topo))
//!         .steal_policy(StealPolicy::LocalThenSocketThenRemote)
//!         .workers_per_node(2)
//!         .build()?;
//!
//!     // Submit work to specific nodes
//!     for node in topo.numa_nodes() {
//!         exec.submit_to_node(node.id(), || {
//!             println!("Running on node!");
//!         });
//!     }
//!
//!     exec.shutdown();
//!     Ok(())
//! }
//! ```
//!
//! ## Crate Organization
//!
//! This facade crate re-exports types from the underlying `numaperf-*` crates:
//!
//! | Module | Description |
//! |--------|-------------|
//! | Core types | [`NodeId`], [`CpuSet`], [`NodeMask`], [`NumaError`] |
//! | Configuration | [`HardMode`], [`EnforcementLevel`], [`Capabilities`] |
//! | Topology | [`Topology`], [`NumaNode`] |
//! | Affinity | [`ScopedPin`] |
//! | Memory | [`NumaRegion`], [`MemPolicy`], [`HugePageMode`], [`Prefault`] |
//! | Scheduling | [`NumaExecutor`], [`StealPolicy`] |
//! | Sharding | [`NumaSharded`], [`ShardedCounter`], [`CachePadded`] |
//! | IO Locality | [`DeviceMap`], [`DeviceLocality`], [`DeviceType`] |
//! | Observability | [`StatsCollector`], [`LocalityStats`], [`LocalityReport`] |
//!
//! ## Soft Mode vs Hard Mode
//!
//! numaperf supports two enforcement modes:
//!
//! - **Soft mode** (default): Best-effort locality. Operations succeed even
//!   when optimal placement isn't possible, with graceful degradation.
//!
//! - **Hard mode**: Strict enforcement. Operations fail if locality guarantees
//!   cannot be met, ensuring predictable performance.
//!
//! Check system capabilities before using hard mode:
//!
//! ```
//! use numaperf::Capabilities;
//!
//! let caps = Capabilities::detect();
//! if caps.supports_hard_mode() {
//!     println!("Hard mode is available!");
//! } else {
//!     println!("Missing capabilities:");
//!     for cap in caps.missing_for_hard_mode() {
//!         println!("  - {}", cap);
//!     }
//! }
//! ```
//!
//! ## Platform Support
//!
//! numaperf is designed for Linux systems with NUMA support:
//!
//! - **Full support**: Linux 5.4+ with CONFIG_NUMA enabled
//! - **Partial support**: Single-socket systems (graceful fallback)
//! - **Limited support**: macOS, Windows (topology only, no memory policies)
//!
//! See the [kernel requirements](https://docs.rs/numaperf/latest/numaperf/docs/kernel_requirements)
//! for detailed platform information.
//!
//! ## Feature Flags
//!
//! Currently no optional features. All functionality is included by default.

// =============================================================================
// Core Types
// =============================================================================

/// NUMA node identifier.
///
/// A lightweight, copyable identifier for a NUMA node. Node IDs are assigned
/// by the kernel and typically start at 0.
pub use numaperf_core::NodeId;

/// A set of CPU identifiers.
///
/// Efficiently stores up to 1024 CPU IDs using a bitmap representation.
/// Supports parsing from Linux CPU list format (e.g., "0-3,8-11").
pub use numaperf_core::CpuSet;

/// A set of NUMA node identifiers.
///
/// Similar to [`CpuSet`] but for NUMA nodes.
pub use numaperf_core::NodeMask;

/// Unified error type for all numaperf operations.
///
/// Provides detailed, actionable error messages that explain both what
/// failed and why, enabling appropriate recovery actions.
pub use numaperf_core::NumaError;

// =============================================================================
// Configuration
// =============================================================================

/// Controls soft vs strict enforcement of NUMA policies.
///
/// - `Soft`: Best-effort with graceful degradation (default)
/// - `Strict`: Fail if policies cannot be enforced
pub use numaperf_core::HardMode;

/// Reports the actual enforcement level achieved.
///
/// When using soft mode, this indicates whether full enforcement,
/// partial enforcement, or no enforcement was achieved.
pub use numaperf_core::EnforcementLevel;

/// Detected system capabilities for NUMA operations.
///
/// Use [`Capabilities::detect()`] to query the current system's
/// NUMA capabilities and determine if hard mode is supported.
pub use numaperf_core::Capabilities;

// =============================================================================
// Topology
// =============================================================================

/// Discovered NUMA topology.
///
/// The central type for understanding system NUMA layout. Discover it once
/// and share via `Arc<Topology>` across your application.
///
/// # Example
///
/// ```no_run
/// use numaperf::Topology;
///
/// let topo = Topology::discover()?;
/// println!("System has {} NUMA nodes", topo.node_count());
///
/// for node in topo.numa_nodes() {
///     println!("  Node {}: {} CPUs", node.id(), node.cpu_count());
/// }
/// # Ok::<(), numaperf::NumaError>(())
/// ```
pub use numaperf_topo::Topology;

/// A single NUMA node in the topology.
///
/// Provides access to the node's ID and associated CPUs.
pub use numaperf_topo::NumaNode;

// =============================================================================
// Thread Affinity
// =============================================================================

/// Scoped CPU pinning guard.
///
/// Pins the current thread to a set of CPUs and automatically restores
/// the previous affinity when dropped. This type is `!Send` and `!Sync`
/// because affinity is thread-local.
///
/// # Example
///
/// ```no_run
/// use numaperf::{ScopedPin, CpuSet};
///
/// let cpus = CpuSet::parse("0-3").expect("valid CPU set");
/// {
///     let _pin = ScopedPin::pin_current(cpus)?;
///     // Thread is pinned to CPUs 0-3
///     // Allocations here will be local to this node
/// }
/// // Previous affinity is restored
/// # Ok::<(), numaperf::NumaError>(())
/// ```
pub use numaperf_affinity::ScopedPin;

/// Get the current thread's CPU affinity mask.
///
/// # Example
///
/// ```no_run
/// let affinity = numaperf::get_affinity()?;
/// println!("Current affinity: {:?}", affinity);
/// # Ok::<(), numaperf::NumaError>(())
/// ```
pub use numaperf_affinity::get_affinity;

// =============================================================================
// Memory Placement
// =============================================================================

/// A NUMA-aware memory region.
///
/// Allocates memory with explicit placement policies. The region is
/// automatically unmapped when dropped.
///
/// # Example
///
/// ```no_run
/// use numaperf::{NumaRegion, MemPolicy, NodeMask, NodeId, Prefault};
///
/// // Allocate 1MB bound to node 0
/// let nodes = NodeMask::single(NodeId::new(0));
/// let mut region = NumaRegion::anon(
///     1024 * 1024,
///     MemPolicy::Bind(nodes),
///     Default::default(),
///     Prefault::Touch,
/// )?;
///
/// // Use the memory
/// let slice = region.as_mut_slice();
/// slice[0] = 42;
/// # Ok::<(), numaperf::NumaError>(())
/// ```
pub use numaperf_mem::NumaRegion;

/// Memory placement policy.
///
/// - `Bind`: Strict allocation on specified nodes
/// - `Preferred`: Prefer specified node, fallback allowed
/// - `Interleave`: Round-robin across nodes
/// - `Local`: Use current thread's node (default)
pub use numaperf_mem::MemPolicy;

/// Huge page configuration.
pub use numaperf_mem::HugePageMode;

/// Memory prefault strategy.
///
/// Controls how pages are faulted in after allocation.
pub use numaperf_mem::Prefault;

// =============================================================================
// Scheduling
// =============================================================================

/// NUMA-aware work executor.
///
/// Maintains per-node worker pools with configurable work stealing.
/// Workers are pinned to their node's CPUs for optimal locality.
///
/// # Example
///
/// ```no_run
/// use numaperf::{NumaExecutor, Topology, StealPolicy, NodeId};
/// use std::sync::Arc;
///
/// let topo = Arc::new(Topology::discover()?);
/// let exec = NumaExecutor::builder(topo)
///     .steal_policy(StealPolicy::LocalThenSocketThenRemote)
///     .workers_per_node(4)
///     .build()?;
///
/// // Submit work to a specific node
/// exec.submit_to_node(NodeId::new(0), || {
///     // This runs on a worker pinned to node 0
/// });
///
/// exec.shutdown();
/// # Ok::<(), numaperf::NumaError>(())
/// ```
pub use numaperf_sched::NumaExecutor;

/// Builder for configuring [`NumaExecutor`].
pub use numaperf_sched::NumaExecutorBuilder;

/// Work stealing policy.
///
/// Controls how workers steal work from other nodes when their
/// local queue is empty.
pub use numaperf_sched::StealPolicy;

// =============================================================================
// Sharded Data Structures
// =============================================================================

/// Per-node sharded data structure.
///
/// Maintains one shard per NUMA node to reduce cross-node contention.
/// Access the local shard with `local()` for optimal performance.
///
/// # Example
///
/// ```no_run
/// use numaperf::{NumaSharded, Topology};
/// use std::sync::Arc;
/// use std::sync::atomic::{AtomicU64, Ordering};
///
/// let topo = Arc::new(Topology::discover()?);
/// let counters = NumaSharded::new(&topo, || AtomicU64::new(0));
///
/// // Increment local counter (fast, no cross-node traffic)
/// counters.local(|counter| {
///     counter.fetch_add(1, Ordering::Relaxed);
/// });
///
/// // Sum across all nodes (crosses NUMA boundaries)
/// let total: u64 = counters.iter()
///     .map(|(_, c)| c.load(Ordering::Relaxed))
///     .sum();
/// # Ok::<(), numaperf::NumaError>(())
/// ```
pub use numaperf_sharded::NumaSharded;

/// Per-node sharded atomic counter.
///
/// A specialized `NumaSharded<AtomicU64>` with convenience methods.
pub use numaperf_sharded::ShardedCounter;

/// Cache-line padded wrapper.
///
/// Prevents false sharing by padding the wrapped value to a full
/// cache line (128 bytes).
pub use numaperf_sharded::CachePadded;

// =============================================================================
// IO Locality
// =============================================================================

/// Maps devices to their NUMA locality.
///
/// Discovers network and block devices and their associated NUMA nodes,
/// enabling IO-aware task placement.
///
/// # Example
///
/// ```no_run
/// use numaperf::{DeviceMap, Topology};
/// use std::sync::Arc;
///
/// let topo = Arc::new(Topology::discover()?);
/// let devices = DeviceMap::discover(Arc::clone(&topo))?;
///
/// // Find which node eth0 is connected to
/// if let Some(node) = devices.device_node("eth0") {
///     println!("eth0 is on NUMA node {}", node);
/// }
/// # Ok::<(), numaperf::NumaError>(())
/// ```
pub use numaperf_io::DeviceMap;

/// Device locality information.
pub use numaperf_io::DeviceLocality;

/// Type of IO device.
pub use numaperf_io::DeviceType;

// =============================================================================
// Observability
// =============================================================================

/// Collects locality statistics using sharded counters.
///
/// Tracks local executions vs cross-node steals to measure
/// NUMA locality effectiveness.
///
/// # Example
///
/// ```no_run
/// use numaperf::{StatsCollector, LocalityReport, Topology};
/// use std::sync::Arc;
///
/// let topo = Arc::new(Topology::discover()?);
/// let collector = StatsCollector::new(&topo);
///
/// // Record metrics (in practice, integrated with your executor)
/// collector.record_local_execution();
/// collector.record_local_execution();
/// collector.record_steal(numaperf::NodeId::new(1));
///
/// // Generate diagnostic report
/// let stats = collector.snapshot();
/// let report = LocalityReport::generate(&stats);
/// report.print();
/// # Ok::<(), numaperf::NumaError>(())
/// ```
pub use numaperf_perf::StatsCollector;

/// Point-in-time snapshot of locality metrics.
pub use numaperf_perf::LocalityStats;

/// Per-node statistics.
pub use numaperf_perf::NodeStats;

/// Diagnostic report with health assessment and recommendations.
pub use numaperf_perf::LocalityReport;

/// Locality health classification.
pub use numaperf_perf::LocalityHealth;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_facade_exports_core_types() {
        // Verify core types are accessible
        let _node_id = NodeId::new(0);
        let _cpus = CpuSet::new();
        let _nodes = NodeMask::new();
    }

    #[test]
    fn test_facade_exports_config() {
        // Verify configuration types
        assert!(HardMode::Soft.is_soft());
        assert!(HardMode::Strict.is_strict());
        let _level = EnforcementLevel::Strict;
    }

    #[test]
    fn test_facade_exports_capabilities() {
        // Verify capability detection works
        let caps = Capabilities::detect();
        assert!(caps.numa_node_count >= 1);
    }

    #[test]
    fn test_facade_exports_topology() {
        // Verify topology types
        let cpus = CpuSet::parse("0-3").unwrap();
        let topo = Topology::single_node(cpus);
        assert!(topo.node_count() >= 1);
    }

    #[test]
    fn test_facade_exports_policy() {
        // Verify memory policy types
        let _policy = MemPolicy::Local;
        let _huge = HugePageMode::default();
        let _prefault = Prefault::default();
    }

    #[test]
    fn test_facade_exports_steal_policy() {
        // Verify scheduler types
        let _policy = StealPolicy::LocalOnly;
        let _policy = StealPolicy::LocalThenSocketThenRemote;
        let _policy = StealPolicy::Any;
    }
}
