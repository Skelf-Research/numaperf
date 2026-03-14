# System Requirements

## Supported Platforms

| Platform | Support Level | Notes |
|----------|--------------|-------|
| Linux (NUMA) | Full | All features available |
| Linux (UMA) | Partial | Graceful single-node fallback |
| macOS | Limited | Topology discovery only |
| Windows | Limited | Basic support |

## Linux Requirements

### Kernel Version

- **Minimum**: Linux 4.9
- **Recommended**: Linux 5.4+

Required kernel configuration options:

```
CONFIG_NUMA=y
CONFIG_NUMA_BALANCING=y (optional, can be disabled at runtime)
```

### Checking NUMA Support

```bash
# Check if NUMA is enabled
cat /proc/cmdline | grep -o numa

# Count NUMA nodes
ls -d /sys/devices/system/node/node* | wc -l

# View NUMA topology
numactl --hardware
```

### Required System Capabilities

Different features require different Linux capabilities:

| Feature | Capability | Purpose |
|---------|-----------|---------|
| Memory binding | `CAP_SYS_ADMIN` | Strict MPOL_BIND enforcement |
| CPU affinity | `CAP_SYS_NICE` | Real-time scheduling priority |
| Memory locking | `CAP_IPC_LOCK` | Prevent page migration |

### Hard Mode Requirements

For strict locality guarantees ("hard mode"), you need:

1. **CAP_SYS_ADMIN** - For strict memory binding
2. **CAP_SYS_NICE** - For guaranteed CPU affinity
3. **NUMA balancing disabled** - Prevent kernel page migration

Check your capabilities:

```bash
# Using numaperf CLI
numaperf-bench info capabilities

# Or manually
cat /proc/self/status | grep Cap
```

### Enabling Capabilities

**Option 1: Run as root**
```bash
sudo ./your-application
```

**Option 2: Use setcap**
```bash
sudo setcap cap_sys_admin,cap_sys_nice,cap_ipc_lock+ep ./your-application
```

**Option 3: Docker**
```bash
docker run --cap-add SYS_ADMIN --cap-add SYS_NICE --cap-add IPC_LOCK ...
```

**Option 4: systemd service**
```ini
[Service]
AmbientCapabilities=CAP_SYS_ADMIN CAP_SYS_NICE CAP_IPC_LOCK
```

### Disabling NUMA Balancing

NUMA balancing can migrate pages between nodes, which may interfere with explicit memory placement:

```bash
# Disable temporarily
echo 0 | sudo tee /proc/sys/kernel/numa_balancing

# Disable permanently (add to /etc/sysctl.conf)
kernel.numa_balancing = 0
```

## Hardware Requirements

### Multi-Socket Systems

numaperf is most beneficial on multi-socket systems where NUMA effects are significant:

- 2+ CPU sockets
- Separate memory controllers per socket
- Typical latency ratio: 1.5-3x for remote vs local access

### Single-Socket Systems

On single-socket systems, numaperf provides:

- Graceful fallback to single-node operation
- CPU affinity still works
- Memory policies have no effect (all memory is "local")

### Virtual Machines

NUMA topology may not be accurately exposed in VMs:

- **VMware**: Enable NUMA virtualization
- **KVM/QEMU**: Use `-numa` options
- **Cloud instances**: Choose NUMA-aware instance types

## Checking System Configuration

Use the numaperf CLI to check your system:

```bash
# Full system information
numaperf-bench info

# Just capabilities
numaperf-bench info capabilities -v

# JSON output for scripting
numaperf-bench info --format json
```

Example output:

```
=== numaperf System Information ===

NUMA Topology
─────────────
Nodes: 2
Total CPUs: 16

  Node 0: 8 CPUs (0-7), 32768 MB memory
  Node 1: 8 CPUs (8-15), 32768 MB memory

Distance Matrix
───────────────
       Node  0  Node  1
Node  0     10      21
Node  1     21      10

Capabilities
────────────
Hard mode: NOT SUPPORTED

  [-] CAP_SYS_ADMIN (strict memory binding)
  [+] CAP_SYS_NICE (strict CPU affinity)
  [-] CAP_IPC_LOCK (memory locking)
  [-] NUMA balancing disabled

NUMA system: yes (2 nodes)
```

## Troubleshooting

### "No NUMA nodes found"

1. Check if NUMA is enabled in kernel: `dmesg | grep -i numa`
2. Verify `/sys/devices/system/node/` exists
3. On VMs, check hypervisor NUMA settings

### "Permission denied" on memory binding

1. Check capabilities: `capsh --print`
2. Run with elevated privileges or add capabilities
3. Consider using soft mode for best-effort locality

### "Affinity not applied"

1. Check if CPUs exist: `cat /proc/cpuinfo`
2. Verify CPU set syntax: `0-3` or `0,1,2,3`
3. Check cgroup CPU restrictions

## Next Steps

- [Soft vs Hard Mode](../concepts/soft-vs-hard-mode.md) - Understanding enforcement modes
- [Hard Mode Guide](../advanced/hard-mode.md) - Detailed hard mode configuration
- [Troubleshooting](../advanced/troubleshooting.md) - Common issues and solutions
