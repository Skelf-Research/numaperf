# Platform Support

numaperf support across different platforms and environments.

## Supported Platforms

| Platform | Status | Notes |
|----------|--------|-------|
| Linux x86_64 | Full support | Primary development platform |
| Linux aarch64 | Full support | ARM64 NUMA systems |
| macOS | Partial | No NUMA, APIs work but have no effect |
| Windows | Not supported | Future consideration |
| FreeBSD | Not supported | Future consideration |

## Linux

### Requirements

- **Kernel**: 4.5+ recommended, 5.4+ for full features
- **Architecture**: x86_64 or aarch64
- **NUMA hardware**: Multi-socket or NUMA-on-die

### Kernel Features

| Feature | Kernel Version | Required |
|---------|----------------|----------|
| NUMA sysfs | 2.6.18+ | Yes |
| `mbind()` / `set_mempolicy()` | 2.6.7+ | Yes |
| `sched_setaffinity()` | 2.5.8+ | Yes |
| `MPOL_PREFERRED_MANY` | 5.15+ | No |
| Transparent huge pages | 2.6.38+ | No |

### Checking NUMA Support

```bash
# Verify NUMA nodes exist
ls /sys/devices/system/node/

# Check topology
numactl --hardware

# Verify kernel config
zcat /proc/config.gz | grep NUMA
# CONFIG_NUMA=y
# CONFIG_NUMA_BALANCING=y
```

### Distributions

Tested on:

- Ubuntu 20.04+ / Debian 11+
- RHEL 8+ / CentOS 8+ / Fedora 34+
- Amazon Linux 2023
- Alpine Linux 3.15+

## macOS

macOS does not have NUMA hardware or APIs. numaperf provides graceful degradation:

- `Topology::discover()` returns a single node
- Memory policies are accepted but have no effect
- Thread affinity uses `pthread_setaffinity_np` (limited)

```rust
let topo = Topology::discover()?;
// topo.node_count() == 1 on macOS

// This works but doesn't affect placement
let region = NumaRegion::anon(size, MemPolicy::Local, ...)?;
```

Use numaperf on macOS for:

- Development and testing
- CI/CD pipelines
- Portable code that runs on both macOS and Linux

## Virtual Machines

### VMware vSphere

Enable NUMA in VM settings:

1. Edit VM Settings > CPU
2. Check "Expose hardware-assisted virtualization"
3. Set "NUMA nodes" to match physical topology

Verify inside VM:

```bash
numactl --hardware
# Should show multiple nodes
```

### KVM/QEMU

Pass through NUMA topology:

```bash
qemu-system-x86_64 \
  -smp 16,sockets=2,cores=8,threads=1 \
  -numa node,nodeid=0,cpus=0-7,mem=8G \
  -numa node,nodeid=1,cpus=8-15,mem=8G \
  ...
```

Or with libvirt:

```xml
<cpu mode='host-passthrough'>
  <numa>
    <cell id='0' cpus='0-7' memory='8388608' unit='KiB'/>
    <cell id='1' cpus='8-15' memory='8388608' unit='KiB'/>
  </numa>
</cpu>
```

### AWS EC2

NUMA-aware instance types:

- **Metal instances**: Full NUMA (e.g., `m5.metal`, `c5.metal`)
- **Large instances**: May expose NUMA (e.g., `m5.24xlarge`)
- **Smaller instances**: Single NUMA node

Check instance type documentation for NUMA support.

### Azure

NUMA-aware VM sizes:

- **M-series**: Full NUMA support
- **HBv2/HBv3**: HPC-focused with NUMA
- **Most others**: Single NUMA node

## Containers

### Docker

Pass NUMA topology to container:

```bash
# Allow access to all NUMA nodes
docker run --privileged myimage

# Or with specific capabilities
docker run \
  --cap-add SYS_ADMIN \
  --cap-add SYS_NICE \
  myimage
```

Restrict to specific nodes/CPUs:

```bash
docker run \
  --cpuset-cpus="0-7" \
  --cpuset-mems="0" \
  myimage
```

### Kubernetes

Use the topology manager for NUMA-aware scheduling:

```yaml
apiVersion: v1
kind: Pod
spec:
  containers:
  - name: myapp
    resources:
      requests:
        cpu: "8"
        memory: "16Gi"
      limits:
        cpu: "8"
        memory: "16Gi"
```

Configure kubelet:

```yaml
# kubelet config
topologyManagerPolicy: single-numa-node
cpuManagerPolicy: static
```

### Podman

Similar to Docker:

```bash
podman run \
  --cap-add SYS_ADMIN \
  --cap-add SYS_NICE \
  --cpuset-cpus="0-7" \
  --cpuset-mems="0" \
  myimage
```

## Single-Socket Systems

On single-socket systems:

- Topology discovery returns one node
- All CPUs belong to that node
- Memory policies work but have no effect
- NUMA-aware code runs without modification

```rust
let topo = Topology::discover()?;
if topo.node_count() == 1 {
    println!("Single NUMA node - optimizations won't apply");
}
```

numaperf detects this automatically and works correctly.

## NUMA-on-Die (SNC)

Modern Intel processors can split each socket into multiple NUMA nodes (Sub-NUMA Clustering / SNC):

```bash
# Check if SNC is enabled
lscpu | grep -i numa
# NUMA node(s): 4  (2 sockets * 2 SNC domains)
```

numaperf handles SNC transparently - each SNC domain appears as a separate NUMA node.

## Capability Detection

Always check capabilities at runtime:

```rust
use numaperf::Capabilities;

fn main() -> Result<(), numaperf::NumaError> {
    let caps = Capabilities::detect();

    println!("Platform Capabilities:");
    println!("  NUMA nodes: {}", caps.numa_node_count);
    println!("  Is NUMA system: {}", caps.is_numa_system());
    println!("  Hard mode: {}", caps.supports_hard_mode());

    if !caps.is_numa_system() {
        println!("Warning: Running on non-NUMA system");
        println!("NUMA optimizations will have no effect");
    }

    Ok(())
}
```

## Feature Flags

numaperf automatically enables features based on platform:

| Feature | Linux | macOS |
|---------|-------|-------|
| Topology discovery | Yes | Limited |
| Memory binding | Yes | No |
| Thread affinity | Yes | Limited |
| Device locality | Yes | No |
| Hard mode | Yes | No |

No manual feature flags required - everything is auto-detected.
