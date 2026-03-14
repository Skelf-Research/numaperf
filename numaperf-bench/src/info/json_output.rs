//! JSON output structures for info command.

use numaperf::{Capabilities, CpuSet, Topology};
use serde::Serialize;

#[derive(Serialize)]
pub struct FullSystemInfo {
    pub topology: TopologyInfo,
    pub capabilities: CapabilitiesInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub affinity: Option<AffinityInfo>,
}

#[derive(Serialize)]
pub struct TopologyInfo {
    pub node_count: usize,
    pub cpu_count: usize,
    pub is_numa: bool,
    pub nodes: Vec<NodeInfo>,
    pub distances: Vec<Vec<Option<u32>>>,
}

#[derive(Serialize)]
pub struct NodeInfo {
    pub id: u32,
    pub cpus: String,
    pub cpu_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_mb: Option<u64>,
}

#[derive(Serialize)]
pub struct CapabilitiesInfo {
    pub hard_mode_supported: bool,
    pub strict_memory_binding: bool,
    pub strict_cpu_affinity: bool,
    pub memory_locking: bool,
    pub numa_balancing_disabled: bool,
    pub numa_node_count: usize,
    pub is_numa_system: bool,
}

#[derive(Serialize)]
pub struct AffinityInfo {
    pub cpus: String,
    pub cpu_count: usize,
    pub nodes: Vec<u32>,
    pub is_node_local: bool,
}

pub fn build_topology_info(topo: &Topology) -> TopologyInfo {
    let nodes: Vec<NodeInfo> = topo
        .numa_nodes()
        .iter()
        .map(|n| NodeInfo {
            id: n.id().as_u32(),
            cpus: format_cpu_list(n.cpus()),
            cpu_count: n.cpu_count(),
            memory_mb: n.memory_mb(),
        })
        .collect();

    let distances = build_distance_matrix(topo);

    TopologyInfo {
        node_count: topo.node_count(),
        cpu_count: topo.cpu_count(),
        is_numa: !topo.is_single_node(),
        nodes,
        distances,
    }
}

pub fn build_distance_matrix(topo: &Topology) -> Vec<Vec<Option<u32>>> {
    topo.numa_nodes()
        .iter()
        .map(|src| {
            topo.numa_nodes()
                .iter()
                .map(|dst| src.distance_to(dst.id()))
                .collect()
        })
        .collect()
}

pub fn build_capabilities_info(caps: &Capabilities) -> CapabilitiesInfo {
    CapabilitiesInfo {
        hard_mode_supported: caps.supports_hard_mode(),
        strict_memory_binding: caps.strict_memory_binding,
        strict_cpu_affinity: caps.strict_cpu_affinity,
        memory_locking: caps.memory_locking,
        numa_balancing_disabled: caps.numa_balancing_disabled,
        numa_node_count: caps.numa_node_count,
        is_numa_system: caps.is_numa_system(),
    }
}

pub fn build_affinity_info(topo: &Topology, affinity: &CpuSet) -> AffinityInfo {
    let nodes: Vec<u32> = topo
        .numa_nodes()
        .iter()
        .filter(|n| n.cpus().iter().any(|cpu| affinity.contains(cpu)))
        .map(|n| n.id().as_u32())
        .collect();

    AffinityInfo {
        cpus: format_cpu_list(affinity),
        cpu_count: affinity.iter().count(),
        is_node_local: nodes.len() == 1,
        nodes,
    }
}

fn format_cpu_list(cpus: &CpuSet) -> String {
    let cpu_vec: Vec<u32> = cpus.iter().collect();
    if cpu_vec.is_empty() {
        return "none".to_string();
    }

    let mut result = String::new();
    let mut i = 0;

    while i < cpu_vec.len() {
        let start = cpu_vec[i];
        let mut end = start;

        while i + 1 < cpu_vec.len() && cpu_vec[i + 1] == cpu_vec[i] + 1 {
            i += 1;
            end = cpu_vec[i];
        }

        if !result.is_empty() {
            result.push_str(",");
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
