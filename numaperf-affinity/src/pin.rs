//! Scoped CPU pinning guard.

use std::marker::PhantomData;

use numaperf_core::{CpuSet, HardMode, NumaError};

use crate::syscall;

/// A guard that pins the current thread to a CPU set and restores the
/// previous affinity when dropped.
///
/// `ScopedPin` is `!Send` and `!Sync` because CPU affinity is thread-local.
/// The pin must be created and dropped on the same thread.
///
/// # Example
///
/// ```no_run
/// use numaperf_affinity::ScopedPin;
/// use numaperf_core::CpuSet;
///
/// let cpus = CpuSet::parse("0-3").unwrap();
///
/// {
///     let _pin = ScopedPin::pin_current(cpus)?;
///     // Thread is pinned to CPUs 0-3 here
///
///     // Memory allocated here will be local to these CPUs' NUMA node
///     let data = vec![0u8; 1024 * 1024];
/// }
/// // Previous affinity is restored
/// # Ok::<(), numaperf_core::NumaError>(())
/// ```
pub struct ScopedPin {
    /// The previous CPU affinity to restore on drop.
    previous: CpuSet,
    /// Marker to make this type !Send and !Sync.
    _marker: PhantomData<*const ()>,
}

impl ScopedPin {
    /// Pin the current thread to the specified CPU set.
    ///
    /// Returns a guard that will restore the previous affinity when dropped.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Getting the current affinity fails
    /// - Setting the new affinity fails (e.g., invalid CPUs, permission denied)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use numaperf_affinity::ScopedPin;
    /// use numaperf_core::CpuSet;
    ///
    /// let cpus = CpuSet::single(0);
    /// let _pin = ScopedPin::pin_current(cpus)?;
    /// // Thread is now pinned to CPU 0
    /// # Ok::<(), numaperf_core::NumaError>(())
    /// ```
    pub fn pin_current(cpus: CpuSet) -> Result<Self, NumaError> {
        if cpus.is_empty() {
            return Err(NumaError::invalid_argument("CPU set cannot be empty"));
        }

        // Save current affinity
        let previous = syscall::get_affinity()?;

        // Set new affinity
        syscall::set_affinity(&cpus)?;

        Ok(Self {
            previous,
            _marker: PhantomData,
        })
    }

    /// Pin the current thread to a single CPU.
    ///
    /// This is a convenience method for pinning to exactly one CPU.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use numaperf_affinity::ScopedPin;
    ///
    /// let _pin = ScopedPin::pin_to_cpu(0)?;
    /// // Thread is now pinned to CPU 0
    /// # Ok::<(), numaperf_core::NumaError>(())
    /// ```
    pub fn pin_to_cpu(cpu: u32) -> Result<Self, NumaError> {
        Self::pin_current(CpuSet::single(cpu))
    }

    /// Pin the current thread with hard mode enforcement.
    ///
    /// In soft mode, this behaves like `pin_current()` but will succeed even
    /// if pinning fails due to permissions. In strict mode, this will:
    /// - Fail if setting affinity is denied
    /// - Verify that the affinity was actually applied as requested
    /// - Restore the previous affinity and fail if verification fails
    ///
    /// # Arguments
    ///
    /// * `cpus` - The CPU set to pin to
    /// * `mode` - The hard mode enforcement level
    ///
    /// # Errors
    ///
    /// In strict mode, returns an error if:
    /// - The CPU set is empty
    /// - Setting affinity fails (permission denied, invalid CPUs)
    /// - The actual affinity doesn't match the requested affinity
    ///
    /// In soft mode, returns an error only if:
    /// - The CPU set is empty
    /// - A non-permission error occurs
    ///
    /// # Example
    ///
    /// ```no_run
    /// use numaperf_affinity::ScopedPin;
    /// use numaperf_core::{CpuSet, HardMode};
    ///
    /// let cpus = CpuSet::parse("0-3").unwrap();
    ///
    /// // Strict mode - fail if pinning can't be guaranteed
    /// let pin = ScopedPin::pin_current_with_mode(cpus.clone(), HardMode::Strict)?;
    ///
    /// // Soft mode - best effort, continue even if pinning fails
    /// let pin = ScopedPin::pin_current_with_mode(cpus, HardMode::Soft)?;
    /// # Ok::<(), numaperf_core::NumaError>(())
    /// ```
    pub fn pin_current_with_mode(cpus: CpuSet, mode: HardMode) -> Result<Self, NumaError> {
        if cpus.is_empty() {
            return Err(NumaError::invalid_argument("CPU set cannot be empty"));
        }

        // Save current affinity
        let previous = syscall::get_affinity()?;

        // Try to set the new affinity
        match syscall::set_affinity(&cpus) {
            Ok(()) => {
                // In strict mode, verify the affinity was actually applied
                if mode.is_strict() {
                    let actual = syscall::get_affinity()?;
                    if actual != cpus {
                        // Restore previous and fail
                        let _ = syscall::set_affinity(&previous);
                        return Err(NumaError::hard_mode_unavailable(
                            "cpu affinity",
                            format!(
                                "affinity not fully applied: requested {:?}, got {:?}",
                                cpus, actual
                            ),
                        ));
                    }
                }

                Ok(Self {
                    previous,
                    _marker: PhantomData,
                })
            }
            Err(e) if e.is_permission_error() && mode.is_soft() => {
                // Soft mode: continue without pinning (return previous as-is)
                Ok(Self {
                    previous,
                    _marker: PhantomData,
                })
            }
            Err(e) if mode.is_strict() => {
                Err(NumaError::hard_mode_unavailable(
                    "cpu affinity",
                    format!("failed to set affinity: {}", e),
                ))
            }
            Err(e) => Err(e),
        }
    }

