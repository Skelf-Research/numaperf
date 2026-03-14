# Core Types

Fundamental types used throughout numaperf.

## NodeId

A NUMA node identifier.

```rust
pub struct NodeId(u32);
```

### Construction

```rust
use numaperf::NodeId;

let node = NodeId::new(0);  // Node 0
```

### Methods

| Method | Description |
|--------|-------------|
| `new(id: u32) -> Self` | Create from raw value |
| `as_u32(self) -> u32` | Get raw value |

### Traits

- `Copy`, `Clone`, `Debug`, `Display`
- `PartialEq`, `Eq`, `Hash`
- `PartialOrd`, `Ord`

---

## CpuSet

An efficient bitmap representing a set of CPUs (up to 1024).

```rust
pub struct CpuSet { /* internal bitmap */ }
```

### Construction

```rust
use numaperf::CpuSet;

// Empty set
let cpus = CpuSet::new();

// Single CPU
let cpus = CpuSet::single(0);

// Parse from string
let cpus = CpuSet::parse("0-3")?;      // 0, 1, 2, 3
let cpus = CpuSet::parse("0,2,4")?;    // 0, 2, 4
let cpus = CpuSet::parse("0-3,8-11")?; // 0-3 and 8-11
```

### Methods

| Method | Description |
|--------|-------------|
| `new() -> Self` | Create empty set |
| `single(cpu: u32) -> Self` | Create with one CPU |
| `parse(s: &str) -> Result<Self, ParseCpuSetError>` | Parse Linux format |
| `add(&mut self, cpu: u32)` | Add a CPU |
| `remove(&mut self, cpu: u32)` | Remove a CPU |
| `contains(&self, cpu: u32) -> bool` | Check membership |
| `is_empty(&self) -> bool` | Check if empty |
| `iter(&self) -> impl Iterator<Item = u32>` | Iterate CPUs |
| `union(&self, other: &Self) -> Self` | Set union |
| `intersection(&self, other: &Self) -> Self` | Set intersection |

### Display

Displays in Linux CPU list format: `"0-3,8-11"`

---

## NodeMask

A set of NUMA node identifiers.

```rust
pub struct NodeMask { /* internal bitmap */ }
```

### Construction

```rust
use numaperf::{NodeMask, NodeId};

// Empty mask
let nodes = NodeMask::new();

// Single node
let nodes = NodeMask::single(NodeId::new(0));

// Build manually
let mut nodes = NodeMask::new();
nodes.add(NodeId::new(0));
nodes.add(NodeId::new(1));
```

### Methods

| Method | Description |
|--------|-------------|
| `new() -> Self` | Create empty mask |
| `single(node: NodeId) -> Self` | Create with one node |
| `add(&mut self, node: NodeId)` | Add a node |
| `remove(&mut self, node: NodeId)` | Remove a node |
| `contains(&self, node: NodeId) -> bool` | Check membership |
| `is_empty(&self) -> bool` | Check if empty |
| `iter(&self) -> impl Iterator<Item = NodeId>` | Iterate nodes |

---

## HardMode

Enforcement mode for NUMA operations.

```rust
pub enum HardMode {
    /// Best-effort, graceful degradation
    Soft,
    /// Strict enforcement, fail if impossible
    Strict,
}
```

### Usage

```rust
use numaperf::HardMode;

// Soft mode (default) - always succeeds
let mode = HardMode::Soft;

// Hard mode - fails if guarantees can't be met
let mode = HardMode::Strict;
```

### Methods

| Method | Description |
|--------|-------------|
| `is_soft(&self) -> bool` | Check if soft mode |
| `is_strict(&self) -> bool` | Check if strict mode |

---

## EnforcementLevel

Reports actual enforcement achieved by an operation.

```rust
pub enum EnforcementLevel {
    /// Policy fully enforced
    Strict,
    /// Applied but not guaranteed
    BestEffort { reason: String },
    /// No NUMA policy applied
    None { reason: String },
}
```

### Usage

```rust
use numaperf::EnforcementLevel;

let region = NumaRegion::anon(...)?;

match region.enforcement() {
    EnforcementLevel::Strict => {
        println!("Guaranteed placement");
    }
    EnforcementLevel::BestEffort { reason } => {
        println!("Best effort: {}", reason);
    }
    EnforcementLevel::None { reason } => {
        println!("No enforcement: {}", reason);
    }
}
```

### Methods

| Method | Description |
|--------|-------------|
| `is_strict(&self) -> bool` | Check if strictly enforced |
| `is_best_effort(&self) -> bool` | Check if best-effort |
| `is_none(&self) -> bool` | Check if no enforcement |
| `reason(&self) -> Option<&str>` | Get reason if not strict |

---

## Capabilities

Detected system capabilities for NUMA operations.

```rust
pub struct Capabilities {
    pub strict_memory_binding: bool,
    pub strict_cpu_affinity: bool,
    pub memory_locking: bool,
    pub numa_balancing_disabled: bool,
    pub numa_node_count: usize,
}
```

### Usage

```rust
use numaperf::Capabilities;

let caps = Capabilities::detect();

println!("NUMA nodes: {}", caps.numa_node_count);
println!("Hard mode: {}", caps.supports_hard_mode());
```

### Methods

| Method | Description |
|--------|-------------|
| `detect() -> Self` | Detect current capabilities |
| `supports_hard_mode(&self) -> bool` | Check if hard mode available |
| `missing_for_hard_mode(&self) -> Vec<&str>` | List missing capabilities |
| `is_numa_system(&self) -> bool` | Check if multi-node system |
| `summary(&self) -> String` | Get human-readable summary |

### Fields

| Field | Description |
|-------|-------------|
| `strict_memory_binding` | Has CAP_SYS_ADMIN |
| `strict_cpu_affinity` | Has CAP_SYS_NICE |
| `memory_locking` | Has CAP_IPC_LOCK |
| `numa_balancing_disabled` | Kernel NUMA balancing off |
| `numa_node_count` | Number of NUMA nodes |
