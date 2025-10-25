# FreezR API Reference

API documentation for FreezR components.

## Available Documentation

### Rust API (Planned)
Detailed API documentation for all public types, functions, and modules.

**Generate locally:**
```bash
cargo doc --open --no-deps
```

This will generate and open HTML documentation for:
- `freezr-core` - Core library
- `freezr-daemon` - Daemon binary
- All public APIs with examples

### Configuration Schema (Planned)
Complete TOML configuration schema with:
- All available options
- Default values
- Validation rules
- Examples

## Quick API Overview

### freezr-core

#### ProcessScanner
```rust
pub struct ProcessScanner;

impl ProcessScanner {
    pub fn new() -> Self;
    pub fn scan_kesl(&self) -> Result<Option<ProcessInfo>>;
    pub fn scan_node_processes(&self) -> Result<Vec<ProcessInfo>>;
}
```

#### ProcessExecutor
```rust
pub struct ProcessExecutor;

impl ProcessExecutor {
    pub fn new() -> Self;
    pub fn kill_process(pid: u32) -> Result<()>;
    pub fn freeze_process(pid: u32) -> Result<()>;
    pub fn unfreeze_process(pid: u32) -> Result<()>;
}
```

#### SystemdService
```rust
pub struct SystemdService {
    pub fn new(name: &str) -> Self;
    pub fn is_active(&self) -> Result<bool>;
    pub fn restart_with_reload(&mut self) -> Result<()>;
    pub fn get_properties(&self) -> Result<String>;
}
```

#### ProcessInfo
```rust
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub command: String,
    pub cpu_percent: f64,
    pub memory_mb: u64,
    pub memory_kb: u64,
}
```

#### MonitorStats
```rust
pub struct MonitorStats {
    pub total_checks: u64,
    pub total_violations: u64,
    pub total_kills: u64,
    pub total_restarts: u64,
    pub cpu_violations: u32,
    pub memory_violations: u32,
}
```

### freezr-daemon

#### ResourceMonitor
```rust
pub struct ResourceMonitor;

impl ResourceMonitor {
    pub fn new(
        service_name: &str,
        cpu_threshold: f64,
        memory_threshold_mb: u64,
        max_violations: u32,
        min_restart_interval_secs: u64,
    ) -> Self;

    pub fn enable_node_monitoring(&mut self, cpu_threshold: f64, auto_kill: bool);
    pub fn check(&mut self) -> Result<()>;
    pub fn stats(&self) -> &MonitorStats;
    pub fn violations(&self) -> (u32, u32);
    pub fn reset_violations(&mut self);
}
```

#### Config
```rust
pub struct Config {
    pub kesl: KeslConfig,
    pub node: NodeConfig,
    pub logging: LogConfig,
    pub monitoring: MonitoringConfig,
}

impl Config {
    pub fn load_from_file(path: &str) -> Result<Self>;
    pub fn save_to_file(&self, path: &str) -> Result<()>;
    pub fn validate(&self) -> Result<(), String>;
}
```

## Error Types

```rust
pub enum Error {
    Scanner(String),
    Executor(String),
    Systemd(String),
    Io(std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
```

## Usage Examples

### Example 1: Scan Processes
```rust
use freezr_core::scanner::ProcessScanner;

let scanner = ProcessScanner::new();

// Scan KESL
if let Some(process) = scanner.scan_kesl()? {
    println!("KESL: PID {}, CPU {:.1}%", process.pid, process.cpu_percent);
}

// Scan Node.js
for process in scanner.scan_node_processes()? {
    println!("Node: PID {}, CPU {:.1}%", process.pid, process.cpu_percent);
}
```

### Example 2: Kill Process
```rust
use freezr_core::executor::ProcessExecutor;

// Kill process (SIGTERM → SIGKILL)
ProcessExecutor::kill_process(12345)?;
```

### Example 3: Monitor Resources
```rust
use freezr_daemon::{Config, ResourceMonitor};

let config = Config::load_from_file("/etc/freezr/config.toml")?;
let mut monitor = ResourceMonitor::new(
    &config.kesl.service_name,
    config.kesl.cpu_threshold,
    config.kesl.memory_threshold_mb,
    config.kesl.max_violations,
    config.monitoring.min_restart_interval_secs,
);

// Enable Node.js monitoring
monitor.enable_node_monitoring(config.node.cpu_threshold, config.node.auto_kill);

// Run monitoring check
monitor.check()?;

// Get statistics
let stats = monitor.stats();
println!("Checks: {}, Violations: {}",
    stats.total_checks, stats.total_violations);
```

## Configuration File Format

### Complete TOML Schema

```toml
# KESL monitoring configuration
[kesl]
cpu_threshold = 30.0              # float, 0.0-100.0
memory_threshold_mb = 600         # u64, > 0
max_violations = 3                # u32, > 0
service_name = "kesl"             # string
enabled = true                    # bool

# Node.js monitoring configuration
[node]
cpu_threshold = 80.0              # float, 0.0-100.0
enabled = true                    # bool
auto_kill = true                  # bool
confirm_kill = false              # bool

# Logging configuration
[logging]
log_dir = "./logs"                # string (path)
kesl_log = "kesl-monitor.log"     # string
node_log = "node-monitor.log"     # string
actions_log = "actions.log"       # string
max_file_size_mb = 10             # u64, > 0
rotate_count = 5                  # u32, > 0

# Monitoring behavior configuration
[monitoring]
check_interval_secs = 3           # u64, > 0
min_restart_interval_secs = 100   # u64, > 0
```

## Building Documentation

```bash
# Generate and open HTML documentation
cargo doc --open --no-deps

# Generate for specific package
cargo doc --package freezr-core --open

# Include private items
cargo doc --document-private-items --open
```

## Future API Additions

Planned APIs for future versions:
- WebSocket API for real-time monitoring
- REST API for remote control
- Plugin API for custom monitors
- Metrics API for Prometheus integration

## Quick Links

- [Development Guide →](../development/README.md)
- [Usage Examples →](../examples/README.md)
- [← Back to Main Docs](../README.md)
