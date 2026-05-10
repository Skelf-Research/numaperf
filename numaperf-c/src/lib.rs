#![allow(clippy::not_unsafe_ptr_arg_deref)]

//! C FFI bindings for numaperf.
//!
//! This crate exposes a stable C API for:
//! - NUMA topology discovery
//! - Thread CPU affinity pinning
//! - NUMA-aware memory allocation
//!
//! All functions that can fail return an `int` where `0` means success and
//! negative values indicate specific error codes. Use `npa_error_string()`
//! to get a human-readable description.

use std::cell::RefCell;
use std::ffi::{c_char, c_void, CString};
use std::ptr;

use numaperf_affinity::{get_affinity, set_affinity};
use numaperf_core::{CpuSet, NodeId, NodeMask, NumaError};
use numaperf_mem::{HugePageMode, MemPolicy, NumaRegion, Prefault};
use numaperf_topo::Topology;

// =============================================================================
// Error Handling
// =============================================================================

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

fn set_last_error(err: &NumaError) {
    let msg = format!("{}", err);
    if let Ok(cstr) = CString::new(msg) {
        LAST_ERROR.with(|e| *e.borrow_mut() = Some(cstr));
    }
}

/// Get a human-readable description of an error code.
///
/// Returns a pointer to a static string. The returned pointer is valid for
/// the lifetime of the program and must not be freed.
///
/// # Arguments
///
/// * `code` - The error code returned by a numaperf function.
#[no_mangle]
pub extern "C" fn npa_error_string(code: i32) -> *const c_char {
    match code {
        0 => "success\0".as_ptr() as *const c_char,
        -1 => "invalid argument\0".as_ptr() as *const c_char,
        -2 => "topology discovery failed\0".as_ptr() as *const c_char,
        -3 => "allocation failed\0".as_ptr() as *const c_char,
        -4 => "thread pinning failed\0".as_ptr() as *const c_char,
        -5 => "memory policy not supported\0".as_ptr() as *const c_char,
        -6 => "missing capability\0".as_ptr() as *const c_char,
        -7 => "hard mode unavailable\0".as_ptr() as *const c_char,
        -8 => "feature not supported on this platform\0".as_ptr() as *const c_char,
        -9 => "I/O error\0".as_ptr() as *const c_char,
        _ => "unknown error\0".as_ptr() as *const c_char,
    }
}

/// Get the last error message for the current thread.
///
/// Returns a pointer to a thread-local C string. The pointer is valid until
/// the next numaperf call on the same thread. It must not be freed.
#[no_mangle]
pub extern "C" fn npa_last_error() -> *const c_char {
    LAST_ERROR.with(|e| {
        e.borrow()
            .as_ref()
            .map(|s| s.as_ptr())
            .unwrap_or_else(|| "no error\0".as_ptr() as *const c_char)
    })
}

fn map_error(err: NumaError) -> i32 {
    set_last_error(&err);
    match err {
        NumaError::InvalidArgument { .. } => -1,
        NumaError::TopologyError { .. } => -2,
        NumaError::AllocationFailed { .. } => -3,
        NumaError::PinningFailed { .. } => -4,
        NumaError::PolicyNotSupported { .. } => -5,
        NumaError::CapabilityMissing { .. } => -6,
        NumaError::HardModeUnavailable { .. } => -7,
        NumaError::NotSupported { .. } => -8,
        NumaError::Io(_) | NumaError::BindFailed { .. } => -9,
    }
}

// =============================================================================
// Types
// =============================================================================

/// Opaque handle to a discovered NUMA topology.
pub enum NpaTopology {}

/// Opaque handle to a NUMA memory region.
pub enum NpaRegion {}

/// A set of up to 1024 CPUs, represented as a bitmap.
///
/// Use `npa_cpuset_add()` and `npa_cpuset_contains()` to manipulate.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct NpaCpuSet {
    bits: [u64; 16],
}

