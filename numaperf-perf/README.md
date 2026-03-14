# numaperf-perf

[![Crates.io](https://img.shields.io/crates/v/numaperf-perf.svg)](https://crates.io/crates/numaperf-perf)
[![Documentation](https://img.shields.io/badge/docs-skelfresearch.com-blue)](https://docs.skelfresearch.com/numaperf/api/observability/)

**NUMA locality observability and metrics collection.**

## Overview

numaperf-perf provides tools for monitoring NUMA locality effectiveness. Track local vs remote work execution, generate health reports, and identify cross-node traffic in your application.

## Usage

```toml
[dependencies]
numaperf-perf = "0.1"
```

Most users should use the `numaperf` facade crate instead.

## Example

```rust
use numaperf_perf::{StatsCollector, LocalityReport, LocalityHealth};
use numaperf_topo::Topology;
use std::sync::Arc;

fn main() -> Result<(), numaperf_core::NumaError> {
    let topo = Arc::new(Topology::discover()?);
    let collector = StatsCollector::new(&topo);

    // Record work execution
    collector.record_local_execution();
    collector.record_steal(numaperf_core::NodeId::new(1));

    // Take a snapshot
    let stats = collector.snapshot();
    println!("Locality: {:.1}%", stats.locality_ratio() * 100.0);

    // Generate health report
    let report = LocalityReport::generate(&stats);
    match report.health() {
        LocalityHealth::Excellent => println!("Great!"),
        LocalityHealth::Poor => {
            for rec in report.recommendations() {
                println!("Recommendation: {}", rec);
            }
        }
        _ => {}
    }

    Ok(())
}
```

## Features

- **`StatsCollector`** - Lock-free metrics collection
- **`LocalityStats`** - Point-in-time snapshots
- **`LocalityReport`** - Health assessment with recommendations
- **Per-node breakdown** - Detailed per-node statistics

## Health Levels

| Level | Locality | Meaning |
|-------|----------|---------|
| Excellent | > 95% | Optimal |
| Good | 85-95% | Acceptable |
| Fair | 70-85% | Consider tuning |
| Poor | < 70% | Needs attention |

## Part of numaperf

This crate is part of the [numaperf](https://github.com/Skelf-Research/numaperf) workspace.

- [Documentation](https://docs.skelfresearch.com/numaperf)
- [GitHub](https://github.com/Skelf-Research/numaperf)

## License

Licensed under the [MIT License](../LICENSE).
