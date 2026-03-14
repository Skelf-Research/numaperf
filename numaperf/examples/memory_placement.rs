//! Memory allocation with explicit NUMA placement.
//!
//! This example demonstrates allocating memory with different NUMA
//! placement policies: Local, Bind, Preferred, and Interleave.
//!
//! Run with: cargo run -p numaperf --example memory_placement

use numaperf::{MemPolicy, NodeId, NodeMask, NumaRegion, Prefault, Topology};

fn main() -> Result<(), numaperf::NumaError> {
    println!("=== numaperf: Memory Placement Example ===\n");

    // Discover topology
    let topo = Topology::discover()?;
    println!("System has {} NUMA nodes", topo.node_count());
    println!();

    let size = 1024 * 1024; // 1 MB

    // 1. Local policy (default) - allocate on current thread's node
    println!("1. Local Policy");
    println!("   Allocates on the current thread's NUMA node.");
    let local_region = NumaRegion::anon(size, MemPolicy::Local, Default::default(), Prefault::Touch)?;
    println!("   Allocated {} bytes with Local policy", local_region.len());
    println!("   Enforcement: {:?}", local_region.enforcement());
    println!();

    // 2. Bind policy - strict allocation on specific node(s)
    println!("2. Bind Policy");
    println!("   Strictly allocates on specified nodes only.");
    let node0 = NodeMask::single(NodeId::new(0));
    let bind_region = NumaRegion::anon(size, MemPolicy::Bind(node0), Default::default(), Prefault::Touch)?;
    println!("   Allocated {} bytes bound to node 0", bind_region.len());
    println!("   Enforcement: {:?}", bind_region.enforcement());
    println!();

    // 3. Preferred policy - prefer one node but allow fallback
    println!("3. Preferred Policy");
    println!("   Prefers the specified node but allows fallback.");
    let preferred_region = NumaRegion::anon(
        size,
        MemPolicy::Preferred(NodeId::new(0)),
        Default::default(),
        Prefault::Touch,
    )?;
    println!(
        "   Allocated {} bytes with preference for node 0",
        preferred_region.len()
    );
    println!("   Enforcement: {:?}", preferred_region.enforcement());
    println!();

    // 4. Interleave policy - round-robin across nodes
    if topo.node_count() > 1 {
        println!("4. Interleave Policy");
        println!("   Round-robins pages across multiple nodes.");
        // Build a node mask containing all nodes
        let mut all_nodes = NodeMask::new();
        for node in topo.numa_nodes() {
            all_nodes.add(node.id());
        }
        let interleave_region = NumaRegion::anon(
            size,
            MemPolicy::Interleave(all_nodes),
            Default::default(),
            Prefault::Touch,
        )?;
        println!(
            "   Allocated {} bytes interleaved across {} nodes",
            interleave_region.len(),
            topo.node_count()
        );
        println!("   Enforcement: {:?}", interleave_region.enforcement());
    } else {
        println!("4. Interleave Policy");
        println!("   (Skipped - requires multiple NUMA nodes)");
    }
    println!();

    // Demonstrate writing to memory
    println!("Writing to allocated regions...");
    let mut region = NumaRegion::anon(size, MemPolicy::Local, Default::default(), Prefault::Touch)?;
    let slice = region.as_mut_slice();

    // Write pattern
    for (i, byte) in slice.iter_mut().enumerate() {
        *byte = (i % 256) as u8;
    }

    // Verify
    let checksum: u64 = slice.iter().map(|&b| b as u64).sum();
    println!("   Written {} bytes, checksum: {}", slice.len(), checksum);

    println!();
    println!("Memory placement example complete.");

    Ok(())
}
