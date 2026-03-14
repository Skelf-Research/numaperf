# Thread Pinning

Learn how to pin threads to specific CPUs for NUMA locality.

## Why Pin Threads?

Without pinning, the OS scheduler can move threads between CPUs:

- Thread migrates to different NUMA node
- Memory that was local becomes remote
- Performance becomes unpredictable

## Basic Pinning

```rust
use numaperf::{ScopedPin, CpuSet};

fn main() -> Result<(), numaperf::NumaError> {
    let cpus = CpuSet::parse("0-3")?;  // CPUs 0, 1, 2, 3

    {
        let _pin = ScopedPin::pin_current(cpus)?;
        // Thread is now restricted to CPUs 0-3

        // Do work here...

    } // Pin automatically restored when dropped

    Ok(())
}
```

## CPU Set Syntax

```rust
// Single CPU
let cpus = CpuSet::single(0);

// Parse from string
let cpus = CpuSet::parse("0")?;        // Just CPU 0
let cpus = CpuSet::parse("0-3")?;      // CPUs 0, 1, 2, 3
let cpus = CpuSet::parse("0,2,4")?;    // CPUs 0, 2, 4
let cpus = CpuSet::parse("0-3,8-11")?; // CPUs 0-3 and 8-11

// From topology
let node0_cpus = topo.cpu_set(NodeId::new(0));
```

## Pin to NUMA Node

```rust
use numaperf::{ScopedPin, Topology, NodeId};

let topo = Topology::discover()?;

// Pin to all CPUs on node 0
let node0_cpus = topo.cpu_set(NodeId::new(0));
let _pin = ScopedPin::pin_current(node0_cpus)?;
```

## Pin to Single CPU

```rust
let _pin = ScopedPin::pin_to_cpu(0)?;  // Pin to CPU 0 only
```

## Check Current Affinity

```rust
use numaperf::get_affinity;

let current = get_affinity()?;
println!("Current affinity: {}", current);
println!("CPU count: {}", current.iter().count());
```

## The Pin-Then-Allocate Pattern

Memory is allocated on the current thread's node:

```rust
use numaperf::{ScopedPin, Topology};

let topo = Topology::discover()?;

for node in topo.numa_nodes() {
    // 1. Pin to this node's CPUs
    let _pin = ScopedPin::pin_current(node.cpus().clone())?;

    // 2. Allocate - will be local to this node
    let data: Vec<u8> = vec![0; 1024 * 1024];

    // 3. Use data while pinned
    process_data(&data);
}
```

## Hard Mode Pinning

For guaranteed pinning:

```rust
use numaperf::{ScopedPin, HardMode, CpuSet};

let cpus = CpuSet::parse("0-3")?;

// Fails if pinning cannot be guaranteed
let _pin = ScopedPin::pin_current_with_mode(cpus, HardMode::Strict)?;
```

## Worker Thread Pattern

Pin worker threads at spawn time:

```rust
use std::thread;

let topo = Arc::new(Topology::discover()?);

for node in topo.numa_nodes() {
    let cpus = node.cpus().clone();

    thread::spawn(move || {
        // Pin immediately after spawn
        let _pin = ScopedPin::pin_current(cpus).unwrap();

        // Worker loop - always runs on this node
        loop {
            // Process work...
        }
    });
}
```

## Important Notes

### ScopedPin is !Send

`ScopedPin` cannot be sent between threads:

```rust
let pin = ScopedPin::pin_current(cpus)?;

// This won't compile!
thread::spawn(move || {
    drop(pin);  // Would restore wrong thread's affinity
});
```

### Nested Pinning

Pins can be nested - each restores to its previous state:

```rust
let cpus_broad = CpuSet::parse("0-7")?;
let cpus_narrow = CpuSet::parse("0-1")?;

let _outer = ScopedPin::pin_current(cpus_broad)?;
// Pinned to 0-7

{
    let _inner = ScopedPin::pin_current(cpus_narrow)?;
    // Pinned to 0-1
}
// Back to 0-7

// Back to original affinity
```

## Best Practices

1. **Pin early** - Pin before allocating memory
2. **Use RAII** - Let `ScopedPin` handle restoration
3. **Pin to nodes** - Use `topo.cpu_set(node_id)` for node-level pinning
4. **Consider hard mode** for production workloads
5. **Don't hold pins across await points** in async code
