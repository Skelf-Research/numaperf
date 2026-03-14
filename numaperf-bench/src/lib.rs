//! Shared utilities for numaperf benchmarks.

use numaperf::{StatsCollector, Topology};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Result of a benchmark run with locality measurement.
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    /// Name of the benchmark.
    pub name: String,
    /// Total duration of the benchmark.
    pub duration: Duration,
    /// Number of operations performed.
    pub operations: u64,
    /// Locality ratio (0.0 - 1.0).
    pub locality_ratio: Option<f64>,
}

impl BenchmarkResult {
    /// Calculate operations per second.
    pub fn ops_per_sec(&self) -> f64 {
        self.operations as f64 / self.duration.as_secs_f64()
    }
}

/// Measure the locality ratio while executing a workload.
pub fn measure_locality<F>(topo: &Arc<Topology>, work: F) -> (Duration, f64)
where
    F: FnOnce(&StatsCollector),
{
    let collector = StatsCollector::new(topo);

    let start = Instant::now();
    work(&collector);
    let elapsed = start.elapsed();

    let stats = collector.snapshot();
    let locality_ratio = stats.locality_ratio();

    (elapsed, locality_ratio)
}

/// Format a duration in a human-readable way.
pub fn format_duration(d: Duration) -> String {
    let nanos = d.as_nanos();
    if nanos < 1_000 {
        format!("{} ns", nanos)
    } else if nanos < 1_000_000 {
        format!("{:.2} µs", nanos as f64 / 1_000.0)
    } else if nanos < 1_000_000_000 {
        format!("{:.2} ms", nanos as f64 / 1_000_000.0)
    } else {
        format!("{:.2} s", d.as_secs_f64())
    }
}

/// Locality health status based on ratio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalityHealth {
    Excellent,
    Good,
    Fair,
    Poor,
}

impl LocalityHealth {
    /// Determine health status from a locality ratio.
    pub fn from_ratio(ratio: f64) -> Self {
        if ratio >= 0.95 {
            LocalityHealth::Excellent
        } else if ratio >= 0.85 {
            LocalityHealth::Good
        } else if ratio >= 0.70 {
            LocalityHealth::Fair
        } else {
            LocalityHealth::Poor
        }
    }

    /// Get a string description of the health status.
    pub fn description(&self) -> &'static str {
        match self {
            LocalityHealth::Excellent => "EXCELLENT",
            LocalityHealth::Good => "GOOD",
            LocalityHealth::Fair => "FAIR",
            LocalityHealth::Poor => "POOR",
        }
    }
}

impl std::fmt::Display for LocalityHealth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}
