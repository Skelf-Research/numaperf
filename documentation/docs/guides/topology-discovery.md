# Topology Discovery

Learn how to discover and work with your system's NUMA topology.

## Basic Discovery

```rust
use numaperf::Topology;
use std::sync::Arc;

fn main() -> Result<(), numaperf::NumaError> {
    // Discover topology (do this once at startup)
    let topo = Arc::new(Topology::discover()?);

    println!("NUMA nodes: {}", topo.node_count());
    println!("Total CPUs: {}", topo.cpu_count());

    Ok(())
}
```

## Examining Nodes

```rust
for node in topo.numa_nodes() {
    println!("Node {}:", node.id().as_u32());
    println!("  CPUs: {} (count: {})", node.cpus(), node.cpu_count());
    println!("  Memory: {:?} MB", node.memory_mb());
}
```

## NUMA Distances

Distances indicate relative memory access cost:

```rust
for src in topo.numa_nodes() {
    for dst in topo.numa_nodes() {
        if let Some(distance) = src.distance_to(dst.id()) {
            println!("Node {} -> Node {}: distance {}",
                src.id().as_u32(),
                dst.id().as_u32(),
                distance);
        }
    }
}
```

- Distance 10 = local (baseline)
- Distance 20+ = remote

## Mapping CPUs to Nodes

```rust
// Find which node a CPU belongs to
if let Some(node_id) = topo.node_for_cpu(5) {
    println!("CPU 5 is on node {}", node_id.as_u32());
}

// Get CPUs for a specific node
let node0_cpus = topo.cpu_set(NodeId::new(0));
println!("Node 0 CPUs: {}", node0_cpus);
```

## Sharing Topology

Topology should be created once and shared:

```rust
use std::sync::Arc;
use std::thread;

let topo = Arc::new(Topology::discover()?);

let handles: Vec<_> = (0..4).map(|i| {
    let topo = Arc::clone(&topo);
    thread::spawn(move || {
        println!("Thread {} sees {} nodes", i, topo.node_count());
    })
}).collect();

for h in handles {
    h.join().unwrap();
}
```

## Single-Node Fallback

On non-NUMA systems, a synthetic single node is created:

```rust
let topo = Topology::discover()?;

if topo.is_single_node() {
    println!("Running on a single-node system");
    // NUMA optimizations still work, just no benefit
}
```

## Testing with Synthetic Topology

For testing, create a synthetic topology:

```rust
use numaperf::{Topology, CpuSet};

// Create a fake single-node topology
let cpus = CpuSet::parse("0-7")?;
let topo = Topology::single_node(cpus);

assert_eq!(topo.node_count(), 1);
```

## Best Practices

1. **Discover once** at application startup
2. **Share via Arc** - topology is immutable and thread-safe
3. **Check is_single_node()** to adapt behavior
4. **Cache node lookups** - don't call `node_for_cpu()` in hot paths
