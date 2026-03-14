//! Locality statistics types.

use std::time::Instant;

use numaperf_core::NodeId;

/// Statistics for a single NUMA node.
#[derive(Debug, Clone)]
pub struct NodeStats {
    /// The node ID.
    pub node_id: NodeId,
    /// Tasks executed locally (from own queue).
    pub local_executions: u64,
    /// Tasks stolen from this node by others.
    pub tasks_stolen: u64,
    /// Tasks this node stole from others.
    pub steals_performed: u64,
    /// Current queue depth (approximate).
    pub queue_depth: usize,
}

impl Default for NodeStats {
    fn default() -> Self {
        Self {
            node_id: NodeId::new(0),
            local_executions: 0,
            tasks_stolen: 0,
            steals_performed: 0,
            queue_depth: 0,
        }
    }
}

impl NodeStats {
    /// Create new node stats.
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
            ..Default::default()
        }
    }

    /// Total tasks processed by this node (local + stolen from others).
    pub fn total_processed(&self) -> u64 {
        self.local_executions + self.steals_performed
    }
}

impl std::fmt::Display for NodeStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: local={}, stolen_from={}, steals={}, queue={}",
            self.node_id,
            self.local_executions,
            self.tasks_stolen,
            self.steals_performed,
            self.queue_depth
        )
    }
}

/// A point-in-time snapshot of locality-related metrics.
///
/// Provides per-node breakdowns and aggregate statistics about
/// NUMA locality behavior.
#[derive(Debug, Clone)]
pub struct LocalityStats {
    /// Per-node statistics, indexed by node ID.
    node_stats: Vec<NodeStats>,
    /// Timestamp when snapshot was taken.
    timestamp: Instant,
}

impl LocalityStats {
    /// Create a new LocalityStats from per-node data.
    pub(crate) fn new(node_stats: Vec<NodeStats>) -> Self {
        Self {
            node_stats,
            timestamp: Instant::now(),
        }
    }

    /// Create stats for testing with specified local and remote counts.
    #[cfg(test)]
    pub(crate) fn mock(local: u64, remote: u64) -> Self {
        let mut stats = NodeStats::new(NodeId::new(0));
        stats.local_executions = local;
        stats.steals_performed = remote;
        Self::new(vec![stats])
    }

    /// Get per-node statistics.
    pub fn node_stats(&self) -> &[NodeStats] {
        &self.node_stats
    }

    /// Get stats for a specific node.
    pub fn node(&self, node: NodeId) -> Option<&NodeStats> {
        let idx = node.as_u32() as usize;
        self.node_stats.get(idx)
    }

    /// Get the number of nodes.
    pub fn node_count(&self) -> usize {
        self.node_stats.len()
    }

    /// Total local tasks executed across all nodes.
    pub fn local_executions(&self) -> u64 {
        self.node_stats.iter().map(|n| n.local_executions).sum()
    }

    /// Total remote steals performed across all nodes.
    ///
    /// This counts tasks that were stolen from their home node
    /// and executed elsewhere.
    pub fn remote_steals(&self) -> u64 {
        self.node_stats.iter().map(|n| n.steals_performed).sum()
    }

    /// Total tasks stolen from nodes (sum of tasks_stolen).
    ///
    /// This should equal `remote_steals()` in a consistent system.
    pub fn tasks_stolen(&self) -> u64 {
        self.node_stats.iter().map(|n| n.tasks_stolen).sum()
    }

    /// Total tasks processed across all nodes.
    pub fn total_processed(&self) -> u64 {
        self.local_executions() + self.remote_steals()
    }

    /// Locality ratio: local / (local + remote), 0.0-1.0.
    ///
    /// A ratio of 1.0 means perfect locality (all tasks executed locally).
    /// A ratio of 0.0 means all tasks were stolen from other nodes.
    pub fn locality_ratio(&self) -> f64 {
        let local = self.local_executions();
        let total = self.total_processed();

        if total == 0 {
            1.0 // No work done = perfect locality
        } else {
            local as f64 / total as f64
        }
    }

    /// Total queue depth across all nodes.
    pub fn total_queue_depth(&self) -> usize {
        self.node_stats.iter().map(|n| n.queue_depth).sum()
    }

    /// Get the timestamp when this snapshot was taken.
    pub fn timestamp(&self) -> Instant {
        self.timestamp
    }

