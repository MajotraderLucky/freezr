//! FreezR Core Library
//!
//! This crate provides the core functionality for FreezR, an intelligent system
//! resource guardian that prevents system freezes by proactively managing processes.
//!
//! # Architecture
//!
//! FreezR consists of three main components:
//!
//! - [`scanner`] - Scans processes and collects resource usage statistics
//! - [`engine`] - Makes decisions about which processes to freeze/kill
//! - [`executor`] - Executes actions (freeze, kill, restart services)
//!
//! # Example
//!
//! ```no_run
//! use freezr_core::{ProcessScanner, DecisionEngine, ActionExecutor, Config};
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = Config::default();
//!     let mut scanner = ProcessScanner::new();
//!     let engine = DecisionEngine::new(config.clone());
//!     let executor = ActionExecutor::new();
//!
//!     loop {
//!         // Scan all processes
//!         let processes = scanner.scan_all().await?;
//!
//!         // Make decisions
//!         let actions = engine.decide(&processes);
//!
//!         // Execute actions
//!         executor.execute(actions).await?;
//!
//!         tokio::time::sleep(Duration::from_millis(500)).await;
//!     }
//! }
//! ```

pub mod config;
pub mod engine;
pub mod error;
pub mod executor;
pub mod scanner;
pub mod types;

// Re-export main types
pub use config::Config;
pub use engine::DecisionEngine;
pub use error::{Error, Result};
pub use executor::ActionExecutor;
pub use scanner::ProcessScanner;
pub use types::{Action, ProcessInfo, ProcessPriority, SystemHealth};

/// FreezR version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}
