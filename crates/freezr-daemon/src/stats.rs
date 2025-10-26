//! Statistics export for dashboard
//!
//! This module provides structures for exporting monitoring statistics
//! to JSON format for consumption by the dashboard viewer.

use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Complete statistics snapshot for dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorStats {
    /// Timestamp when stats were collected
    pub timestamp: u64,

    /// Runtime duration in seconds
    pub runtime_secs: u64,

    /// Total monitoring checks performed
    pub total_checks: u64,

    /// KESL process statistics
    pub kesl: ProcessStats,

    /// Node.js statistics
    pub node: NodeStats,

    /// Snap statistics
    pub snap: SnapStats,

    /// Firefox statistics
    pub firefox: BrowserStats,

    /// Brave statistics
    pub brave: BrowserStats,

    /// Telegram statistics
    pub telegram: BrowserStats,

    /// Memory pressure statistics
    pub memory_pressure: MemoryPressureStats,

    /// System health
    pub system_health: SystemHealth,

    /// Log statistics
    pub log_stats: LogStats,
}

/// KESL process statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessStats {
    pub pid: Option<u32>,
    pub cpu_percent: f64,
    pub memory_mb: u64,
    pub cpu_threshold: f64,
    pub memory_threshold_mb: u64,
    pub total_cpu_violations: u32,
    pub total_memory_violations: u32,
    pub current_cpu_violations: u32,
    pub current_memory_violations: u32,
    pub max_violations: u32,
    pub violation_rate: f64,
    pub total_restarts: u32,
}

/// Node.js statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStats {
    pub enabled: bool,
    pub cpu_threshold: f64,
    pub auto_kill: bool,
    pub total_kills: u32,
}

/// Snap statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapStats {
    pub enabled: bool,
    pub cpu_threshold: f64,
    pub action: String,
    pub nice_level: i32,
    pub total_actions: u32,
}

/// Browser/Telegram statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserStats {
    pub enabled: bool,
    pub freeze_threshold: f64,
    pub kill_threshold: f64,
    pub freeze_violations: u32,
    pub kill_violations: u32,
    pub max_violations_freeze: u32,
    pub max_violations_kill: u32,
    pub total_freezes: u32,
    pub total_kills: u32,
}

/// Memory pressure statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPressureStats {
    pub enabled: bool,
    pub some_avg10: f64,
    pub full_avg10: f64,
    pub status: String,
    pub warning_count: u32,
    pub critical_count: u32,
    pub some_threshold_warning: f64,
    pub some_threshold_critical: f64,
    pub full_threshold_warning: f64,
    pub full_threshold_critical: f64,
    pub action_warning: String,
    pub action_critical: String,
}

/// System health metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    pub load_1min: f64,
    pub load_5min: f64,
    pub load_15min: f64,
    pub memory_used_percent: f64,
    pub memory_total_mb: u64,
    pub memory_available_mb: u64,
}

/// Log statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogStats {
    pub active_files: usize,
    pub active_size: String,
    pub archive_files: usize,
    pub archive_size: String,
}

impl MonitorStats {
    /// Get current timestamp
    pub fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs()
    }
}

impl Default for ProcessStats {
    fn default() -> Self {
        Self {
            pid: None,
            cpu_percent: 0.0,
            memory_mb: 0,
            cpu_threshold: 30.0,
            memory_threshold_mb: 600,
            total_cpu_violations: 0,
            total_memory_violations: 0,
            current_cpu_violations: 0,
            current_memory_violations: 0,
            max_violations: 3,
            violation_rate: 0.0,
            total_restarts: 0,
        }
    }
}

impl Default for NodeStats {
    fn default() -> Self {
        Self {
            enabled: false,
            cpu_threshold: 80.0,
            auto_kill: false,
            total_kills: 0,
        }
    }
}

impl Default for SnapStats {
    fn default() -> Self {
        Self {
            enabled: false,
            cpu_threshold: 300.0,
            action: "nice".to_string(),
            nice_level: 15,
            total_actions: 0,
        }
    }
}

impl Default for BrowserStats {
    fn default() -> Self {
        Self {
            enabled: false,
            freeze_threshold: 80.0,
            kill_threshold: 95.0,
            freeze_violations: 0,
            kill_violations: 0,
            max_violations_freeze: 2,
            max_violations_kill: 3,
            total_freezes: 0,
            total_kills: 0,
        }
    }
}

impl Default for MemoryPressureStats {
    fn default() -> Self {
        Self {
            enabled: false,
            some_avg10: 0.0,
            full_avg10: 0.0,
            status: "NONE".to_string(),
            warning_count: 0,
            critical_count: 0,
            some_threshold_warning: 10.0,
            some_threshold_critical: 30.0,
            full_threshold_warning: 5.0,
            full_threshold_critical: 15.0,
            action_warning: "log".to_string(),
            action_critical: "freeze".to_string(),
        }
    }
}

impl Default for SystemHealth {
    fn default() -> Self {
        Self {
            load_1min: 0.0,
            load_5min: 0.0,
            load_15min: 0.0,
            memory_used_percent: 0.0,
            memory_total_mb: 0,
            memory_available_mb: 0,
        }
    }
}

impl Default for LogStats {
    fn default() -> Self {
        Self {
            active_files: 0,
            active_size: "0".to_string(),
            archive_files: 0,
            archive_size: "0".to_string(),
        }
    }
}
