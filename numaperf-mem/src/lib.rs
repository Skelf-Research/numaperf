//! NUMA-aware memory allocation and placement for numaperf.
//!
//! This crate provides explicit control over memory placement on NUMA systems,
//! allowing you to allocate memory bound to specific NUMA nodes.
//!
//! # Example
//!
//! ```no_run
//! use numaperf_mem::{NumaRegion, MemPolicy, HugePageMode, Prefault};
//! use numaperf_core::NodeId;
//!
//! // Allocate 1 GB bound to node 0
//! let region = NumaRegion::anon(
//!     1024 * 1024 * 1024,
//!     MemPolicy::Bind([NodeId::new(0)].into()),
//!     HugePageMode::TransparentOn,
//!     Prefault::Touch,
//! )?;
//!
//! // Use the memory
//! let slice = region.as_slice();
//! # Ok::<(), numaperf_core::NumaError>(())
//! ```

mod huge;
mod policy;
mod prefault;
mod region;
mod syscall;

pub use huge::HugePageMode;
pub use policy::MemPolicy;
pub use prefault::Prefault;
pub use region::NumaRegion;

// Re-export core types for convenience
pub use numaperf_core::{EnforcementLevel, HardMode, NodeId, NodeMask, NumaError};
