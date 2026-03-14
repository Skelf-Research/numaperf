//! Locality diagnostic reports.

use crate::stats::LocalityStats;

/// Health classification based on locality ratio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LocalityHealth {
    /// > 90% local execution rate.
    Excellent,
    /// 70-90% local execution rate.
    Good,
    /// 50-70% local execution rate.
    Fair,
    /// < 50% local execution rate.
    Poor,
}

impl LocalityHealth {
    /// Classify health based on locality ratio.
    pub fn from_ratio(ratio: f64) -> Self {
        match ratio {
            r if r >= 0.9 => LocalityHealth::Excellent,
            r if r >= 0.7 => LocalityHealth::Good,
            r if r >= 0.5 => LocalityHealth::Fair,
            _ => LocalityHealth::Poor,
        }
    }

    /// Get a description of this health level.
    pub fn description(&self) -> &'static str {
        match self {
            LocalityHealth::Excellent => "Excellent locality (>90% local)",
            LocalityHealth::Good => "Good locality (70-90% local)",
            LocalityHealth::Fair => "Fair locality (50-70% local)",
            LocalityHealth::Poor => "Poor locality (<50% local)",
        }
    }

    /// Check if this health level is acceptable.
    pub fn is_acceptable(&self) -> bool {
        matches!(self, LocalityHealth::Excellent | LocalityHealth::Good)
    }
}

impl std::fmt::Display for LocalityHealth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LocalityHealth::Excellent => write!(f, "EXCELLENT"),
            LocalityHealth::Good => write!(f, "GOOD"),
            LocalityHealth::Fair => write!(f, "FAIR"),
            LocalityHealth::Poor => write!(f, "POOR"),
        }
    }
}

impl Default for LocalityHealth {
    fn default() -> Self {
        LocalityHealth::Excellent
    }
}

/// A diagnostic report summarizing locality health.
///
/// Provides actionable insights about NUMA behavior based on
/// collected statistics.
///
/// # Example
///
/// ```no_run
/// use std::sync::Arc;
/// use numaperf_perf::{StatsCollector, LocalityReport};
/// use numaperf_topo::Topology;
///
/// let topo = Arc::new(Topology::discover()?);
/// let collector = StatsCollector::new(&topo);
///
/// // ... run workload ...
///
/// let stats = collector.snapshot();
/// let report = LocalityReport::generate(&stats);
///
/// println!("Health: {}", report.health());
/// for rec in report.recommendations() {
///     println!("  - {}", rec);
/// }
/// # Ok::<(), numaperf_core::NumaError>(())
/// ```
#[derive(Debug, Clone)]
pub struct LocalityReport {
    /// The underlying statistics.
    stats: LocalityStats,
    /// Overall health assessment.
    health: LocalityHealth,
    /// Actionable recommendations.
    recommendations: Vec<String>,
}

impl LocalityReport {
    /// Generate a report from statistics.
    pub fn generate(stats: &LocalityStats) -> Self {
        let ratio = stats.locality_ratio();
        let health = LocalityHealth::from_ratio(ratio);

        let mut recommendations = Vec::new();

        // Recommendation: Poor locality
        if health == LocalityHealth::Poor {
            recommendations.push(
                "Consider using StealPolicy::LocalOnly to prevent cross-node stealing".into(),
            );
            recommendations.push(
                "Review work submission patterns - tasks should be submitted to their data's home node".into(),
            );
        } else if health == LocalityHealth::Fair {
            recommendations.push(
                "Consider using StealPolicy::LocalThenSocketThenRemote for better locality".into(),
            );
        }

        // Recommendation: Imbalanced queues
        let depths: Vec<_> = stats.node_stats().iter().map(|n| n.queue_depth).collect();
        if let (Some(&min), Some(&max)) = (depths.iter().min(), depths.iter().max()) {
            if max > 0 && min == 0 && depths.len() > 1 {
                recommendations.push(
                    "Some nodes have empty queues while others are busy. Distribute work more evenly.".into(),
                );
            } else if max > min * 10 && max > 100 {
                recommendations.push(format!(
                    "Queue imbalance detected: min={}, max={}. Consider rebalancing work submission.",
                    min, max
                ));
            }
        }

        // Recommendation: High steal rate on specific nodes
        for node_stats in stats.node_stats() {
            let total = node_stats.local_executions + node_stats.tasks_stolen;
            if total > 100 && node_stats.tasks_stolen > node_stats.local_executions {
                recommendations.push(format!(
                    "{} has more stolen tasks ({}) than local executions ({}). \
                     Consider pinning related work to this node.",
                    node_stats.node_id,
                    node_stats.tasks_stolen,
                    node_stats.local_executions
                ));
            }
        }

        // Recommendation: No work processed
        if stats.total_processed() == 0 {
            recommendations.push(
                "No tasks processed yet. Collect more data before drawing conclusions.".into(),
            );
        }

        Self {
            stats: stats.clone(),
            health,
            recommendations,
        }
    }

    /// Get the overall health assessment.
    pub fn health(&self) -> LocalityHealth {
        self.health
    }

    /// Get the actionable recommendations.
    pub fn recommendations(&self) -> &[String] {
        &self.recommendations
    }

    /// Check if there are any recommendations.
    pub fn has_recommendations(&self) -> bool {
        !self.recommendations.is_empty()
    }