impl From<&CpuSet> for NpaCpuSet {
    fn from(cpus: &CpuSet) -> Self {
        let raw = cpus.as_raw();
        let mut bits = [0u64; 16];
        for (i, &v) in raw.iter().enumerate() {
            bits[i] = v;
        }
        Self { bits }
    }
}

impl From<&NpaCpuSet> for CpuSet {
    fn from(cpus: &NpaCpuSet) -> Self {
        let mut set = CpuSet::new();
        for cpu in 0..1024u32 {
            let idx = cpu as usize / 64;
            let bit = cpu as usize % 64;
            if (cpus.bits[idx] & (1u64 << bit)) != 0 {
                set.add(cpu);
            }
        }
        set
    }
}

/// A set of up to 64 NUMA nodes, represented as a bitmap.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct NpaNodeMask {
    bits: u64,
}

impl From<&NodeMask> for NpaNodeMask {
    fn from(mask: &NodeMask) -> Self {
        Self {
            bits: mask.as_raw(),
        }
    }
}

impl From<&NpaNodeMask> for NodeMask {
    fn from(mask: &NpaNodeMask) -> Self {
        NodeMask::from_raw(mask.bits)
    }
}

// =============================================================================
// Topology
// =============================================================================

/// Discover the system's NUMA topology.
///
/// Returns an opaque handle on success, or NULL on failure. Check
/// `npa_last_error()` for details on failure.
///
/// The returned handle must be freed with `npa_topology_free()`.
#[no_mangle]
pub extern "C" fn npa_topology_discover() -> *mut NpaTopology {
    match Topology::discover() {
        Ok(topo) => Box::into_raw(Box::new(topo)) as *mut NpaTopology,
        Err(e) => {
            let _ = map_error(e);
            ptr::null_mut()
        }
    }
}

/// Free a topology handle obtained from `npa_topology_discover()`.
///
/// Passing NULL is a no-op.
#[no_mangle]
pub extern "C" fn npa_topology_free(topo: *mut NpaTopology) {
    if !topo.is_null() {
        unsafe {
            let _ = Box::from_raw(topo as *mut Topology);
        }
    }
}

/// Get the number of NUMA nodes.
#[no_mangle]
pub extern "C" fn npa_topology_node_count(topo: *const NpaTopology) -> u32 {
    if topo.is_null() {
        return 0;
    }
    let topo = unsafe { &*(topo as *const Topology) };
    topo.node_count() as u32
}

/// Get the total number of CPUs across all nodes.
#[no_mangle]
pub extern "C" fn npa_topology_cpu_count(topo: *const NpaTopology) -> u32 {
    if topo.is_null() {
        return 0;
    }
    let topo = unsafe { &*(topo as *const Topology) };
    topo.cpu_count() as u32
}

/// Get the ID of a NUMA node by index.
///
/// Returns the node ID on success, or `u32::MAX` if `idx` is out of range.
#[no_mangle]
pub extern "C" fn npa_topology_node_id(topo: *const NpaTopology, idx: u32) -> u32 {
    if topo.is_null() {
        return u32::MAX;
    }
    let topo = unsafe { &*(topo as *const Topology) };
    match topo.numa_nodes().get(idx as usize) {
        Some(node) => node.id().as_u32(),
        None => u32::MAX,
    }
}

/// Get the number of CPUs on a specific node.
///
/// Returns the CPU count on success, or `0` if the node does not exist.
#[no_mangle]
pub extern "C" fn npa_topology_node_cpu_count(topo: *const NpaTopology, node_id: u32) -> u32 {
    if topo.is_null() {
        return 0;
    }
    let topo = unsafe { &*(topo as *const Topology) };
    match topo.node(NodeId::new(node_id)) {
        Some(node) => node.cpu_count() as u32,
        None => 0,
    }
}

