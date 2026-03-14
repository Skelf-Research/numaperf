# numaperf-io

[![Crates.io](https://img.shields.io/crates/v/numaperf-io.svg)](https://crates.io/crates/numaperf-io)
[![Documentation](https://img.shields.io/badge/docs-skelfresearch.com-blue)](https://docs.skelfresearch.com/numaperf/api/io/)

**Device locality discovery for NICs and storage.**

## Overview

numaperf-io discovers which NUMA node I/O devices (network interfaces, block devices) are attached to. Use this to allocate buffers and schedule work on the device's local node for optimal performance.

## Usage

```toml
[dependencies]
numaperf-io = "0.1"
```

Most users should use the `numaperf` facade crate instead.

## Example

```rust
use numaperf_io::DeviceMap;
use numaperf_topo::Topology;
use std::sync::Arc;

fn main() -> Result<(), numaperf_core::NumaError> {
    let topo = Arc::new(Topology::discover()?);
    let devices = DeviceMap::discover(Arc::clone(&topo))?;

    // Find which node a NIC is attached to
    if let Some(node) = devices.device_node("eth0") {
        println!("eth0 is on NUMA node {}", node.as_u32());
    }

    // List all network devices
    for dev in devices.network_devices() {
        println!("{}: {:?}", dev.name(), dev.node_id());
    }

    Ok(())
}
```

## Features

- **Device discovery** from `/sys/class/net/` and `/sys/class/block/`
- **Network devices** - NICs, virtual interfaces
- **Block devices** - NVMe, SATA, SCSI
- **Node mapping** - Find device's local NUMA node

## Types

- **`DeviceMap`** - Collection of device localities
- **`DeviceLocality`** - Info about a single device
- **`DeviceType`** - Network or BlockDevice

## Part of numaperf

This crate is part of the [numaperf](https://github.com/Skelf-Research/numaperf) workspace.

- [Documentation](https://docs.skelfresearch.com/numaperf)
- [GitHub](https://github.com/Skelf-Research/numaperf)

## License

Licensed under the [MIT License](../LICENSE).
