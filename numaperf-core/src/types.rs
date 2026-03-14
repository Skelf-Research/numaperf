//! Core NUMA types: NodeId, CpuSet, NodeMask.

use std::fmt;

/// Identifier for a NUMA node.
///
/// Node IDs are stable identifiers assigned by the kernel. They typically
/// start at 0 and increment, but gaps may exist on some systems.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub u32);

impl NodeId {
    /// Create a new NodeId from a raw value.
    #[inline]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the raw node ID value.
    #[inline]
    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "node{}", self.0)
    }
}

impl From<u32> for NodeId {
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl From<NodeId> for u32 {
    fn from(id: NodeId) -> Self {
        id.0
    }
}

/// A set of CPU IDs.
///
/// CpuSet represents a collection of CPU cores, typically used for thread
/// affinity. It supports up to 1024 CPUs, matching the Linux kernel limit.
#[derive(Clone, PartialEq, Eq)]
pub struct CpuSet {
    /// Bitmap of CPUs, where bit N indicates CPU N is in the set.
    bits: [u64; 16], // 16 * 64 = 1024 CPUs max
}

impl CpuSet {
    /// Maximum number of CPUs supported.
    pub const MAX_CPUS: usize = 1024;

    /// Create an empty CPU set.
    #[inline]
    pub const fn new() -> Self {
        Self { bits: [0; 16] }
    }

    /// Create a CPU set containing a single CPU.
    pub fn single(cpu: u32) -> Self {
        let mut set = Self::new();
        set.add(cpu);
        set
    }

    /// Add a CPU to the set.
    ///
    /// # Panics
    /// Panics if `cpu >= MAX_CPUS`.
    #[inline]
    pub fn add(&mut self, cpu: u32) {
        assert!((cpu as usize) < Self::MAX_CPUS, "CPU ID out of range");
        let idx = cpu as usize / 64;
        let bit = cpu as usize % 64;
        self.bits[idx] |= 1 << bit;
    }

    /// Remove a CPU from the set.
    #[inline]
    pub fn remove(&mut self, cpu: u32) {
        if (cpu as usize) < Self::MAX_CPUS {
            let idx = cpu as usize / 64;
            let bit = cpu as usize % 64;
            self.bits[idx] &= !(1 << bit);
        }
    }

    /// Check if a CPU is in the set.
    #[inline]
    pub fn contains(&self, cpu: u32) -> bool {
        if (cpu as usize) >= Self::MAX_CPUS {
            return false;
        }
        let idx = cpu as usize / 64;
        let bit = cpu as usize % 64;
        (self.bits[idx] & (1 << bit)) != 0
    }

    /// Check if the set is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.bits.iter().all(|&b| b == 0)
    }

    /// Count the number of CPUs in the set.
    #[inline]
    pub fn count(&self) -> usize {
        self.bits.iter().map(|b| b.count_ones() as usize).sum()
    }

    /// Iterate over the CPUs in the set.
    pub fn iter(&self) -> impl Iterator<Item = u32> + '_ {
        (0..Self::MAX_CPUS as u32).filter(|&cpu| self.contains(cpu))
    }

    /// Get the first CPU in the set, if any.
    pub fn first(&self) -> Option<u32> {
        self.iter().next()
    }

    /// Create a union of two CPU sets.
    pub fn union(&self, other: &Self) -> Self {
        let mut result = Self::new();
        for i in 0..16 {
            result.bits[i] = self.bits[i] | other.bits[i];
        }
        result
    }

    /// Create an intersection of two CPU sets.
    pub fn intersection(&self, other: &Self) -> Self {
        let mut result = Self::new();
        for i in 0..16 {
            result.bits[i] = self.bits[i] & other.bits[i];
        }
        result
    }

    /// Get the raw bitmap for FFI.
    #[inline]
    pub fn as_raw(&self) -> &[u64; 16] {
        &self.bits
    }

    /// Get a mutable reference to the raw bitmap for FFI.
    #[inline]
    pub fn as_raw_mut(&mut self) -> &mut [u64; 16] {
        &mut self.bits
    }

    /// Parse a CPU list string like "0-3,8-11" or "0,1,2,3".
    pub fn parse(s: &str) -> Result<Self, ParseCpuSetError> {
        let mut set = Self::new();
        if s.trim().is_empty() {
            return Ok(set);
        }

        for part in s.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }

            if let Some((start, end)) = part.split_once('-') {
                let start: u32 = start
                    .trim()
                    .parse()
                    .map_err(|_| ParseCpuSetError::InvalidNumber(start.to_string()))?;
                let end: u32 = end
                    .trim()
                    .parse()
                    .map_err(|_| ParseCpuSetError::InvalidNumber(end.to_string()))?;

                if start > end {
                    return Err(ParseCpuSetError::InvalidRange(start, end));
                }
                if end as usize >= Self::MAX_CPUS {
                    return Err(ParseCpuSetError::CpuOutOfRange(end));
                }

                for cpu in start..=end {
                    set.add(cpu);
                }
            } else {
                let cpu: u32 = part
                    .parse()
                    .map_err(|_| ParseCpuSetError::InvalidNumber(part.to_string()))?;
                if cpu as usize >= Self::MAX_CPUS {
                    return Err(ParseCpuSetError::CpuOutOfRange(cpu));
                }
                set.add(cpu);
            }
        }

        Ok(set)
    }
}

