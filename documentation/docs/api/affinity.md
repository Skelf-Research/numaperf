# Affinity API

Types and functions for thread CPU affinity management.

## ScopedPin

RAII guard for thread CPU affinity. Restores previous affinity when dropped.

```rust
pub struct ScopedPin { /* internal */ }
```

!!! warning "Thread-Local"
    `ScopedPin` is `!Send` and `!Sync`. It can only be used on the thread that created it.

### Construction

```rust
use numaperf::{ScopedPin, CpuSet};

// Pin to CPU set
let cpus = CpuSet::parse("0-3")?;
let _pin = ScopedPin::pin_current(cpus)?;

// Pin to single CPU
let _pin = ScopedPin::pin_to_cpu(0)?;

// With hard mode
let _pin = ScopedPin::pin_current_with_mode(cpus, HardMode::Strict)?;
let _pin = ScopedPin::pin_to_cpu_with_mode(0, HardMode::Strict)?;
```

### Methods

| Method | Description |
|--------|-------------|
| `pin_current(cpus: CpuSet) -> Result<Self, NumaError>` | Pin to CPU set |
| `pin_to_cpu(cpu: u32) -> Result<Self, NumaError>` | Pin to single CPU |
| `pin_current_with_mode(cpus, mode) -> Result<Self, NumaError>` | Pin with mode |
| `pin_to_cpu_with_mode(cpu, mode) -> Result<Self, NumaError>` | Pin CPU with mode |
| `current_cpus(&self) -> Result<CpuSet, NumaError>` | Get current affinity |
| `previous_cpus(&self) -> &CpuSet` | Get previous affinity |
| `restore(&self) -> Result<(), NumaError>` | Manually restore (also on drop) |

### Example

```rust
use numaperf::{ScopedPin, CpuSet, Topology};

let topo = Topology::discover()?;

// Pin to node 0's CPUs
let node0_cpus = topo.cpu_set(NodeId::new(0));
{
    let _pin = ScopedPin::pin_current(node0_cpus)?;

    // Thread is pinned here
    do_work();

} // Affinity automatically restored
```

### Nested Pinning

Pins can be nested:

```rust
let _outer = ScopedPin::pin_current(broad_cpus)?;
{
    let _inner = ScopedPin::pin_current(narrow_cpus)?;
    // Pinned to narrow_cpus
}
// Back to broad_cpus
```

---

## get_affinity

Query the current thread's CPU affinity.

```rust
pub fn get_affinity() -> Result<CpuSet, NumaError>
```

### Example

```rust
use numaperf::get_affinity;

let current = get_affinity()?;
println!("Allowed CPUs: {}", current);
println!("Count: {}", current.iter().count());
```

---

## set_affinity

Set the current thread's CPU affinity (low-level).

```rust
pub fn set_affinity(cpus: &CpuSet) -> Result<(), NumaError>
```

!!! note
    Prefer `ScopedPin` for automatic restoration.

### Example

```rust
use numaperf::{set_affinity, get_affinity, CpuSet};

let original = get_affinity()?;

// Set new affinity
set_affinity(&CpuSet::parse("0-3")?)?;

// Do work...

// Manually restore
set_affinity(&original)?;
```

---

## Error Handling

Affinity operations can fail with:

| Error | Cause |
|-------|-------|
| `NumaError::IoError` | System call failed |
| `NumaError::EmptyCpuSet` | Empty CPU set provided |
| `NumaError::InvalidCpuId` | CPU doesn't exist |
| `NumaError::HardModeUnavailable` | Hard mode pinning failed |

### Example

```rust
match ScopedPin::pin_current(cpus) {
    Ok(pin) => { /* pinned */ }
    Err(NumaError::EmptyCpuSet) => {
        panic!("Bug: empty CPU set");
    }
    Err(NumaError::HardModeUnavailable { reason, .. }) => {
        eprintln!("Cannot pin: {}", reason);
    }
    Err(e) => return Err(e),
}
```
