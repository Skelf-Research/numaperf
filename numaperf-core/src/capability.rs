//! System capability detection for NUMA operations.

use std::fs;
use std::path::Path;

/// Detected system capabilities for NUMA operations.
///
/// This struct provides information about what NUMA features are available
/// on the current system, including kernel capabilities, system settings,
/// and hardware topology.
///
/// # Example
///
/// ```
/// use numaperf_core::Capabilities;
///
/// let caps = Capabilities::detect();
/// println!("System capabilities: {}", caps.summary());
///
/// if caps.supports_hard_mode() {
///     println!("Hard mode is fully supported");
/// } else {
///     println!("Missing capabilities:");
///     for cap in caps.missing_for_hard_mode() {
///         println!("  - {}", cap);
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Capabilities {
    /// Has CAP_SYS_ADMIN (for strict memory binding with MPOL_BIND).
    pub strict_memory_binding: bool,
    /// Has CAP_SYS_NICE (for strict CPU affinity with realtime scheduling).
    pub strict_cpu_affinity: bool,
    /// Has CAP_IPC_LOCK (for memory locking with mlock).
    pub memory_locking: bool,
    /// Kernel NUMA balancing is disabled (kernel.numa_balancing=0).
    pub numa_balancing_disabled: bool,
    /// Number of NUMA nodes detected on the system.
    pub numa_node_count: usize,
}

impl Capabilities {
    /// Detect current system capabilities.
    ///
    /// This reads from `/proc` and `/sys` filesystems to determine
    /// what NUMA features are available.
    pub fn detect() -> Self {
        Self {
            strict_memory_binding: has_capability(CAP_SYS_ADMIN),
            strict_cpu_affinity: has_capability(CAP_SYS_NICE),
            memory_locking: has_capability(CAP_IPC_LOCK),
            numa_balancing_disabled: check_numa_balancing_disabled(),
            numa_node_count: count_numa_nodes(),
        }
    }

    /// Check if all hard mode requirements are met.
    ///
    /// Hard mode requires:
    /// - CAP_SYS_ADMIN for strict memory binding
    /// - CAP_SYS_NICE for strict CPU affinity
    /// - NUMA balancing disabled to prevent kernel migration
    pub fn supports_hard_mode(&self) -> bool {
        self.strict_memory_binding
            && self.strict_cpu_affinity
            && self.numa_balancing_disabled
    }

    /// Get a list of missing capabilities required for hard mode.
    ///
    /// Returns an empty list if `supports_hard_mode()` returns true.
    pub fn missing_for_hard_mode(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if !self.strict_memory_binding {
            missing.push("CAP_SYS_ADMIN (for strict memory binding)");
        }
        if !self.strict_cpu_affinity {
            missing.push("CAP_SYS_NICE (for strict CPU affinity)");
        }
        if !self.numa_balancing_disabled {
            missing.push("kernel.numa_balancing=0 (to prevent automatic migration)");
        }
        missing
    }

    /// Check if this is a NUMA system (more than one node).
    pub fn is_numa_system(&self) -> bool {
        self.numa_node_count > 1
    }

    /// Generate a human-readable summary of detected capabilities.
    pub fn summary(&self) -> String {
        let mut s = String::new();
        s.push_str("NUMA System Capabilities\n");
        s.push_str("========================\n");
        s.push_str(&format!("NUMA nodes detected: {}\n", self.numa_node_count));
        s.push_str(&format!(
            "CAP_SYS_ADMIN (strict memory binding): {}\n",
            if self.strict_memory_binding { "yes" } else { "no" }
        ));
        s.push_str(&format!(
            "CAP_SYS_NICE (strict CPU affinity): {}\n",
            if self.strict_cpu_affinity { "yes" } else { "no" }
        ));
        s.push_str(&format!(
            "CAP_IPC_LOCK (memory locking): {}\n",
            if self.memory_locking { "yes" } else { "no" }
        ));
        s.push_str(&format!(
            "NUMA balancing disabled: {}\n",
            if self.numa_balancing_disabled { "yes" } else { "no" }
        ));
        s.push_str(&format!(
            "\nHard mode supported: {}\n",
            if self.supports_hard_mode() { "YES" } else { "NO" }
        ));

        if !self.supports_hard_mode() {
            s.push_str("\nMissing for hard mode:\n");
            for cap in self.missing_for_hard_mode() {
                s.push_str(&format!("  - {}\n", cap));
            }
        }
        s
    }
}

impl std::fmt::Display for Capabilities {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Capabilities(nodes={}, hard_mode={})",
            self.numa_node_count,
            if self.supports_hard_mode() { "supported" } else { "unavailable" }
        )
    }
}

// Linux capability numbers from include/uapi/linux/capability.h
const CAP_IPC_LOCK: u8 = 14;
const CAP_SYS_ADMIN: u8 = 21;
const CAP_SYS_NICE: u8 = 23;

