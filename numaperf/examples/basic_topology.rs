//! Discover and display NUMA topology.
//!
//! This example shows how to discover the system's NUMA topology and
//! check system capabilities for NUMA operations.
//!
//! Run with: cargo run -p numaperf --example basic_topology

use numaperf::{Capabilities, Topology};

fn main() -> Result<(), numaperf::NumaError> {
    println!("=== numaperf: Basic Topology Example ===\n");

    // Check system capabilities
    let caps = Capabilities::detect();
    println!("{}", caps.summary());

    // Discover topology
    let topo = Topology::discover()?;

    println!("NUMA Topology Details");
    println!("=====================");
    println!("Total nodes: {}", topo.node_count());
    println!();

    for node in topo.numa_nodes() {
        println!("Node {}:", node.id());
        println!("  CPU count: {}", node.cpu_count());
        println!("  CPUs: {:?}", node.cpus());

        // Show distances to other nodes if available
        for other_node in topo.numa_nodes() {
            if other_node.id() != node.id() {
                if let Some(dist) = node.distance_to(other_node.id()) {
                    println!("  Distance to node {}: {}", other_node.id(), dist);
                }
            }
        }
        println!();
    }

    // Show which node the current thread would use
    println!("System Summary");
    println!("==============");
    if caps.is_numa_system() {
        println!("This is a multi-node NUMA system.");
        println!("NUMA-aware programming will provide benefits.");
    } else {
        println!("This is a single-node system (or NUMA is not detected).");
        println!("numaperf will work but NUMA optimizations won't apply.");
    }

    Ok(())
}
