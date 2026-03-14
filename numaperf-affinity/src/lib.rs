//! Thread affinity and CPU pinning for numaperf.
//!
//! This crate provides functionality to pin threads to specific CPUs or NUMA nodes,
//! ensuring that memory allocations and computation happen in the desired location.
//!
//! # Example
//!
//! ```no_run
//! use numaperf_affinity::ScopedPin;
//! use numaperf_topo::Topology;
//!
//! let topo = Topology::discover()?;
//! let node0_cpus = topo.cpu_set(numaperf_core::NodeId::new(0));
//!
//! {
//!     // Pin current thread to node 0
//!     let _pin = ScopedPin::pin_current(node0_cpus)?;
//!
//!     // Thread is pinned here - allocations will be local to node 0
//!     do_work();
//! }
//! // Affinity is restored when _pin is dropped
//!
//! fn do_work() { /* ... */ }
//! # Ok::<(), numaperf_core::NumaError>(())
//! ```

mod pin;
mod syscall;

pub use pin::ScopedPin;

// Re-export core types for convenience
pub use numaperf_core::{CpuSet, NodeId, NumaError};

/// Get the current thread's CPU affinity.
pub fn get_affinity() -> Result<CpuSet, NumaError> {
    syscall::get_affinity()
}

/// Set the current thread's CPU affinity.
///
/// This is a low-level function. Prefer using [`ScopedPin`] which automatically
/// restores the previous affinity when dropped.
pub fn set_affinity(cpus: &CpuSet) -> Result<(), NumaError> {
    syscall::set_affinity(cpus)
}
