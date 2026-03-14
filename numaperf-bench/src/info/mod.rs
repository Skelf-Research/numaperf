//! System information display module.

mod affinity;
mod capabilities;
mod json_output;
mod topology;

use crate::{InfoArgs, InfoSection, OutputFormat};
use numaperf::{get_affinity, Capabilities, CpuSet, Topology};
use std::sync::Arc;

pub fn run(args: InfoArgs) -> Result<(), Box<dyn std::error::Error>> {
    let topo = Arc::new(Topology::discover()?);
    let caps = Capabilities::detect();
    let current_affinity = get_affinity().ok();

    match args.format {
        OutputFormat::Text => {
            print_text(&topo, &caps, current_affinity.as_ref(), &args)
        }
        OutputFormat::Json => {
            print_json(&topo, &caps, current_affinity.as_ref(), &args)
        }
    }

    Ok(())
}

fn print_text(
    topo: &Topology,
    caps: &Capabilities,
    affinity: Option<&CpuSet>,
    args: &InfoArgs,
) {
    match args.section {
        InfoSection::All => {
            println!("=== numaperf System Information ===\n");
            topology::print_topology(topo, args.verbose);
            println!();
            topology::print_distances(topo);
            println!();
            capabilities::print_capabilities(caps, args.verbose);
            if let Some(aff) = affinity {
                println!();
                affinity::print_affinity(topo, aff);
            }
        }
        InfoSection::Topology => {
            topology::print_topology(topo, args.verbose);
        }
        InfoSection::Capabilities => {
            capabilities::print_capabilities(caps, args.verbose);
        }
        InfoSection::Affinity => {
            if let Some(aff) = affinity {
                affinity::print_affinity(topo, aff);
            } else {
                println!("Could not read current thread affinity");
            }
        }
        InfoSection::Distances => {
            topology::print_distances(topo);
        }
    }
}

fn print_json(
    topo: &Topology,
    caps: &Capabilities,
    affinity: Option<&CpuSet>,
    args: &InfoArgs,
) {
    let output = match args.section {
        InfoSection::All => {
            let full = json_output::FullSystemInfo {
                topology: json_output::build_topology_info(topo),
                capabilities: json_output::build_capabilities_info(caps),
                affinity: affinity.map(|a| json_output::build_affinity_info(topo, a)),
            };
            serde_json::to_string_pretty(&full).unwrap()
        }
        InfoSection::Topology => {
            let info = json_output::build_topology_info(topo);
            serde_json::to_string_pretty(&info).unwrap()
        }
        InfoSection::Capabilities => {
            let info = json_output::build_capabilities_info(caps);
            serde_json::to_string_pretty(&info).unwrap()
        }
        InfoSection::Affinity => {
            if let Some(aff) = affinity {
                let info = json_output::build_affinity_info(topo, aff);
                serde_json::to_string_pretty(&info).unwrap()
            } else {
                r#"{"error": "could not read affinity"}"#.to_string()
            }
        }
        InfoSection::Distances => {
            let distances = json_output::build_distance_matrix(topo);
            serde_json::to_string_pretty(&distances).unwrap()
        }
    };

    println!("{}", output);
}
