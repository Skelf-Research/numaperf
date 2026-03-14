# Observability

Learn how to monitor and diagnose NUMA locality in your application.

## StatsCollector

Collects locality metrics using sharded counters:

```rust
use numaperf::{StatsCollector, Topology};
use std::sync::Arc;

let topo = Arc::new(Topology::discover()?);
let collector = StatsCollector::new(&topo);

// Record metrics
collector.record_local_execution();      // Work ran locally
collector.record_steal(NodeId::new(1));  // Stole from node 1
collector.record_remote_access();        // Remote memory access

// Take snapshot
let stats = collector.snapshot();
println!("Locality ratio: {:.1}%", stats.locality_ratio() * 100.0);
```

## LocalityStats

Point-in-time snapshot of metrics:

```rust
let stats = collector.snapshot();

// Global metrics
println!("Local executions: {}", stats.local_executions());
println!("Remote steals: {}", stats.remote_steals());
println!("Locality ratio: {:.2}", stats.locality_ratio());

// Per-node metrics
for (node_id, node_stats) in stats.per_node() {
    println!("Node {}:", node_id.as_u32());
    println!("  Local: {}", node_stats.local_executions);
    println!("  Steals from: {}", node_stats.steals_from);
    println!("  Steals to: {}", node_stats.steals_to);
}
```

## LocalityReport

Diagnostic report with health assessment:

```rust
use numaperf::LocalityReport;

let stats = collector.snapshot();
let report = LocalityReport::generate(&stats);

// Print full report
println!("{}", report);

// Check health
println!("Health: {:?}", report.health());

// Get recommendations
if report.has_recommendations() {
    println!("Recommendations:");
    for rec in report.recommendations() {
        println!("  - {}", rec);
    }
}
```

## LocalityHealth

Classification of locality effectiveness:

```rust
use numaperf::LocalityHealth;

match report.health() {
    LocalityHealth::Excellent => println!("95%+ local - optimal"),
    LocalityHealth::Good => println!("85-95% local - good"),
    LocalityHealth::Fair => println!("70-85% local - acceptable"),
    LocalityHealth::Poor => println!("<70% local - needs attention"),
}

// Check acceptability
if report.health().is_acceptable() {
    println!("Locality is acceptable");
}
```

## Integration with Executor

Track locality in your workload:

```rust
let topo = Arc::new(Topology::discover()?);
let stats = Arc::new(StatsCollector::new(&topo));
let exec = NumaExecutor::builder(Arc::clone(&topo))
    .steal_policy(StealPolicy::LocalThenSocketThenRemote)
    .build()?;

// Submit work with tracking
for node in topo.numa_nodes() {
    let s = Arc::clone(&stats);
    exec.submit_to_node(node.id(), move || {
        s.record_local_execution();
        // Do work...
    });
}

exec.shutdown();

// Analyze
let snapshot = stats.snapshot();
let report = LocalityReport::generate(&snapshot);
println!("{}", report);
```

## Periodic Reporting

```rust
use std::thread;
use std::time::Duration;

let stats = Arc::new(StatsCollector::new(&topo));
let stats_clone = Arc::clone(&stats);

// Reporter thread
thread::spawn(move || {
    loop {
        thread::sleep(Duration::from_secs(10));

        let snapshot = stats_clone.snapshot();
        let report = LocalityReport::generate(&snapshot);

        log::info!("Locality: {:.1}%, Health: {:?}",
            snapshot.locality_ratio() * 100.0,
            report.health());

        if !report.health().is_acceptable() {
            for rec in report.recommendations() {
                log::warn!("Recommendation: {}", rec);
            }
        }
    }
});
```

## Sample Report Output

```
=== Locality Report ===

Overall Statistics:
  Local executions:  9,847
  Remote steals:     153
  Locality ratio:    98.5%

Per-Node Statistics:
  Node 0: 4,923 local, 12 steals from, 8 steals to
  Node 1: 4,924 local, 8 steals from, 12 steals to

Health: EXCELLENT

No recommendations - locality is optimal.
```

## Metrics Reference

| Metric | Description |
|--------|-------------|
| `local_executions` | Work items that ran on their submitted node |
| `remote_steals` | Work items stolen from other nodes |
| `locality_ratio` | `local / (local + remote)` |
| `steals_from` | Work stolen FROM this node |
| `steals_to` | Work stolen TO this node |

## Best Practices

1. **Create collector early** - Before starting work
2. **Record consistently** - Every work item should be tracked
3. **Report periodically** - Don't wait until shutdown
4. **Act on recommendations** - Adjust steal policy or pinning
5. **Set alerting thresholds** - Alert when health degrades