impl Default for CpuSet {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for CpuSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CpuSet({})", self)
    }
}

impl fmt::Display for CpuSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let cpus: Vec<u32> = self.iter().collect();
        if cpus.is_empty() {
            return write!(f, "");
        }

        // Format as ranges where possible
        let mut parts = Vec::new();
        let mut start = cpus[0];
        let mut end = cpus[0];

        for &cpu in &cpus[1..] {
            if cpu == end + 1 {
                end = cpu;
            } else {
                if start == end {
                    parts.push(format!("{}", start));
                } else {
                    parts.push(format!("{}-{}", start, end));
                }
                start = cpu;
                end = cpu;
            }
        }

        // Handle last range
        if start == end {
            parts.push(format!("{}", start));
        } else {
            parts.push(format!("{}-{}", start, end));
        }

        write!(f, "{}", parts.join(","))
    }
}

impl FromIterator<u32> for CpuSet {
    fn from_iter<I: IntoIterator<Item = u32>>(iter: I) -> Self {
        let mut set = Self::new();
        for cpu in iter {
            set.add(cpu);
        }
        set
    }
}

/// Error parsing a CPU set string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseCpuSetError {
    /// Invalid number in CPU list.
    InvalidNumber(String),
    /// CPU ID exceeds maximum.
    CpuOutOfRange(u32),
    /// Invalid range (start > end).
    InvalidRange(u32, u32),
}

impl fmt::Display for ParseCpuSetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidNumber(s) => write!(f, "invalid CPU number: {}", s),
            Self::CpuOutOfRange(cpu) => {
                write!(f, "CPU {} exceeds maximum {}", cpu, CpuSet::MAX_CPUS)
            }
            Self::InvalidRange(start, end) => write!(f, "invalid range: {}-{}", start, end),
        }
    }
}

impl std::error::Error for ParseCpuSetError {}

/// A set of NUMA node IDs.
///
/// NodeMask represents a collection of NUMA nodes, used for memory policies
/// like Bind and Interleave.
#[derive(Clone, PartialEq, Eq)]
pub struct NodeMask {
    /// Bitmap of nodes, where bit N indicates node N is in the set.
    bits: u64, // Supports up to 64 nodes
}

impl NodeMask {
    /// Maximum number of nodes supported.
    pub const MAX_NODES: usize = 64;

    /// Create an empty node mask.
    #[inline]
    pub const fn new() -> Self {
        Self { bits: 0 }
    }

    /// Create a node mask containing a single node.
    pub fn single(node: NodeId) -> Self {
        let mut mask = Self::new();
        mask.add(node);
        mask
    }

    /// Add a node to the mask.
    ///
    /// # Panics
    /// Panics if `node.0 >= MAX_NODES`.
    #[inline]
    pub fn add(&mut self, node: NodeId) {
        assert!((node.0 as usize) < Self::MAX_NODES, "Node ID out of range");
        self.bits |= 1 << node.0;
    }

    /// Remove a node from the mask.
    #[inline]
    pub fn remove(&mut self, node: NodeId) {
        if (node.0 as usize) < Self::MAX_NODES {
            self.bits &= !(1 << node.0);
        }
    }

    /// Check if a node is in the mask.
    #[inline]
    pub fn contains(&self, node: NodeId) -> bool {
        if (node.0 as usize) >= Self::MAX_NODES {
            return false;
        }
        (self.bits & (1 << node.0)) != 0
    }