    /// Generate a human-readable summary.
    pub fn summary(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!(
            "Locality Stats ({} nodes)\n",
            self.node_count()
        ));
        s.push_str(&format!(
            "  Total processed: {}\n",
            self.total_processed()
        ));
        s.push_str(&format!(
            "  Local executions: {}\n",
            self.local_executions()
        ));
        s.push_str(&format!(
            "  Remote steals: {}\n",
            self.remote_steals()
        ));
        s.push_str(&format!(
            "  Locality ratio: {:.1}%\n",
            self.locality_ratio() * 100.0
        ));
        s.push_str(&format!(
            "  Queue depth: {}\n",
            self.total_queue_depth()
        ));
        s.push_str("\nPer-node breakdown:\n");
        for stats in &self.node_stats {
            s.push_str(&format!("  {}\n", stats));
        }
        s
    }
}

impl std::fmt::Display for LocalityStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "LocalityStats(processed={}, local={}, remote={}, ratio={:.1}%)",
            self.total_processed(),
            self.local_executions(),
            self.remote_steals(),
            self.locality_ratio() * 100.0
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_stats_default() {
        let stats = NodeStats::new(NodeId::new(0));
        assert_eq!(stats.node_id, NodeId::new(0));
        assert_eq!(stats.local_executions, 0);
        assert_eq!(stats.tasks_stolen, 0);
        assert_eq!(stats.steals_performed, 0);
        assert_eq!(stats.queue_depth, 0);
    }

    #[test]
    fn test_node_stats_total_processed() {
        let mut stats = NodeStats::new(NodeId::new(0));
        stats.local_executions = 100;
        stats.steals_performed = 20;
        assert_eq!(stats.total_processed(), 120);
    }

    #[test]
    fn test_locality_stats_empty() {
        let stats = LocalityStats::new(vec![]);
        assert_eq!(stats.node_count(), 0);
        assert_eq!(stats.local_executions(), 0);
        assert_eq!(stats.remote_steals(), 0);
        assert_eq!(stats.locality_ratio(), 1.0); // No work = perfect locality
    }

    #[test]
    fn test_locality_stats_single_node() {
        let mut node = NodeStats::new(NodeId::new(0));
        node.local_executions = 90;
        node.steals_performed = 10;

        let stats = LocalityStats::new(vec![node]);
        assert_eq!(stats.local_executions(), 90);
        assert_eq!(stats.remote_steals(), 10);
        assert_eq!(stats.total_processed(), 100);
        assert!((stats.locality_ratio() - 0.9).abs() < 0.001);
    }

    #[test]
    fn test_locality_stats_multi_node() {
        let mut node0 = NodeStats::new(NodeId::new(0));
        node0.local_executions = 80;
        node0.steals_performed = 5;
        node0.tasks_stolen = 10;

        let mut node1 = NodeStats::new(NodeId::new(1));
        node1.local_executions = 70;
        node1.steals_performed = 10;
        node1.tasks_stolen = 5;

        let stats = LocalityStats::new(vec![node0, node1]);
        assert_eq!(stats.local_executions(), 150);
        assert_eq!(stats.remote_steals(), 15);
        assert_eq!(stats.tasks_stolen(), 15);
        assert_eq!(stats.total_processed(), 165);
    }

    #[test]
    fn test_locality_stats_node_lookup() {
        let node0 = NodeStats::new(NodeId::new(0));
        let node1 = NodeStats::new(NodeId::new(1));

        let stats = LocalityStats::new(vec![node0, node1]);

        assert!(stats.node(NodeId::new(0)).is_some());
        assert!(stats.node(NodeId::new(1)).is_some());
        assert!(stats.node(NodeId::new(2)).is_none());
    }

    #[test]
    fn test_locality_ratio_perfect() {
        let mut node = NodeStats::new(NodeId::new(0));
        node.local_executions = 100;
        node.steals_performed = 0;

        let stats = LocalityStats::new(vec![node]);
        assert_eq!(stats.locality_ratio(), 1.0);
    }

    #[test]
    fn test_locality_ratio_poor() {
        let mut node = NodeStats::new(NodeId::new(0));
        node.local_executions = 20;
        node.steals_performed = 80;

        let stats = LocalityStats::new(vec![node]);
        assert!((stats.locality_ratio() - 0.2).abs() < 0.001);
    }

    #[test]
    fn test_locality_stats_display() {
        let stats = LocalityStats::mock(90, 10);
        let display = format!("{}", stats);
        assert!(display.contains("90.0%"));
    }
}
