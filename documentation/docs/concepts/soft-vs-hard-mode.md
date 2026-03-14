# Soft vs Hard Mode

numaperf supports two enforcement modes that control how strictly locality guarantees are applied.

## Overview

| Mode | Behavior | Use Case |
|------|----------|----------|
| **Soft** (default) | Best-effort, graceful degradation | Development, general use |
| **Hard** | Strict enforcement, fail if impossible | Production, latency-critical |

## Soft Mode (Default)

In soft mode, numaperf applies NUMA optimizations on a **best-effort basis**:

- Operations succeed even when optimal placement isn't possible
- Degraded enforcement is reported via `EnforcementLevel`
- No special privileges required

```rust
use numaperf::{NumaRegion, MemPolicy, NodeMask, NodeId, Prefault};

// Soft mode (default) - always succeeds
let region = NumaRegion::anon(
    1024 * 1024,
    MemPolicy::Bind(NodeMask::single(NodeId::new(0))),
    Default::default(),
    Prefault::Touch,
)?;

// Check what we actually got
match region.enforcement() {
    EnforcementLevel::Strict => println!("Memory bound to node 0"),
    EnforcementLevel::BestEffort { reason } => {
        println!("Best effort: {}", reason);
        // Continue anyway - memory is allocated, just not strictly bound
    }
    EnforcementLevel::None { reason } => {
        println!("No binding: {}", reason);
    }
}
```

### When to Use Soft Mode

- **Development** - Don't need root/capabilities to develop
- **Portable applications** - Run on any system
- **Non-critical workloads** - Performance nice-to-have, not required
- **Gradual optimization** - Add NUMA awareness incrementally

## Hard Mode

In hard mode, operations **fail** if locality guarantees cannot be enforced:

- Strict enforcement or error
- Requires system capabilities
- Predictable performance characteristics

```rust
use numaperf::{NumaRegion, MemPolicy, NodeMask, NodeId, Prefault, HardMode};

// Hard mode - fails if binding not possible
let region = NumaRegion::anon_with_mode(
    1024 * 1024,
    MemPolicy::Bind(NodeMask::single(NodeId::new(0))),
    Default::default(),
    Prefault::Touch,
    HardMode::Strict,
)?;

// If we get here, memory is definitely on node 0
assert!(region.enforcement().is_strict());
```

### When to Use Hard Mode

- **Production** - Guaranteed performance characteristics
- **Latency-critical** - Every nanosecond matters
- **Benchmarking** - Reproducible results
- **Compliance** - Must prove locality guarantees

## Checking Hard Mode Support

Before using hard mode, check if your system supports it:

```rust
use numaperf::Capabilities;

let caps = Capabilities::detect();

if caps.supports_hard_mode() {
    println!("Hard mode available!");
} else {
    println!("Missing capabilities:");
    for cap in caps.missing_for_hard_mode() {
        println!("  - {}", cap);
    }
}
```

### Requirements for Hard Mode

| Requirement | Purpose | How to Enable |
|------------|---------|---------------|
| `CAP_SYS_ADMIN` | Strict memory binding (MPOL_BIND) | Root or `setcap` |
| `CAP_SYS_NICE` | Guaranteed CPU affinity | Root or `setcap` |
| NUMA balancing disabled | Prevent page migration | `sysctl kernel.numa_balancing=0` |

## Enabling Hard Mode

### Option 1: Run as Root

```bash
sudo ./your-application
```

### Option 2: Linux Capabilities

```bash
# Add capabilities to binary
sudo setcap cap_sys_admin,cap_sys_nice,cap_ipc_lock+ep ./your-application

# Verify
getcap ./your-application
```

### Option 3: Docker

```dockerfile
FROM rust:latest
# ... build your app ...
```

```bash
docker run --cap-add SYS_ADMIN --cap-add SYS_NICE --cap-add IPC_LOCK your-image
```

### Option 4: systemd Service

```ini
[Service]
ExecStart=/path/to/your-application
AmbientCapabilities=CAP_SYS_ADMIN CAP_SYS_NICE CAP_IPC_LOCK
```

### Disable NUMA Balancing

```bash
# Temporary (until reboot)
echo 0 | sudo tee /proc/sys/kernel/numa_balancing

# Permanent
echo "kernel.numa_balancing = 0" | sudo tee -a /etc/sysctl.conf
sudo sysctl -p
```

## EnforcementLevel

Operations report their actual enforcement:

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

### Checking Enforcement

```rust
use numaperf::EnforcementLevel;

let region = NumaRegion::anon(...)?;

if region.enforcement().is_strict() {
    // Guaranteed placement
} else if region.enforcement().is_best_effort() {
    // Probably placed correctly, but not guaranteed
} else {
    // No NUMA policy applied
}

// Get the reason if not strict
if let Some(reason) = region.enforcement().reason() {
    println!("Degraded: {}", reason);
}
```

## Hard Mode with Executor

The executor also supports hard mode for worker pinning:

```rust
use numaperf::{NumaExecutor, HardMode, StealPolicy};

let exec = NumaExecutor::builder(topo)
    .hard_mode(HardMode::Strict)  // Workers must be pinned
    .steal_policy(StealPolicy::LocalOnly)
    .workers_per_node(2)
    .build()?;  // Fails if pinning not possible
```

## Mixed Mode Strategy

You can use soft mode for most operations and hard mode for critical ones:

```rust
use numaperf::{NumaRegion, HardMode, MemPolicy};

// Non-critical buffer - soft mode is fine
let scratch = NumaRegion::anon(
    buffer_size,
    MemPolicy::Local,
    Default::default(),
    Prefault::None,
)?;

// Critical hot data - must be on specific node
let hot_data = NumaRegion::anon_with_mode(
    data_size,
    MemPolicy::Bind(target_nodes),
    Default::default(),
    Prefault::Touch,
    HardMode::Strict,
)?;
```

## Error Handling

Hard mode failures return specific errors:

```rust
use numaperf::NumaError;

match NumaRegion::anon_with_mode(..., HardMode::Strict) {
    Ok(region) => {
        // Guaranteed placement
    }
    Err(NumaError::HardModeUnavailable { operation, reason }) => {
        // Hard mode not possible
        eprintln!("Cannot bind memory: {}", reason);

        // Options:
        // 1. Fall back to soft mode
        // 2. Exit with error
        // 3. Try alternative placement
    }
    Err(e) => {
        // Other error (I/O, invalid parameters, etc.)
        return Err(e);
    }
}
```

## Best Practices

1. **Start with soft mode** during development
2. **Check capabilities** at startup, not per-operation
3. **Use hard mode selectively** for critical data paths
4. **Log enforcement levels** to track degradation
5. **Test both modes** in CI with appropriate capabilities

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let caps = Capabilities::detect();

    // Decide mode based on capabilities and requirements
    let mode = if caps.supports_hard_mode() && std::env::var("REQUIRE_HARD_MODE").is_ok() {
        HardMode::Strict
    } else {
        HardMode::Soft
    };

    // Use chosen mode throughout
    let exec = NumaExecutor::builder(topo)
        .hard_mode(mode)
        .build()?;

    Ok(())
}
```

## Next Steps

- [Hard Mode Guide](../advanced/hard-mode.md) - Detailed configuration
- [Troubleshooting](../advanced/troubleshooting.md) - Common issues
- [Memory Policies](memory-policies.md) - Placement options