    /// Get the underlying statistics.
    pub fn stats(&self) -> &LocalityStats {
        &self.stats
    }

    /// Print the report to stdout.
    pub fn print(&self) {
        println!("{}", self);
    }

    /// Format the report as a string.
    pub fn format(&self) -> String {
        format!("{}", self)
    }
}

impl std::fmt::Display for LocalityReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== NUMA Locality Report ===")?;
        writeln!(f)?;
        writeln!(f, "Health: {} - {}", self.health, self.health.description())?;
        writeln!(f)?;
        writeln!(f, "Statistics:")?;
        writeln!(f, "  Total processed: {}", self.stats.total_processed())?;
        writeln!(f, "  Local executions: {}", self.stats.local_executions())?;
        writeln!(f, "  Remote steals: {}", self.stats.remote_steals())?;
        writeln!(
            f,
            "  Locality ratio: {:.1}%",
            self.stats.locality_ratio() * 100.0
        )?;

        if self.stats.node_count() > 1 {
            writeln!(f)?;
            writeln!(f, "Per-node breakdown:")?;
            for node in self.stats.node_stats() {
                writeln!(f, "  {}", node)?;
            }
        }

        if !self.recommendations.is_empty() {
            writeln!(f)?;
            writeln!(f, "Recommendations:")?;
            for (i, rec) in self.recommendations.iter().enumerate() {
                writeln!(f, "  {}. {}", i + 1, rec)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::NodeStats;
    use numaperf_core::NodeId;

    fn make_stats(local: u64, remote: u64) -> LocalityStats {
        let mut node = NodeStats::new(NodeId::new(0));
        node.local_executions = local;
        node.steals_performed = remote;
        LocalityStats::new(vec![node])
    }

    #[test]
    fn test_health_from_ratio() {
        assert_eq!(LocalityHealth::from_ratio(0.95), LocalityHealth::Excellent);
        assert_eq!(LocalityHealth::from_ratio(0.90), LocalityHealth::Excellent);
        assert_eq!(LocalityHealth::from_ratio(0.89), LocalityHealth::Good);
        assert_eq!(LocalityHealth::from_ratio(0.70), LocalityHealth::Good);
        assert_eq!(LocalityHealth::from_ratio(0.69), LocalityHealth::Fair);
        assert_eq!(LocalityHealth::from_ratio(0.50), LocalityHealth::Fair);
        assert_eq!(LocalityHealth::from_ratio(0.49), LocalityHealth::Poor);
        assert_eq!(LocalityHealth::from_ratio(0.0), LocalityHealth::Poor);
    }

    #[test]
    fn test_health_is_acceptable() {
        assert!(LocalityHealth::Excellent.is_acceptable());
        assert!(LocalityHealth::Good.is_acceptable());
        assert!(!LocalityHealth::Fair.is_acceptable());
        assert!(!LocalityHealth::Poor.is_acceptable());
    }

    #[test]
    fn test_report_excellent() {
        let stats = make_stats(95, 5);
        let report = LocalityReport::generate(&stats);

        assert_eq!(report.health(), LocalityHealth::Excellent);
        // Excellent health shouldn't have locality recommendations
        assert!(
            report
                .recommendations()
                .iter()
                .all(|r| !r.contains("StealPolicy")),
            "Excellent health shouldn't recommend changing steal policy"
        );
    }

    #[test]
    fn test_report_poor() {
        let stats = make_stats(30, 70);
        let report = LocalityReport::generate(&stats);

        assert_eq!(report.health(), LocalityHealth::Poor);
        assert!(report.has_recommendations());
        assert!(
            report
                .recommendations()
                .iter()
                .any(|r| r.contains("LocalOnly")),
            "Poor health should recommend LocalOnly policy"
        );
    }

    #[test]
    fn test_report_no_work() {
        let stats = make_stats(0, 0);
        let report = LocalityReport::generate(&stats);

        assert_eq!(report.health(), LocalityHealth::Excellent); // No work = perfect locality
        assert!(report.has_recommendations());
        assert!(
            report
                .recommendations()
                .iter()
                .any(|r| r.contains("No tasks processed")),
            "Should recommend collecting more data"
        );
    }

    #[test]
    fn test_report_display() {
        let stats = make_stats(80, 20);
        let report = LocalityReport::generate(&stats);

        let output = format!("{}", report);
        assert!(output.contains("NUMA Locality Report"));
        assert!(output.contains("GOOD"));
        assert!(output.contains("80.0%"));
    }

    #[test]
    fn test_health_display() {
        assert_eq!(format!("{}", LocalityHealth::Excellent), "EXCELLENT");
        assert_eq!(format!("{}", LocalityHealth::Good), "GOOD");
        assert_eq!(format!("{}", LocalityHealth::Fair), "FAIR");
        assert_eq!(format!("{}", LocalityHealth::Poor), "POOR");
    }

    #[test]
    fn test_high_steal_recommendation() {
        // Create a node where tasks_stolen > local_executions
        let mut node = NodeStats::new(NodeId::new(0));
        node.local_executions = 50;
        node.tasks_stolen = 150; // More stolen than local
        node.steals_performed = 0;

        let stats = LocalityStats::new(vec![node]);
        let report = LocalityReport::generate(&stats);

        assert!(
            report
                .recommendations()
                .iter()
                .any(|r| r.contains("pinning")),
            "Should recommend pinning when steal ratio is high"
        );
    }
}
