//! Topology display functions.

use numaperf::Topology;

pub fn print_topology(topo: &Topology, verbose: bool) {
    println!("NUMA Topology");
    println!("─────────────");
    println!("Nodes: {}", topo.node_count());
    println!("Total CPUs: {}", topo.cpu_count());

    if topo.is_single_node() && verbose {
        println!("(Single-node system or NUMA not detected)");
    }

    println!();

    for node in topo.numa_nodes() {
        let mem = node
            .memory_mb()
            .map(|m| format!("{} MB", m))
            .unwrap_or_else(|| "unknown".to_string());

        // Format CPU list
        let cpu_list = format_cpu_list(node.cpus());

        println!(
            "  Node {}: {} CPUs ({}), {} memory",
            node.id().as_u32(),
            node.cpu_count(),
            cpu_list,
            mem
        );
    }
}

pub fn print_distances(topo: &Topology) {
    println!("Distance Matrix");
    println!("───────────────");

    if topo.node_count() == 0 {
        println!("  (No NUMA nodes detected)");
        return;
    }

    // Print header
    print!("       ");
    for node in topo.numa_nodes() {
        print!("Node {:>2}  ", node.id().as_u32());
    }
    println!();

    // Print each row
    for src_node in topo.numa_nodes() {
        print!("Node {:>2}", src_node.id().as_u32());
        for dst_node in topo.numa_nodes() {
            let distance = src_node
                .distance_to(dst_node.id())
                .map(|d| format!("{:>5}", d))
                .unwrap_or_else(|| "    -".to_string());
            print!("  {}", distance);
        }
        println!();
    }
}

fn format_cpu_list(cpus: &numaperf::CpuSet) -> String {
    let cpu_vec: Vec<u32> = cpus.iter().collect();
    if cpu_vec.is_empty() {
        return "none".to_string();
    }

    // Try to find ranges
    let mut result = String::new();
    let mut i = 0;

    while i < cpu_vec.len() {
        let start = cpu_vec[i];
        let mut end = start;

        // Find consecutive range
        while i + 1 < cpu_vec.len() && cpu_vec[i + 1] == cpu_vec[i] + 1 {
            i += 1;
            end = cpu_vec[i];
        }

        if !result.is_empty() {
            result.push_str(", ");
        }

        if end > start {
            result.push_str(&format!("{}-{}", start, end));
        } else {
            result.push_str(&format!("{}", start));
        }

        i += 1;
    }

    result
}
