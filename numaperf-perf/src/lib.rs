//! NUMA locality observability and diagnostics.
//!
//! This crate provides tools for monitoring and diagnosing NUMA locality behavior
//! in your application. It tracks metrics like local task executions vs. cross-node
//! steals and provides diagnostic reports with actionable recommendations.
//!
//! # Example
//!
//! ```no_run
//! use std::sync::Arc;
//! use numaperf_perf::{StatsCollector, LocalityReport};
//! use numaperf_topo::Topology;
//!
//! let topo = Arc::new(Topology::discover()?);
//! let collector = StatsCollector::new(&topo);
//!
//! // Record some metrics (in real usage, integrated with scheduler)
//! collector.record_local_execution();
//! collector.record_local_execution();
//! collector.record_steal(numaperf_core::NodeId::new(1));
//!
//! // Get a snapshot and generate a report
//! let stats = collector.snapshot();
//! println!("Locality ratio: {:.1}%", stats.locality_ratio() * 100.0);
//!
//! let report = LocalityReport::generate(&stats);
//! report.print();
//! # Ok::<(), numaperf_core::NumaError>(())
//! ```

mod collector;
mod report;
mod stats;

pub use collector::StatsCollector;
pub use report::{LocalityHealth, LocalityReport};
pub use stats::{LocalityStats, NodeStats};

// Re-export commonly used types
pub use numaperf_core::{NodeId, NumaError};
pub use numaperf_topo::Topology;
