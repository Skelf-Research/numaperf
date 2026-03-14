# Memory Placement Example

Memory allocation with explicit NUMA placement policies.

## Overview

This example demonstrates:

- Local policy for current-node allocation
- Bind policy for strict node placement
- Preferred policy with fallback
- Interleave policy for bandwidth optimization

## Running the Example

```bash
cargo run -p numaperf --example memory_placement
```

## Full Source Code

```rust
use numaperf::{MemPolicy, NodeId, NodeMask, NumaRegion, Prefault, Topology};

fn main() -> Result<(), numaperf::NumaError> {
    println!("=== numaperf: Memory Placement Example ===\n");

    // Discover topology
    let topo = Topology::discover()?;
    println!("System has {} NUMA nodes", topo.node_count());
    println!();

    let size = 1024 * 1024; // 1 MB

    // 1. Local policy (default) - allocate on current thread's node
    println!("1. Local Policy");
    println!("   Allocates on the current thread's NUMA node.");
    let local_region = NumaRegion::anon(
        size,
        MemPolicy::Local,
        Default::default(),
        Prefault::Touch
    )?;
    println!("   Allocated {} bytes with Local policy", local_region.len());
    println!("   Enforcement: {:?}", local_region.enforcement());
    println!();

    // 2. Bind policy - strict allocation on specific node(s)
    println!("2. Bind Policy");
    println!("   Strictly allocates on specified nodes only.");
    let node0 = NodeMask::single(NodeId::new(0));
    let bind_region = NumaRegion::anon(
        size,
        MemPolicy::Bind(node0),
        Default::default(),
        Prefault::Touch
    )?;
    println!("   Allocated {} bytes bound to node 0", bind_region.len());
    println!("   Enforcement: {:?}", bind_region.enforcement());
    println!();

    // 3. Preferred policy - prefer one node but allow fallback
    println!("3. Preferred Policy");
    println!("   Prefers the specified node but allows fallback.");
    let preferred_region = NumaRegion::anon(
        size,
        MemPolicy::Preferred(NodeId::new(0)),
        Default::default(),
        Prefault::Touch,
    )?;
    println!(
        "   Allocated {} bytes with preference for node 0",
        preferred_region.len()
    );
    println!("   Enforcement: {:?}", preferred_region.enforcement());
    println!();

    // 4. Interleave policy - round-robin across nodes
    if topo.node_count() > 1 {
        println!("4. Interleave Policy");
        println!("   Round-robins pages across multiple nodes.");
        // Build a node mask containing all nodes
        let mut all_nodes = NodeMask::new();
        for node in topo.numa_nodes() {
            all_nodes.add(node.id());
        }
        let interleave_region = NumaRegion::anon(
            size,
            MemPolicy::Interleave(all_nodes),
            Default::default(),
            Prefault::Touch,
        )?;
        println!(
            "   Allocated {} bytes interleaved across {} nodes",
            interleave_region.len(),
            topo.node_count()
        );
        println!("   Enforcement: {:?}", interleave_region.enforcement());
    } else {
        println!("4. Interleave Policy");
        println!("   (Skipped - requires multiple NUMA nodes)");
    }
    println!();

    // Demonstrate writing to memory
    println!("Writing to allocated regions...");
    let mut region = NumaRegion::anon(
        size,
        MemPolicy::Local,
        Default::default(),
        Prefault::Touch
    )?;
    let slice = region.as_mut_slice();

    // Write pattern
    for (i, byte) in slice.iter_mut().enumerate() {
        *byte = (i % 256) as u8;
    }

    // Verify
    let checksum: u64 = slice.iter().map(|&b| b as u64).sum();
    println!("   Written {} bytes, checksum: {}", slice.len(), checksum);

    println!();
    println!("Memory placement example complete.");

    Ok(())
}
```

## Policy Comparison

| Policy | Placement | Fallback | Use Case |
|--------|-----------|----------|----------|
| `Local` | Current thread's node | Next available | Default, thread-local data |
| `Bind` | Specified nodes only | Allocation fails | Strict placement requirements |
| `Preferred` | Specified node first | Any available | Soft preference |
| `Interleave` | Round-robin across nodes | N/A | High-bandwidth sequential |

