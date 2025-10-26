//! Cgroup v2 integration module
//!
//! Provides high-level API for managing Linux cgroup v2 resources.
//! Supports CPU and memory limits with systemd integration.
//!
//! # Safety
//! - Only works when systemd service is active
//! - Automatically cleans up on service stop
//! - Restores all processes to original cgroups on shutdown

pub mod controller;
pub mod error;
pub mod types;
mod utils;

pub use controller::{CpuController, CpuStats, MemoryController, MemoryPressure, MemoryStats};
pub use error::{CgroupError, Result};
pub use types::{
    Cgroup, CgroupConfig, CgroupManager, CgroupStrategy, CgroupType, DynamicCgroupSettings,
    HealthStatus, ResourceLimits, StaticCgroupConfig,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_structure() {
        // Verify module compiles
        assert!(true);
    }
}