/// Get the CPU set for a specific node.
///
/// Returns `0` on success, or a negative error code. The `cpus` struct
/// must be valid and will be written to.
#[no_mangle]
pub extern "C" fn npa_topology_node_cpus(
    topo: *const NpaTopology,
    node_id: u32,
    cpus: *mut NpaCpuSet,
) -> i32 {
    if topo.is_null() || cpus.is_null() {
        return -1;
    }
    let topo = unsafe { &*(topo as *const Topology) };
    match topo.node(NodeId::new(node_id)) {
        Some(node) => {
            let cs: NpaCpuSet = node.cpus().into();
            unsafe {
                *cpus = cs;
            }
            0
        }
        None => -1,
    }
}

/// Get the NUMA distance between two nodes.
///
/// Returns the distance value on success, or `u32::MAX` if unavailable.
/// A distance of 10 typically indicates local access.
#[no_mangle]
pub extern "C" fn npa_topology_node_distance(topo: *const NpaTopology, from: u32, to: u32) -> u32 {
    if topo.is_null() {
        return u32::MAX;
    }
    let topo = unsafe { &*(topo as *const Topology) };
    match topo.node(NodeId::new(from)) {
        Some(node) => node.distance_to(NodeId::new(to)).unwrap_or(u32::MAX),
        None => u32::MAX,
    }
}

/// Get the total memory in bytes for a specific node.
///
/// Returns the memory size on success, or `0` if unavailable.
#[no_mangle]
pub extern "C" fn npa_topology_node_memory_bytes(topo: *const NpaTopology, node_id: u32) -> u64 {
    if topo.is_null() {
        return 0;
    }
    let topo = unsafe { &*(topo as *const Topology) };
    match topo.node(NodeId::new(node_id)) {
        Some(node) => node.memory_bytes().unwrap_or(0),
        None => 0,
    }
}

/// Get the NUMA node that contains a specific CPU.
///
/// Returns the node ID on success, or `u32::MAX` if the CPU is not found.
#[no_mangle]
pub extern "C" fn npa_topology_node_for_cpu(topo: *const NpaTopology, cpu: u32) -> u32 {
    if topo.is_null() {
        return u32::MAX;
    }
    let topo = unsafe { &*(topo as *const Topology) };
    match topo.node_for_cpu(cpu) {
        Some(node) => node.as_u32(),
        None => u32::MAX,
    }
}

// =============================================================================
// CPU Set Helpers
// =============================================================================

/// Create an empty CPU set.
#[no_mangle]
pub extern "C" fn npa_cpuset_new() -> NpaCpuSet {
    NpaCpuSet::default()
}

/// Add a CPU to a CPU set.
#[no_mangle]
pub extern "C" fn npa_cpuset_add(cpus: *mut NpaCpuSet, cpu: u32) {
    if !cpus.is_null() {
        let idx = cpu as usize / 64;
        let bit = cpu as usize % 64;
        if idx < 16 {
            unsafe {
                (*cpus).bits[idx] |= 1u64 << bit;
            }
        }
    }
}

/// Remove a CPU from a CPU set.
#[no_mangle]
pub extern "C" fn npa_cpuset_remove(cpus: *mut NpaCpuSet, cpu: u32) {
    if !cpus.is_null() {
        let idx = cpu as usize / 64;
        let bit = cpu as usize % 64;
        if idx < 16 {
            unsafe {
                (*cpus).bits[idx] &= !(1u64 << bit);
            }
        }
    }
}

/// Check if a CPU is in a CPU set.
#[no_mangle]
pub extern "C" fn npa_cpuset_contains(cpus: *const NpaCpuSet, cpu: u32) -> i32 {
    if cpus.is_null() {
        return 0;
    }
    let idx = cpu as usize / 64;
    let bit = cpu as usize % 64;
    if idx >= 16 {
        return 0;
    }
    let val = unsafe { (*cpus).bits[idx] };
    if (val & (1u64 << bit)) != 0 {
        1
    } else {
        0
    }
}

