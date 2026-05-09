//! Low-level syscall wrappers for memory mapping and NUMA binding.

use std::io;

use numaperf_core::{EnforcementLevel, NumaError};

use crate::huge::HugePageMode;
use crate::policy::MemPolicy;

/// NUMA memory policy flags for mbind/set_mempolicy.
#[cfg(target_os = "linux")]
mod mpol {
    pub const MPOL_DEFAULT: i32 = 0;
    pub const MPOL_PREFERRED: i32 = 1;
    pub const MPOL_BIND: i32 = 2;
    pub const MPOL_INTERLEAVE: i32 = 3;
    pub const MPOL_LOCAL: i32 = 4;

    // Flags
    pub const MPOL_F_STATIC_NODES: u32 = 1 << 15;
}

/// Result of an mmap operation.
pub struct MmapResult {
    pub ptr: *mut u8,
    pub len: usize,
}

/// Allocate anonymous memory with mmap.
#[cfg(target_os = "linux")]
pub fn mmap_anon(size: usize, huge: HugePageMode) -> Result<MmapResult, NumaError> {
    let mut flags = libc::MAP_PRIVATE | libc::MAP_ANONYMOUS;

    // Add huge page flags
    if huge.is_explicit() {
        flags |= libc::MAP_HUGETLB;
        if matches!(huge, HugePageMode::Explicit1GB) {
            flags |= libc::MAP_HUGE_1GB;
        }
    }

    let ptr = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            size,
            libc::PROT_READ | libc::PROT_WRITE,
            flags,
            -1,
            0,
        )
    };

    if ptr == libc::MAP_FAILED {
        return Err(NumaError::allocation(size, io::Error::last_os_error()));
    }

    let ptr = ptr as *mut u8;

    // Apply madvise for transparent huge pages
    match huge {
        HugePageMode::TransparentOn => unsafe {
            libc::madvise(ptr as *mut libc::c_void, size, libc::MADV_HUGEPAGE);
        },
        HugePageMode::TransparentOff => unsafe {
            libc::madvise(ptr as *mut libc::c_void, size, libc::MADV_NOHUGEPAGE);
        },
        _ => {}
    }

    Ok(MmapResult { ptr, len: size })
}

/// Unmap memory.
#[cfg(target_os = "linux")]
pub fn munmap(ptr: *mut u8, len: usize) -> Result<(), NumaError> {
    let ret = unsafe { libc::munmap(ptr as *mut libc::c_void, len) };
    if ret != 0 {
        return Err(NumaError::Io(io::Error::last_os_error()));
    }
    Ok(())
}

/// Bind memory to NUMA nodes using mbind.
#[cfg(target_os = "linux")]
pub fn mbind(ptr: *mut u8, len: usize, policy: &MemPolicy) -> Result<EnforcementLevel, NumaError> {
    let (mode, nodemask, maxnode) = policy_to_mbind_args(policy);

    // For MPOL_LOCAL, pass null for nodemask
    let nodemask_ptr = if matches!(policy, MemPolicy::Local) {
        std::ptr::null()
    } else {
        nodemask.as_ptr()
    };

    // Call mbind
    let ret = unsafe {
        libc::syscall(
            libc::SYS_mbind,
            ptr as *mut libc::c_void,
            len,
            mode,
            nodemask_ptr,
            maxnode,
            0u32, // flags
        )
    };

    if ret != 0 {
        let err = io::Error::last_os_error();

        // Check if it's a permission error
        if err.raw_os_error() == Some(libc::EPERM) {
            // In soft mode, we might want to fall back to preferred
            return Ok(EnforcementLevel::best_effort(
                "mbind failed with EPERM, policy not enforced",
            ));
        }

        return Err(NumaError::bind(err));
    }

    // Determine enforcement level
    let enforcement = match policy {
        MemPolicy::Bind(_) => {
            // Bind requires CAP_SYS_ADMIN for strict enforcement
            // We succeeded, but we can't know if it's truly strict without checking caps
            EnforcementLevel::Strict
        }
        MemPolicy::Preferred(_) | MemPolicy::Interleave(_) | MemPolicy::Local => {
            EnforcementLevel::Strict
        }
    };

    Ok(enforcement)
}

