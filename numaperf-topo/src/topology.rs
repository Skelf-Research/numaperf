//! NUMA topology discovery and representation.

use std::collections::HashMap;

use numaperf_core::{CpuSet, NodeId, NumaError};

use crate::discovery;
use crate::node::NumaNode;

/// The discovered NUMA topology of the system.
///
/// `Topology` is immutable after creation and safe to share across threads
/// via `Arc<Topology>`. It provides a consistent view of the hardware NUMA
/// topology including nodes, CPUs, and their relationships.
///
/// # Example
///
/// ```no_run
/// use std::sync::Arc;
/// use numaperf_topo::Topology;
///
/// let topo = Arc::new(Topology::discover()?);
///
/// // Share with another thread
/// let topo2 = Arc::clone(&topo);
/// std::thread::spawn(move || {
///     println!("Nodes: {}", topo2.node_count());
/// });
/// # Ok::<(), numaperf_core::NumaError>(())
/// ```
#[derive(Debug)]
pub struct Topology {
    /// NUMA nodes in the system, sorted by node ID.
    nodes: Vec<NumaNode>,
    /// Map from CPU ID to node ID.
    cpu_to_node: HashMap<u32, NodeId>,
    /// Total number of CPUs.
    total_cpus: usize,
}

impl Topology {
    /// Discover the system's NUMA topology.
    ///
    /// On Linux, this reads from `/sys/devices/system/node/`. On other platforms
    /// or when NUMA information is unavailable, it returns a single-node topology.
    ///
    /// # Errors
    ///
    /// Returns an error if topology discovery fails due to I/O errors.
    pub fn discover() -> Result<Self, NumaError> {
        discovery::discover()
    }

    /// Create a topology from pre-discovered nodes.
    ///
    /// This is primarily for testing or when topology information comes from
    /// an external source.
    pub fn from_nodes(nodes: Vec<NumaNode>) -> Self {
        let mut cpu_to_node = HashMap::new();
        let mut total_cpus = 0;

        for node in &nodes {
            for cpu in node.cpus().iter() {
                cpu_to_node.insert(cpu, node.id());
                total_cpus += 1;
            }
        }

        Self {
            nodes,
            cpu_to_node,
            total_cpus,
        }
    }

    /// Create a single-node topology as a fallback.
    ///
    /// This is used on platforms without NUMA support or single-socket systems.
    pub fn single_node(cpus: CpuSet) -> Self {
        let total_cpus = cpus.count();
        let node = NumaNode::new(NodeId::new(0), cpus);
        let mut cpu_to_node = HashMap::new();

        for cpu in node.cpus().iter() {
            cpu_to_node.insert(cpu, NodeId::new(0));
        }

        Self {
            nodes: vec![node],
            cpu_to_node,
            total_cpus,
        }
    }

    /// Get all NUMA nodes in the system.
    #[inline]
    pub fn numa_nodes(&self) -> &[NumaNode] {
        &self.nodes
    }

    /// Get the number of NUMA nodes.
    #[inline]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get the total number of CPUs.
    #[inline]
    pub fn cpu_count(&self) -> usize {
        self.total_cpus
    }

    /// Check if this is a single-node (non-NUMA) system.
    #[inline]
    pub fn is_single_node(&self) -> bool {
        self.nodes.len() == 1
    }

    /// Get a node by its ID.
    pub fn node(&self, id: NodeId) -> Option<&NumaNode> {
        self.nodes.iter().find(|n| n.id() == id)
    }

    /// Get the CPU set for a specific node.
    ///
    /// Returns an empty set if the node ID is not found.
    pub fn cpu_set(&self, node: NodeId) -> CpuSet {
        self.node(node)
            .map(|n| n.cpus().clone())
            .unwrap_or_default()
    }

    /// Get the node that contains a specific CPU.
    pub fn node_for_cpu(&self, cpu: u32) -> Option<NodeId> {
        self.cpu_to_node.get(&cpu).copied()
    }

    /// Get an iterator over (NodeId, &NumaNode) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (NodeId, &NumaNode)> {
        self.nodes.iter().map(|n| (n.id(), n))
    }

    /// Print a human-readable summary of the topology.
    pub fn summary(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!(
            "NUMA Topology: {} nodes, {} CPUs\n",
            self.node_count(),
            self.cpu_count()
        ));

        for node in &self.nodes {
            s.push_str(&format!("  {}\n", node));
        }

        s
    }
}

// Topology is immutable and safe to share.
unsafe impl Send for Topology {}
unsafe impl Sync for Topology {}

impl std::fmt::Display for Topology {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Topology({} nodes, {} CPUs)",
            self.node_count(),
            self.cpu_count()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_node_topology() {
        let cpus = CpuSet::parse("0-7").unwrap();
        let topo = Topology::single_node(cpus);

        assert_eq!(topo.node_count(), 1);
        assert_eq!(topo.cpu_count(), 8);
        assert!(topo.is_single_node());
        assert_eq!(topo.node_for_cpu(0), Some(NodeId::new(0)));
        assert_eq!(topo.node_for_cpu(7), Some(NodeId::new(0)));
        assert_eq!(topo.node_for_cpu(8), None);
    }

    #[test]
    fn test_multi_node_topology() {
        let cpus0 = CpuSet::parse("0-3").unwrap();
        let cpus1 = CpuSet::parse("4-7").unwrap();

        let nodes = vec![
            NumaNode::new(NodeId::new(0), cpus0),
            NumaNode::new(NodeId::new(1), cpus1),
        ];

        let topo = Topology::from_nodes(nodes);

        assert_eq!(topo.node_count(), 2);
        assert_eq!(topo.cpu_count(), 8);
        assert!(!topo.is_single_node());
        assert_eq!(topo.node_for_cpu(0), Some(NodeId::new(0)));
        assert_eq!(topo.node_for_cpu(4), Some(NodeId::new(1)));
    }
}
