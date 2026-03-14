//! Huge page configuration.

/// Controls huge page usage for memory regions.
///
/// Huge pages (typically 2 MB or 1 GB) reduce TLB pressure and can
/// significantly improve performance for large memory regions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HugePageMode {
    /// Do not use huge pages.
    #[default]
    None,

    /// Enable transparent huge pages (THP).
    ///
    /// The kernel will try to back the region with huge pages where possible.
    /// This requires THP to be enabled system-wide (`/sys/kernel/mm/transparent_hugepage/enabled`).
    TransparentOn,

    /// Disable transparent huge pages for this region.
    ///
    /// Forces the kernel to use regular 4 KB pages, even if THP is enabled.
    TransparentOff,

    /// Use explicit 2 MB huge pages from hugetlbfs.
    ///
    /// Requires huge pages to be pre-reserved and the process to have
    /// access to hugetlbfs. More predictable than THP but requires setup.
    Explicit2MB,

    /// Use explicit 1 GB huge pages from hugetlbfs.
    ///
    /// Requires 1 GB huge pages to be available, which must typically be
    /// reserved at boot time via kernel command line.
    Explicit1GB,
}

impl HugePageMode {
    /// Check if this mode uses transparent huge pages.
    pub fn is_transparent(&self) -> bool {
        matches!(self, Self::TransparentOn | Self::TransparentOff)
    }

    /// Check if this mode uses explicit huge pages.
    pub fn is_explicit(&self) -> bool {
        matches!(self, Self::Explicit2MB | Self::Explicit1GB)
    }

    /// Get the page size for this mode.
    pub fn page_size(&self) -> usize {
        match self {
            Self::None | Self::TransparentOff => 4096,
            Self::TransparentOn => 4096, // Base size; actual backing may be 2MB
            Self::Explicit2MB => 2 * 1024 * 1024,
            Self::Explicit1GB => 1024 * 1024 * 1024,
        }
    }
}

impl std::fmt::Display for HugePageMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::TransparentOn => write!(f, "thp-on"),
            Self::TransparentOff => write!(f, "thp-off"),
            Self::Explicit2MB => write!(f, "2MB"),
            Self::Explicit1GB => write!(f, "1GB"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_huge_page_mode() {
        assert_eq!(HugePageMode::default(), HugePageMode::None);
        assert!(!HugePageMode::None.is_transparent());
        assert!(HugePageMode::TransparentOn.is_transparent());
        assert!(HugePageMode::Explicit2MB.is_explicit());
        assert_eq!(HugePageMode::Explicit2MB.page_size(), 2 * 1024 * 1024);
    }
}
