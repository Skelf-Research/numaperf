//! Enforcement level reporting.

/// Reports the actual enforcement level achieved for an operation.
///
/// When using soft mode, operations may succeed with reduced guarantees.
/// This enum allows callers to inspect what level of NUMA policy enforcement
/// was actually achieved.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum EnforcementLevel {
    /// The policy is fully enforced.
    ///
    /// Memory is bound to the requested nodes, threads are pinned, and
    /// the kernel guarantees the placement.
    #[default]
    Strict,

    /// The policy was applied but is not guaranteed.
    ///
    /// The kernel will try to honor the policy, but may move pages or
    /// threads under memory pressure or other conditions.
    BestEffort {
        /// Why strict enforcement was not possible.
        reason: String,
    },

    /// No NUMA policy was applied.
    ///
    /// The operation succeeded, but no NUMA-specific behavior is in effect.
    /// Memory may be allocated from any node.
    None {
        /// Why no policy could be applied.
        reason: String,
    },
}

impl EnforcementLevel {
    /// Create a strict enforcement level.
    #[inline]
    pub fn strict() -> Self {
        Self::Strict
    }

    /// Create a best-effort enforcement level.
    pub fn best_effort(reason: impl Into<String>) -> Self {
        Self::BestEffort {
            reason: reason.into(),
        }
    }

    /// Create a none enforcement level.
    pub fn none(reason: impl Into<String>) -> Self {
        Self::None {
            reason: reason.into(),
        }
    }

    /// Check if this is strict enforcement.
    #[inline]
    pub fn is_strict(&self) -> bool {
        matches!(self, Self::Strict)
    }

    /// Check if this is best-effort enforcement.
    #[inline]
    pub fn is_best_effort(&self) -> bool {
        matches!(self, Self::BestEffort { .. })
    }

    /// Check if no enforcement was applied.
    #[inline]
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None { .. })
    }

    /// Get the degradation reason, if any.
    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::Strict => None,
            Self::BestEffort { reason } | Self::None { reason } => Some(reason),
        }
    }
}

impl std::fmt::Display for EnforcementLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Strict => write!(f, "strict"),
            Self::BestEffort { reason } => write!(f, "best-effort ({})", reason),
            Self::None { reason } => write!(f, "none ({})", reason),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enforcement_level() {
        let strict = EnforcementLevel::strict();
        assert!(strict.is_strict());
        assert!(!strict.is_best_effort());
        assert!(strict.reason().is_none());

        let best = EnforcementLevel::best_effort("missing CAP_SYS_ADMIN");
        assert!(best.is_best_effort());
        assert_eq!(best.reason(), Some("missing CAP_SYS_ADMIN"));

        let none = EnforcementLevel::none("NUMA not supported");
        assert!(none.is_none());
        assert_eq!(none.reason(), Some("NUMA not supported"));
    }

    #[test]
    fn test_enforcement_display() {
        assert_eq!(format!("{}", EnforcementLevel::strict()), "strict");
        assert_eq!(
            format!("{}", EnforcementLevel::best_effort("reason")),
            "best-effort (reason)"
        );
    }
}
