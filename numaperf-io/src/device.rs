//! Device locality types and mapping.

use std::collections::HashMap;
use std::sync::Arc;

use numaperf_core::{NodeId, NumaError};
use numaperf_topo::Topology;

use crate::discovery;

/// The type of IO device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeviceType {
    /// Network interface (e.g., eth0, ens33).
    Network,
    /// Block device (e.g., nvme0n1, sda).
    Block,
    /// Other device type.
    Other,
}

impl std::fmt::Display for DeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeviceType::Network => write!(f, "network"),
            DeviceType::Block => write!(f, "block"),
            DeviceType::Other => write!(f, "other"),
        }
    }
}

/// NUMA locality information for a device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceLocality {
    /// The device name (e.g., "eth0", "nvme0n1").
    pub name: String,
    /// The device type.
    pub device_type: DeviceType,
    /// The NUMA node this device is closest to, if known.
    ///
    /// `None` means the device either:
    /// - Doesn't have a specific NUMA affinity (e.g., virtual devices)
    /// - The kernel doesn't expose NUMA info for this device
    pub numa_node: Option<NodeId>,
}

impl DeviceLocality {
    /// Check if this device has a known NUMA node.
    #[inline]
    pub fn has_numa_locality(&self) -> bool {
        self.numa_node.is_some()
    }

    /// Check if this device is on a specific node.
    #[inline]
    pub fn is_on_node(&self, node: NodeId) -> bool {
        self.numa_node == Some(node)
    }
}

impl std::fmt::Display for DeviceLocality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.numa_node {
            Some(node) => write!(f, "{} ({}) on {}", self.name, self.device_type, node),
            None => write!(f, "{} ({}) [no NUMA affinity]", self.name, self.device_type),
        }
    }
}

/// A mapping of devices to their NUMA nodes.
///
/// `DeviceMap` discovers all network and block devices on the system and
/// their NUMA locality, enabling optimal placement of IO workers.
///
/// # Example
///
/// ```no_run
/// use std::sync::Arc;
/// use numaperf_io::DeviceMap;
/// use numaperf_topo::Topology;
///
/// let topo = Arc::new(Topology::discover()?);
/// let devices = DeviceMap::discover(Arc::clone(&topo))?;
///
/// // Find devices on a specific NUMA node
/// for dev in devices.devices_on_node(numaperf_core::NodeId::new(0)) {
///     println!("Device on node 0: {}", dev.name);
/// }
/// # Ok::<(), numaperf_core::NumaError>(())
/// ```
pub struct DeviceMap {
    /// All discovered devices, keyed by name.
    devices: HashMap<String, DeviceLocality>,
    /// The topology used for NUMA information.
    topo: Arc<Topology>,
}

impl DeviceMap {
    /// Discover all devices and their NUMA locality.
    ///
    /// This reads from `/sys/class/net/` and `/sys/block/` to find devices
    /// and their NUMA node assignments.
    pub fn discover(topo: Arc<Topology>) -> Result<Self, NumaError> {
        let mut devices = HashMap::new();

        // Discover network devices
        for dev in discovery::discover_network_devices() {
            devices.insert(dev.name.clone(), dev);
        }

        // Discover block devices
        for dev in discovery::discover_block_devices() {
            devices.insert(dev.name.clone(), dev);
        }

        Ok(Self { devices, topo })
    }

    /// Create an empty device map.
    ///
    /// Useful for testing or when device discovery isn't needed.
    pub fn empty(topo: Arc<Topology>) -> Self {
        Self {
            devices: HashMap::new(),
            topo,
        }
    }

    /// Get the NUMA node for a specific device.
    ///
    /// Returns `None` if the device is not found or doesn't have NUMA locality.
    pub fn device_node(&self, name: &str) -> Option<NodeId> {
        self.devices.get(name).and_then(|d| d.numa_node)
    }

    /// Get the full locality info for a device.
    pub fn device(&self, name: &str) -> Option<&DeviceLocality> {
        self.devices.get(name)
    }

    /// Get all network devices.
    pub fn network_devices(&self) -> impl Iterator<Item = &DeviceLocality> {
        self.devices
            .values()
            .filter(|d| d.device_type == DeviceType::Network)
    }

    /// Get all block devices.
    pub fn block_devices(&self) -> impl Iterator<Item = &DeviceLocality> {
        self.devices
            .values()
            .filter(|d| d.device_type == DeviceType::Block)
    }

    /// Get all devices on a specific NUMA node.
    pub fn devices_on_node(&self, node: NodeId) -> impl Iterator<Item = &DeviceLocality> {
        self.devices.values().filter(move |d| d.is_on_node(node))
    }

    /// Get all devices that have NUMA locality information.
    pub fn devices_with_locality(&self) -> impl Iterator<Item = &DeviceLocality> {
        self.devices.values().filter(|d| d.has_numa_locality())
    }

    /// Get all devices.
    pub fn all_devices(&self) -> impl Iterator<Item = &DeviceLocality> {
        self.devices.values()
    }

    /// Get the number of devices.
    #[inline]
    pub fn len(&self) -> usize {
        self.devices.len()
    }