/// Count the number of CPUs in a CPU set.
#[no_mangle]
pub extern "C" fn npa_cpuset_count(cpus: *const NpaCpuSet) -> u32 {
    if cpus.is_null() {
        return 0;
    }
    let set = unsafe { &*cpus };
    set.bits.iter().map(|b| b.count_ones()).sum::<u32>()
}

/// Build a CPU set for a single CPU.
#[no_mangle]
pub extern "C" fn npa_cpuset_single(cpu: u32) -> NpaCpuSet {
    let mut set = NpaCpuSet::default();
    npa_cpuset_add(&mut set, cpu);
    set
}

// =============================================================================
// Node Mask Helpers
// =============================================================================

/// Create an empty node mask.
#[no_mangle]
pub extern "C" fn npa_nodemask_new() -> NpaNodeMask {
    NpaNodeMask::default()
}

/// Add a node to a node mask.
#[no_mangle]
pub extern "C" fn npa_nodemask_add(mask: *mut NpaNodeMask, node: u32) {
    if !mask.is_null() && node < 64 {
        unsafe {
            (*mask).bits |= 1u64 << node;
        }
    }
}

/// Remove a node from a node mask.
#[no_mangle]
pub extern "C" fn npa_nodemask_remove(mask: *mut NpaNodeMask, node: u32) {
    if !mask.is_null() && node < 64 {
        unsafe {
            (*mask).bits &= !(1u64 << node);
        }
    }
}

/// Check if a node is in a node mask.
#[no_mangle]
pub extern "C" fn npa_nodemask_contains(mask: *const NpaNodeMask, node: u32) -> i32 {
    if mask.is_null() || node >= 64 {
        return 0;
    }
    let val = unsafe { (*mask).bits };
    if (val & (1u64 << node)) != 0 {
        1
    } else {
        0
    }
}

/// Count the number of nodes in a node mask.
#[no_mangle]
pub extern "C" fn npa_nodemask_count(mask: *const NpaNodeMask) -> u32 {
    if mask.is_null() {
        return 0;
    }
    let val = unsafe { (*mask).bits };
    val.count_ones()
}

/// Build a node mask containing a single node.
#[no_mangle]
pub extern "C" fn npa_nodemask_single(node: u32) -> NpaNodeMask {
    let mut mask = NpaNodeMask::default();
    npa_nodemask_add(&mut mask, node);
    mask
}

// =============================================================================
// Affinity
// =============================================================================

/// Pin the current thread to a set of CPUs.
///
/// Returns `0` on success, or a negative error code. The previous affinity
/// is NOT saved. Use `npa_unpin_thread()` with the original CPU set to
/// restore, or call `npa_get_affinity()` before pinning.
#[no_mangle]
pub extern "C" fn npa_pin_thread(cpus: *const NpaCpuSet) -> i32 {
    if cpus.is_null() {
        return -1;
    }
    let cpus: CpuSet = unsafe { &*cpus }.into();
    match set_affinity(&cpus) {
        Ok(()) => 0,
        Err(e) => map_error(e),
    }
}

/// Get the current thread's CPU affinity.
///
/// Returns `0` on success, or a negative error code. The `cpus` struct
/// must be valid and will be written to.
#[no_mangle]
pub extern "C" fn npa_get_affinity(cpus: *mut NpaCpuSet) -> i32 {
    if cpus.is_null() {
        return -1;
    }
    match get_affinity() {
        Ok(set) => {
            let cs: NpaCpuSet = (&set).into();
            unsafe {
                *cpus = cs;
            }
            0
        }
        Err(e) => map_error(e),
    }
}

/// Set the current thread's CPU affinity.
///
/// Alias for `npa_pin_thread()`. Provided for clarity.
#[no_mangle]
pub extern "C" fn npa_set_affinity(cpus: *const NpaCpuSet) -> i32 {
    npa_pin_thread(cpus)
}

// =============================================================================
// Memory
// =============================================================================

