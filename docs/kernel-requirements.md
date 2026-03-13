# Kernel Requirements

This document describes the Linux kernel features and capabilities required for full `numaperf` functionality.

## Quick compatibility check

Run these commands to verify your system supports NUMA features:

```bash
# Check if NUMA is enabled
numactl --hardware

# Check kernel version (4.5+ recommended)
uname -r

# Check available capabilities
capsh --print

# Verify NUMA sysfs is present
ls /sys/devices/system/node/
```

## Kernel version requirements

| Feature | Minimum kernel | Notes |
|---------|----------------|-------|
| Basic NUMA topology | 2.6.18 | `/sys/devices/system/node/` |
| `mbind()` / `set_mempolicy()` | 2.6.7 | Memory policy syscalls |
| `MPOL_PREFERRED_MANY` | 5.15 | Preferred policy with multiple nodes |
| Transparent huge pages | 2.6.38 | `/sys/kernel/mm/transparent_hugepage/` |
| `move_pages()` | 2.6.18 | Page migration queries |
| CPU affinity | 2.5.8 | `sched_setaffinity()` |
| `perf_event_open` | 2.6.31 | Hardware performance counters |

**Recommended:** Linux 5.4+ for full functionality.

## Capability requirements

| Capability | Required for | Soft mode behavior |
|------------|--------------|-------------------|
| None (unprivileged) | Topology discovery, `MPOL_PREFERRED`, thread affinity | Works fully |
| `CAP_SYS_NICE` | Strict CPU pinning, real-time priorities | Falls back to best-effort affinity |
| `CAP_IPC_LOCK` | `mlock()` for pinned pages | Skips memory locking |
| `CAP_SYS_ADMIN` | `MPOL_BIND` strict enforcement, perf counters | Falls back to preferred policy |

### Granting capabilities

To run without root but with required capabilities:

```bash
# Grant capabilities to a binary
sudo setcap 'cap_sys_nice,cap_ipc_lock+ep' ./your_binary

# Or run with specific capabilities
sudo capsh --caps='cap_sys_nice+eip' -- -c './your_binary'
```

## What works unprivileged

These features work without elevated privileges:

- **Topology discovery**: Full access to `/sys/devices/system/node/`
- **CPU affinity**: Setting affinity to any CPU the process can access
- **`MPOL_PREFERRED`**: Preferred node hint (kernel may ignore under pressure)
- **`MPOL_LOCAL`**: Use current thread's node
- **`MPOL_INTERLEAVE`**: Round-robin across nodes
- **Huge pages**: Transparent huge pages (if enabled system-wide)
- **Statistics**: Reading `/proc/self/numa_maps`

## What requires privileges

These features require elevated privileges or capabilities:

| Feature | Requirement | Alternative |
|---------|-------------|-------------|
| `MPOL_BIND` strict | `CAP_SYS_ADMIN` or `root` | Use `MPOL_PREFERRED` |
| `mlock()` / `mlockall()` | `CAP_IPC_LOCK` or RLIMIT_MEMLOCK | Skip memory locking |
| Real-time scheduling | `CAP_SYS_NICE` | Use normal priorities |
| Hardware perf counters | `CAP_PERFMON` or `perf_event_paranoid <= 1` | Use software counters |
| Explicit huge pages | `hugetlb` group membership | Use transparent huge pages |

## Sysctl settings

These kernel parameters affect NUMA behavior:

```bash
# View current settings
sysctl vm.zone_reclaim_mode
sysctl kernel.numa_balancing
sysctl kernel.perf_event_paranoid

# Recommended for NUMA-aware workloads
sudo sysctl -w vm.zone_reclaim_mode=0      # Disable zone reclaim
sudo sysctl -w kernel.numa_balancing=0     # Disable automatic balancing
```

| Setting | Recommended | Reason |
|---------|-------------|--------|
| `vm.zone_reclaim_mode` | `0` | Prevents premature reclaim on local node |
| `kernel.numa_balancing` | `0` | Prevents kernel from moving pages unexpectedly |
| `kernel.perf_event_paranoid` | `1` or lower | Enables perf counters for non-root |

## Transparent huge pages

```bash
# Check THP status
cat /sys/kernel/mm/transparent_hugepage/enabled

# Options: always, madvise, never
# Recommended: madvise (explicit opt-in per region)
echo madvise | sudo tee /sys/kernel/mm/transparent_hugepage/enabled
```

## Explicit huge pages (hugetlbfs)

For 2 MB huge pages:

```bash
# Reserve huge pages at boot or runtime
echo 1024 | sudo tee /sys/kernel/mm/hugepages/hugepages-2048kB/nr_hugepages

# Mount hugetlbfs
sudo mount -t hugetlbfs none /mnt/hugepages

# Grant access to a group
sudo chown root:hugetlb /mnt/hugepages
sudo chmod 1770 /mnt/hugepages
```

## Hard mode checklist

Before enabling hard mode, verify:

- [ ] Kernel version 5.4+
- [ ] `CAP_SYS_ADMIN` or root access (for `MPOL_BIND`)
- [ ] `CAP_SYS_NICE` (for strict pinning)
- [ ] `kernel.numa_balancing=0`
- [ ] `vm.zone_reclaim_mode=0`
- [ ] Sufficient huge pages reserved (if using explicit huge pages)

## Platform notes

### Single-socket machines

On single-socket machines:
- Topology discovery returns one NUMA node
- Affinity APIs work but have no locality benefit
- Memory policies are accepted but have no effect
- All features degrade gracefully

### Virtual machines

NUMA topology may be virtualized:
- Verify with `numactl --hardware` inside the VM
- Some hypervisors expose fake NUMA for testing
- Memory policies may not reflect physical locality

### Containers

Container NUMA support depends on runtime configuration:
- Docker: Use `--cpuset-cpus` and `--cpuset-mems`
- Kubernetes: Use topology manager and CPU manager
- cgroups v2: Check `cpuset.cpus.effective` and `cpuset.mems.effective`
