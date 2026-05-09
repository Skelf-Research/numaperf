//! NUMA node representation.

use numaperf_core::{CpuSet, NodeId};

/// Represents a single NUMA node in the system.
///
/// A NUMA node is a group of CPUs that share local memory. Access to memory
/// on the same node is faster than access to memory on remote nodes.
#[derive(Debug, Clone)]
pub struct NumaNode {
    /// The node identifier.
    id: NodeId,
    /// CPUs belonging to this node.
    cpus: CpuSet,
    /// Total memory in bytes, if known.
    memory_bytes: Option<u64>,
    /// Distances to other nodes (indexed by node ID).
    distances: Vec<u32>,
}

impl NumaNode {
    /// Create a new NUMA node.
    pub(crate) fn new(id: NodeId, cpus: CpuSet) -> Self {
        Self {
            id,
            cpus,
            memory_bytes: None,
            distances: Vec::new(),
        }
    }

    /// Set the total memory for this node.
    #[allow(dead_code)]
    pub(crate) fn with_memory(mut self, bytes: u64) -> Self {
        self.memory_bytes = Some(bytes);
        self
    }

    /// Set the distance table for this node (builder pattern).
    #[allow(dead_code)]
    pub(crate) fn with_distances(mut self, distances: Vec<u32>) -> Self {
        self.distances = distances;
        self
    }

    /// Set the distance table for this node (in-place).
    #[allow(dead_code)]
    pub(crate) fn set_distances(&mut self, distances: Vec<u32>) {
        self.distances = distances;
    }

    /// Get the node identifier.
    #[inline]
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// Get the CPUs belonging to this node.
    #[inline]
    pub fn cpus(&self) -> &CpuSet {
        &self.cpus
    }

    /// Get the number of CPUs on this node.
    #[inline]
    pub fn cpu_count(&self) -> usize {
        self.cpus.count()
    }

    /// Get the total memory in bytes, if known.
    #[inline]
    pub fn memory_bytes(&self) -> Option<u64> {
        self.memory_bytes
    }

    /// Get the total memory in megabytes, if known.
    #[inline]
    pub fn memory_mb(&self) -> Option<u64> {
        self.memory_bytes.map(|b| b / (1024 * 1024))
    }

    /// Get the distance to another node.
    ///
    /// Returns `None` if the distance is not known. A distance of 10 typically
    /// represents local access, while higher values indicate remote access.
    pub fn distance_to(&self, other: NodeId) -> Option<u32> {
        self.distances.get(other.as_u32() as usize).copied()
    }

    /// Check if this is a local access (distance <= 10).
    pub fn is_local(&self, other: NodeId) -> bool {
        self.distance_to(other).is_some_and(|d| d <= 10)
    }
}

impl std::fmt::Display for NumaNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (CPUs: {})", self.id, self.cpus)?;
        if let Some(mb) = self.memory_mb() {
            write!(f, ", {} MB", mb)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_numa_node() {
        let mut cpus = CpuSet::new();
        cpus.add(0);
        cpus.add(1);
        cpus.add(2);
        cpus.add(3);

        let node = NumaNode::new(NodeId::new(0), cpus)
            .with_memory(16 * 1024 * 1024 * 1024)
            .with_distances(vec![10, 20]);

        assert_eq!(node.id(), NodeId::new(0));
        assert_eq!(node.cpu_count(), 4);
        assert_eq!(node.memory_mb(), Some(16384));
        assert_eq!(node.distance_to(NodeId::new(0)), Some(10));
        assert_eq!(node.distance_to(NodeId::new(1)), Some(20));
        assert!(node.is_local(NodeId::new(0)));
        assert!(!node.is_local(NodeId::new(1)));
    }
}