/// Memory placement policy.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NpaMemPolicy {
    /// Use the current thread's local NUMA node.
    Local = 0,
    /// Strictly bind to the nodes in the node mask.
    Bind = 1,
    /// Prefer the specified node, with fallback allowed.
    Preferred = 2,
    /// Interleave pages across the nodes in the node mask.
    Interleave = 3,
}

/// Huge page configuration.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NpaHugePageMode {
    /// No huge pages.
    None = 0,
    /// Enable transparent huge pages.
    TransparentOn = 1,
    /// Disable transparent huge pages.
    TransparentOff = 2,
    /// Explicit 2 MB huge pages.
    Explicit2MB = 3,
    /// Explicit 1 GB huge pages.
    Explicit1GB = 4,
}

impl From<NpaHugePageMode> for HugePageMode {
    fn from(mode: NpaHugePageMode) -> Self {
        match mode {
            NpaHugePageMode::None => HugePageMode::None,
            NpaHugePageMode::TransparentOn => HugePageMode::TransparentOn,
            NpaHugePageMode::TransparentOff => HugePageMode::TransparentOff,
            NpaHugePageMode::Explicit2MB => HugePageMode::Explicit2MB,
            NpaHugePageMode::Explicit1GB => HugePageMode::Explicit1GB,
        }
    }
}

/// Memory prefault strategy.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NpaPrefault {
    /// Do not prefault pages.
    None = 0,
    /// Touch pages sequentially from the current thread.
    Touch = 1,
    /// Touch pages in parallel using multiple threads.
    ParallelTouch = 2,
}

impl From<NpaPrefault> for Prefault {
    fn from(p: NpaPrefault) -> Self {
        match p {
            NpaPrefault::None => Prefault::None,
            NpaPrefault::Touch => Prefault::Touch,
            NpaPrefault::ParallelTouch => Prefault::ParallelTouch,
        }
    }
}

/// Allocate a NUMA-aware memory region.
///
/// Returns an opaque handle on success, or NULL on failure. Check
/// `npa_last_error()` for details on failure.
///
/// The returned handle must be freed with `npa_region_free()`.
///
/// # Arguments
///
/// * `size` - Size of the region in bytes.
/// * `policy` - Memory placement policy.
/// * `node_mask` - Node mask for `Bind` and `Interleave` policies. Ignored for
///   `Local` and `Preferred`.
/// * `preferred_node` - Node ID for `Preferred` policy. Ignored for others.
/// * `huge_mode` - Huge page configuration.
/// * `prefault` - Prefault strategy.
#[no_mangle]
pub extern "C" fn npa_region_alloc(
    size: u64,
    policy: NpaMemPolicy,
    node_mask: NpaNodeMask,
    preferred_node: u32,
    huge_mode: NpaHugePageMode,
    prefault: NpaPrefault,
) -> *mut NpaRegion {
    let mem_policy = match policy {
        NpaMemPolicy::Local => MemPolicy::Local,
        NpaMemPolicy::Bind => MemPolicy::Bind((&node_mask).into()),
        NpaMemPolicy::Preferred => MemPolicy::Preferred(NodeId::new(preferred_node)),
        NpaMemPolicy::Interleave => MemPolicy::Interleave((&node_mask).into()),
    };

    match NumaRegion::anon(size as usize, mem_policy, huge_mode.into(), prefault.into()) {
        Ok(region) => Box::into_raw(Box::new(region)) as *mut NpaRegion,
        Err(e) => {
            let _ = map_error(e);
            ptr::null_mut()
        }
    }
}

/// Free a memory region obtained from `npa_region_alloc()`.
///
/// Passing NULL is a no-op.
#[no_mangle]
pub extern "C" fn npa_region_free(region: *mut NpaRegion) {
    if !region.is_null() {
        unsafe {
            let _ = Box::from_raw(region as *mut NumaRegion);
        }
    }
}

