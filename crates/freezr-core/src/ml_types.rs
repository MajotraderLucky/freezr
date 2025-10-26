//! ML-oriented data structures for process analytics
//!
//! This module defines rich data structures for collecting detailed process statistics
//! that will be used for machine learning analysis and predictions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Detailed process snapshot for ML analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessSnapshot {
    // ===== Identity =====
    pub pid: u32,
    pub name: String,
    pub cmdline: String,
    pub user: String,

    // ===== Timestamps =====
    pub timestamp: DateTime<Utc>,
    pub start_time: DateTime<Utc>,
    pub uptime_seconds: u64,

    // ===== Resource Usage =====
    pub cpu_percent: f64,
    pub memory_rss_mb: u64,
    pub memory_vms_mb: u64,
    pub memory_percent: f64,

    // ===== I/O Statistics =====
    pub io_stats: Option<IOStats>,

    // ===== CPU Details =====
    pub user_time_ticks: u64,   // CPU time in user mode
    pub system_time_ticks: u64, // CPU time in kernel mode
    pub num_threads: u32,

    // ===== Context Switches =====
    pub voluntary_ctxt_switches: u64,
    pub nonvoluntary_ctxt_switches: u64,

    // ===== Priority =====
    pub nice_value: i32,
    pub priority: i32,

    // ===== State =====
    pub state: ProcessState,

    // ===== Classification =====
    pub category: ProcessCategory,
}

/// I/O statistics from /proc/[pid]/io
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct IOStats {
    /// Bytes read from storage
    pub read_bytes: u64,
    /// Bytes written to storage
    pub write_bytes: u64,
    /// Number of read syscalls
    pub read_ops: u64,
    /// Number of write syscalls
    pub write_ops: u64,
    /// Cancelled write bytes (e.g., truncated files)
    pub cancelled_write_bytes: u64,
}

/// Process state from /proc/[pid]/stat
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessState {
    Running,
    Sleeping,
    DiskSleep,  // Uninterruptible sleep (usually I/O)
    Zombie,
    Stopped,
    TracingStop,
    Dead,
    Unknown,
}

impl From<char> for ProcessState {
    fn from(c: char) -> Self {
        match c {
            'R' => ProcessState::Running,
            'S' => ProcessState::Sleeping,
            'D' => ProcessState::DiskSleep,
            'Z' => ProcessState::Zombie,
            'T' => ProcessState::Stopped,
            't' => ProcessState::TracingStop,
            'X' | 'x' => ProcessState::Dead,
            _ => ProcessState::Unknown,
        }
    }
}

/// Process category for ML classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessCategory {
    Browser,
    IDE,
    BuildTool,
    SystemService,
    Antivirus,
    Database,
    WebServer,
    Background,
    Gaming,
    Media,
    Unknown,
}

impl ProcessCategory {
    /// Classify process based on name and command
    pub fn classify(name: &str, cmdline: &str) -> Self {
        let name_lower = name.to_lowercase();
        let cmdline_lower = cmdline.to_lowercase();

        // Browser detection
        if name_lower.contains("firefox")
            || name_lower.contains("chrome")
            || name_lower.contains("brave")
            || cmdline_lower.contains("browser")
        {
            return ProcessCategory::Browser;
        }

        // IDE detection
        if name_lower.contains("code")
            || name_lower.contains("idea")
            || name_lower.contains("pycharm")
            || name_lower.contains("cursor")
            || cmdline_lower.contains("vscode")
        {
            return ProcessCategory::IDE;
        }

        // Build tools
        if name_lower.contains("cargo")
            || name_lower.contains("rustc")
            || name_lower.contains("gcc")
            || name_lower.contains("make")
            || name_lower.contains("npm")
            || name_lower.contains("yarn")
            || cmdline_lower.contains("build")
        {
            return ProcessCategory::BuildTool;
        }

        // Antivirus
        if name_lower.contains("kesl")
            || name_lower.contains("kaspersky")
            || name_lower.contains("clamav")
            || cmdline_lower.contains("antivirus")
        {
            return ProcessCategory::Antivirus;
        }

        // Databases
        if name_lower.contains("postgres")
            || name_lower.contains("mysql")
            || name_lower.contains("mongo")
            || name_lower.contains("redis")
        {
            return ProcessCategory::Database;
        }

        // Web servers
        if name_lower.contains("nginx")
            || name_lower.contains("apache")
            || name_lower.contains("node")
        {
            return ProcessCategory::WebServer;
        }

        // Gaming
        if name_lower.contains("steam")
            || name_lower.contains("game")
            || cmdline_lower.contains(".exe")
        {
            return ProcessCategory::Gaming;
        }

        // Media
        if name_lower.contains("vlc")
            || name_lower.contains("mpv")
            || name_lower.contains("ffmpeg")
        {
            return ProcessCategory::Media;
        }

        // System services
        if name_lower.starts_with("systemd")
            || name_lower.contains("dbus")
            || name_lower.contains("NetworkManager")
            || cmdline_lower.contains("/lib/systemd")
        {
            return ProcessCategory::SystemService;
        }

        ProcessCategory::Unknown
    }
}