/// Check if the current process has a specific capability.
///
/// Reads from /proc/self/status and checks the CapEff bitmask.
fn has_capability(cap: u8) -> bool {
    let status = match fs::read_to_string("/proc/self/status") {
        Ok(s) => s,
        Err(_) => return false,
    };

    for line in status.lines() {
        if let Some(hex) = line.strip_prefix("CapEff:\t") {
            return check_capability_bit(hex.trim(), cap);
        }
    }
    false
}

/// Check if a capability bit is set in a hex string.
fn check_capability_bit(hex: &str, cap: u8) -> bool {
    // CapEff is a hex string like "0000000000000001"
    // Each hex digit represents 4 bits
    let cap_bit = cap as u64;

    match u64::from_str_radix(hex, 16) {
        Ok(caps) => (caps & (1 << cap_bit)) != 0,
        Err(_) => false,
    }
}

/// Check if kernel NUMA balancing is disabled.
fn check_numa_balancing_disabled() -> bool {
    let path = Path::new("/proc/sys/kernel/numa_balancing");
    if !path.exists() {
        // If the file doesn't exist, NUMA balancing is likely disabled
        // or not supported by this kernel.
        return true;
    }

    match fs::read_to_string(path) {
        Ok(content) => {
            // File contains "0" if disabled, "1" if enabled
            content.trim() == "0"
        }
        Err(_) => false,
    }
}

/// Count the number of NUMA nodes on the system.
fn count_numa_nodes() -> usize {
    let node_dir = Path::new("/sys/devices/system/node");
    if !node_dir.exists() {
        // Not a NUMA system or sysfs not mounted
        return 1;
    }

    match fs::read_dir(node_dir) {
        Ok(entries) => {
            let count = entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .starts_with("node")
                })
                .count();
            // Ensure at least 1 node
            std::cmp::max(count, 1)
        }
        Err(_) => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_detection() {
        let caps = Capabilities::detect();
        // Should at least detect one node
        assert!(caps.numa_node_count >= 1);
    }

    #[test]
    fn test_missing_capabilities() {
        let caps = Capabilities::detect();
        let missing = caps.missing_for_hard_mode();

        // If hard mode is supported, missing should be empty
        if caps.supports_hard_mode() {
            assert!(missing.is_empty());
        } else {
            // If not supported, there should be at least one missing capability
            assert!(!missing.is_empty());
        }
    }

    #[test]
    fn test_capability_bit_check() {
        // Test hex parsing
        assert!(check_capability_bit("0000000000200000", CAP_SYS_ADMIN)); // bit 21
        assert!(check_capability_bit("0000000000800000", CAP_SYS_NICE));  // bit 23
        assert!(check_capability_bit("0000000000004000", CAP_IPC_LOCK));  // bit 14

        // Full caps (all bits set)
        assert!(check_capability_bit("ffffffffffffffff", CAP_SYS_ADMIN));
        assert!(check_capability_bit("ffffffffffffffff", CAP_SYS_NICE));
        assert!(check_capability_bit("ffffffffffffffff", CAP_IPC_LOCK));

        // No caps
        assert!(!check_capability_bit("0000000000000000", CAP_SYS_ADMIN));
        assert!(!check_capability_bit("0000000000000000", CAP_SYS_NICE));
        assert!(!check_capability_bit("0000000000000000", CAP_IPC_LOCK));
    }

    #[test]
    fn test_summary_format() {
        let caps = Capabilities::detect();
        let summary = caps.summary();

        assert!(summary.contains("NUMA System Capabilities"));
        assert!(summary.contains("NUMA nodes detected:"));
        assert!(summary.contains("CAP_SYS_ADMIN"));
        assert!(summary.contains("CAP_SYS_NICE"));
        assert!(summary.contains("Hard mode supported:"));
    }

    #[test]
    fn test_display() {
        let caps = Capabilities::detect();
        let display = format!("{}", caps);

        assert!(display.contains("Capabilities"));
        assert!(display.contains("nodes="));
    }

    #[test]
    fn test_is_numa_system() {
        let caps = Capabilities::detect();

        if caps.numa_node_count > 1 {
            assert!(caps.is_numa_system());
        } else {
            assert!(!caps.is_numa_system());
        }
    }

    #[test]
    fn test_capability_struct_fields() {
        // Create a mock capabilities struct
        let caps = Capabilities {
            strict_memory_binding: true,
            strict_cpu_affinity: true,
            memory_locking: true,
            numa_balancing_disabled: true,
            numa_node_count: 2,
        };

        assert!(caps.supports_hard_mode());
        assert!(caps.missing_for_hard_mode().is_empty());
        assert!(caps.is_numa_system());
    }

    #[test]
    fn test_missing_some_capabilities() {
        let caps = Capabilities {
            strict_memory_binding: false,
            strict_cpu_affinity: true,
            memory_locking: true,
            numa_balancing_disabled: false,
            numa_node_count: 1,
        };

        assert!(!caps.supports_hard_mode());
        let missing = caps.missing_for_hard_mode();
        assert_eq!(missing.len(), 2);
        assert!(missing.iter().any(|s| s.contains("CAP_SYS_ADMIN")));
        assert!(missing.iter().any(|s| s.contains("numa_balancing")));
    }
}
