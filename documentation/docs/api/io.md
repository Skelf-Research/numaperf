# I/O API

Types for mapping I/O devices to NUMA nodes.

## DeviceMap

Maps devices to their NUMA nodes.

```rust
pub struct DeviceMap { /* internal */ }
```

### Construction

```rust
use numaperf::{DeviceMap, Topology};
use std::sync::Arc;

let topo = Arc::new(Topology::discover()?);
let devices = DeviceMap::discover(Arc::clone(&topo))?;
```

### Methods

| Method | Description |
|--------|-------------|
| `discover(topo) -> Result<Self, NumaError>` | Discover device localities |
| `device_node(&self, name: &str) -> Option<NodeId>` | Get node for device |
| `get(&self, name: &str) -> Option<&DeviceLocality>` | Get device info |
| `network_devices(&self) -> impl Iterator` | Iterate network devices |
| `block_devices(&self) -> impl Iterator` | Iterate block devices |

### Example

```rust
let devices = DeviceMap::discover(topo)?;

// Find node for network device
if let Some(node) = devices.device_node("eth0") {
    println!("eth0 is on node {}", node.as_u32());
}

// List all network devices
for dev in devices.network_devices() {
    println!("{}: node {:?}", dev.name(), dev.node_id());
}
```

---

## DeviceLocality

Information about a device's NUMA locality.

```rust
pub struct DeviceLocality { /* internal */ }
```

### Methods

| Method | Description |
|--------|-------------|
| `name(&self) -> &str` | Device name |
| `device_type(&self) -> DeviceType` | Device type |
| `node_id(&self) -> Option<NodeId>` | NUMA node (if known) |
| `sysfs_path(&self) -> Option<&Path>` | Sysfs path |

### Example

```rust
if let Some(dev) = devices.get("eth0") {
    println!("Name: {}", dev.name());
    println!("Type: {:?}", dev.device_type());
    println!("Node: {:?}", dev.node_id());
}
```

---

## DeviceType

Type of I/O device.

```rust
pub enum DeviceType {
    /// Network interface
    Network,
    /// Block storage device
    BlockDevice,
}
```

### Usage

```rust
match dev.device_type() {
    DeviceType::Network => println!("Network device"),
    DeviceType::BlockDevice => println!("Storage device"),
}
```

---

## Discovery Source

Device locality is read from Linux sysfs:

```
/sys/class/net/<device>/device/numa_node
/sys/class/block/<device>/device/numa_node
```

### Special Values

- `0`, `1`, ... - Device on that NUMA node
- `-1` - No NUMA affinity (returned as `None`)

---

## Pattern: Device-Local Processing

```rust
let devices = DeviceMap::discover(topo)?;
let exec = NumaExecutor::builder(topo).build()?;

// Process on the NIC's node
let nic_node = devices.device_node("eth0")
    .unwrap_or(NodeId::new(0));

exec.submit_to_node(nic_node, || {
    // Process packet on local node
});
```
