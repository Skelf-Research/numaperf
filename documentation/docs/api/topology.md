# Topology API

Types for NUMA topology discovery and inspection.

## Topology

Discovered NUMA topology for the system.

```rust
pub struct Topology { /* internal */ }
```

### Construction

```rust
use numaperf::Topology;
use std::sync::Arc;

// Discover system topology
let topo = Arc::new(Topology::discover()?);

// Create synthetic single-node (for testing)
let topo = Topology::single_node(CpuSet::parse("0-7")?);
```

### Methods

| Method | Description |
|--------|-------------|
| `discover() -> Result<Self, NumaError>` | Discover system topology |
| `single_node(cpus: CpuSet) -> Self` | Create single-node topology |
| `from_nodes(nodes: Vec<NumaNode>) -> Self` | Create from node list |

### Query Methods

| Method | Description |
|--------|-------------|
| `numa_nodes(&self) -> &[NumaNode]` | Get all nodes |
| `node_count(&self) -> usize` | Number of nodes |
| `cpu_count(&self) -> usize` | Total CPU count |
| `is_single_node(&self) -> bool` | Check if single-node |
| `node(&self, id: NodeId) -> Option<&NumaNode>` | Get node by ID |
| `cpu_set(&self, node: NodeId) -> CpuSet` | Get CPUs for node |
| `node_for_cpu(&self, cpu: u32) -> Option<NodeId>` | Find node for CPU |

### Iteration

```rust
// Iterate nodes
for node in topo.numa_nodes() {
    println!("Node {}", node.id().as_u32());
}

// With node IDs
for (node_id, node) in topo.iter() {
    println!("{}: {} CPUs", node_id.as_u32(), node.cpu_count());
}
```

### Summary

```rust
println!("{}", topo.summary());
// Output: "2 NUMA nodes, 16 CPUs"
```

---

## NumaNode

Information about a single NUMA node.

```rust
pub struct NumaNode { /* internal */ }
```

### Methods

| Method | Description |
|--------|-------------|
| `id(&self) -> NodeId` | Node identifier |
| `cpus(&self) -> &CpuSet` | CPUs on this node |
| `cpu_count(&self) -> usize` | Number of CPUs |
| `memory_bytes(&self) -> Option<u64>` | Total memory in bytes |
| `memory_mb(&self) -> Option<u64>` | Total memory in MB |
| `distance_to(&self, other: NodeId) -> Option<u32>` | Distance to other node |
| `is_local(&self, other: NodeId) -> bool` | Check if distance ≤ 10 |

### Example

```rust
let topo = Topology::discover()?;

for node in topo.numa_nodes() {
    println!("Node {}:", node.id().as_u32());
    println!("  CPUs: {}", node.cpus());
    println!("  Memory: {:?} MB", node.memory_mb());

    // Print distances to other nodes
    for other in topo.numa_nodes() {
        if let Some(dist) = node.distance_to(other.id()) {
            println!("  -> Node {}: distance {}", other.id().as_u32(), dist);
        }
    }
}
```

### Distance Values

- `10` = Local (same node)
- `20-30` = Remote (different socket)
- Higher values indicate farther distance

### Thread Safety

Both `Topology` and `NumaNode` are `Send + Sync` and can be shared across threads via `Arc`.
