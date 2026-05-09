//! Fallback topology discovery for non-Linux platforms or when sysfs is unavailable.

#[cfg(target_os = "linux")]
use std::fs;

use numaperf_core::{CpuSet, NumaError};

use crate::Topology;

/// Discover topology using fallback methods.
///
/// This creates a single-node topology with all available CPUs.
pub fn discover() -> Result<Topology, NumaError> {
    let cpu_count = get_cpu_count()?;
    let mut cpus = CpuSet::new();

    for cpu in 0..cpu_count {
        cpus.add(cpu);
    }

    Ok(Topology::single_node(cpus))
}

/// Get the number of CPUs on the system.
fn get_cpu_count() -> Result<u32, NumaError> {
    // Try /proc/cpuinfo on Linux
    #[cfg(target_os = "linux")]
    {
        if let Ok(content) = fs::read_to_string("/proc/cpuinfo") {
            let count = content
                .lines()
                .filter(|line| line.starts_with("processor"))
                .count() as u32;

            if count > 0 {
                return Ok(count);
            }
        }
    }

    // Try sysconf
    #[cfg(unix)]
    {
        let count = unsafe { libc::sysconf(libc::_SC_NPROCESSORS_ONLN) };
        if count > 0 {
            return Ok(count as u32);
        }
    }

    // Last resort: assume at least 1 CPU
    Ok(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_discover() {
        let topo = discover().expect("fallback should succeed");
        assert!(topo.node_count() > 0);
        assert!(topo.cpu_count() > 0);
        assert!(topo.is_single_node());
    }
}
