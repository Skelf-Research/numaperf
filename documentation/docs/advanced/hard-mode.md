# Hard Mode

Hard mode provides strict enforcement of NUMA locality guarantees. When enabled, operations fail if locality constraints cannot be guaranteed, rather than silently degrading.

## Why Hard Mode?

In soft mode (the default), numaperf makes best-effort attempts at locality but continues even when optimal placement isn't possible. This can lead to:

- **Silent performance degradation**: Cross-node memory access without warning
- **Unpredictable latencies**: Some requests hit local memory, others don't
- **Difficult debugging**: Hard to tell if performance issues are NUMA-related

Hard mode ensures that if your application starts successfully, you have guaranteed locality. If guarantees can't be met, you'll know immediately.

## Capability Requirements

Hard mode requires specific Linux capabilities and kernel settings.

### Required Capabilities

| Capability | Purpose | How to Get It |
|------------|---------|---------------|
| `CAP_SYS_ADMIN` | Strict memory binding with `MPOL_BIND` | Run as root, or use `setcap` |
| `CAP_SYS_NICE` | Strict CPU affinity with realtime scheduling | Run as root, or use `setcap` |

### Required Kernel Settings

| Setting | Purpose | How to Set |
|---------|---------|------------|
| `kernel.numa_balancing=0` | Prevent automatic page migration | `sysctl -w kernel.numa_balancing=0` |

## Checking Capabilities

Use `Capabilities::detect()` to check your system:

```rust
use numaperf::Capabilities;

fn main() {
    let caps = Capabilities::detect();

    println!("System Capabilities:");
    println!("  NUMA nodes: {}", caps.numa_node_count);
    println!("  Strict memory binding: {}", caps.strict_memory_binding);
    println!("  Strict CPU affinity: {}", caps.strict_cpu_affinity);
    println!("  Memory locking: {}", caps.memory_locking);
    println!("  NUMA balancing disabled: {}", caps.numa_balancing_disabled);

    if caps.supports_hard_mode() {
        println!("\nHard mode is SUPPORTED");
    } else {
        println!("\nHard mode is NOT supported. Missing:");
        for cap in caps.missing_for_hard_mode() {
            println!("  - {}", cap);
        }
    }
}
```

## Enabling Capabilities

### Option 1: Run as Root

The simplest approach for development and testing:

```bash
sudo cargo run --example worker_pool
```

### Option 2: Set Capabilities on Binary

For production, grant specific capabilities to your binary:

```bash
# Build the release binary
cargo build --release

# Grant required capabilities
sudo setcap 'cap_sys_admin,cap_sys_nice+ep' target/release/myapp

# Verify
getcap target/release/myapp
```

### Option 3: Container with Capabilities

When running in Docker/Podman:

```bash
docker run --cap-add SYS_ADMIN --cap-add SYS_NICE myimage
```

Or in a Dockerfile/compose file:

```yaml
services:
  myapp:
    cap_add:
      - SYS_ADMIN
      - SYS_NICE
```

### Option 4: Systemd Service

For systemd-managed services, use `AmbientCapabilities`:

```ini
[Service]
AmbientCapabilities=CAP_SYS_ADMIN CAP_SYS_NICE
```

## Disabling NUMA Balancing

Check current status:

```bash
cat /proc/sys/kernel/numa_balancing
# 0 = disabled, 1 = enabled
```

Disable temporarily:

```bash
sudo sysctl -w kernel.numa_balancing=0
```

Disable permanently (survives reboot):

```bash
echo "kernel.numa_balancing = 0" | sudo tee /etc/sysctl.d/99-numa.conf
sudo sysctl --system
```

## Using Hard Mode

### Executor with Hard Mode

```rust
use numaperf::{NumaExecutor, Topology, HardMode};
use std::sync::Arc;

fn main() -> Result<(), numaperf::NumaError> {
    let topo = Arc::new(Topology::discover()?);

    // Enable hard mode - will fail if workers can't be pinned
    let exec = NumaExecutor::builder(Arc::clone(&topo))
        .hard_mode(HardMode::Strict)
        .workers_per_node(2)
        .build()?;

    // If we get here, workers are guaranteed to be pinned
    exec.submit_to_node(topo.numa_nodes()[0].id(), || {
        println!("Running on pinned worker");
    });

    exec.shutdown();
    Ok(())
}
```

### Thread Pinning with Hard Mode

