# Memory Allocation

Learn how to allocate NUMA-aware memory with explicit placement policies.

## Basic Allocation

```rust
use numaperf::{NumaRegion, MemPolicy, Prefault};

// Allocate 1MB with local policy
let mut region = NumaRegion::anon(
    1024 * 1024,        // Size in bytes
    MemPolicy::Local,   // Policy
    Default::default(), // Huge pages (default = none)
    Prefault::Touch,    // Fault in pages immediately
)?;

// Access the memory
let slice = region.as_mut_slice();
slice[0] = 42;
```

## Memory Policies

### Local (Default)

Allocates on current thread's node:

```rust
let region = NumaRegion::anon(
    size,
    MemPolicy::Local,
    Default::default(),
    Prefault::Touch,
)?;
```

### Bind

Strictly allocates on specified nodes:

```rust
use numaperf::{NodeMask, NodeId};

// Bind to node 0
let nodes = NodeMask::single(NodeId::new(0));
let region = NumaRegion::anon(
    size,
    MemPolicy::Bind(nodes),
    Default::default(),
    Prefault::Touch,
)?;
```

### Preferred

Prefers specified node, allows fallback:

```rust
let region = NumaRegion::anon(
    size,
    MemPolicy::Preferred(NodeId::new(0)),
    Default::default(),
    Prefault::Touch,
)?;
```

### Interleave

Round-robin across multiple nodes:

```rust
let mut all_nodes = NodeMask::new();
for node in topo.numa_nodes() {
    all_nodes.add(node.id());
}

let region = NumaRegion::anon(
    size,
    MemPolicy::Interleave(all_nodes),
    Default::default(),
    Prefault::Touch,
)?;
```

## Prefault Options

### None (Lazy)

Pages allocated on first access:

```rust
let region = NumaRegion::anon(
    size,
    policy,
    Default::default(),
    Prefault::None,  // Fast allocation
)?;
// Memory allocated lazily on access
```

### Touch (Eager)

All pages allocated immediately:

```rust
let region = NumaRegion::anon(
    size,
    policy,
    Default::default(),
    Prefault::Touch,  // Slower but predictable
)?;
// All memory already allocated
```

**Recommendation**: Use `Touch` for hot data to ensure placement.

## Checking Enforcement

```rust
use numaperf::EnforcementLevel;

let region = NumaRegion::anon(...)?;

match region.enforcement() {
    EnforcementLevel::Strict => {
        println!("Memory bound to specified nodes");
    }
    EnforcementLevel::BestEffort { reason } => {
        println!("Best effort: {}", reason);
    }
    EnforcementLevel::None { reason } => {
        println!("No NUMA policy: {}", reason);
    }
}
```

## Hard Mode Allocation

```rust
use numaperf::HardMode;

// Fails if binding not possible
let region = NumaRegion::anon_with_mode(
    size,
    MemPolicy::Bind(nodes),
    Default::default(),
    Prefault::Touch,
    HardMode::Strict,
)?;
```

## Access Patterns

### Read-Only Access

```rust
let region = NumaRegion::anon(...)?;
let slice = region.as_slice();  // &[u8]

for byte in slice {
    // Read...
}
```

### Mutable Access

```rust
let mut region = NumaRegion::anon(...)?;
let slice = region.as_mut_slice();  // &mut [u8]

slice[0] = 42;
```

### Typed Access

```rust
use std::mem;

let mut region = NumaRegion::anon(
    count * mem::size_of::<u64>(),
    policy,
    Default::default(),
    Prefault::Touch,
)?;

// Cast to typed slice (unsafe)
let data: &mut [u64] = unsafe {
    std::slice::from_raw_parts_mut(
        region.as_mut_ptr() as *mut u64,
        count
    )
};
```

## Common Patterns

### Per-Node Buffers

```rust
let topo = Arc::new(Topology::discover()?);

let buffers: Vec<NumaRegion> = topo.numa_nodes()
    .iter()
    .map(|node| {
        NumaRegion::anon(
            buffer_size,
            MemPolicy::Bind(NodeMask::single(node.id())),
            Default::default(),
            Prefault::Touch,
        )
    })
    .collect::<Result<_, _>>()?;
```

### Large Shared Buffer

```rust
// Interleave for bandwidth
let mut all_nodes = NodeMask::new();
for node in topo.numa_nodes() {
    all_nodes.add(node.id());
}

let shared_buffer = NumaRegion::anon(
    large_size,
    MemPolicy::Interleave(all_nodes),
    Default::default(),
    Prefault::Touch,
)?;
```

### Pin-Then-Allocate

```rust
use numaperf::ScopedPin;

for node in topo.numa_nodes() {
    // Pin to node
    let _pin = ScopedPin::pin_current(node.cpus().clone())?;

    // Allocate local memory
    let local_data = NumaRegion::anon(
        size,
        MemPolicy::Local,
        Default::default(),
        Prefault::Touch,
    )?;

    // Use while pinned
    process(&local_data);
}
```

## Memory Lifetime

`NumaRegion` owns its memory and unmaps on drop:

```rust
{
    let region = NumaRegion::anon(...)?;
    // Memory is mapped
}
// Memory is automatically unmapped
```

## Best Practices

1. **Use Prefault::Touch** for hot data
2. **Check enforcement** in debug builds
3. **Use Bind** for critical data, **Local** for general use
4. **Combine with pinning** for best locality
5. **Use Interleave** for bandwidth-bound workloads
