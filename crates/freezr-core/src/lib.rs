//! FreezR Core Library
//!
//! Core library for FreezR - intelligent system resource guardian.
//! Provides process scanning, systemd service management, and resource monitoring.

pub mod cgroups;
pub mod error;
pub mod executor;
pub mod memory_pressure;
pub mod ml_types;
pub mod scanner;
pub mod systemd;
pub mod types;

pub use cgroups::{
    Cgroup, CgroupConfig, CgroupError as CgroupErr, CgroupManager, CgroupStrategy, CgroupType,
    CpuController, CpuStats, DynamicCgroupSettings, HealthStatus, MemoryController,
    MemoryPressure as CgroupMemoryPressure, MemoryStats, ResourceLimits, StaticCgroupConfig,
};
pub use error::{Error, Result};
pub use executor::ProcessExecutor;
pub use memory_pressure::MemoryPressure;
pub use ml_types::{
    EventDetails, EventType, IOStats, ProcessCategory, ProcessDailySummary, ProcessEvent,
    ProcessSnapshot, ProcessState,
};
pub use scanner::ProcessScanner;
pub use systemd::SystemdService;
pub use types::{MonitorStats, ProcessInfo};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}
