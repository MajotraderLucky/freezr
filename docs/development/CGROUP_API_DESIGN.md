# Cgroup Module API Design

## Overview

The cgroups module will provide high-level API for managing Linux cgroup v2 resources. It will support both static (pre-configured) and dynamic (runtime-created) cgroups with CPU and memory limits.

## Module Structure

```
crates/freezr-core/src/cgroups/
├── mod.rs              # Public API and CgroupManager
├── controller.rs       # Controller-specific operations (CPU, Memory, IO)
├── types.rs            # Data structures (Cgroup, CgroupConfig, etc.)
├── error.rs            # Error types
└── utils.rs            # Helper functions (path validation, parsing, etc.)
```

## Core Types

### CgroupManager

Main entry point for cgroup operations.

```rust
pub struct CgroupManager {
    /// Root path for FreezR cgroups (e.g., /sys/fs/cgroup/freezr.slice)
    root_path: PathBuf,

    /// Strategy: Static, Dynamic, or Hybrid
    strategy: CgroupStrategy,

    /// Cache of active cgroups
    cgroups: HashMap<String, Cgroup>,

    /// Static cgroup configurations
    static_configs: Vec<StaticCgroupConfig>,
}

impl CgroupManager {
    /// Create new cgroup manager with validation
    pub fn new(config: CgroupConfig) -> Result<Self>;

    /// Initialize: create root slice, apply static configs
    pub fn initialize(&mut self) -> Result<()>;

    /// Create a new cgroup (dynamic)
    pub fn create_cgroup(&mut self, name: &str) -> Result<Cgroup>;

    /// Get existing cgroup by name
    pub fn get_cgroup(&self, name: &str) -> Option<&Cgroup>;

    /// Remove cgroup (cleanup)
    pub fn remove_cgroup(&mut self, name: &str) -> Result<()>;

    /// Apply limits to a cgroup
    pub fn apply_limits(&self, cgroup: &Cgroup, limits: ResourceLimits) -> Result<()>;

    /// Move process to cgroup
    pub fn assign_process(&self, cgroup: &Cgroup, pid: u32) -> Result<()>;

    /// Get cgroup for process (if any)
    pub fn get_process_cgroup(&self, pid: u32) -> Result<Option<PathBuf>>;

    /// Cleanup: remove all dynamic cgroups
    pub fn cleanup(&mut self) -> Result<()>;
}
```

### Cgroup

Represents a single cgroup.

```rust
pub struct Cgroup {
    /// Cgroup name (e.g., "kesl", "firefox-throttle")
    pub name: String,

    /// Full path (/sys/fs/cgroup/freezr.slice/kesl)
    pub path: PathBuf,

    /// Type: Static or Dynamic
    pub cgroup_type: CgroupType,

    /// Current resource limits
    pub limits: ResourceLimits,

    /// PIDs currently in this cgroup
    pub pids: Vec<u32>,
}

impl Cgroup {
    /// Read current CPU usage statistics
    pub fn get_cpu_stats(&self) -> Result<CpuStats>;

    /// Read current memory usage
    pub fn get_memory_usage(&self) -> Result<MemoryStats>;

    /// Check if cgroup still exists
    pub fn exists(&self) -> bool;

    /// Get list of processes in cgroup
    pub fn get_processes(&self) -> Result<Vec<u32>>;
}
```

### ResourceLimits

Resource limits to apply.

```rust
pub struct ResourceLimits {
    /// CPU limit (percentage, 0-100 for single core, >100 for multi-core)
    pub cpu_limit_percent: Option<f64>,

    /// Hard memory limit (bytes)
    pub memory_max: Option<u64>,

    /// Soft memory limit (bytes)
    pub memory_high: Option<u64>,

    /// IO limits (optional, future)
    pub io_limits: Option<IOLimits>,
}

impl ResourceLimits {
    pub fn new() -> Self;
    pub fn with_cpu_limit(mut self, percent: f64) -> Self;
    pub fn with_memory_max(mut self, bytes: u64) -> Self;
    pub fn with_memory_high(mut self, bytes: u64) -> Self;
}
```

