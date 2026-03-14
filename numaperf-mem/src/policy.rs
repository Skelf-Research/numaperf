//! Memory placement policies.

use numaperf_core::{NodeId, NodeMask};

/// Memory placement policy for NUMA regions.
///
/// These policies control where physical memory pages are allocated when
/// the region is first accessed (first-touch) or explicitly prefaulted.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum MemPolicy {
    /// Strictly bind memory to the specified nodes.
    ///
    /// Allocation will fail if sufficient memory is not available on
    /// the specified nodes. This is the strongest locality guarantee.
    ///
    /// Requires `CAP_SYS_ADMIN` for strict enforcement; falls back to
    /// `Preferred` in soft mode without privileges.
    Bind(NodeMask),

    /// Prefer allocating from the specified node.
    ///
    /// If memory is unavailable on the preferred node, allocation may
    /// fall back to other nodes. This is the most commonly used policy.
    Preferred(NodeId),

    /// Interleave pages across the specified nodes in round-robin fashion.
    ///
    /// This is useful for large read-mostly data structures that are
    /// accessed from multiple nodes, spreading bandwidth across all
    /// memory controllers.
    Interleave(NodeMask),

    /// Use the current thread's local node.
    ///
    /// Pages are allocated on the node where the thread accessing them
    /// is currently running. This is the kernel's default behavior.
    #[default]
    Local,
}

impl MemPolicy {
    /// Create a Bind policy for a single node.
    pub fn bind_single(node: NodeId) -> Self {
        Self::Bind(NodeMask::single(node))
    }

    /// Create an Interleave policy across all nodes in the mask.
    pub fn interleave(nodes: impl Into<NodeMask>) -> Self {
        Self::Interleave(nodes.into())
    }

    /// Get a human-readable name for this policy.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Bind(_) => "Bind",
            Self::Preferred(_) => "Preferred",
            Self::Interleave(_) => "Interleave",
            Self::Local => "Local",
        }
    }

    /// Get the nodes affected by this policy, if any.
    pub fn nodes(&self) -> Option<NodeMask> {
        match self {
            Self::Bind(mask) | Self::Interleave(mask) => Some(mask.clone()),
            Self::Preferred(node) => Some(NodeMask::single(*node)),
            Self::Local => None,
        }
    }
}

impl std::fmt::Display for MemPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bind(mask) => write!(f, "Bind({})", mask),
            Self::Preferred(node) => write!(f, "Preferred({})", node),
            Self::Interleave(mask) => write!(f, "Interleave({})", mask),
            Self::Local => write!(f, "Local"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mem_policy() {
        let bind = MemPolicy::bind_single(NodeId::new(0));
        assert_eq!(bind.name(), "Bind");

        let preferred = MemPolicy::Preferred(NodeId::new(1));
        assert_eq!(preferred.name(), "Preferred");

        let interleave = MemPolicy::interleave([NodeId::new(0), NodeId::new(1)]);
        assert_eq!(interleave.name(), "Interleave");

        let local = MemPolicy::Local;
        assert_eq!(local.name(), "Local");
        assert!(local.nodes().is_none());
    }

    #[test]
    fn test_mem_policy_display() {
        let bind = MemPolicy::bind_single(NodeId::new(0));
        assert_eq!(format!("{}", bind), "Bind(0)");

        let local = MemPolicy::Local;
        assert_eq!(format!("{}", local), "Local");
    }
}
