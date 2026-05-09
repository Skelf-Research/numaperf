//! NUMA locality diagnostics and reporting.
//!
//! This example demonstrates collecting locality statistics and
//! generating diagnostic reports to analyze NUMA behavior.
//!
//! Run with: cargo run -p numaperf --example diagnostics

use numaperf::{LocalityHealth, LocalityReport, NodeId, StatsCollector, Topology};
use std::sync::Arc;

fn main() -> Result<(), numaperf::NumaError> {
    println!("=== numaperf: Diagnostics Example ===\n");

    // Discover topology
    let topo = Arc::new(Topology::discover()?);
    println!("System has {} NUMA nodes", topo.node_count());
    println!();

    // Create statistics collector
    let collector = StatsCollector::new(&topo);

    // Simulate workload with different locality patterns
    println!("Simulating workload...\n");

    // Scenario 1: Good locality (mostly local executions)
    println!("Scenario 1: Good locality pattern");
    collector.reset();

    // 90 local executions, 10 steals
    collector.record_local_executions(90);
    for _ in 0..10 {
        collector.record_steal(NodeId::new(0));
    }

    let stats = collector.snapshot();
    println!("  Local executions: {}", stats.local_executions());
    println!("  Remote steals: {}", stats.remote_steals());
    println!("  Locality ratio: {:.1}%", stats.locality_ratio() * 100.0);

    let report = LocalityReport::generate(&stats);
    println!("  Health: {}", report.health());
    if report.has_recommendations() {
        println!("  Recommendations:");
        for rec in report.recommendations() {
            println!("    - {}", rec);
        }
    }
    println!();

    // Scenario 2: Poor locality (many steals)
    println!("Scenario 2: Poor locality pattern");
    collector.reset();

    // 30 local executions, 70 steals
    collector.record_local_executions(30);
    for _ in 0..70 {
        collector.record_steal(NodeId::new(0));
    }

    let stats = collector.snapshot();
    println!("  Local executions: {}", stats.local_executions());
    println!("  Remote steals: {}", stats.remote_steals());
    println!("  Locality ratio: {:.1}%", stats.locality_ratio() * 100.0);

    let report = LocalityReport::generate(&stats);
    println!("  Health: {}", report.health());
    if report.has_recommendations() {
        println!("  Recommendations:");
        for rec in report.recommendations() {
            println!("    - {}", rec);
        }
    }
    println!();

    // Generate full report
    println!("=== Full Diagnostic Report ===\n");

    // Create a more realistic workload pattern
    collector.reset();
    collector.record_local_executions(80);
    for _ in 0..20 {
        collector.record_steal(NodeId::new(0));
    }

    let stats = collector.snapshot();
    let report = LocalityReport::generate(&stats);
    report.print();

    // Show how to interpret health levels
    println!("\n=== Health Level Reference ===\n");
    println!("EXCELLENT: >90% local execution - optimal NUMA behavior");
    println!("GOOD:      70-90% local execution - acceptable performance");
    println!("FAIR:      50-70% local execution - consider optimization");
    println!("POOR:      <50% local execution - significant cross-node traffic");
    println!();

    // Demonstrate health checking
    println!("Checking current health...");
    match report.health() {
        LocalityHealth::Excellent => {
            println!("  Your workload has excellent NUMA locality!");
        }
        LocalityHealth::Good => {
            println!("  Your workload has good NUMA locality.");
            println!("  Minor optimizations may help.");
        }
        LocalityHealth::Fair => {
            println!("  Your workload has fair NUMA locality.");
            println!("  Consider reviewing work submission patterns.");
        }
        LocalityHealth::Poor => {
            println!("  Your workload has poor NUMA locality!");
            println!("  Review recommendations above to improve performance.");
        }
    }

    if report.health().is_acceptable() {
        println!("\n  Status: ACCEPTABLE for production");
    } else {
        println!("\n  Status: NEEDS ATTENTION before production");
    }

    println!();
    println!("Diagnostics example complete.");

    Ok(())
}
