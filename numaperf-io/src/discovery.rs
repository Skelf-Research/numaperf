//! Device NUMA locality discovery via sysfs.

use std::fs;
use std::path::Path;

use numaperf_core::NodeId;

use crate::device::{DeviceLocality, DeviceType};

/// sysfs paths for device discovery.
const SYSFS_NET_PATH: &str = "/sys/class/net";
const SYSFS_BLOCK_PATH: &str = "/sys/block";

/// Discover all network devices and their NUMA locality.
pub fn discover_network_devices() -> Vec<DeviceLocality> {
    let mut devices = Vec::new();

    let net_path = Path::new(SYSFS_NET_PATH);
    if !net_path.exists() {
        return devices;
    }

    if let Ok(entries) = fs::read_dir(net_path) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip loopback for NUMA purposes (it's virtual)
            // But we still include it in the list
            let numa_node = read_device_numa_node(&entry.path());

            devices.push(DeviceLocality {
                name,
                device_type: DeviceType::Network,
                numa_node,
            });
        }
    }

    devices
}

/// Discover all block devices and their NUMA locality.
pub fn discover_block_devices() -> Vec<DeviceLocality> {
    let mut devices = Vec::new();

    let block_path = Path::new(SYSFS_BLOCK_PATH);
    if !block_path.exists() {
        return devices;
    }

    if let Ok(entries) = fs::read_dir(block_path) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip certain virtual devices that don't have NUMA locality
            if name.starts_with("loop") || name.starts_with("ram") || name.starts_with("dm-") {
                continue;
            }

            let numa_node = read_device_numa_node(&entry.path());

            devices.push(DeviceLocality {
                name,
                device_type: DeviceType::Block,
                numa_node,
            });
        }
    }

    devices
}

/// Read the NUMA node for a device from its sysfs directory.
///
/// Looks for `device/numa_node` within the device's sysfs path.
/// Returns `None` if:
/// - The file doesn't exist (device doesn't have NUMA locality info)
/// - The file contains -1 (device not bound to a specific node)
/// - The file can't be read or parsed
fn read_device_numa_node(device_path: &Path) -> Option<NodeId> {
    // Try device/numa_node first (most common for physical devices)
    let numa_node_path = device_path.join("device/numa_node");

    if let Ok(content) = fs::read_to_string(&numa_node_path) {
        if let Ok(node_id) = content.trim().parse::<i32>() {
            if node_id >= 0 {
                return Some(NodeId::new(node_id as u32));
            }
        }
    }

    // Some devices might have numa_node directly in their path
    let direct_path = device_path.join("numa_node");
    if let Ok(content) = fs::read_to_string(&direct_path) {
        if let Ok(node_id) = content.trim().parse::<i32>() {
            if node_id >= 0 {
                return Some(NodeId::new(node_id as u32));
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_network_devices() {
        let devices = discover_network_devices();

        // Should find at least loopback on most systems
        // But don't fail if sysfs isn't available (e.g., in containers)
        if Path::new(SYSFS_NET_PATH).exists() {
            // Just verify we get some devices
            println!("Found {} network devices", devices.len());
            for dev in &devices {
                println!("  {}: {:?}", dev.name, dev.numa_node);
            }
        }
    }

    #[test]
    fn test_discover_block_devices() {
        let devices = discover_block_devices();

        if Path::new(SYSFS_BLOCK_PATH).exists() {
            println!("Found {} block devices", devices.len());
            for dev in &devices {
                println!("  {}: {:?}", dev.name, dev.numa_node);
            }
        }
    }

    #[test]
    fn test_device_locality_fields() {
        let dev = DeviceLocality {
            name: "eth0".to_string(),
            device_type: DeviceType::Network,
            numa_node: Some(NodeId::new(0)),
        };

        assert_eq!(dev.name, "eth0");
        assert_eq!(dev.device_type, DeviceType::Network);
        assert_eq!(dev.numa_node, Some(NodeId::new(0)));
    }
}
