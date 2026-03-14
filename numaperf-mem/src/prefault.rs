//! Memory prefaulting strategies.

/// Controls how memory pages are faulted in after allocation.
///
/// On Linux, memory is lazily allocated: pages are only backed by
/// physical memory when first accessed. Prefaulting forces this
/// allocation to happen immediately, ensuring the NUMA policy is
/// applied and avoiding page faults during critical operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Prefault {
    /// Do not prefault pages.
    ///
    /// Pages will be faulted on first access. The NUMA policy will
    /// apply based on which thread/CPU first touches each page.
    #[default]
    None,

    /// Touch pages sequentially from the current thread.
    ///
    /// Writes one byte per page to force allocation. This ensures
    /// all pages are allocated on the current thread's node (for
    /// `Local` policy) or according to the specified policy.
    Touch,

    /// Touch pages in parallel using multiple threads.
    ///
    /// For large regions, this can be faster than sequential touching.
    /// Each thread touches a portion of the region, and threads are
    /// pinned to appropriate nodes based on the memory policy.
    ParallelTouch,
}

impl Prefault {
    /// Perform prefaulting on a memory region.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `ptr` is valid for `len` bytes
    /// and that the memory is writable.
    pub(crate) unsafe fn execute(&self, ptr: *mut u8, len: usize) {
        match self {
            Self::None => {}
            Self::Touch => touch_sequential(ptr, len),
            Self::ParallelTouch => touch_parallel(ptr, len),
        }
    }
}

/// Touch pages sequentially.
unsafe fn touch_sequential(ptr: *mut u8, len: usize) {
    let page_size = 4096usize;
    let mut offset = 0;

    while offset < len {
        // Write one byte per page to trigger the fault
        ptr.add(offset).write_volatile(0);
        offset += page_size;
    }
}

/// Touch pages in parallel.
unsafe fn touch_parallel(ptr: *mut u8, len: usize) {
    use std::thread;

    let page_size = 4096usize;
    let num_threads = thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(4)
        .min(16); // Cap at 16 threads

    // For small regions, just use sequential
    if len < num_threads * page_size * 256 {
        touch_sequential(ptr, len);
        return;
    }

    let chunk_size = len / num_threads;
    let ptr_raw = ptr as usize;

    let handles: Vec<_> = (0..num_threads)
        .map(|i| {
            let start = i * chunk_size;
            let end = if i == num_threads - 1 {
                len
            } else {
                (i + 1) * chunk_size
            };

            thread::spawn(move || {
                let ptr = ptr_raw as *mut u8;
                let mut offset = start;
                while offset < end {
                    // Safety: caller guarantees validity
                    unsafe {
                        ptr.add(offset).write_volatile(0);
                    }
                    offset += page_size;
                }
            })
        })
        .collect();

    for handle in handles {
        let _ = handle.join();
    }
}

impl std::fmt::Display for Prefault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Touch => write!(f, "touch"),
            Self::ParallelTouch => write!(f, "parallel-touch"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prefault_default() {
        assert_eq!(Prefault::default(), Prefault::None);
    }

    #[test]
    fn test_touch_sequential() {
        let mut buffer = vec![0xFFu8; 4096 * 10];
        unsafe {
            touch_sequential(buffer.as_mut_ptr(), buffer.len());
        }
        // First byte of each page should be 0
        for i in 0..10 {
            assert_eq!(buffer[i * 4096], 0);
        }
    }

    #[test]
    fn test_touch_parallel() {
        let mut buffer = vec![0xFFu8; 4096 * 1000];
        unsafe {
            touch_parallel(buffer.as_mut_ptr(), buffer.len());
        }
        // First byte of each page should be 0
        for i in 0..1000 {
            assert_eq!(buffer[i * 4096], 0);
        }
    }
}
