//! NUMA-aware work scheduler for numaperf.
//!
//! This crate provides a topology-aware work scheduler that respects NUMA
//! locality by maintaining per-node work queues and configurable work stealing.
//!
//! # Example
//!
//! ```no_run
//! use std::sync::Arc;
//! use numaperf_sched::{NumaExecutor, StealPolicy};
//! use numaperf_topo::Topology;
//!
//! let topo = Arc::new(Topology::discover()?);
//! let exec = NumaExecutor::new(Arc::clone(&topo), StealPolicy::default())?;
//!
//! // Submit work to a specific node
//! exec.submit_to_node(numaperf_core::NodeId::new(0), || {
//!     println!("Running on node 0!");
//! });
//!
//! // Wait for completion
//! exec.shutdown();
//! # Ok::<(), numaperf_core::NumaError>(())
//! ```

mod executor;
mod queue;
mod steal;
mod task;
mod worker;

pub use executor::{NumaExecutor, NumaExecutorBuilder};
pub use steal::StealPolicy;
pub use task::Task;

// Re-export commonly needed types
pub use numaperf_core::{NodeId, NumaError};
pub use numaperf_topo::Topology;
