//! Cache-line padded wrapper to prevent false sharing.

use std::ops::{Deref, DerefMut};

/// A wrapper that ensures the contained value is aligned to a cache line boundary.
///
/// This prevents false sharing when multiple threads access adjacent memory
/// locations. The alignment of 128 bytes is conservative and covers most
/// modern CPU architectures (x86-64 uses 64 bytes, some ARM uses 128).
///
/// # Example
///
/// ```
/// use numaperf_sharded::CachePadded;
/// use std::sync::atomic::{AtomicU64, Ordering};
///
/// let padded = CachePadded::new(AtomicU64::new(0));
/// padded.fetch_add(1, Ordering::Relaxed);
/// assert_eq!(padded.load(Ordering::Relaxed), 1);
/// ```
#[repr(align(128))]
pub struct CachePadded<T> {
    value: T,
}

impl<T> CachePadded<T> {
    /// Create a new cache-padded value.
    #[inline]
    pub const fn new(value: T) -> Self {
        Self { value }
    }

    /// Get a reference to the inner value.
    #[inline]
    pub fn get(&self) -> &T {
        &self.value
    }

    /// Get a mutable reference to the inner value.
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.value
    }

    /// Consume the wrapper and return the inner value.
    #[inline]
    pub fn into_inner(self) -> T {
        self.value
    }
}

impl<T> Deref for CachePadded<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for CachePadded<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<T: Default> Default for CachePadded<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: Clone> Clone for CachePadded<T> {
    fn clone(&self) -> Self {
        Self::new(self.value.clone())
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for CachePadded<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("CachePadded").field(&self.value).finish()
    }
}

// Safety: CachePadded<T> is Send if T is Send
unsafe impl<T: Send> Send for CachePadded<T> {}

// Safety: CachePadded<T> is Sync if T is Sync
unsafe impl<T: Sync> Sync for CachePadded<T> {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[test]
    fn test_cache_padded_alignment() {
        // Verify the struct is 128-byte aligned
        assert_eq!(mem::align_of::<CachePadded<u64>>(), 128);
    }

    #[test]
    fn test_cache_padded_size() {
        // Size should be at least 128 bytes due to alignment
        assert!(mem::size_of::<CachePadded<u64>>() >= 128);
    }

    #[test]
    fn test_cache_padded_deref() {
        let padded = CachePadded::new(42u64);
        assert_eq!(*padded, 42);
    }

    #[test]
    fn test_cache_padded_deref_mut() {
        let mut padded = CachePadded::new(42u64);
        *padded = 100;
        assert_eq!(*padded, 100);
    }

    #[test]
    fn test_cache_padded_atomic() {
        let padded = CachePadded::new(AtomicU64::new(0));
        padded.fetch_add(5, Ordering::SeqCst);
        assert_eq!(padded.load(Ordering::SeqCst), 5);
    }

    #[test]
    fn test_cache_padded_into_inner() {
        let padded = CachePadded::new(String::from("hello"));
        let inner = padded.into_inner();
        assert_eq!(inner, "hello");
    }

    #[test]
    fn test_cache_padded_default() {
        let padded: CachePadded<u64> = CachePadded::default();
        assert_eq!(*padded, 0);
    }

    #[test]
    fn test_adjacent_padded_values_no_false_sharing() {
        // Two adjacent CachePadded values should be on different cache lines
        let values: [CachePadded<AtomicU64>; 2] = [
            CachePadded::new(AtomicU64::new(0)),
            CachePadded::new(AtomicU64::new(0)),
        ];

        let addr0 = &values[0] as *const _ as usize;
        let addr1 = &values[1] as *const _ as usize;

        // They should be at least 128 bytes apart
        assert!(addr1 - addr0 >= 128, "Adjacent CachePadded values too close");
    }
}
