# Memory API

Types for NUMA-aware memory allocation.

## NumaRegion

A NUMA-aware memory region. Automatically unmapped when dropped.

```rust
pub struct NumaRegion { /* internal */ }
```

### Construction

```rust
use numaperf::{NumaRegion, MemPolicy, Prefault, HardMode};

// Basic allocation
let region = NumaRegion::anon(
    1024 * 1024,        // size
    MemPolicy::Local,   // policy
    Default::default(), // huge pages
    Prefault::Touch,    // prefault
)?;

// With hard mode
let region = NumaRegion::anon_with_mode(
    size,
    policy,
    Default::default(),
    Prefault::Touch,
    HardMode::Strict,
)?;
```

### Methods

| Method | Description |
|--------|-------------|
| `anon(size, policy, huge, prefault) -> Result<Self, NumaError>` | Allocate with soft mode |
| `anon_with_mode(..., mode) -> Result<Self, NumaError>` | Allocate with explicit mode |
| `as_ptr(&self) -> *const u8` | Get raw pointer |
| `as_mut_ptr(&mut self) -> *mut u8` | Get mutable pointer |
| `as_slice(&self) -> &[u8]` | Get slice |
| `as_mut_slice(&mut self) -> &mut [u8]` | Get mutable slice |
| `len(&self) -> usize` | Size in bytes |
| `enforcement(&self) -> EnforcementLevel` | Get enforcement level |

### Example

```rust
let mut region = NumaRegion::anon(
    1024 * 1024,
    MemPolicy::Bind(NodeMask::single(NodeId::new(0))),
    Default::default(),
    Prefault::Touch,
)?;

// Check enforcement
println!("Enforcement: {:?}", region.enforcement());

// Use the memory
let slice = region.as_mut_slice();
slice[0] = 42;
```

---

## MemPolicy

Memory placement policy.

```rust
pub enum MemPolicy {
    /// Allocate on current thread's node
    Local,
    /// Strictly allocate on specified nodes
    Bind(NodeMask),
    /// Prefer specified node, allow fallback
    Preferred(NodeId),
    /// Round-robin across specified nodes
    Interleave(NodeMask),
}
```

### Usage

```rust
use numaperf::{MemPolicy, NodeMask, NodeId};

// Local (default)
let policy = MemPolicy::Local;

// Bind to node 0
let policy = MemPolicy::Bind(NodeMask::single(NodeId::new(0)));

// Prefer node 0
let policy = MemPolicy::Preferred(NodeId::new(0));

// Interleave across all nodes
let mut all = NodeMask::new();
all.add(NodeId::new(0));
all.add(NodeId::new(1));
let policy = MemPolicy::Interleave(all);
```

### Methods

| Method | Description |
|--------|-------------|
| `name(&self) -> &'static str` | Get policy name |
| `nodes(&self) -> Option<NodeMask>` | Get associated nodes |

---

## HugePageMode

Huge page configuration.

```rust
pub enum HugePageMode {
    /// No huge pages
    None,
    /// Transparent huge pages (system default)
    Transparent,
    /// Explicit 2MB huge pages
    Explicit2MB,
    /// Explicit 1GB huge pages
    Explicit1GB,
}
```

### Usage

```rust
let region = NumaRegion::anon(
    size,
    policy,
    HugePageMode::Transparent,  // Use THP
    Prefault::Touch,
)?;
```

---

## Prefault

Page fault strategy.

```rust
pub enum Prefault {
    /// Allocate pages on first access (lazy)
    None,
    /// Fault in all pages immediately (eager)
    Touch,
}
```

### Comparison

| Strategy | Allocation Speed | First Access | Memory Usage |
|----------|------------------|--------------|--------------|
| `None` | Fast | Slow (page fault) | Only used pages |
| `Touch` | Slow | Fast | Full allocation |

### Recommendation

- Use `Touch` for hot data and when using `Bind` policy
- Use `None` for large sparse allocations

---

## Thread Safety

`NumaRegion` is `Send + Sync`:

- Can be moved between threads
- Can be shared via `Arc<NumaRegion>`
- Multiple threads can read concurrently
- Mutable access requires synchronization