    /// Check if the mask is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.bits == 0
    }

    /// Count the number of nodes in the mask.
    #[inline]
    pub fn count(&self) -> usize {
        self.bits.count_ones() as usize
    }

    /// Iterate over the nodes in the mask.
    pub fn iter(&self) -> impl Iterator<Item = NodeId> + '_ {
        (0..Self::MAX_NODES as u32)
            .filter(|&n| self.contains(NodeId(n)))
            .map(NodeId)
    }

    /// Get the first node in the mask, if any.
    pub fn first(&self) -> Option<NodeId> {
        self.iter().next()
    }

    /// Get the raw bitmap for FFI.
    #[inline]
    pub fn as_raw(&self) -> u64 {
        self.bits
    }

    /// Create from a raw bitmap.
    #[inline]
    pub const fn from_raw(bits: u64) -> Self {
        Self { bits }
    }
}

impl Default for NodeMask {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for NodeMask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeMask({:?})", self.iter().collect::<Vec<_>>())
    }
}

impl fmt::Display for NodeMask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let nodes: Vec<String> = self.iter().map(|n| n.0.to_string()).collect();
        write!(f, "{}", nodes.join(","))
    }
}

impl From<NodeId> for NodeMask {
    fn from(node: NodeId) -> Self {
        Self::single(node)
    }
}

impl From<&[NodeId]> for NodeMask {
    fn from(nodes: &[NodeId]) -> Self {
        let mut mask = Self::new();
        for &node in nodes {
            mask.add(node);
        }
        mask
    }
}

impl<const N: usize> From<[NodeId; N]> for NodeMask {
    fn from(nodes: [NodeId; N]) -> Self {
        Self::from(&nodes[..])
    }
}

impl FromIterator<NodeId> for NodeMask {
    fn from_iter<I: IntoIterator<Item = NodeId>>(iter: I) -> Self {
        let mut mask = Self::new();
        for node in iter {
            mask.add(node);
        }
        mask
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_id() {
        let node = NodeId::new(5);
        assert_eq!(node.as_u32(), 5);
        assert_eq!(format!("{}", node), "node5");
    }

    #[test]
    fn test_cpu_set_basic() {
        let mut set = CpuSet::new();
        assert!(set.is_empty());
        assert_eq!(set.count(), 0);

        set.add(0);
        set.add(3);
        set.add(7);
        assert!(!set.is_empty());
        assert_eq!(set.count(), 3);
        assert!(set.contains(0));
        assert!(set.contains(3));
        assert!(set.contains(7));
        assert!(!set.contains(1));

        set.remove(3);
        assert_eq!(set.count(), 2);
        assert!(!set.contains(3));
    }

    #[test]
    fn test_cpu_set_parse() {
        let set = CpuSet::parse("0-3,8-11").unwrap();
        assert_eq!(set.count(), 8);
        assert!(set.contains(0));
        assert!(set.contains(3));
        assert!(!set.contains(4));
        assert!(set.contains(8));
        assert!(set.contains(11));
        assert!(!set.contains(12));

        let set2 = CpuSet::parse("1,3,5").unwrap();
        assert_eq!(set2.count(), 3);
        assert!(set2.contains(1));
        assert!(set2.contains(3));
        assert!(set2.contains(5));
    }

    #[test]
    fn test_cpu_set_display() {
        let set = CpuSet::parse("0-3,8-11").unwrap();
        assert_eq!(format!("{}", set), "0-3,8-11");

        let set2 = CpuSet::parse("1,3,5").unwrap();
        assert_eq!(format!("{}", set2), "1,3,5");
    }

    #[test]
    fn test_node_mask() {
        let mut mask = NodeMask::new();
        assert!(mask.is_empty());

        mask.add(NodeId(0));
        mask.add(NodeId(2));
        assert_eq!(mask.count(), 2);
        assert!(mask.contains(NodeId(0)));
        assert!(mask.contains(NodeId(2)));
        assert!(!mask.contains(NodeId(1)));
    }

    #[test]
    fn test_node_mask_from_array() {
        let mask: NodeMask = [NodeId(0), NodeId(3)].into();
        assert_eq!(mask.count(), 2);
        assert!(mask.contains(NodeId(0)));
        assert!(mask.contains(NodeId(3)));
    }
}
