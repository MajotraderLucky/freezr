//! FreezR Daemon
//!
//! System daemon for FreezR - prevents system freezes by managing runaway processes.

pub mod config;
pub mod monitor;

pub use config::Config;
pub use monitor::ResourceMonitor;