/// Get a pointer to the memory region's data.
///
/// Returns NULL if the region handle is invalid.
#[no_mangle]
pub extern "C" fn npa_region_ptr(region: *const NpaRegion) -> *mut c_void {
    if region.is_null() {
        return ptr::null_mut();
    }
    let region = unsafe { &*(region as *const NumaRegion) };
    region.as_ptr() as *mut c_void
}

/// Get the size of a memory region in bytes.
///
/// Returns `0` if the region handle is invalid.
#[no_mangle]
pub extern "C" fn npa_region_len(region: *const NpaRegion) -> u64 {
    if region.is_null() {
        return 0;
    }
    let region = unsafe { &*(region as *const NumaRegion) };
    region.len() as u64
}

/// Get the human-readable name of the memory policy applied to a region.
///
/// Returns a pointer to a static string, or NULL on error.
#[no_mangle]
pub extern "C" fn npa_region_policy_name(region: *const NpaRegion) -> *const c_char {
    if region.is_null() {
        return ptr::null();
    }
    let region = unsafe { &*(region as *const NumaRegion) };
    match region.policy() {
        MemPolicy::Bind(_) => "Bind\0".as_ptr() as *const c_char,
        MemPolicy::Preferred(_) => "Preferred\0".as_ptr() as *const c_char,
        MemPolicy::Interleave(_) => "Interleave\0".as_ptr() as *const c_char,
        MemPolicy::Local => "Local\0".as_ptr() as *const c_char,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topology_discover_and_free() {
        let topo = npa_topology_discover();
        assert!(!topo.is_null());
        assert!(npa_topology_node_count(topo) >= 1);
        npa_topology_free(topo);
    }

    #[test]
    fn test_cpuset_helpers() {
        let mut set = npa_cpuset_new();
        npa_cpuset_add(&mut set, 5);
        npa_cpuset_add(&mut set, 10);
        assert_eq!(npa_cpuset_contains(&set, 5), 1);
        assert_eq!(npa_cpuset_contains(&set, 10), 1);
        assert_eq!(npa_cpuset_contains(&set, 6), 0);
        assert_eq!(npa_cpuset_count(&set), 2);

        npa_cpuset_remove(&mut set, 5);
        assert_eq!(npa_cpuset_contains(&set, 5), 0);
        assert_eq!(npa_cpuset_count(&set), 1);
    }

    #[test]
    fn test_nodemask_helpers() {
        let mut mask = npa_nodemask_new();
        npa_nodemask_add(&mut mask, 0);
        npa_nodemask_add(&mut mask, 2);
        assert_eq!(npa_nodemask_contains(&mask, 0), 1);
        assert_eq!(npa_nodemask_contains(&mask, 2), 1);
        assert_eq!(npa_nodemask_contains(&mask, 1), 0);
        assert_eq!(npa_nodemask_count(&mask), 2);
    }

    #[test]
    fn test_region_alloc_and_free() {
        let region = npa_region_alloc(
            4096,
            NpaMemPolicy::Local,
            npa_nodemask_new(),
            0,
            NpaHugePageMode::None,
            NpaPrefault::Touch,
        );
        assert!(!region.is_null());
        assert_eq!(npa_region_len(region), 4096);
        assert!(!npa_region_ptr(region).is_null());
        npa_region_free(region);
    }

    #[test]
    fn test_error_codes() {
        assert!(!npa_error_string(0).is_null());
        assert!(!npa_error_string(-1).is_null());
        assert!(!npa_error_string(-99).is_null());
    }

    #[test]
    fn test_affinity_roundtrip() {
        let mut cpus = npa_cpuset_new();
        let result = npa_get_affinity(&mut cpus);

        // On Linux, affinity should work
        #[cfg(target_os = "linux")]
        {
            assert_eq!(result, 0);
            assert!(npa_cpuset_count(&cpus) >= 1);
        }

        // On non-Linux platforms, it may return NotSupported
        #[cfg(not(target_os = "linux"))]
        {
            assert!(result == 0 || result == -8);
        }
    }
}
