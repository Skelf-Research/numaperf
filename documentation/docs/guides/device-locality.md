# Device Locality

Learn how to map I/O devices to NUMA nodes for optimal data placement.

## Why Device Locality?

Network cards and storage devices are connected to specific NUMA nodes. Processing I/O on the wrong node causes remote memory accesses:

```
┌─────────────┐     ┌─────────────┐
│   Node 0    │     │   Node 1    │
│  ┌───────┐  │     │  ┌───────┐  │
│  │  CPU  │  │     │  │  CPU  │  │
│  └───┬───┘  │     │  └───┬───┘  │
│      │      │     │      │      │
│  ┌───▼───┐  │     │  ┌───▼───┐  │
│  │ Memory│  │     │  │ Memory│  │
│  └───────┘  │     │  └───────┘  │
│      │      │     │             │
│  ┌───▼───┐  │     │             │
│  │  NIC  │  │     │             │
│  └───────┘  │     │             │
└─────────────┘     └─────────────┘

If NIC is on Node 0, process packets on Node 0!
```

## Discovering Device Locality

```rust
use numaperf::{DeviceMap, Topology};
use std::sync::Arc;

let topo = Arc::new(Topology::discover()?);
let devices = DeviceMap::discover(Arc::clone(&topo))?;

// Find NUMA node for a network device
if let Some(node_id) = devices.device_node("eth0") {
    println!("eth0 is on node {}", node_id.as_u32());
}
```

## Listing Devices

```rust
// List all network devices
for device in devices.network_devices() {
    println!("{}: node {:?}", device.name(), device.node_id());
}

// List all block devices
for device in devices.block_devices() {
    println!("{}: node {:?}", device.name(), device.node_id());
}
```

## DeviceLocality

Information about a specific device:

```rust
if let Some(locality) = devices.get("eth0") {
    println!("Name: {}", locality.name());
    println!("Type: {:?}", locality.device_type());
    println!("Node: {:?}", locality.node_id());
    println!("Path: {:?}", locality.sysfs_path());
}
```

## Pattern: Device-Local Processing

Route I/O processing to the device's NUMA node:

```rust
use numaperf::{DeviceMap, NumaExecutor, NodeId};

let topo = Arc::new(Topology::discover()?);
let devices = DeviceMap::discover(Arc::clone(&topo))?;
let exec = NumaExecutor::builder(Arc::clone(&topo)).build()?;

// Get the node for our network device
let nic_node = devices.device_node("eth0")
    .unwrap_or(NodeId::new(0));

// Process packets on the NIC's node
fn handle_packet(exec: &NumaExecutor, nic_node: NodeId, packet: Packet) {
    exec.submit_to_node(nic_node, move || {
        // Process packet locally to the NIC
        process(packet);
    });
}
```

## Pattern: Per-Device Buffer Pools

Allocate buffers on the device's node:

```rust
use numaperf::{NumaRegion, MemPolicy, NodeMask, Prefault};

let topo = Arc::new(Topology::discover()?);
let devices = DeviceMap::discover(Arc::clone(&topo))?;

// Create buffer pool for each network device
for device in devices.network_devices() {
    if let Some(node_id) = device.node_id() {
        let pool = NumaRegion::anon(
            pool_size,
            MemPolicy::Bind(NodeMask::single(node_id)),
            Default::default(),
            Prefault::Touch,
        )?;

        register_buffer_pool(device.name(), pool);
    }
}
```

## Pattern: NUMA-Aware Network Server

```rust
struct NumaServer {
    topo: Arc<Topology>,
    devices: DeviceMap,
    exec: NumaExecutor,
}

impl NumaServer {
    fn handle_connection(&self, conn: Connection) {
        // Determine which NIC received this connection
        let nic = conn.interface();

        // Get the node for that NIC
        let node = self.devices.device_node(nic)
            .unwrap_or(NodeId::new(0));

        // Process on the NIC's node
        self.exec.submit_to_node(node, move || {
            conn.process();
        });
    }
}
```

## Device Types

```rust
use numaperf::DeviceType;

match device.device_type() {
    DeviceType::Network => println!("Network interface"),
    DeviceType::BlockDevice => println!("Storage device"),
}
```

## How Discovery Works

Device locality is read from Linux sysfs:

```
/sys/class/net/<device>/device/numa_node
/sys/class/block/<device>/device/numa_node
```

Values:
- `0`, `1`, etc. - Device is on that NUMA node
- `-1` - No NUMA affinity (treat as node 0)

## Fallback Behavior

When device locality can't be determined:

```rust
// device_node() returns None if:
// - Device doesn't exist
// - No sysfs entry
// - sysfs shows -1

let node = devices.device_node("eth0")
    .unwrap_or(NodeId::new(0));  // Default to node 0
```

## Best Practices

1. **Check device locality** at startup
2. **Process I/O on device node** to minimize remote accesses
3. **Allocate DMA buffers** on device node
4. **Pin I/O threads** to device node
5. **Handle missing locality** gracefully