/// Convert MemPolicy to mbind arguments.
#[cfg(target_os = "linux")]
fn policy_to_mbind_args(policy: &MemPolicy) -> (i32, Vec<libc::c_ulong>, libc::c_ulong) {
    match policy {
        MemPolicy::Bind(mask) => {
            let (nodemask, maxnode) = node_mask_to_bits(mask);
            (mpol::MPOL_BIND, nodemask, maxnode)
        }
        MemPolicy::Preferred(node) => {
            let mask = numaperf_core::NodeMask::single(*node);
            let (nodemask, maxnode) = node_mask_to_bits(&mask);
            (mpol::MPOL_PREFERRED, nodemask, maxnode)
        }
        MemPolicy::Interleave(mask) => {
            let (nodemask, maxnode) = node_mask_to_bits(mask);
            (mpol::MPOL_INTERLEAVE, nodemask, maxnode)
        }
        MemPolicy::Local => (mpol::MPOL_LOCAL, vec![0], 0),
    }
}

/// Convert NodeMask to the format expected by mbind.
#[cfg(target_os = "linux")]
fn node_mask_to_bits(mask: &NodeMask) -> (Vec<libc::c_ulong>, libc::c_ulong) {
    let bits = mask.as_raw();

    // mbind expects an array of unsigned longs
    // On 64-bit Linux, c_ulong is 64 bits
    let nodemask = vec![bits as libc::c_ulong];
    let maxnode = 64 as libc::c_ulong;

    (nodemask, maxnode)
}

// Non-Linux fallbacks

#[cfg(not(target_os = "linux"))]
pub fn mmap_anon(size: usize, _huge: HugePageMode) -> Result<MmapResult, NumaError> {
    // Use standard allocation on non-Linux
    let layout = std::alloc::Layout::from_size_align(size, 4096)
        .map_err(|_| NumaError::invalid_argument("invalid size/alignment"))?;

    let ptr = unsafe { std::alloc::alloc_zeroed(layout) };
    if ptr.is_null() {
        return Err(NumaError::allocation(
            size,
            io::Error::new(io::ErrorKind::OutOfMemory, "allocation failed"),
        ));
    }

    Ok(MmapResult { ptr, len: size })
}

#[cfg(not(target_os = "linux"))]
pub fn munmap(ptr: *mut u8, len: usize) -> Result<(), NumaError> {
    let layout = std::alloc::Layout::from_size_align(len, 4096)
        .map_err(|_| NumaError::invalid_argument("invalid size/alignment"))?;

    unsafe {
        std::alloc::dealloc(ptr, layout);
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn mbind(
    _ptr: *mut u8,
    _len: usize,
    _policy: &MemPolicy,
) -> Result<EnforcementLevel, NumaError> {
    // No NUMA support on non-Linux
    Ok(EnforcementLevel::none(
        "NUMA not supported on this platform",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mmap_munmap() {
        let size = 4096 * 10;
        let result = mmap_anon(size, HugePageMode::None).expect("mmap should succeed");

        assert!(!result.ptr.is_null());
        assert_eq!(result.len, size);

        // Write to verify it's accessible
        unsafe {
            for i in 0..size {
                result.ptr.add(i).write(0x42);
            }
        }

        munmap(result.ptr, result.len).expect("munmap should succeed");
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_mbind_local() {
        let size = 4096 * 10;
        let result = mmap_anon(size, HugePageMode::None).expect("mmap should succeed");

        let enforcement =
            mbind(result.ptr, result.len, &MemPolicy::Local).expect("mbind should succeed");

        assert!(enforcement.is_strict() || enforcement.is_best_effort());

        munmap(result.ptr, result.len).expect("munmap should succeed");
    }
}
