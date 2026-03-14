//! Hard mode configuration.

/// Controls whether numaperf enforces strict NUMA policies or falls back gracefully.
///
/// # Soft Mode (default)
///
/// In soft mode, numaperf does its best to honor NUMA policies but will
/// degrade gracefully when kernel features or privileges are unavailable.
/// Operations succeed with reduced guarantees, and the actual enforcement
/// level is reported to the caller.
///
/// # Strict Mode
///
/// In strict mode, numaperf requires that all requested policies can be
/// fully enforced. If a policy cannot be guaranteed (due to missing
/// privileges, kernel features, or hardware), the operation fails with
/// a structured error.
///
/// Strict mode is useful for:
/// - Production systems where NUMA locality is critical
/// - Benchmarking with guaranteed placement
/// - Detecting configuration issues early
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum HardMode {
    /// Best-effort enforcement with graceful degradation.
    #[default]
    Soft,

    /// Strict enforcement; fail if policy cannot be guaranteed.
    Strict,
}

impl HardMode {
    /// Check if this is soft mode.
    #[inline]
    pub fn is_soft(self) -> bool {
        matches!(self, Self::Soft)
    }

    /// Check if this is strict mode.
    #[inline]
    pub fn is_strict(self) -> bool {
        matches!(self, Self::Strict)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hard_mode_default() {
        assert_eq!(HardMode::default(), HardMode::Soft);
    }

    #[test]
    fn test_hard_mode_checks() {
        assert!(HardMode::Soft.is_soft());
        assert!(!HardMode::Soft.is_strict());
        assert!(!HardMode::Strict.is_soft());
        assert!(HardMode::Strict.is_strict());
    }
}
