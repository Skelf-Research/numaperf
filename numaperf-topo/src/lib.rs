//! NUMA topology discovery for numaperf.
//!
//! This crate provides functionality to discover the hardware NUMA topology,
//! including nodes, CPUs, and their relationships.
//!
//! # Example
//!
//! ```no_run
//! use numaperf_topo::Topology;
//!
//! let topo = Topology::discover().expect("topology discovery failed");
//! println!("Found {} NUMA nodes", topo.node_count());
//!
//! for node in topo.numa_nodes() {
//!     println!("  {}: CPUs {}", node.id(), topo.cpu_set(node.id()));
//! }
//! ```

mod discovery;
mod node;
mod topology;

pub use node::NumaNode;
pub use topology::Topology;

// Re-export core types for convenience
pub use numaperf_core::{CpuSet, NodeId, NodeMask, NumaError};
