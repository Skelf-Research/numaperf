//! Affinity display functions.

use numaperf::{CpuSet, NodeId, Topology};

pub fn print_affinity(topo: &Topology, affinity: &CpuSet) {
    println!("Current Thread Affinity");
    println!("───────────────────────");

    // Format CPU list
    let cpu_list = format_cpu_list(affinity);
    let cpu_count = affinity.iter().count();

    println!("CPUs: {}", cpu_list);
    println!("CPU count: {}", cpu_count);

    // Determine which nodes the affinity spans
    let nodes: Vec<NodeId> = topo
        .numa_nodes()
        .iter()
        .filter(|n| n.cpus().iter().any(|cpu| affinity.contains(cpu)))
        .map(|n| n.id())
        .collect();

    if nodes.is_empty() {
        println!("Node: unknown");
    } else if nodes.len() == 1 {
        println!("Node: {} (single node - optimal)", nodes[0].as_u32());
    } else {
        let node_list: Vec<u32> = nodes.iter().map(|n| n.as_u32()).collect();
        println!("Nodes: {:?} (spans {} nodes)", node_list, nodes.len());
    }

    // Check if affinity matches all CPUs
    let total_cpus = topo.cpu_count();
    if cpu_count == total_cpus {
        println!("Status: unrestricted (all CPUs)");
    } else if nodes.len() == 1 {
        println!("Status: node-local (good for NUMA locality)");
    } else {
        println!("Status: restricted but spans multiple nodes");
    }
}

fn format_cpu_list(cpus: &CpuSet) -> String {
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
