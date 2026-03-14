# Observability API

Types for monitoring NUMA locality.

## StatsCollector

Collects locality metrics using lock-free sharded counters.

```rust
pub struct StatsCollector { /* internal */ }
```

### Construction

```rust
use numaperf::{StatsCollector, Topology};
use std::sync::Arc;

let topo = Arc::new(Topology::discover()?);
let collector = StatsCollector::new(&topo);
```

### Methods

| Method | Description |
|--------|-------------|
| `new(topo) -> Self` | Create collector |
| `record_local_execution(&self)` | Record local work |
| `record_steal(&self, from: NodeId)` | Record work stolen from node |
| `record_remote_access(&self)` | Record remote memory access |
| `snapshot(&self) -> LocalityStats` | Take point-in-time snapshot |

### Example

```rust
let collector = StatsCollector::new(&topo);

// In worker threads
collector.record_local_execution();

// When stealing work
collector.record_steal(NodeId::new(1));

// Periodically snapshot
let stats = collector.snapshot();
println!("Locality: {:.1}%", stats.locality_ratio() * 100.0);
```

---

## LocalityStats

Point-in-time snapshot of locality metrics.

```rust
pub struct LocalityStats { /* internal */ }
```

### Methods

| Method | Description |
|--------|-------------|
| `local_executions(&self) -> u64` | Total local work items |
| `remote_steals(&self) -> u64` | Total stolen work items |
| `locality_ratio(&self) -> f64` | Ratio of local to total (0.0-1.0) |
| `per_node(&self) -> impl Iterator` | Per-node statistics |

### Example

```rust
let stats = collector.snapshot();

println!("Local: {}", stats.local_executions());
println!("Steals: {}", stats.remote_steals());
println!("Ratio: {:.1}%", stats.locality_ratio() * 100.0);

for (node_id, node_stats) in stats.per_node() {
    println!("Node {}: {} local", node_id.as_u32(), node_stats.local_executions);
}
```

---

## NodeStats

Per-node statistics.

```rust
pub struct NodeStats {
    pub local_executions: u64,
    pub steals_from: u64,
    pub steals_to: u64,
}
```

### Fields

| Field | Description |
|-------|-------------|
| `local_executions` | Work executed locally on this node |
| `steals_from` | Work stolen FROM this node |
| `steals_to` | Work stolen TO this node |

---

## LocalityReport

Diagnostic report with health assessment and recommendations.

```rust
pub struct LocalityReport { /* internal */ }
```

### Construction

```rust
use numaperf::LocalityReport;

let stats = collector.snapshot();
let report = LocalityReport::generate(&stats);
```

### Methods

| Method | Description |
|--------|-------------|
| `generate(stats) -> Self` | Generate from snapshot |
| `health(&self) -> LocalityHealth` | Health classification |
| `has_recommendations(&self) -> bool` | Whether recommendations exist |
| `recommendations(&self) -> impl Iterator` | Get recommendations |

### Display

```rust
let report = LocalityReport::generate(&stats);
println!("{}", report);  // Prints formatted report
```

---

## LocalityHealth

Health classification of locality effectiveness.

```rust
pub enum LocalityHealth {
    Excellent,  // 95%+ local
    Good,       // 85-95% local
    Fair,       // 70-85% local
    Poor,       // <70% local
}
```

### Methods

| Method | Description |
|--------|-------------|
| `is_acceptable(&self) -> bool` | True for Excellent, Good, Fair |

### Example

```rust
match report.health() {
    LocalityHealth::Excellent => println!("Optimal"),
    LocalityHealth::Good => println!("Good"),
    LocalityHealth::Fair => println!("Acceptable"),
    LocalityHealth::Poor => println!("Needs attention"),
}

if !report.health().is_acceptable() {
    for rec in report.recommendations() {
        println!("Recommendation: {}", rec);
    }
}
```

---

## Thread Safety

All observability types are `Send + Sync`:

- `StatsCollector` uses lock-free sharded counters
- Safe to share across threads
- Low overhead for recording