## Step-by-Step Walkthrough

### 1. Local Policy

```rust
let region = NumaRegion::anon(
    size,
    MemPolicy::Local,
    Default::default(),
    Prefault::Touch
)?;
```

Memory is allocated on whichever node the calling thread is running on. This is the most common policy for thread-local data.

### 2. Bind Policy

```rust
let node0 = NodeMask::single(NodeId::new(0));
let region = NumaRegion::anon(
    size,
    MemPolicy::Bind(node0),
    Default::default(),
    Prefault::Touch
)?;
```

Memory is strictly allocated on the specified nodes. If those nodes are out of memory, allocation fails rather than falling back.

### 3. Preferred Policy

```rust
let region = NumaRegion::anon(
    size,
    MemPolicy::Preferred(NodeId::new(0)),
    Default::default(),
    Prefault::Touch
)?;
```

Memory is preferentially allocated on the specified node, but the kernel may use other nodes if necessary.

### 4. Interleave Policy

```rust
let mut all_nodes = NodeMask::new();
for node in topo.numa_nodes() {
    all_nodes.add(node.id());
}

let region = NumaRegion::anon(
    size,
    MemPolicy::Interleave(all_nodes),
    Default::default(),
    Prefault::Touch
)?;
```

Pages are distributed round-robin across all specified nodes. This maximizes aggregate memory bandwidth for large sequential accesses.

## Prefault Options

The `Prefault` parameter controls when physical pages are allocated:

| Option | Behavior |
|--------|----------|
| `Prefault::None` | Lazy allocation on first access |
| `Prefault::Touch` | Touch pages immediately after mapping |
| `Prefault::Populate` | Use `MAP_POPULATE` to fault all pages |

Use `Prefault::Touch` or `Prefault::Populate` when you need deterministic placement and want to avoid page faults during critical operations.

## Working with Regions

### Read and Write

```rust
let mut region = NumaRegion::anon(size, policy, ...)?;

// Write
let slice = region.as_mut_slice();
slice[0] = 42;

// Read
let slice = region.as_slice();
println!("First byte: {}", slice[0]);
```

### Check Enforcement

```rust
println!("Enforcement: {:?}", region.enforcement());
// Soft - kernel may have migrated pages
// Hard - strict enforcement guaranteed
```

### RAII Cleanup

```rust
{
    let region = NumaRegion::anon(...)?;
    // Use region
} // Automatically unmapped when dropped
```

## Sample Output

```
=== numaperf: Memory Placement Example ===

System has 2 NUMA nodes

1. Local Policy
   Allocates on the current thread's NUMA node.
   Allocated 1048576 bytes with Local policy
   Enforcement: Soft

2. Bind Policy
   Strictly allocates on specified nodes only.
   Allocated 1048576 bytes bound to node 0
   Enforcement: Soft

3. Preferred Policy
   Prefers the specified node but allows fallback.
   Allocated 1048576 bytes with preference for node 0
   Enforcement: Soft

4. Interleave Policy
   Round-robins pages across multiple nodes.
   Allocated 1048576 bytes interleaved across 2 nodes
   Enforcement: Soft

Writing to allocated regions...
   Written 1048576 bytes, checksum: 133169152

Memory placement example complete.
```

## When to Use Each Policy

### Local (Default)

- Thread-local data structures
- Per-worker buffers
- Data that's only accessed by one thread

### Bind

- Data with strict placement requirements
- When you need guaranteed locality
- Shared data that multiple threads on one node access

### Preferred

- Soft preference with graceful degradation
- When availability is more important than locality

### Interleave

- Large arrays accessed sequentially
- Read-mostly data accessed from all nodes
- Maximizing aggregate memory bandwidth

## Next Steps

- [Buffer Pool Example](buffer-pool.md) - Per-node buffer management
- [Worker Pool Example](worker-pool.md) - Execute tasks on specific nodes