    /// Pin the current thread to a single CPU with hard mode enforcement.
    ///
    /// This is a convenience method combining `pin_to_cpu` with hard mode.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use numaperf_affinity::ScopedPin;
    /// use numaperf_core::HardMode;
    ///
    /// let _pin = ScopedPin::pin_to_cpu_with_mode(0, HardMode::Strict)?;
    /// # Ok::<(), numaperf_core::NumaError>(())
    /// ```
    pub fn pin_to_cpu_with_mode(cpu: u32, mode: HardMode) -> Result<Self, NumaError> {
        Self::pin_current_with_mode(CpuSet::single(cpu), mode)
    }

    /// Get the CPU set this thread is pinned to.
    pub fn current_cpus(&self) -> Result<CpuSet, NumaError> {
        syscall::get_affinity()
    }

    /// Get the previous CPU affinity that will be restored on drop.
    pub fn previous_cpus(&self) -> &CpuSet {
        &self.previous
    }

    /// Explicitly restore the previous affinity without dropping.
    ///
    /// After calling this method, the guard will still attempt to restore
    /// the affinity on drop, but it will already be restored.
    pub fn restore(&self) -> Result<(), NumaError> {
        syscall::set_affinity(&self.previous)
    }
}

impl Drop for ScopedPin {
    fn drop(&mut self) {
        // Best effort restore - ignore errors during drop
        let _ = syscall::set_affinity(&self.previous);
    }
}

// ScopedPin is !Send and !Sync because of PhantomData<*const ()>

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scoped_pin_empty_cpus() {
        let result = ScopedPin::pin_current(CpuSet::new());
        assert!(result.is_err());
    }

    #[test]
    fn test_pin_with_mode_empty_cpus() {
        let result = ScopedPin::pin_current_with_mode(CpuSet::new(), HardMode::Soft);
        assert!(result.is_err());

        let result = ScopedPin::pin_current_with_mode(CpuSet::new(), HardMode::Strict);
        assert!(result.is_err());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_scoped_pin_and_restore() {
        let original = syscall::get_affinity().expect("should get affinity");

        // Only test if we have multiple CPUs
        if original.count() <= 1 {
            return;
        }

        let first_cpu = original.first().unwrap();

        {
            let _pin = ScopedPin::pin_to_cpu(first_cpu).expect("should pin to first CPU");

            let current = syscall::get_affinity().expect("should get affinity");
            assert_eq!(current.count(), 1);
            assert!(current.contains(first_cpu));
        }

        // Should be restored after drop
        let restored = syscall::get_affinity().expect("should get affinity");
        assert_eq!(restored, original);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_pin_with_soft_mode() {
        let original = syscall::get_affinity().expect("should get affinity");

        // Only test if we have multiple CPUs
        if original.count() <= 1 {
            return;
        }

        let first_cpu = original.first().unwrap();
        let cpus = CpuSet::single(first_cpu);

        // Soft mode should succeed for a valid CPU
        let result = ScopedPin::pin_current_with_mode(cpus, HardMode::Soft);
        assert!(result.is_ok());

        // Restore
        let _ = syscall::set_affinity(&original);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_pin_with_strict_mode() {
        let original = syscall::get_affinity().expect("should get affinity");

        // Only test if we have multiple CPUs
        if original.count() <= 1 {
            return;
        }

        let first_cpu = original.first().unwrap();
        let cpus = CpuSet::single(first_cpu);

        // Strict mode should also succeed for a valid CPU we're allowed to pin to
        {
            let _pin = ScopedPin::pin_current_with_mode(cpus, HardMode::Strict)
                .expect("should pin in strict mode");

            let current = syscall::get_affinity().expect("should get affinity");
            assert_eq!(current.count(), 1);
            assert!(current.contains(first_cpu));
        }

        // Should be restored after drop
        let restored = syscall::get_affinity().expect("should get affinity");
        assert_eq!(restored, original);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_pin_to_cpu_with_mode() {
        let original = syscall::get_affinity().expect("should get affinity");

        if original.count() <= 1 {
            return;
        }

        let first_cpu = original.first().unwrap();

        {
            let _pin = ScopedPin::pin_to_cpu_with_mode(first_cpu, HardMode::Soft)
                .expect("should pin");

            let current = syscall::get_affinity().expect("should get affinity");
            assert_eq!(current.count(), 1);
        }

        // Restore
        let _ = syscall::set_affinity(&original);
    }
}
