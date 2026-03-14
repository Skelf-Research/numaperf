# Kernel Configuration

Linux kernel features and settings for optimal numaperf operation.

## Quick Compatibility Check

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

## Kernel Version Requirements

| Feature | Minimum Kernel | Notes |
|---------|----------------|-------|
| Basic NUMA topology | 2.6.18 | `/sys/devices/system/node/` |
| `mbind()` / `set_mempolicy()` | 2.6.7 | Memory policy syscalls |
| CPU affinity | 2.5.8 | `sched_setaffinity()` |
| Transparent huge pages | 2.6.38 | `/sys/kernel/mm/transparent_hugepage/` |
| `move_pages()` | 2.6.18 | Page migration queries |
| `MPOL_PREFERRED_MANY` | 5.15 | Preferred policy with multiple nodes |
| `perf_event_open` | 2.6.31 | Hardware performance counters |

**Recommended**: Linux 5.4+ for full functionality.

## Capability Requirements

| Capability | Required For | Soft Mode Behavior |
|------------|--------------|-------------------|
| None (unprivileged) | Topology discovery, `MPOL_PREFERRED`, thread affinity | Works fully |
| `CAP_SYS_NICE` | Strict CPU pinning, real-time priorities | Falls back to best-effort affinity |
| `CAP_IPC_LOCK` | `mlock()` for pinned pages | Skips memory locking |
| `CAP_SYS_ADMIN` | `MPOL_BIND` strict enforcement, perf counters | Falls back to preferred policy |

### Granting Capabilities

```bash
# Grant capabilities to a binary
sudo setcap 'cap_sys_nice,cap_ipc_lock,cap_sys_admin+ep' ./your_binary

# Verify capabilities
getcap ./your_binary

# Run with specific capabilities
sudo capsh --caps='cap_sys_nice+eip' -- -c './your_binary'
```

## Unprivileged Features

These work without elevated privileges:

- **Topology discovery**: Full access to `/sys/devices/system/node/`
- **CPU affinity**: Setting affinity to any CPU the process can access
- **`MPOL_PREFERRED`**: Preferred node hint (kernel may ignore under pressure)
- **`MPOL_LOCAL`**: Use current thread's node
- **`MPOL_INTERLEAVE`**: Round-robin across nodes
- **Huge pages**: Transparent huge pages (if enabled system-wide)
- **Statistics**: Reading `/proc/self/numa_maps`

## Privileged Features

| Feature | Requirement | Alternative |
|---------|-------------|-------------|
| `MPOL_BIND` strict | `CAP_SYS_ADMIN` or root | Use `MPOL_PREFERRED` |
| `mlock()` / `mlockall()` | `CAP_IPC_LOCK` or `RLIMIT_MEMLOCK` | Skip memory locking |
| Real-time scheduling | `CAP_SYS_NICE` | Use normal priorities |
| Hardware perf counters | `CAP_PERFMON` or `perf_event_paranoid <= 1` | Use software counters |
| Explicit huge pages | `hugetlb` group membership | Use transparent huge pages |

## Sysctl Settings

These kernel parameters affect NUMA behavior:

```bash
# View current settings
sysctl vm.zone_reclaim_mode
sysctl kernel.numa_balancing
sysctl kernel.perf_event_paranoid
```

### Recommended Settings

| Setting | Value | Reason |
|---------|-------|--------|
| `vm.zone_reclaim_mode` | `0` | Prevents premature reclaim on local node |
| `kernel.numa_balancing` | `0` | Prevents kernel from moving pages unexpectedly |
| `kernel.perf_event_paranoid` | `1` or lower | Enables perf counters for non-root |

### Applying Settings

Temporarily:

```bash
sudo sysctl -w vm.zone_reclaim_mode=0
sudo sysctl -w kernel.numa_balancing=0
```

Permanently:

