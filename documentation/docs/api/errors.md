# Errors

The `NumaError` type and error handling patterns.

## NumaError

Unified error type for all numaperf operations.

```rust
pub enum NumaError {
    // System errors
    IoError(std::io::Error),

    // Invalid parameters
    InvalidNodeId(u32),
    InvalidCpuId(u32),
    EmptyCpuSet,
    EmptyNodeMask,

    // Capability errors
    NotSupported(String),
    HardModeUnavailable { operation: String, reason: String },

    // Runtime errors
    TopologyMismatch,
    WorkerPanic,
}
```

## Variants

### IoError

Wraps a standard I/O error from system calls.

```rust
match error {
    NumaError::IoError(io_err) => {
        eprintln!("System error: {}", io_err);
    }
    // ...
}
```

### InvalidNodeId

NUMA node ID doesn't exist on this system.

```rust
NumaError::InvalidNodeId(node_id)
```

### InvalidCpuId

CPU ID doesn't exist on this system.

```rust
NumaError::InvalidCpuId(cpu_id)
```

### EmptyCpuSet

Operation requires a non-empty CPU set.

```rust
NumaError::EmptyCpuSet
```

### EmptyNodeMask

Operation requires a non-empty node mask.

```rust
NumaError::EmptyNodeMask
```

### NotSupported

Feature not supported on this platform.

```rust
NumaError::NotSupported(description)
```

### HardModeUnavailable

Hard mode requested but cannot be enforced.

```rust
NumaError::HardModeUnavailable {
    operation: String,
    reason: String,
}
```

### TopologyMismatch

Topology doesn't match expected configuration.

```rust
NumaError::TopologyMismatch
```

### WorkerPanic

A worker thread panicked during execution.

```rust
NumaError::WorkerPanic
```

## Error Handling Patterns

### Basic Pattern

```rust
use numaperf::{Topology, NumaError};

fn main() -> Result<(), NumaError> {
    let topo = Topology::discover()?;
    Ok(())
}
```

### Match on Specific Errors

```rust
match NumaRegion::anon_with_mode(..., HardMode::Strict) {
    Ok(region) => {
        // Success
    }
    Err(NumaError::HardModeUnavailable { operation, reason }) => {
        eprintln!("Cannot enforce {}: {}", operation, reason);
        // Fall back to soft mode
    }
    Err(NumaError::IoError(e)) if e.kind() == std::io::ErrorKind::PermissionDenied => {
        eprintln!("Permission denied - try running as root");
    }
    Err(e) => {
        return Err(e);
    }
}
```

### Graceful Degradation

```rust
let region = match NumaRegion::anon_with_mode(..., HardMode::Strict) {
    Ok(r) => r,
    Err(NumaError::HardModeUnavailable { .. }) => {
        // Fall back to soft mode
        NumaRegion::anon(...)?
    }
    Err(e) => return Err(e),
};
```

## Display

All error variants implement `Display`:

```rust
let error: NumaError = ...;
println!("Error: {}", error);
// Output: "hard mode unavailable for memory binding: permission denied"
```

## Traits

`NumaError` implements:

- `std::error::Error`
- `Display`
- `Debug`
- `Send + Sync`

## Converting from io::Error

```rust
impl From<std::io::Error> for NumaError {
    fn from(e: std::io::Error) -> Self {
        NumaError::IoError(e)
    }
}
```

This allows using `?` with I/O operations in functions returning `Result<T, NumaError>`.