### CgroupStrategy

```rust
pub enum CgroupStrategy {
    /// Only use pre-configured static cgroups
    Static,

    /// Only create dynamic cgroups at runtime
    Dynamic,

    /// Use both static and dynamic (recommended)
    Hybrid,
}
```

### CgroupType

```rust
pub enum CgroupType {
    /// Pre-configured, persistent cgroup
    Static,

    /// Temporary cgroup for runtime throttling
    Dynamic,
}
```

### CgroupConfig

Configuration from TOML.

```rust
pub struct CgroupConfig {
    /// Enable cgroup integration
    pub enabled: bool,

    /// Root path for FreezR cgroups
    pub root_path: PathBuf,

    /// Strategy
    pub strategy: CgroupStrategy,

    /// Static cgroup configurations
    pub static_groups: Vec<StaticCgroupConfig>,

    /// Dynamic cgroup settings
    pub dynamic_settings: DynamicCgroupSettings,
}
```

### StaticCgroupConfig

Configuration for static cgroup.

```rust
pub struct StaticCgroupConfig {
    /// Cgroup name (e.g., "kesl")
    pub name: String,

    /// Process names to assign to this cgroup
    pub process_patterns: Vec<String>,

    /// Resource limits
    pub limits: ResourceLimits,
}
```

### DynamicCgroupSettings

```rust
pub struct DynamicCgroupSettings {
    /// Maximum number of dynamic cgroups
    pub max_dynamic_cgroups: usize,

    /// Auto-cleanup after this duration (seconds) if process exited
    pub cleanup_timeout_secs: u64,

    /// Default CPU limit for dynamic cgroups
    pub default_cpu_limit: f64,

    /// Default memory limit for dynamic cgroups
    pub default_memory_limit: u64,
}
```

## Controller Module

Handles controller-specific operations.

```rust
pub mod controller {
    use super::*;

    /// CPU controller operations
    pub struct CpuController;

    impl CpuController {
        /// Set CPU quota (percentage -> microseconds)
        pub fn set_quota(cgroup_path: &Path, percent: f64) -> Result<()>;

        /// Get CPU statistics
        pub fn get_stats(cgroup_path: &Path) -> Result<CpuStats>;

        /// Set CPU weight (relative share)
        pub fn set_weight(cgroup_path: &Path, weight: u32) -> Result<()>;
    }

    /// Memory controller operations
    pub struct MemoryController;

    impl MemoryController {
        /// Set hard memory limit
        pub fn set_max(cgroup_path: &Path, bytes: u64) -> Result<()>;

        /// Set soft memory limit
        pub fn set_high(cgroup_path: &Path, bytes: u64) -> Result<()>;

        /// Get memory usage
        pub fn get_current(cgroup_path: &Path) -> Result<u64>;

        /// Get memory statistics
        pub fn get_stats(cgroup_path: &Path) -> Result<MemoryStats>;

        /// Get memory pressure (PSI)
        pub fn get_pressure(cgroup_path: &Path) -> Result<MemoryPressure>;
    }
}
```

## Statistics Types

```rust
pub struct CpuStats {
    /// Total CPU time used (microseconds)
    pub usage_usec: u64,

    /// User-space CPU time
    pub user_usec: u64,

    /// Kernel-space CPU time
    pub system_usec: u64,

    /// Number of enforcement periods
    pub nr_periods: u64,

    /// Number of throttled periods
    pub nr_throttled: u64,

    /// Total throttled time (microseconds)
    pub throttled_usec: u64,
}

impl CpuStats {
    /// Calculate throttle percentage
    pub fn throttle_percentage(&self) -> f64;
}

pub struct MemoryStats {
    /// Current memory usage (bytes)
    pub current: u64,

    /// Peak memory usage (bytes)
    pub peak: u64,

    /// Swap usage (bytes)
    pub swap_current: u64,
}
```

## Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum CgroupError {
    #[error("Cgroup not found: {0}")]
    NotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Invalid limit value: {0}")]
    InvalidLimit(String),

    #[error("Cgroup already exists: {0}")]
    AlreadyExists(String),

    #[error("Failed to parse cgroup file: {0}")]
    ParseError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Process not found: {0}")]
    ProcessNotFound(u32),

    #[error("Max dynamic cgroups reached")]
    MaxCgroupsReached,
}

pub type Result<T> = std::result::Result<T, CgroupError>;
```

## Usage Examples

### Example 1: Initialize with Static Cgroups

```rust
use freezr_core::cgroups::{CgroupManager, CgroupConfig, StaticCgroupConfig, ResourceLimits};

// Create config
let config = CgroupConfig {
    enabled: true,
    root_path: PathBuf::from("/sys/fs/cgroup/freezr.slice"),
    strategy: CgroupStrategy::Hybrid,
    static_groups: vec![
        StaticCgroupConfig {
            name: "kesl".to_string(),
            process_patterns: vec!["kesl".to_string()],
            limits: ResourceLimits::new()
                .with_cpu_limit(30.0)
                .with_memory_max(512 * 1024 * 1024), // 512MB
        },
        StaticCgroupConfig {
            name: "browsers".to_string(),
            process_patterns: vec!["firefox".to_string(), "brave".to_string()],
            limits: ResourceLimits::new()
                .with_cpu_limit(200.0) // 2 cores
                .with_memory_max(4 * 1024 * 1024 * 1024), // 4GB
        },
    ],
    dynamic_settings: DynamicCgroupSettings {
        max_dynamic_cgroups: 50,
        cleanup_timeout_secs: 300,
        default_cpu_limit: 50.0,
        default_memory_limit: 1024 * 1024 * 1024, // 1GB
    },
};

// Initialize manager
let mut manager = CgroupManager::new(config)?;
manager.initialize()?;
```

### Example 2: Dynamic Cgroup for High-CPU Process

```rust
// Detect high-CPU process
if process.cpu_percent > 80.0 {
    // Create dynamic cgroup
    let cgroup_name = format!("throttle-{}", process.pid);
    let cgroup = manager.create_cgroup(&cgroup_name)?;

    // Apply limits
    let limits = ResourceLimits::new()
        .with_cpu_limit(30.0)
        .with_memory_high(512 * 1024 * 1024);

    manager.apply_limits(&cgroup, limits)?;

    // Move process
    manager.assign_process(&cgroup, process.pid)?;

    println!("Process {} throttled to 30% CPU", process.pid);
}
```

### Example 3: Monitor Cgroup Stats

```rust
// Get cgroup
let cgroup = manager.get_cgroup("kesl").unwrap();

// Read stats
let cpu_stats = cgroup.get_cpu_stats()?;
let mem_stats = cgroup.get_memory_usage()?;

println!("CPU throttle: {:.2}%", cpu_stats.throttle_percentage());
println!("Memory usage: {} MB", mem_stats.current / 1024 / 1024);

// Check if being throttled
if cpu_stats.nr_throttled > 0 {
    println!("Warning: Process being CPU throttled!");
}
```

### Example 4: Cleanup on Shutdown

```rust
// On shutdown
manager.cleanup()?;
println!("All dynamic cgroups removed");
```

## Integration with Monitor Loop

```rust
// In monitor.rs

pub struct ProcessMonitor {
    scanner: ProcessScanner,
    executor: ProcessExecutor,
    cgroup_manager: Option<CgroupManager>, // NEW
    config: Config,
}

impl ProcessMonitor {
    pub fn new(config: Config) -> Result<Self> {
        // Initialize cgroup manager if enabled
        let cgroup_manager = if config.cgroups.enabled {
            let mut manager = CgroupManager::new(config.cgroups.clone())?;
            manager.initialize()?;
            Some(manager)
        } else {
            None
        };

        Ok(Self {
            scanner: ProcessScanner::new(),
            executor: ProcessExecutor::new(),
            cgroup_manager,
            config,
        })
    }

