//! Device-to-NUMA-node mapping and IO locality.
//!
//! This crate discovers the NUMA locality of IO devices (network interfaces,
//! block devices) to enable optimal placement of IO workers near their devices.
//!
//! # Example
//!
//! ```no_run
//! use std::sync::Arc;
//! use numaperf_io::{DeviceMap, DeviceType};
//! use numaperf_topo::Topology;
//!
//! let topo = Arc::new(Topology::discover()?);
//! let devices = DeviceMap::discover(Arc::clone(&topo))?;
//!
//! // Find the NUMA node for a network device
//! if let Some(node) = devices.device_node("eth0") {
//!     println!("eth0 is on NUMA node {}", node);
//! }
//!
//! // List all network devices and their NUMA nodes
//! for dev in devices.network_devices() {
//!     println!("{}: {:?}", dev.name, dev.numa_node);
//! }
//! # Ok::<(), numaperf_core::NumaError>(())
//! ```

mod device;
mod discovery;

pub use device::{DeviceLocality, DeviceMap, DeviceType};

// Re-export commonly used types
pub use numaperf_core::{NodeId, NumaError};
pub use numaperf_topo::Topology;
