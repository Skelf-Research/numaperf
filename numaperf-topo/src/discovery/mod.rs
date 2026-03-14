//! Platform-specific topology discovery.

mod fallback;

#[cfg(target_os = "linux")]
mod linux;

use numaperf_core::NumaError;

use crate::Topology;

/// Discover the system topology using the appropriate platform-specific method.
pub fn discover() -> Result<Topology, NumaError> {
    #[cfg(target_os = "linux")]
    {
        linux::discover().or_else(|_| fallback::discover())
    }

    #[cfg(not(target_os = "linux"))]
    {
        fallback::discover()
    }
}