```bash
cat << 'EOF' | sudo tee /etc/sysctl.d/99-numa.conf
# NUMA optimization settings
vm.zone_reclaim_mode = 0
kernel.numa_balancing = 0
EOF

sudo sysctl --system
```

## Transparent Huge Pages

```bash
# Check THP status
cat /sys/kernel/mm/transparent_hugepage/enabled

# Options: always, madvise, never
# Recommended: madvise (explicit opt-in per region)
echo madvise | sudo tee /sys/kernel/mm/transparent_hugepage/enabled
```

### THP Recommendations

| Workload | Setting | Reason |
|----------|---------|--------|
| General server | `madvise` | Opt-in per allocation |
| Database | `never` | Predictable latency |
| HPC | `always` | Maximum throughput |

## Explicit Huge Pages (hugetlbfs)

For 2 MB huge pages:

```bash
# Check current huge page count
cat /sys/kernel/mm/hugepages/hugepages-2048kB/nr_hugepages

# Reserve huge pages at runtime
echo 1024 | sudo tee /sys/kernel/mm/hugepages/hugepages-2048kB/nr_hugepages

# Reserve at boot (add to /etc/sysctl.conf)
vm.nr_hugepages = 1024
```

### Mounting hugetlbfs

```bash
# Mount hugetlbfs
sudo mount -t hugetlbfs none /mnt/hugepages

# Grant access to a group
sudo chown root:hugetlb /mnt/hugepages
sudo chmod 1770 /mnt/hugepages

# Add users to hugetlb group
sudo usermod -aG hugetlb $USER
```

### Persistent mount (fstab)

```
none /mnt/hugepages hugetlbfs defaults,gid=hugetlb,mode=1770 0 0
```

## Hard Mode Checklist

Before enabling hard mode, verify:

- [ ] Kernel version 5.4+
- [ ] `CAP_SYS_ADMIN` or root access (for `MPOL_BIND`)
- [ ] `CAP_SYS_NICE` (for strict pinning)
- [ ] `kernel.numa_balancing=0`
- [ ] `vm.zone_reclaim_mode=0`
- [ ] Sufficient huge pages reserved (if using explicit huge pages)

## Kernel Configuration Options

For custom kernels, ensure these options are enabled:

```
CONFIG_NUMA=y
CONFIG_NUMA_BALANCING=y
CONFIG_NUMA_BALANCING_DEFAULT_ENABLED=n  # Disable by default
CONFIG_TRANSPARENT_HUGEPAGE=y
CONFIG_HUGETLBFS=y
CONFIG_CPUSETS=y
CONFIG_CGROUP_CPUACCT=y
```

## Verifying NUMA Support

```bash
# Check kernel config
zcat /proc/config.gz | grep -i numa

# Should show:
# CONFIG_NUMA=y
# CONFIG_NUMA_BALANCING=y

# Check boot parameters
cat /proc/cmdline | grep numa
# If "numa=off" appears, NUMA is disabled
```

## Platform-Specific Notes

### Intel

- **SNC (Sub-NUMA Clustering)**: Enable in BIOS for more NUMA nodes per socket
- **Memory Interleaving**: Disable in BIOS for proper NUMA topology

### AMD EPYC

- **NPS (NUMA Per Socket)**: Configure in BIOS for NUMA granularity
- **CCX/CCD awareness**: Each compute complex may be a NUMA node

### AWS EC2

- Metal instances provide full NUMA access
- Check instance documentation for NUMA support

### Containers

```bash
# Docker: Check cgroup NUMA restrictions
cat /sys/fs/cgroup/cpuset/cpuset.mems

# Kubernetes: Use topology manager
# kubelet --topology-manager-policy=single-numa-node
```

## See Also

- [Hard Mode](../advanced/hard-mode.md) - Enabling strict enforcement
- [Platform Support](../advanced/platform-support.md) - OS and VM support
- [Troubleshooting](../advanced/troubleshooting.md) - Common issues
