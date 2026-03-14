# Installation

## Requirements

- **Rust 1.70+** (check with `rustc --version`)
- **Linux kernel 5.4+** with CONFIG_NUMA enabled (for full functionality)
- For testing on single-socket systems, numaperf provides graceful fallbacks

## Adding numaperf

Add numaperf to your `Cargo.toml`:

```toml
[dependencies]
numaperf = "0.1"
```

Or use cargo add:

```bash
cargo add numaperf
```

## Verifying Installation

Create a simple test program to verify numaperf is working:

```rust
use numaperf::{Topology, Capabilities};

fn main() -> Result<(), numaperf::NumaError> {
    // Check capabilities
    let caps = Capabilities::detect();
    println!("NUMA nodes: {}", caps.numa_node_count);
    println!("Hard mode supported: {}", caps.supports_hard_mode());

    // Discover topology
    let topo = Topology::discover()?;
    println!("CPUs: {}", topo.cpu_count());

    for node in topo.numa_nodes() {
        println!("  Node {}: {} CPUs",
            node.id().as_u32(),
            node.cpu_count());
    }

    Ok(())
}
```

Run it:

```bash
cargo run
```

## Using the CLI Tool

numaperf includes a CLI tool for system information and benchmarking:

```bash
# Build the CLI
cargo build -p numaperf-bench --release

# Show system information
./target/release/numaperf-bench info

# Run benchmarks
./target/release/numaperf-bench bench
```

## Minimum Supported Rust Version (MSRV)

numaperf supports Rust 1.70 and later. The MSRV is specified in the workspace `Cargo.toml`:

```toml
[workspace.package]
rust-version = "1.70"
```

## Crate Organization

numaperf is organized as a workspace of specialized crates:

| Crate | Description |
|-------|-------------|
| `numaperf` | Facade crate (recommended) |
| `numaperf-core` | Core types and errors |
| `numaperf-topo` | Topology discovery |
| `numaperf-affinity` | Thread pinning |
| `numaperf-mem` | Memory placement |
| `numaperf-sched` | Work scheduling |
| `numaperf-sharded` | Sharded data structures |
| `numaperf-io` | Device locality |
| `numaperf-perf` | Observability |

We recommend using the main `numaperf` crate, which re-exports all public types:

```rust
// Recommended: use the facade crate
use numaperf::{Topology, NumaExecutor, MemPolicy};

// Also works: use individual crates directly
use numaperf_topo::Topology;
use numaperf_sched::NumaExecutor;
```

## Next Steps

- [Quickstart Guide](quickstart.md) - Build your first NUMA-aware application
- [System Requirements](system-requirements.md) - Detailed platform requirements
- [NUMA Basics](../concepts/numa-basics.md) - Understand NUMA fundamentals