    /// Check if no devices were discovered.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.devices.is_empty()
    }

    /// Get the topology.
    #[inline]
    pub fn topology(&self) -> &Arc<Topology> {
        &self.topo
    }

    /// Add a device manually.
    ///
    /// This is useful for testing or for adding devices that aren't
    /// discovered automatically.
    pub fn add_device(&mut self, device: DeviceLocality) {
        self.devices.insert(device.name.clone(), device);
    }

    /// Print a summary of all devices.
    pub fn summary(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!("Device Map: {} devices\n", self.len()));

        let mut network: Vec<_> = self.network_devices().collect();
        network.sort_by(|a, b| a.name.cmp(&b.name));

        let mut block: Vec<_> = self.block_devices().collect();
        block.sort_by(|a, b| a.name.cmp(&b.name));

        if !network.is_empty() {
            s.push_str("  Network devices:\n");
            for dev in network {
                s.push_str(&format!("    {}\n", dev));
            }
        }

        if !block.is_empty() {
            s.push_str("  Block devices:\n");
            for dev in block {
                s.push_str(&format!("    {}\n", dev));
            }
        }

        s
    }
}

impl std::fmt::Debug for DeviceMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeviceMap")
            .field("device_count", &self.devices.len())
            .field("devices", &self.devices)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use numaperf_core::CpuSet;

    fn test_topology() -> Arc<Topology> {
        Arc::new(Topology::discover().unwrap_or_else(|_| {
            let cpus = CpuSet::parse("0-7").unwrap();
            Topology::single_node(cpus)
        }))
    }

    #[test]
    fn test_device_type_display() {
        assert_eq!(format!("{}", DeviceType::Network), "network");
        assert_eq!(format!("{}", DeviceType::Block), "block");
        assert_eq!(format!("{}", DeviceType::Other), "other");
    }

    #[test]
    fn test_device_locality_display() {
        let dev = DeviceLocality {
            name: "eth0".to_string(),
            device_type: DeviceType::Network,
            numa_node: Some(NodeId::new(0)),
        };
        assert!(format!("{}", dev).contains("eth0"));
        assert!(format!("{}", dev).contains("network"));
        assert!(format!("{}", dev).contains("node0"));

        let dev_no_numa = DeviceLocality {
            name: "lo".to_string(),
            device_type: DeviceType::Network,
            numa_node: None,
        };
        assert!(format!("{}", dev_no_numa).contains("no NUMA"));
    }

    #[test]
    fn test_device_locality_helpers() {
        let dev_with_numa = DeviceLocality {
            name: "eth0".to_string(),
            device_type: DeviceType::Network,
            numa_node: Some(NodeId::new(1)),
        };

        assert!(dev_with_numa.has_numa_locality());
        assert!(dev_with_numa.is_on_node(NodeId::new(1)));
        assert!(!dev_with_numa.is_on_node(NodeId::new(0)));

        let dev_without_numa = DeviceLocality {
            name: "lo".to_string(),
            device_type: DeviceType::Network,
            numa_node: None,
        };

        assert!(!dev_without_numa.has_numa_locality());
        assert!(!dev_without_numa.is_on_node(NodeId::new(0)));
    }

    #[test]
    fn test_device_map_discover() {
        let topo = test_topology();
        let devices = DeviceMap::discover(Arc::clone(&topo)).unwrap();

        // On most systems we should find at least the loopback interface
        // But don't fail in containers where sysfs might not be available
        println!("Discovered {} devices", devices.len());
        println!("{}", devices.summary());
    }

    #[test]
    fn test_device_map_empty() {
        let topo = test_topology();
        let devices = DeviceMap::empty(Arc::clone(&topo));

        assert!(devices.is_empty());
        assert_eq!(devices.len(), 0);
    }

    #[test]
    fn test_device_map_add_device() {
        let topo = test_topology();
        let mut devices = DeviceMap::empty(Arc::clone(&topo));

        devices.add_device(DeviceLocality {
            name: "test0".to_string(),
            device_type: DeviceType::Network,
            numa_node: Some(NodeId::new(0)),
        });

        assert_eq!(devices.len(), 1);
        assert_eq!(devices.device_node("test0"), Some(NodeId::new(0)));
    }

    #[test]
    fn test_device_map_device_node() {
        let topo = test_topology();
        let mut devices = DeviceMap::empty(Arc::clone(&topo));

        devices.add_device(DeviceLocality {
            name: "eth0".to_string(),
            device_type: DeviceType::Network,
            numa_node: Some(NodeId::new(1)),
        });

        devices.add_device(DeviceLocality {
            name: "lo".to_string(),
            device_type: DeviceType::Network,
            numa_node: None,
        });

        assert_eq!(devices.device_node("eth0"), Some(NodeId::new(1)));
        assert_eq!(devices.device_node("lo"), None);
        assert_eq!(devices.device_node("nonexistent"), None);
    }

    #[test]
    fn test_device_map_filters() {
        let topo = test_topology();
        let mut devices = DeviceMap::empty(Arc::clone(&topo));

        devices.add_device(DeviceLocality {
            name: "eth0".to_string(),
            device_type: DeviceType::Network,
            numa_node: Some(NodeId::new(0)),
        });

        devices.add_device(DeviceLocality {
            name: "nvme0n1".to_string(),
            device_type: DeviceType::Block,
            numa_node: Some(NodeId::new(0)),
        });

        devices.add_device(DeviceLocality {
            name: "lo".to_string(),
            device_type: DeviceType::Network,
            numa_node: None,
        });

        // Filter by type
        assert_eq!(devices.network_devices().count(), 2);
        assert_eq!(devices.block_devices().count(), 1);

        // Filter by node
        assert_eq!(devices.devices_on_node(NodeId::new(0)).count(), 2);
        assert_eq!(devices.devices_on_node(NodeId::new(1)).count(), 0);

        // Filter by locality
        assert_eq!(devices.devices_with_locality().count(), 2);
    }
}
