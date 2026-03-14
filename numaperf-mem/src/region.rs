//! NUMA memory region.

use std::slice;

use numaperf_core::{EnforcementLevel, HardMode, NumaError};

use crate::huge::HugePageMode;
use crate::policy::MemPolicy;
use crate::prefault::Prefault;
use crate::syscall;

/// A memory region with explicit NUMA placement.
///
/// `NumaRegion` owns a contiguous block of memory that has been allocated
/// with a specific NUMA policy. The memory is automatically unmapped when
/// the region is dropped.
///
/// # Thread Safety
///
/// `NumaRegion` is `Send` but not `Sync`. You can transfer ownership to
/// another thread, but concurrent access requires external synchronization.
///
/// # Example
///
/// ```no_run
/// use numaperf_mem::{NumaRegion, MemPolicy, HugePageMode, Prefault};
/// use numaperf_core::NodeId;
///
/// // Allocate 64 MB on node 0
/// let mut region = NumaRegion::anon(
///     64 * 1024 * 1024,
///     MemPolicy::Preferred(NodeId::new(0)),
///     HugePageMode::TransparentOn,
///     Prefault::Touch,
/// )?;
///
/// // Write to the region
/// let slice = region.as_mut_slice();
/// slice[0] = 42;
/// # Ok::<(), numaperf_core::NumaError>(())
/// ```
pub struct NumaRegion {
    ptr: *mut u8,
    len: usize,
    policy: MemPolicy,
    enforcement: EnforcementLevel,
}

impl NumaRegion {
    /// Allocate an anonymous NUMA region with the specified policy.
    ///
    /// This uses soft mode by default, which will succeed even if the
    /// policy cannot be strictly enforced.
    ///
    /// # Arguments
    ///
    /// * `size` - The size of the region in bytes
    /// * `policy` - The NUMA memory policy to apply
    /// * `huge` - Huge page mode
    /// * `prefault` - Whether and how to prefault pages
    ///
    /// # Errors
    ///
    /// Returns an error if memory allocation fails.
    pub fn anon(
        size: usize,
        policy: MemPolicy,
        huge: HugePageMode,
        prefault: Prefault,
    ) -> Result<Self, NumaError> {
        Self::anon_with_mode(size, policy, huge, prefault, HardMode::Soft)
    }

    /// Allocate an anonymous NUMA region with explicit hard/soft mode.
    ///
    /// # Arguments
    ///
    /// * `size` - The size of the region in bytes
    /// * `policy` - The NUMA memory policy to apply
    /// * `huge` - Huge page mode
    /// * `prefault` - Whether and how to prefault pages
    /// * `mode` - Hard or soft enforcement mode
    ///
    /// # Errors
    ///
    /// In soft mode, returns an error only if allocation fails.
    /// In hard mode, also returns an error if the policy cannot be enforced.
    pub fn anon_with_mode(
        size: usize,
        policy: MemPolicy,
        huge: HugePageMode,
        prefault: Prefault,
        mode: HardMode,
    ) -> Result<Self, NumaError> {
        if size == 0 {
            return Err(NumaError::invalid_argument("size cannot be zero"));
        }

        // Allocate memory
        let result = syscall::mmap_anon(size, huge)?;

        // Apply NUMA policy
        let enforcement = match syscall::mbind(result.ptr, result.len, &policy) {
            Ok(level) => level,
            Err(e) => {
                // Clean up on failure
                let _ = syscall::munmap(result.ptr, result.len);

                if mode.is_strict() {
                    return Err(e);
                }

                // In soft mode, continue with no enforcement
                EnforcementLevel::none(format!("mbind failed: {}", e))
            }
        };

        // Check enforcement in hard mode
        if mode.is_strict() && !enforcement.is_strict() {
            let _ = syscall::munmap(result.ptr, result.len);
            return Err(NumaError::hard_mode_unavailable(
                "memory policy",
                enforcement.reason().unwrap_or("unknown reason").to_string(),
            ));
        }

        // Prefault pages
        unsafe {
            prefault.execute(result.ptr, result.len);
        }

        Ok(Self {
            ptr: result.ptr,
            len: result.len,
            policy,
            enforcement,
        })
    }

