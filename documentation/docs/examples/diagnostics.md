# Diagnostics Example

NUMA locality diagnostics and reporting.

## Overview

This example demonstrates:

- Collecting locality statistics
- Generating diagnostic reports
- Interpreting health classifications
- Acting on recommendations

## Running the Example

```bash
cargo run -p numaperf --example diagnostics
```

## Full Source Code

```rust
use numaperf::{
    LocalityHealth, LocalityReport, NodeId, StatsCollector, Topology,
};
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
```

## Key Concepts

### StatsCollector

Lock-free statistics collection with per-node sharded counters:

```rust
let collector = StatsCollector::new(&topo);

// Record local work execution
collector.record_local_execution();

// Record work stolen from another node
collector.record_steal(source_node_id);

// Take a point-in-time snapshot
let stats = collector.snapshot();
```

### LocalityStats

Snapshot of locality metrics:

```rust
let stats = collector.snapshot();

// Total work executed locally
stats.local_executions()    // -> u64

// Total work stolen from other nodes
stats.remote_steals()       // -> u64

// Ratio of local to total (0.0 - 1.0)
stats.locality_ratio()      // -> f64

// Per-node breakdown
for (node_id, node_stats) in stats.per_node() {
    println!("Node {}: {} local", node_id, node_stats.local_executions);
}
```

### LocalityReport

Diagnostic report with health assessment:

```rust
let report = LocalityReport::generate(&stats);

// Health classification
report.health()             // -> LocalityHealth

// Whether recommendations exist
report.has_recommendations() // -> bool

// Get recommendations
for rec in report.recommendations() {
    println!("- {}", rec);
}

// Print formatted report
report.print();
```

### LocalityHealth

Health classification enum:

| Level | Locality Ratio | Interpretation |
|-------|----------------|----------------|
| `Excellent` | > 95% | Optimal NUMA behavior |
| `Good` | 85-95% | Acceptable performance |
| `Fair` | 70-85% | Consider optimization |
| `Poor` | < 70% | Needs attention |

```rust
// Check if health is acceptable for production
if report.health().is_acceptable() {
    // Excellent, Good, or Fair
}
```

## Sample Output

```
=== numaperf: Diagnostics Example ===

System has 2 NUMA nodes

Simulating workload...

Scenario 1: Good locality pattern
  Local executions: 90
  Remote steals: 10
  Locality ratio: 90.0%
  Health: Excellent

Scenario 2: Poor locality pattern
  Local executions: 30
  Remote steals: 70
  Locality ratio: 30.0%
  Health: Poor
  Recommendations:
    - Consider using LocalOnly steal policy
    - Review work submission patterns
    - Ensure data is allocated on the processing node

=== Full Diagnostic Report ===

Locality Report
===============
Local executions: 80
Remote steals: 20
Locality ratio: 80.0%
Health: Good

=== Health Level Reference ===

EXCELLENT: >90% local execution - optimal NUMA behavior
GOOD:      70-90% local execution - acceptable performance
FAIR:      50-70% local execution - consider optimization
POOR:      <50% local execution - significant cross-node traffic

Checking current health...
  Your workload has good NUMA locality.
  Minor optimizations may help.

  Status: ACCEPTABLE for production

Diagnostics example complete.
```

## Integration Patterns

### Periodic Monitoring

```rust
use std::time::Duration;

let collector = StatsCollector::new(&topo);

// In a monitoring thread
loop {
    std::thread::sleep(Duration::from_secs(60));

    let stats = collector.snapshot();
    let report = LocalityReport::generate(&stats);

    if !report.health().is_acceptable() {
        log::warn!("Poor NUMA locality: {:.1}%",
            stats.locality_ratio() * 100.0);
    }

    // Reset for next interval
    collector.reset();
}
```

### Metrics Export

```rust
// Export to Prometheus or similar
fn export_metrics(stats: &LocalityStats) {
    gauge!("numa_local_executions", stats.local_executions() as f64);
    gauge!("numa_remote_steals", stats.remote_steals() as f64);
    gauge!("numa_locality_ratio", stats.locality_ratio());
}
```

### Adaptive Tuning

```rust
let report = LocalityReport::generate(&stats);

match report.health() {
    LocalityHealth::Poor => {
        // Switch to stricter steal policy
        executor.set_steal_policy(StealPolicy::LocalOnly);
    }
    LocalityHealth::Excellent => {
        // Allow more aggressive stealing for throughput
        executor.set_steal_policy(StealPolicy::Any);
    }
    _ => {
        // Keep balanced policy
    }
}
```

## Improving Poor Locality

If you see poor locality:

1. **Review work submission**: Submit work to the node where its data lives
2. **Check memory allocation**: Use `MemPolicy::Bind` for strict placement
3. **Adjust steal policy**: Use `LocalOnly` to prevent cross-node stealing
4. **Verify thread pinning**: Ensure workers stay on their assigned nodes
5. **Profile memory access**: Use `perf` to identify remote memory accesses

## Next Steps

- [Worker Pool Example](worker-pool.md) - Configure work stealing
- [Memory Placement Example](memory-placement.md) - Control data placement
- [Performance Tuning Guide](../advanced/performance-tuning.md) - Advanced optimization