    pub async fn monitor_loop(&mut self) -> Result<()> {
        loop {
            // Scan processes
            let processes = self.scanner.scan_all()?;

            // Check violations
            for process in processes {
                if process.cpu_exceeds(80.0) {
                    // Option 1: Use cgroup throttling (gentle)
                    if let Some(manager) = &mut self.cgroup_manager {
                        self.throttle_with_cgroup(manager, &process)?;
                    }
                    // Option 2: Freeze (more aggressive)
                    else {
                        self.executor.freeze_process(process.pid)?;
                    }
                }
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    fn throttle_with_cgroup(&mut self, manager: &mut CgroupManager, process: &ProcessInfo) -> Result<()> {
        // Check if static cgroup matches
        if let Some(static_cgroup) = manager.get_matching_static_cgroup(process) {
            manager.assign_process(static_cgroup, process.pid)?;
            println!("Assigned {} to static cgroup {}", process.pid, static_cgroup.name);
        }
        // Create dynamic cgroup
        else {
            let cgroup_name = format!("throttle-{}", process.pid);
            let cgroup = manager.create_cgroup(&cgroup_name)?;

            let limits = ResourceLimits::new()
                .with_cpu_limit(30.0);

            manager.apply_limits(&cgroup, limits)?;
            manager.assign_process(&cgroup, process.pid)?;

            println!("Created dynamic cgroup for PID {}", process.pid);
        }

        Ok(())
    }
}
```

## TOML Configuration

```toml
[cgroups]
enabled = true
root_path = "/sys/fs/cgroup/freezr.slice"
strategy = "hybrid"  # static, dynamic, hybrid

# Static cgroups (persistent)
[[cgroups.static_groups]]
name = "kesl"
process_patterns = ["kesl"]
cpu_limit_percent = 30.0
memory_max_mb = 512

[[cgroups.static_groups]]
name = "browsers"
process_patterns = ["firefox", "brave", "chrome"]
cpu_limit_percent = 200.0  # 2 cores
memory_max_mb = 4096

[[cgroups.static_groups]]
name = "development"
process_patterns = ["node", "python3"]
cpu_limit_percent = 150.0  # 1.5 cores
memory_max_mb = 2048

# Dynamic cgroup settings
[cgroups.dynamic_settings]
max_dynamic_cgroups = 50
cleanup_timeout_secs = 300
default_cpu_limit = 50.0
default_memory_limit_mb = 1024
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cgroup_creation() {
        let manager = CgroupManager::new_test();
        let cgroup = manager.create_cgroup("test-group").unwrap();
        assert_eq!(cgroup.name, "test-group");
        assert!(cgroup.exists());
    }

    #[test]
    fn test_cpu_limit_conversion() {
        // 30% -> 30000 microseconds per 100000
        let (quota, period) = convert_percent_to_quota(30.0);
        assert_eq!(quota, 30000);
        assert_eq!(period, 100000);
    }

    #[test]
    fn test_resource_limits_builder() {
        let limits = ResourceLimits::new()
            .with_cpu_limit(50.0)
            .with_memory_max(1024 * 1024 * 1024);

        assert_eq!(limits.cpu_limit_percent, Some(50.0));
        assert_eq!(limits.memory_max, Some(1024 * 1024 * 1024));
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_throttle_high_cpu_process() {
    // Requires root or capabilities
    if !is_root() {
        return;
    }

    let config = test_config();
    let mut manager = CgroupManager::new(config).unwrap();
    manager.initialize().unwrap();

    // Spawn high-CPU process
    let child = spawn_cpu_stress();
    let pid = child.id();

    // Create cgroup and throttle
    let cgroup = manager.create_cgroup("test-throttle").unwrap();
    let limits = ResourceLimits::new().with_cpu_limit(10.0);

    manager.apply_limits(&cgroup, limits).unwrap();
    manager.assign_process(&cgroup, pid).unwrap();

    // Wait and verify CPU usage is limited
    tokio::time::sleep(Duration::from_secs(5)).await;

    let stats = cgroup.get_cpu_stats().unwrap();
    assert!(stats.nr_throttled > 0, "Process should be throttled");

    // Cleanup
    child.kill().unwrap();
    manager.remove_cgroup("test-throttle").unwrap();
}
```

## Systemd Integration and Safe Shutdown

**CRITICAL**: Cgroups must ONLY work when systemd service is enabled. When service is stopped/disabled, ALL cgroup rules must be removed to restore system to normal state.

### Systemd Service Lifecycle

```rust
impl CgroupManager {
    /// Called when systemd service starts
    pub fn on_service_start(&mut self) -> Result<()> {
        self.initialize()?;
        self.apply_static_cgroups()?;
        self.log("Cgroup integration enabled");
        Ok(())
    }

    /// Called when systemd service stops (CRITICAL)
    pub fn on_service_stop(&mut self) -> Result<()> {
        self.restore_all_processes()?;  // Move processes back to original cgroups
        self.remove_all_cgroups()?;     // Delete all FreezR cgroups
        self.cleanup_root_slice()?;     // Remove freezr.slice
        self.log("Cgroup integration disabled - system restored");
        Ok(())
    }

    /// Restore all processes to their original cgroups
    fn restore_all_processes(&self) -> Result<()> {
        for cgroup in &self.cgroups.values() {
            let pids = cgroup.get_processes()?;
            for pid in pids {
                // Move process back to root cgroup or original location
                self.restore_process_cgroup(pid)?;
            }
        }
        Ok(())
    }

    /// Remove all FreezR cgroups (static + dynamic)
    fn remove_all_cgroups(&mut self) -> Result<()> {
        // Remove dynamic cgroups first
        for name in self.cgroups.keys().cloned().collect::<Vec<_>>() {
            self.remove_cgroup(&name)?;
        }

        // Remove static cgroups
        for static_config in &self.static_configs {
            let path = self.root_path.join(&static_config.name);
            if path.exists() {
                std::fs::remove_dir(&path)?;
            }
        }

        Ok(())
    }

    /// Remove root slice
    fn cleanup_root_slice(&self) -> Result<()> {
        if self.root_path.exists() {
            std::fs::remove_dir(&self.root_path)?;
            println!("Removed root cgroup: {:?}", self.root_path);
        }
        Ok(())
    }
}
```

### Systemd Service File Integration

```ini
[Unit]
Description=FreezR Process Monitor with Cgroup Integration
After=network.target

[Service]
Type=notify
ExecStart=/usr/local/bin/freezr-daemon
ExecStop=/usr/local/bin/freezr-daemon cleanup  # NEW: Safe cleanup on stop
Restart=on-failure
RestartSec=10

# Cgroup delegation
Delegate=yes
DelegateSubgroup=freezr

# Ensure cleanup on exit
KillMode=mixed
TimeoutStopSec=30

[Install]
WantedBy=multi-user.target
```

### Safe Cleanup Command

Add new CLI subcommand for manual cleanup:

```rust
// In main.rs

#[derive(Subcommand)]
enum Commands {
    /// Start monitoring daemon
    Watch {
        #[arg(long)]
        config: PathBuf,
    },

    /// Cleanup cgroups and restore system (NEW)
    Cleanup,

    /// Show status
    Status,
}

async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Cleanup => {
            // Load config
            let config = load_config()?;

            // Create cgroup manager
            let mut manager = CgroupManager::new(config.cgroups)?;

            // Perform safe cleanup
            println!("Removing all FreezR cgroups...");
            manager.on_service_stop()?;
            println!("✅ System restored to original state");

            Ok(())
        }
        // ... other commands
    }
}
```

### Emergency Recovery

If service crashes or is killed forcefully:

```bash
# Manual emergency cleanup
sudo freezr-daemon cleanup

# Or direct filesystem cleanup
sudo rmdir /sys/fs/cgroup/freezr.slice/*
sudo rmdir /sys/fs/cgroup/freezr.slice

# Verify cleanup
ls /sys/fs/cgroup/ | grep freezr
# Should return nothing
```

### Graceful Shutdown Handler

```rust
use tokio::signal;

pub struct ProcessMonitor {
    cgroup_manager: Option<CgroupManager>,
    shutdown_tx: broadcast::Sender<()>,
}

impl ProcessMonitor {
    pub async fn run(&mut self) -> Result<()> {
        // Setup signal handlers
        let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())?;
        let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())?;

        tokio::select! {
            _ = sigterm.recv() => {
                println!("Received SIGTERM, cleaning up...");
                self.shutdown().await?;
            }
            _ = sigint.recv() => {
                println!("Received SIGINT, cleaning up...");
                self.shutdown().await?;
            }
            _ = self.monitor_loop() => {}
        }

        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        // Send shutdown signal
        let _ = self.shutdown_tx.send(());

        // Cleanup cgroups
        if let Some(manager) = &mut self.cgroup_manager {
            manager.on_service_stop()?;
        }

        println!("✅ Shutdown complete");
        Ok(())
    }
}
```

### Health Check

Add systemd health check to verify cgroups are working:

```rust
impl CgroupManager {
    /// Verify cgroup subsystem is healthy
    pub fn health_check(&self) -> Result<HealthStatus> {
        let mut issues = Vec::new();

        // Check root slice exists
        if !self.root_path.exists() {
            issues.push("Root cgroup slice missing".to_string());
        }

        // Check static cgroups
        for config in &self.static_configs {
            let path = self.root_path.join(&config.name);
            if !path.exists() {
                issues.push(format!("Static cgroup missing: {}", config.name));
            }
        }

        // Check permissions
        if !self.has_write_permission()? {
            issues.push("Insufficient permissions for cgroup operations".to_string());
        }

        if issues.is_empty() {
            Ok(HealthStatus::Healthy)
        } else {
            Ok(HealthStatus::Degraded(issues))
        }
    }
}

pub enum HealthStatus {
    Healthy,
    Degraded(Vec<String>),
    Failed(String),
}
```

### systemctl Integration

```bash
# Start service - cgroups enabled
sudo systemctl start freezr

# Stop service - cgroups automatically cleaned up
sudo systemctl stop freezr

# Disable service - cleanup persists
sudo systemctl disable freezr

# Check status
sudo systemctl status freezr

# Manual cleanup if needed
sudo freezr-daemon cleanup
```

### Configuration Flag

Add safety flag in TOML:

```toml
[cgroups]
enabled = true
root_path = "/sys/fs/cgroup/freezr.slice"
strategy = "hybrid"

# Safety settings
auto_cleanup_on_stop = true          # Cleanup when service stops
restore_processes_on_stop = true     # Move processes back
remove_on_disable = true             # Remove cgroups when disabled
emergency_cleanup_timeout_secs = 30  # Max time for cleanup
```

## Security Considerations

1. **Root Path Validation**: Only allow cgroup creation under FreezR root
2. **Process Ownership**: Verify permission to move process to cgroup
3. **Limit Validation**: Ensure limits are reasonable (0-100% CPU, etc.)
4. **Cgroup Limit**: Prevent DoS by limiting number of dynamic cgroups
5. **Cleanup**: Always cleanup on exit to prevent cgroup leaks
6. **Audit Log**: Log all cgroup operations for security audit
7. **Systemd Integration**: ONLY work when systemd service is active
8. **Safe Shutdown**: Restore system state on service stop
9. **Emergency Recovery**: Provide manual cleanup command
10. **Health Checks**: Verify cgroup subsystem health

## Performance Considerations

1. **Caching**: Cache active cgroups to avoid repeated filesystem access
2. **Batch Operations**: Group multiple process assignments
3. **Lazy Initialization**: Only create cgroups when needed
4. **Cleanup Timer**: Periodic cleanup of empty dynamic cgroups
5. **Stats Polling**: Rate-limit statistics reading

## Next Steps

1. ✅ API design complete
2. ⏭️ Implement basic cgroups.rs module
3. ⏭️ Implement CPU controller
4. ⏭️ Implement memory controller
5. ⏭️ Add configuration parsing
6. ⏭️ Integrate with monitor loop
7. ⏭️ Write tests
8. ⏭️ Update documentation