    /// Get the size of the region in bytes.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if the region is empty (size is 0).
    ///
    /// Note: This should never be true for a valid NumaRegion.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get the memory policy applied to this region.
    #[inline]
    pub fn policy(&self) -> &MemPolicy {
        &self.policy
    }

    /// Get the enforcement level achieved for this region.
    #[inline]
    pub fn enforcement(&self) -> &EnforcementLevel {
        &self.enforcement
    }

    /// Get the region as a byte slice.
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.ptr, self.len) }
    }

    /// Get the region as a mutable byte slice.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.ptr, self.len) }
    }

    /// Get the raw pointer to the region.
    ///
    /// # Safety
    ///
    /// The caller must ensure proper lifetime and synchronization.
    #[inline]
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr
    }

    /// Get the raw mutable pointer to the region.
    ///
    /// # Safety
    ///
    /// The caller must ensure proper lifetime and synchronization.
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.ptr
    }

    /// Cast the region to a slice of a specific type.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    /// - The type `T` has compatible alignment
    /// - The region size is a multiple of `size_of::<T>()`
    /// - The memory contents are valid for type `T`
    pub unsafe fn as_typed_slice<T>(&self) -> &[T] {
        let count = self.len / std::mem::size_of::<T>();
        slice::from_raw_parts(self.ptr as *const T, count)
    }

    /// Cast the region to a mutable slice of a specific type.
    ///
    /// # Safety
    ///
    /// Same requirements as `as_typed_slice`, plus the caller must ensure
    /// exclusive access.
    pub unsafe fn as_typed_slice_mut<T>(&mut self) -> &mut [T] {
        let count = self.len / std::mem::size_of::<T>();
        slice::from_raw_parts_mut(self.ptr as *mut T, count)
    }
}

impl Drop for NumaRegion {
    fn drop(&mut self) {
        // Best effort unmapping - ignore errors
        let _ = syscall::munmap(self.ptr, self.len);
    }
}

// NumaRegion can be sent to another thread
unsafe impl Send for NumaRegion {}

// NumaRegion is NOT Sync - concurrent access requires external synchronization

impl std::fmt::Debug for NumaRegion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NumaRegion")
            .field("len", &self.len)
            .field("policy", &self.policy)
            .field("enforcement", &self.enforcement)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use numaperf_core::NodeId;

    #[test]
    fn test_region_basic() {
        let region = NumaRegion::anon(
            4096 * 10,
            MemPolicy::Local,
            HugePageMode::None,
            Prefault::None,
        )
        .expect("allocation should succeed");

        assert_eq!(region.len(), 4096 * 10);
        assert!(!region.is_empty());
        assert_eq!(region.policy(), &MemPolicy::Local);
    }

    #[test]
    fn test_region_write_read() {
        let mut region =
            NumaRegion::anon(4096, MemPolicy::Local, HugePageMode::None, Prefault::Touch)
                .expect("allocation should succeed");

        let slice = region.as_mut_slice();
        slice[0] = 42;
        slice[4095] = 99;

        let slice = region.as_slice();
        assert_eq!(slice[0], 42);
        assert_eq!(slice[4095], 99);
    }

    #[test]
    fn test_region_with_policy() {
        let region = NumaRegion::anon(
            4096 * 100,
            MemPolicy::Preferred(NodeId::new(0)),
            HugePageMode::None,
            Prefault::Touch,
        )
        .expect("allocation should succeed");

        assert_eq!(region.policy(), &MemPolicy::Preferred(NodeId::new(0)));
    }

    #[test]
    fn test_region_zero_size() {
        let result = NumaRegion::anon(0, MemPolicy::Local, HugePageMode::None, Prefault::None);

        assert!(result.is_err());
    }

    #[test]
    fn test_region_large() {
        // Allocate 10 MB
        let region = NumaRegion::anon(
            10 * 1024 * 1024,
            MemPolicy::Local,
            HugePageMode::None,
            Prefault::ParallelTouch,
        )
        .expect("allocation should succeed");

        assert_eq!(region.len(), 10 * 1024 * 1024);
    }
}
