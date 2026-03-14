//! NUMA-aware sharded data structures.
//!
//! This crate provides data structures that are sharded across NUMA nodes,
//! reducing cross-node contention for concurrent access patterns.
//!
//! # Example
//!
//! ```no_run
//! use std::sync::Arc;
//! use numaperf_sharded::{NumaSharded, ShardedCounter};
//! use numaperf_topo::Topology;
//!
//! let topo = Arc::new(Topology::discover()?);
//!
//! // Create a sharded counter
//! let counter = ShardedCounter::new(&topo);
//! counter.increment();
//! println!("Total count: {}", counter.sum());
//!
//! // Create custom sharded data
//! let shards = NumaSharded::new(&topo, || Vec::<u64>::new());
//! shards.local(|v| println!("Local shard has {} items", v.len()));
//! # Ok::<(), numaperf_core::NumaError>(())
//! ```

mod counter;
mod padded;
mod sharded;

pub use counter::ShardedCounter;
pub use padded::CachePadded;
pub use sharded::NumaSharded;

// Re-export commonly used types
pub use numaperf_core::{NodeId, NumaError};
pub use numaperf_topo::Topology;
