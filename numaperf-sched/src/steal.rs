//! Work stealing policy configuration.

use numaperf_core::NodeId;
use numaperf_topo::Topology;

/// Configures how workers steal work from other nodes when their local queue is empty.
///
/// Work stealing is essential for load balancing, but stealing from remote nodes
/// incurs higher latency due to NUMA memory access patterns. The steal policy
/// controls the order in which nodes are considered for stealing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StealPolicy {
    /// Never steal from other nodes.
    ///
    /// Workers only process work from their local queue. This provides the
    /// strongest locality guarantees but may lead to load imbalance.
    LocalOnly,

    /// Steal from closer nodes first, based on NUMA distance.
    ///
    /// This is the recommended default. Workers prefer to steal from nodes
    /// on the same socket (lower distance) before stealing from remote nodes.
    #[default]
    LocalThenSocketThenRemote,

    /// Steal from any node without locality preference.
    ///
    /// Workers steal from whichever node has work available, regardless of
    /// NUMA distance. This maximizes throughput but may increase memory
    /// access latency.
    Any,
}

impl StealPolicy {
    /// Get the ordered list of nodes to try stealing from.
    ///
    /// Returns nodes in priority order based on the policy. The `from_node`
    /// is excluded from the result.
    pub fn steal_order(&self, from_node: NodeId, topo: &Topology) -> Vec<NodeId> {
        match self {
            StealPolicy::LocalOnly => Vec::new(),

            StealPolicy::LocalThenSocketThenRemote => {
                let mut nodes: Vec<NodeId> = topo
                    .numa_nodes()
                    .iter()
                    .map(|n| n.id())
                    .filter(|&n| n != from_node)
                    .collect();

                // Sort by distance from the stealing node
                if let Some(from) = topo.node(from_node) {
                    nodes.sort_by_key(|&n| from.distance_to(n).unwrap_or(u32::MAX));
                }

                nodes
            }

            StealPolicy::Any => topo
                .numa_nodes()
                .iter()
                .map(|n| n.id())
                .filter(|&n| n != from_node)
                .collect(),
        }
    }

    /// Check if this policy allows stealing from other nodes.
    #[inline]
    pub fn allows_stealing(&self) -> bool {
        !matches!(self, StealPolicy::LocalOnly)
    }
}

impl std::fmt::Display for StealPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StealPolicy::LocalOnly => write!(f, "local-only"),
            StealPolicy::LocalThenSocketThenRemote => write!(f, "local-then-socket-then-remote"),
            StealPolicy::Any => write!(f, "any"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use numaperf_core::CpuSet;

    fn make_test_topology() -> Topology {
        // Use the real topology or fall back to single-node
        Topology::discover().unwrap_or_else(|_| {
            let cpus = CpuSet::parse("0-7").unwrap();
            Topology::single_node(cpus)
        })
    }

    #[test]
    fn test_local_only_returns_empty() {
        let topo = make_test_topology();
        let first_node = topo.numa_nodes()[0].id();
        let order = StealPolicy::LocalOnly.steal_order(first_node, &topo);
        assert!(order.is_empty());
    }

    #[test]
    fn test_any_excludes_self() {
        let topo = make_test_topology();
        let first_node = topo.numa_nodes()[0].id();
        let order = StealPolicy::Any.steal_order(first_node, &topo);

        // Should not include self
        assert!(!order.contains(&first_node));

        // Should include all other nodes
        assert_eq!(order.len(), topo.node_count() - 1);
    }

    #[test]
    fn test_default_is_local_then_socket() {
        assert_eq!(StealPolicy::default(), StealPolicy::LocalThenSocketThenRemote);
    }

    #[test]
    fn test_allows_stealing() {
        assert!(!StealPolicy::LocalOnly.allows_stealing());
        assert!(StealPolicy::LocalThenSocketThenRemote.allows_stealing());
        assert!(StealPolicy::Any.allows_stealing());
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", StealPolicy::LocalOnly), "local-only");
        assert_eq!(
            format!("{}", StealPolicy::LocalThenSocketThenRemote),
            "local-then-socket-then-remote"
        );
        assert_eq!(format!("{}", StealPolicy::Any), "any");
    }
}
