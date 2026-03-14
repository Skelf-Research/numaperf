# Basic Topology Discovery

Discover and display your system's NUMA topology.

## Overview

This example shows how to:

- Detect system capabilities
- Discover NUMA topology
- Query node information
- Display inter-node distances

## Running the Example

```bash
cargo run -p numaperf --example basic_topology
```

## Full Source Code

```rust
use numaperf::{Capabilities, Topology};

fn main() -> Result<(), numaperf::NumaError> {
    println!("=== numaperf: Basic Topology Example ===\n");

    // Check system capabilities
    let caps = Capabilities::detect();
    println!("{}", caps.summary());

    // Discover topology
    let topo = Topology::discover()?;

    println!("NUMA Topology Details");
    println!("=====================");
    println!("Total nodes: {}", topo.node_count());
    println!();

    for node in topo.numa_nodes() {
        println!("Node {}:", node.id());
        println!("  CPU count: {}", node.cpu_count());
        println!("  CPUs: {:?}", node.cpus());

        // Show distances to other nodes if available
        for other_node in topo.numa_nodes() {
            if other_node.id() != node.id() {
                if let Some(dist) = node.distance_to(other_node.id()) {
                    println!("  Distance to node {}: {}", other_node.id(), dist);
                }
            }
        }
        println!();
    }

    // Show which node the current thread would use
    println!("System Summary");
    println!("==============");
    if caps.is_numa_system() {
        println!("This is a multi-node NUMA system.");
        println!("NUMA-aware programming will provide benefits.");
    } else {
        println!("This is a single-node system (or NUMA is not detected).");
        println!("numaperf will work but NUMA optimizations won't apply.");
    }

    Ok(())
}
```

## Step-by-Step Walkthrough

### 1. Check Capabilities

```rust
let caps = Capabilities::detect();
println!("{}", caps.summary());
```

`Capabilities::detect()` probes the system for NUMA support:

- Number of NUMA nodes
- Whether hard mode (strict enforcement) is available
- Required kernel features

### 2. Discover Topology

```rust
let topo = Topology::discover()?;
```

This reads from `/sys/devices/system/node/` on Linux to build a complete picture of the NUMA topology.

### 3. Iterate Nodes

```rust
for node in topo.numa_nodes() {
    println!("Node {}:", node.id());
    println!("  CPU count: {}", node.cpu_count());
    println!("  CPUs: {:?}", node.cpus());
}
```

Each `NumaNode` provides:

| Method | Description |
|--------|-------------|
| `id()` | The node's `NodeId` |
| `cpu_count()` | Number of CPUs on this node |
| `cpus()` | `CpuSet` of CPU IDs |
| `distance_to(other)` | Distance to another node |

### 4. Query Distances

```rust
if let Some(dist) = node.distance_to(other_node.id()) {
    println!("  Distance to node {}: {}", other_node.id(), dist);
}
```

NUMA distances indicate memory access latency:

- `10` = Local access (same node)
- `20-30` = One hop (adjacent node)
- `40+` = Multiple hops

### 5. Check NUMA Status

```rust
if caps.is_numa_system() {
    println!("This is a multi-node NUMA system.");
} else {
    println!("This is a single-node system.");
}
```

Use this to decide whether to enable NUMA optimizations.

## Sample Output

On a 2-node system:

```
=== numaperf: Basic Topology Example ===

System Capabilities:
  NUMA nodes: 2
  Hard mode: available
  Affinity support: yes

NUMA Topology Details
=====================
Total nodes: 2

Node 0:
  CPU count: 8
  CPUs: CpuSet([0, 1, 2, 3, 4, 5, 6, 7])
  Distance to node 1: 20

Node 1:
  CPU count: 8
  CPUs: CpuSet([8, 9, 10, 11, 12, 13, 14, 15])
  Distance to node 0: 20

System Summary
==============
This is a multi-node NUMA system.
NUMA-aware programming will provide benefits.
```

## Key Takeaways

1. Always check `Capabilities` before assuming NUMA is available
2. Use `Topology` to discover node layout at runtime
3. Node distances help inform work placement decisions
4. numaperf works on single-node systems (it just won't provide NUMA benefits)

## Next Steps

- [Worker Pool Example](worker-pool.md) - Execute tasks with NUMA awareness
- [Memory Placement Example](memory-placement.md) - Allocate memory on specific nodes
