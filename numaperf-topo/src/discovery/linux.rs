//! Linux-specific topology discovery via sysfs.

use std::fs;
use std::path::Path;

use numaperf_core::{CpuSet, NodeId, NumaError};

use crate::node::NumaNode;
use crate::Topology;

/// Base path for NUMA sysfs.
const SYSFS_NODE_PATH: &str = "/sys/devices/system/node";

/// Discover NUMA topology from Linux sysfs.
pub fn discover() -> Result<Topology, NumaError> {
    let node_path = Path::new(SYSFS_NODE_PATH);

    if !node_path.exists() {
        return Err(NumaError::topology(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "NUMA sysfs not found",
        )));
    }

    let mut nodes = Vec::new();

    // Read directory entries for node* directories
    let entries = fs::read_dir(node_path).map_err(NumaError::topology)?;

    for entry in entries {
        let entry = entry.map_err(NumaError::topology)?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Look for directories named "node0", "node1", etc.
        if let Some(suffix) = name_str.strip_prefix("node") {
            if let Ok(id) = suffix.parse::<u32>() {
                let node = discover_node(&entry.path(), NodeId::new(id))?;
                nodes.push(node);
            }
        }
    }

    // Sort nodes by ID
    nodes.sort_by_key(|n| n.id().as_u32());

    if nodes.is_empty() {
        return Err(NumaError::topology(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "no NUMA nodes found",
        )));
    }

    // Read distance tables
    let nodes = read_distances(node_path, nodes)?;

    Ok(Topology::from_nodes(nodes))
}

/// Discover a single NUMA node.
fn discover_node(node_path: &Path, id: NodeId) -> Result<NumaNode, NumaError> {
    // Read CPU list
    let cpulist_path = node_path.join("cpulist");
    let cpus = read_cpulist(&cpulist_path)?;

    // Read memory info
    let meminfo_path = node_path.join("meminfo");
    let memory_bytes = read_meminfo(&meminfo_path).ok();

    let mut node = NumaNode::new(id, cpus);
    if let Some(mem) = memory_bytes {
        node = node.with_memory(mem);
    }

    Ok(node)
}

/// Read the cpulist file and parse it into a CpuSet.
fn read_cpulist(path: &Path) -> Result<CpuSet, NumaError> {
    let content = fs::read_to_string(path).map_err(NumaError::topology)?;
    CpuSet::parse(content.trim()).map_err(|e| {
        NumaError::topology(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("failed to parse cpulist: {}", e),
        ))
    })
}

/// Read the meminfo file and extract total memory.
fn read_meminfo(path: &Path) -> Result<u64, NumaError> {
    let content = fs::read_to_string(path).map_err(NumaError::topology)?;

    // Look for "MemTotal:" line
    for line in content.lines() {
        if line.contains("MemTotal:") {
            // Format: "Node 0 MemTotal:    16384000 kB"
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                if let Ok(kb) = parts[3].parse::<u64>() {
                    return Ok(kb * 1024); // Convert kB to bytes
                }
            }
        }
    }

    Err(NumaError::topology(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "MemTotal not found in meminfo",
    )))
}

/// Read distance tables for all nodes.
fn read_distances(node_path: &Path, mut nodes: Vec<NumaNode>) -> Result<Vec<NumaNode>, NumaError> {
    for node in &mut nodes {
        let distance_path = node_path
            .join(format!("node{}", node.id().as_u32()))
            .join("distance");

        if let Ok(content) = fs::read_to_string(&distance_path) {
            let distances: Vec<u32> = content
                .split_whitespace()
                .filter_map(|s| s.parse().ok())
                .collect();

            if !distances.is_empty() {
                node.set_distances(distances);
            }
        }
    }

    Ok(nodes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpulist_parse() {
        // This test uses our CpuSet::parse which is tested elsewhere,
        // but we can test the wrapper behavior
        let set = CpuSet::parse("0-3,8-11").unwrap();
        assert_eq!(set.count(), 8);
    }

    #[test]
    #[ignore] // Only run on systems with NUMA
    fn test_discover_real() {
        let topo = discover().expect("should discover topology");
        assert!(topo.node_count() > 0);
        assert!(topo.cpu_count() > 0);
    }
}
