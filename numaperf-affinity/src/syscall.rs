//! Low-level syscall wrappers for CPU affinity.

use numaperf_core::{CpuSet, NumaError};

/// Get the current thread's CPU affinity mask.
#[cfg(target_os = "linux")]
pub fn get_affinity() -> Result<CpuSet, NumaError> {
    let mut mask: libc::cpu_set_t = unsafe { std::mem::zeroed() };
    let size = std::mem::size_of::<libc::cpu_set_t>();

    let ret = unsafe { libc::sched_getaffinity(0, size, &mut mask) };

    if ret != 0 {
        return Err(NumaError::pinning(std::io::Error::last_os_error()));
    }

    Ok(cpu_set_from_libc(&mask))
}

/// Set the current thread's CPU affinity mask.
#[cfg(target_os = "linux")]
pub fn set_affinity(cpus: &CpuSet) -> Result<(), NumaError> {
    let mask = cpu_set_to_libc(cpus);
    let size = std::mem::size_of::<libc::cpu_set_t>();

    let ret = unsafe { libc::sched_setaffinity(0, size, &mask) };

    if ret != 0 {
        return Err(NumaError::pinning(std::io::Error::last_os_error()));
    }

    Ok(())
}

/// Convert libc cpu_set_t to our CpuSet.
#[cfg(target_os = "linux")]
fn cpu_set_from_libc(mask: &libc::cpu_set_t) -> CpuSet {
    let mut cpus = CpuSet::new();

    // CPU_SETSIZE is typically 1024
    for cpu in 0..CpuSet::MAX_CPUS {
        if unsafe { libc::CPU_ISSET(cpu, mask) } {
            cpus.add(cpu as u32);
        }
    }

    cpus
}

/// Convert our CpuSet to libc cpu_set_t.
#[cfg(target_os = "linux")]
fn cpu_set_to_libc(cpus: &CpuSet) -> libc::cpu_set_t {
    let mut mask: libc::cpu_set_t = unsafe { std::mem::zeroed() };

    unsafe {
        libc::CPU_ZERO(&mut mask);
    }

    for cpu in cpus.iter() {
        unsafe {
            libc::CPU_SET(cpu as usize, &mut mask);
        }
    }

    mask
}

// Non-Linux fallbacks

#[cfg(not(target_os = "linux"))]
pub fn get_affinity() -> Result<CpuSet, NumaError> {
    Err(NumaError::NotSupported {
        feature: "CPU affinity",
    })
}

#[cfg(not(target_os = "linux"))]
pub fn set_affinity(_cpus: &CpuSet) -> Result<(), NumaError> {
    Err(NumaError::NotSupported {
        feature: "CPU affinity",
    })
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "linux")]
    use super::*;

    #[test]
    #[cfg(target_os = "linux")]
    fn test_get_set_affinity() {
        // Get current affinity
        let original = get_affinity().expect("should get affinity");
        assert!(!original.is_empty(), "should have at least one CPU");

        // If we have multiple CPUs, try restricting to just the first one
        if original.count() > 1 {
            let first_cpu = original.first().unwrap();
            let restricted = CpuSet::single(first_cpu);

            set_affinity(&restricted).expect("should set affinity");

            let current = get_affinity().expect("should get affinity");
            assert_eq!(current.count(), 1);
            assert!(current.contains(first_cpu));

            // Restore original
            set_affinity(&original).expect("should restore affinity");
        }
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_cpu_set_conversion() {
        let mut cpus = CpuSet::new();
        cpus.add(0);
        cpus.add(3);
        cpus.add(7);

        let libc_set = cpu_set_to_libc(&cpus);
        let converted = cpu_set_from_libc(&libc_set);

        assert_eq!(cpus, converted);
    }
}
