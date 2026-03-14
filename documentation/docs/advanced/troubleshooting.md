# Troubleshooting

Common issues and solutions when using numaperf.

## Diagnostic Commands

Start by gathering system information:

```bash
# NUMA topology
numactl --hardware
lscpu | grep -i numa

# Current process NUMA stats
numastat -p $(pgrep your_app)

# Memory policy of a process
cat /proc/$(pgrep your_app)/numa_maps

# Check kernel NUMA settings
cat /proc/sys/kernel/numa_balancing
cat /proc/sys/vm/zone_reclaim_mode
```

## Common Issues

### "NUMA not detected" on a NUMA system

**Symptoms**: `Topology::discover()` returns only one node on a multi-socket system.

**Causes**:

1. NUMA disabled in BIOS
2. Kernel booted with `numa=off`
3. Running in a VM without NUMA passthrough

**Solutions**:

```bash
# Check kernel command line
cat /proc/cmdline | grep numa

# If numa=off, remove it from GRUB config
# /etc/default/grub: GRUB_CMDLINE_LINUX_DEFAULT="..."

# For VMs, enable NUMA in hypervisor settings
```

### Permission denied for memory binding

**Symptoms**: `NumaError::IoError` with permission denied when using `MemPolicy::Bind`.

**Cause**: `MPOL_BIND` strict mode requires `CAP_SYS_ADMIN`.

**Solutions**:

```bash
# Option 1: Run as root
sudo ./your_app

# Option 2: Grant capability
sudo setcap 'cap_sys_admin+ep' ./your_app

# Option 3: Use soft mode (default)
# MemPolicy::Bind will fall back to best-effort
```

### Thread affinity not applied

**Symptoms**: Workers running on unexpected CPUs despite pinning.

**Causes**:

1. Missing `CAP_SYS_NICE`
2. cgroup CPU restrictions
3. CPU isolation settings

**Diagnostic**:

```bash
# Check which CPUs the process can use
cat /proc/$(pgrep your_app)/status | grep Cpus_allowed_list

# Check cgroup restrictions
cat /sys/fs/cgroup/$(cat /proc/$(pgrep your_app)/cgroup | cut -d: -f3)/cpuset.cpus
```

**Solutions**:

```bash
# Grant capability
sudo setcap 'cap_sys_nice+ep' ./your_app

# Or adjust cgroup settings
echo "0-15" > /sys/fs/cgroup/.../cpuset.cpus
```

### Poor locality despite correct configuration

**Symptoms**: `LocalityStats` shows high remote steals even with `LocalOnly` policy.

**Causes**:

1. NUMA balancing migrating pages
2. Data allocated before worker starts
3. Shared data structures

**Diagnostic**:

```bash
# Check NUMA balancing
cat /proc/sys/kernel/numa_balancing
# Should be 0

# Check where pages actually reside
grep -E "^[0-9a-f]+" /proc/$(pgrep your_app)/numa_maps | head
```

**Solutions**:

```bash
# Disable NUMA balancing
sudo sysctl -w kernel.numa_balancing=0

# In code: allocate data AFTER pinning
let _pin = ScopedPin::pin_current(cpus)?;
let data = NumaRegion::anon(size, MemPolicy::Local, ...)?;
```

### Memory allocation fails with OOM

**Symptoms**: `NumaError::IoError` with out of memory when using `MemPolicy::Bind`.

**Cause**: Requested node has insufficient free memory.

**Diagnostic**:

```bash
# Check per-node memory
numastat -m

# Check for memory pressure
cat /proc/meminfo | grep -i numa
```

**Solutions**:

```rust
// Option 1: Use Preferred instead of Bind
MemPolicy::Preferred(node_id)

// Option 2: Spread across multiple nodes
let mut nodes = NodeMask::new();
nodes.add(NodeId::new(0));
nodes.add(NodeId::new(1));
MemPolicy::Bind(nodes)

// Option 3: Use Interleave
MemPolicy::Interleave(all_nodes)
```

### Executor hangs on shutdown

**Symptoms**: `exec.shutdown()` never returns.

**Causes**:

1. Task that never completes
2. Deadlock in task
3. Task waiting for more work

**Diagnostic**:

```bash
# Check thread states
cat /proc/$(pgrep your_app)/task/*/stat | awk '{print $1, $3}'

# Use gdb to inspect
gdb -p $(pgrep your_app)
(gdb) info threads
(gdb) thread apply all bt
```

**Solutions**:

```rust
// Add timeout to tasks
exec.submit_to_node(node, || {
    let result = std::panic::catch_unwind(|| {
        // Your task
    });
    if result.is_err() {
        log::error!("Task panicked");
    }
});

// Consider using a watchdog
```

### HardModeUnavailable errors

**Symptoms**: `NumaError::HardModeUnavailable` when building executor.

**Diagnostic**:

```rust
let caps = Capabilities::detect();
println!("{}", caps.summary());

for missing in caps.missing_for_hard_mode() {
    println!("Missing: {}", missing);
}
```

**Solutions**:

See [Hard Mode](hard-mode.md) for capability setup.

## Performance Issues

### High latency variance

**Symptoms**: P99 latency much higher than P50.

**Causes**:

1. Page faults during execution
2. NUMA balancing moving pages
3. Work stealing from remote nodes

**Solutions**:

```rust
// Prefault memory
NumaRegion::anon(size, policy, opts, Prefault::Touch)?;

// Disable NUMA balancing
// sysctl -w kernel.numa_balancing=0

// Use LocalOnly steal policy
.steal_policy(StealPolicy::LocalOnly)
```

### Lower throughput than expected

**Symptoms**: Adding NUMA awareness didn't improve performance.

**Causes**:

1. Workload isn't memory-bandwidth bound
2. Tasks too fine-grained
3. Single-socket system

**Diagnostic**:

```bash
# Check if memory bound
perf stat -e cycles,instructions,cache-misses ./your_app

# High cache-misses suggests memory bound
```

**Solutions**:

```rust
// Batch small tasks
for chunk in items.chunks(100) {
    exec.submit_to_node(node, move || {
        for item in chunk {
            process(item);
        }
    });
}
```

### Memory usage higher than expected

**Symptoms**: Process uses more memory than data size.

**Causes**:

1. Per-node sharding overhead
2. Cache padding
3. Huge page alignment

**This is normal**: NUMA-aware allocation trades memory for performance.

## Debug Logging

Enable detailed logging:

```bash
RUST_LOG=numaperf=debug ./your_app
```

Or in code:

```rust
env_logger::Builder::from_env(
    env_logger::Env::default().default_filter_or("numaperf=debug")
).init();
```

## Reporting Issues

When reporting issues, include:

1. **System info**: `lscpu`, `numactl --hardware`
2. **Kernel version**: `uname -a`
3. **Capabilities**: Output of `Capabilities::detect().summary()`
4. **Minimal reproduction**: Smallest code that demonstrates the issue
5. **Error message**: Full error including backtrace if available

```bash
# Capture system info
lscpu > system-info.txt
numactl --hardware >> system-info.txt
uname -a >> system-info.txt
cat /proc/cmdline >> system-info.txt
```