/// Event that occurred with a process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessEvent {
    pub timestamp: DateTime<Utc>,
    pub pid: u32,
    pub process_name: String,
    pub event_type: EventType,
    pub details: EventDetails,
}

/// Types of events we track
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    // Lifecycle
    ProcessStarted,
    ProcessExited { exit_code: i32 },
    ProcessKilled { signal: i32 },

    // Actions
    ProcessFrozen { duration_sec: u64 },
    ProcessUnfrozen,
    ServiceRestarted { service_name: String },
    NiceAdjusted { old_nice: i32, new_nice: i32 },

    // Violations
    CpuViolation { cpu_percent: f64, threshold: f64 },
    MemoryViolation { memory_mb: u64, threshold: u64 },

    // Anomalies (for future ML)
    AnomalyDetected { anomaly_score: f64, description: String },
    UnusualBehavior { reason: String },
}

/// Additional event details (flexible JSON object)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventDetails {
    #[serde(flatten)]
    pub data: serde_json::Value,
}

/// Aggregated daily statistics for a process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessDailySummary {
    pub date: chrono::NaiveDate,
    pub process_name: String,

    // Runtime
    pub total_runtime_seconds: u64,
    pub num_starts: u32,
    pub num_kills: u32,
    pub num_crashes: u32,

    // Resource Stats
    pub avg_cpu_percent: f64,
    pub max_cpu_percent: f64,
    pub avg_memory_mb: u64,
    pub max_memory_mb: u64,

    // I/O Stats
    pub total_read_gb: f64,
    pub total_write_gb: f64,

    // Violations
    pub cpu_violations: u32,
    pub memory_violations: u32,

    // Behavior patterns
    pub typical_runtime_hours: Vec<u8>, // Hours (0-23) when process typically runs
    pub avg_uptime_minutes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_state_from_char() {
        assert_eq!(ProcessState::from('R'), ProcessState::Running);
        assert_eq!(ProcessState::from('S'), ProcessState::Sleeping);
        assert_eq!(ProcessState::from('D'), ProcessState::DiskSleep);
        assert_eq!(ProcessState::from('Z'), ProcessState::Zombie);
        assert_eq!(ProcessState::from('T'), ProcessState::Stopped);
        assert_eq!(ProcessState::from('?'), ProcessState::Unknown);
    }

    #[test]
    fn test_process_category_browser() {
        assert_eq!(
            ProcessCategory::classify("firefox", "/usr/bin/firefox"),
            ProcessCategory::Browser
        );
        assert_eq!(
            ProcessCategory::classify("chrome", "/opt/google/chrome/chrome"),
            ProcessCategory::Browser
        );
        assert_eq!(
            ProcessCategory::classify("brave-browser", "brave --flag"),
            ProcessCategory::Browser
        );
    }

    #[test]
    fn test_process_category_ide() {
        assert_eq!(
            ProcessCategory::classify("code", "/usr/share/code/code"),
            ProcessCategory::IDE
        );
        assert_eq!(
            ProcessCategory::classify("cursor", "cursor --extensions-dir"),
            ProcessCategory::IDE
        );
    }

    #[test]
    fn test_process_category_build() {
        assert_eq!(
            ProcessCategory::classify("cargo", "cargo build --release"),
            ProcessCategory::BuildTool
        );
        assert_eq!(
            ProcessCategory::classify("rustc", "rustc main.rs"),
            ProcessCategory::BuildTool
        );
    }

    #[test]
    fn test_process_category_antivirus() {
        assert_eq!(
            ProcessCategory::classify("kesl", "/opt/kaspersky/kesl/libexec/kesl"),
            ProcessCategory::Antivirus
        );
    }

    #[test]
    fn test_process_category_database() {
        assert_eq!(
            ProcessCategory::classify("postgres", "postgres -D /var/lib/postgresql"),
            ProcessCategory::Database
        );
        assert_eq!(
            ProcessCategory::classify("mysqld", "/usr/sbin/mysqld"),
            ProcessCategory::Database
        );
    }

    #[test]
    fn test_process_category_system() {
        assert_eq!(
            ProcessCategory::classify("systemd", "/lib/systemd/systemd"),
            ProcessCategory::SystemService
        );
    }

    #[test]
    fn test_process_category_unknown() {
        assert_eq!(
            ProcessCategory::classify("unknown_process", "some command"),
            ProcessCategory::Unknown
        );
    }

    #[test]
    fn test_io_stats_serialization() {
        let io = IOStats {
            read_bytes: 1024,
            write_bytes: 2048,
            read_ops: 10,
            write_ops: 20,
            cancelled_write_bytes: 0,
        };

        let json = serde_json::to_string(&io).unwrap();
        let deserialized: IOStats = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.read_bytes, 1024);
        assert_eq!(deserialized.write_bytes, 2048);
    }
}
