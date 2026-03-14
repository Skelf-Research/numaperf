//! Error types for numaperf.

use std::io;
use thiserror::Error;

/// Unified error type for all numaperf operations.
///
/// Errors are structured to explain both what failed and why, enabling
/// callers to take appropriate recovery actions.
#[derive(Debug, Error)]
pub enum NumaError {
    /// The requested memory policy is not supported on this kernel.
    #[error("memory policy not supported: {reason}")]
    PolicyNotSupported {
        /// Description of the policy that failed.
        policy: String,
        /// Why the policy is not supported.
        reason: String,
    },

    /// A required capability is missing.
    #[error("missing capability: {capability}")]
    CapabilityMissing {
        /// The name of the missing capability (e.g., "CAP_SYS_NICE").
        capability: &'static str,
    },

    /// Hard mode was requested but cannot be enforced.
    #[error("hard mode unavailable for {feature}: {reason}")]
    HardModeUnavailable {
        /// The feature that cannot be enforced.
        feature: &'static str,
        /// Why hard mode is unavailable.
        reason: String,
    },

    /// Topology discovery failed.
    #[error("topology discovery failed: {source}")]
    TopologyError {
        /// The underlying I/O error.
        #[source]
        source: io::Error,
    },

    /// Memory allocation or mapping failed.
    #[error("allocation of {size} bytes failed: {source}")]
    AllocationFailed {
        /// The requested allocation size.
        size: usize,
        /// The underlying I/O error.
        #[source]
        source: io::Error,
    },

    /// Thread pinning failed.
    #[error("thread pinning failed: {source}")]
    PinningFailed {
        /// The underlying I/O error.
        #[source]
        source: io::Error,
    },

    /// Memory binding (mbind) failed.
    #[error("memory binding failed: {source}")]
    BindFailed {
        /// The underlying I/O error.
        #[source]
        source: io::Error,
    },

    /// An invalid argument was provided.
    #[error("invalid argument: {message}")]
    InvalidArgument {
        /// Description of the invalid argument.
        message: String,
    },

    /// Platform does not support the requested feature.
    #[error("feature not supported on this platform: {feature}")]
    NotSupported {
        /// The unsupported feature.
        feature: &'static str,
    },

    /// Generic I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

impl NumaError {
    /// Create a topology error from an I/O error.
    pub fn topology(source: io::Error) -> Self {
        Self::TopologyError { source }
    }

    /// Create an allocation error.
    pub fn allocation(size: usize, source: io::Error) -> Self {
        Self::AllocationFailed { size, source }
    }

    /// Create a pinning error from an I/O error.
    pub fn pinning(source: io::Error) -> Self {
        Self::PinningFailed { source }
    }

    /// Create a bind error from an I/O error.
    pub fn bind(source: io::Error) -> Self {
        Self::BindFailed { source }
    }

    /// Create a policy not supported error.
    pub fn policy_not_supported(policy: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::PolicyNotSupported {
            policy: policy.into(),
            reason: reason.into(),
        }
    }

    /// Create a hard mode unavailable error.
    pub fn hard_mode_unavailable(feature: &'static str, reason: impl Into<String>) -> Self {
        Self::HardModeUnavailable {
            feature,
            reason: reason.into(),
        }
    }

    /// Create an invalid argument error.
    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self::InvalidArgument {
            message: message.into(),
        }
    }

    /// Check if this error indicates a permission issue.
    pub fn is_permission_error(&self) -> bool {
        match self {
            Self::CapabilityMissing { .. } => true,
            Self::TopologyError { source }
            | Self::AllocationFailed { source, .. }
            | Self::PinningFailed { source }
            | Self::BindFailed { source } => source.kind() == io::ErrorKind::PermissionDenied,
            Self::Io(e) => e.kind() == io::ErrorKind::PermissionDenied,
            _ => false,
        }
    }

    /// Check if this error indicates the feature is not supported.
    pub fn is_not_supported(&self) -> bool {
        matches!(
            self,
            Self::NotSupported { .. } | Self::PolicyNotSupported { .. }
        )
    }
}

/// Result type alias for numaperf operations.
pub type Result<T> = std::result::Result<T, NumaError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = NumaError::policy_not_supported("Bind", "kernel too old");
        assert_eq!(
            format!("{}", err),
            "memory policy not supported: kernel too old"
        );

        let err = NumaError::CapabilityMissing {
            capability: "CAP_SYS_NICE",
        };
        assert_eq!(format!("{}", err), "missing capability: CAP_SYS_NICE");
    }

    #[test]
    fn test_error_is_permission() {
        let err = NumaError::CapabilityMissing {
            capability: "CAP_SYS_NICE",
        };
        assert!(err.is_permission_error());

        // EPERM = 1 on Linux
        let err = NumaError::pinning(io::Error::from_raw_os_error(1));
        assert!(err.is_permission_error());
    }
}
