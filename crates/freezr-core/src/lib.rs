//! FreezR Core Library
//!
//! Core library for FreezR - intelligent system resource guardian.
//! Provides process scanning, systemd service management, and resource monitoring.

pub mod error;
pub mod executor;
pub mod scanner;
pub mod systemd;
pub mod types;

pub use error::{Error, Result};
pub use executor::ProcessExecutor;
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
