//! FreezR Daemon
//!
//! System daemon for FreezR - prevents system freezes by managing runaway processes.

pub mod config;
pub mod monitor;
pub mod stats;

pub use config::Config;
pub use monitor::ResourceMonitor;
pub use stats::MonitorStats;