```rust
use numaperf::{ScopedPin, CpuSet, HardMode};

fn main() -> Result<(), numaperf::NumaError> {
    let cpus = CpuSet::parse("0-3").expect("valid CPU set");

    // Strict mode - fail if pinning can't be verified
    let _pin = ScopedPin::pin_current_with_mode(cpus, HardMode::Strict)?;

    // If we get here, we're guaranteed to be pinned to CPUs 0-3
    println!("Strictly pinned to CPUs 0-3");

    Ok(())
}
```

### Memory Allocation with Hard Mode

```rust
use numaperf::{NumaRegion, MemPolicy, NodeMask, NodeId, HardMode, Prefault};

fn main() -> Result<(), numaperf::NumaError> {
    let nodes = NodeMask::single(NodeId::new(0));

    // Strict mode - fail if binding can't be guaranteed
    let region = NumaRegion::anon_with_mode(
        1024 * 1024,
        MemPolicy::Bind(nodes),
        Default::default(),
        Prefault::Touch,
        HardMode::Strict,
    )?;

    // If we get here, memory is guaranteed to be on node 0
    println!("Allocated {} bytes strictly on node 0", region.len());

    Ok(())
}
```

## Error Handling

Hard mode failures return `NumaError::HardModeUnavailable`:

```rust
use numaperf::{NumaExecutor, Topology, HardMode, NumaError};
use std::sync::Arc;

fn main() {
    let topo = Arc::new(Topology::discover().expect("topology"));

    match NumaExecutor::builder(Arc::clone(&topo))
        .hard_mode(HardMode::Strict)
        .workers_per_node(2)
        .build()
    {
        Ok(exec) => {
            println!("Hard mode executor created successfully");
            exec.shutdown();
        }
        Err(NumaError::HardModeUnavailable { operation, reason }) => {
            eprintln!("Hard mode failed for {}: {}", operation, reason);
            eprintln!("Falling back to soft mode...");

            // Create soft mode executor instead
            let exec = NumaExecutor::builder(topo)
                .hard_mode(HardMode::Soft)
                .workers_per_node(2)
                .build()
                .expect("soft mode should succeed");
            exec.shutdown();
        }
        Err(e) => {
            eprintln!("Unexpected error: {}", e);
        }
    }
}
```

## Soft Mode Fallback Pattern

A common pattern is to try hard mode and fall back to soft mode:

```rust
use numaperf::{NumaExecutor, Topology, HardMode, Capabilities};
use std::sync::Arc;

fn create_executor(topo: Arc<Topology>) -> numaperf::NumaExecutor {
    let caps = Capabilities::detect();

    let mode = if caps.supports_hard_mode() {
        println!("Using hard mode");
        HardMode::Strict
    } else {
        println!("Hard mode unavailable, using soft mode");
        for cap in caps.missing_for_hard_mode() {
            println!("  Missing: {}", cap);
        }
        HardMode::Soft
    };

    NumaExecutor::builder(topo)
        .hard_mode(mode)
        .workers_per_node(2)
        .build()
        .expect("executor creation should succeed")
}
```

## Deployment Checklist

Before deploying with hard mode:

- [ ] Verify NUMA hardware is present (`lscpu | grep NUMA`)
- [ ] Check kernel NUMA support (`cat /proc/cmdline | grep numa`)
- [ ] Disable NUMA balancing (`cat /proc/sys/kernel/numa_balancing` should be 0)
- [ ] Grant required capabilities to the binary
- [ ] Test with `Capabilities::detect()` in your startup code
- [ ] Handle `HardModeUnavailable` errors gracefully
- [ ] Monitor locality metrics in production

## Troubleshooting

### "Hard mode unavailable: permission denied"

**Cause**: Missing capabilities

**Fix**: Run as root or grant capabilities with `setcap`

### "Hard mode unavailable: affinity not fully applied"

**Cause**: Requested CPUs are offline or isolated

**Fix**: Check available CPUs with `nproc` and CPU isolation settings

### "Hard mode unavailable: NUMA balancing is enabled"

**Cause**: Kernel is configured to automatically migrate pages

**Fix**: Disable with `sysctl -w kernel.numa_balancing=0`

### Performance worse with hard mode

**Cause**: Load imbalance when work stealing is restricted

**Fix**: Consider using `StealPolicy::LocalThenSocketThenRemote` instead of `LocalOnly`
