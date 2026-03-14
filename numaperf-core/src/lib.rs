//! Core types and error handling for numaperf.
//!
//! This crate provides the foundational types shared across all numaperf crates:
//! - [`NodeId`], [`CpuSet`], [`NodeMask`] - NUMA topology primitives
//! - [`NumaError`] - Unified error type with actionable messages
//! - [`HardMode`] - Soft vs strict enforcement toggle
//! - [`EnforcementLevel`] - Reports actual enforcement achieved
//! - [`Capabilities`] - System capability detection for hard mode

mod capability;
mod enforcement;
mod error;
mod mode;
mod types;

pub use capability::Capabilities;
pub use enforcement::EnforcementLevel;
pub use error::NumaError;
pub use mode::HardMode;
pub use types::{CpuSet, NodeId, NodeMask};
