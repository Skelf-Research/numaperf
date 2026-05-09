//! Output formatting for benchmark results.

use numaperf_bench::{format_duration, BenchmarkResult, LocalityHealth};
use serde::Serialize;

/// JSON output structure.
#[derive(Serialize)]
pub struct JsonOutput {
    pub system: SystemInfo,
    pub benchmarks: Vec<BenchmarkJson>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locality_health: Option<LocalityHealthJson>,
}

/// System information.
#[derive(Serialize)]
pub struct SystemInfo {
    pub numa_nodes: usize,
    pub cpus: usize,
    pub hard_mode_supported: bool,
}

/// Benchmark result in JSON format.
#[derive(Serialize)]
pub struct BenchmarkJson {
    pub name: String,
    pub ops_per_sec: f64,
    pub duration_ns: u64,
    pub operations: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locality_ratio: Option<f64>,
}

/// Locality health in JSON format.
#[derive(Serialize)]
pub struct LocalityHealthJson {
    pub ratio: f64,
    pub status: String,
}

/// Print results in text format.
pub fn print_text_results(results: &[BenchmarkResult], overall_locality: Option<f64>) {
    for result in results {
        println!("{}:", result.name);
        println!("  Operations: {}", result.operations);
        println!("  Duration: {}", format_duration(result.duration));
        println!("  Throughput: {:.2} ops/sec", result.ops_per_sec());
        if let Some(ratio) = result.locality_ratio {
            println!("  Locality ratio: {:.1}%", ratio * 100.0);
        }
        println!();
    }

    if let Some(ratio) = overall_locality {
        let health = LocalityHealth::from_ratio(ratio);
        println!("────────────────────────────────");
        println!("Locality Health: {} ({:.1}% local)", health, ratio * 100.0);
    }
}
