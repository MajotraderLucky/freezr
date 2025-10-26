//! Process Monitor Binary
//!
//! Advanced process monitoring with statistics tracking

use anyhow::Result;
use chrono::Timelike;
use clap::Parser;
use freezr_daemon::{Config, ResourceMonitor};
use nix::libc;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use tracing::{error, info, warn};

/// Command line arguments
#[derive(Parser, Debug)]
#[clap(name = "process_monitor")]
#[clap(about = "Advanced process monitoring with statistics", long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Path to configuration file
    #[arg(short, long, default_value = "freezr.toml")]
    config: PathBuf,

    /// Enable extended statistics
    #[arg(long)]
    stats: bool,

    /// Report interval in seconds (for stats mode)
    #[arg(long, default_value = "60")]
    report_interval: u64,
}

#[derive(Parser, Debug)]
enum Commands {
    /// Install FreezR as a systemd service
    InstallService {
        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,
    },

    /// Uninstall FreezR systemd service
    UninstallService {
        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,
    },

    /// Show systemd service status
    ServiceStatus,

    /// Show live monitoring dashboard (read-only)
    Dashboard {
        /// Update interval in seconds
        #[arg(short, long, default_value = "3")]
        interval: u64,
    },
}

/// Ensure required directories exist and are writable
fn ensure_directories() -> Result<String> {
    use std::fs;
    use std::io::Write;

    let dirs = vec!["logs/", "logs/archive/", "data/process_stats/"];

    for dir in &dirs {
        // Create directory if it doesn't exist
        fs::create_dir_all(dir)
            .map_err(|e| anyhow::anyhow!("Failed to create directory {}: {}", dir, e))?;

        // Test write permissions
        let test_file = format!("{}/.write_test", dir);
        let mut file = fs::File::create(&test_file)
            .map_err(|e| anyhow::anyhow!("Directory {} is not writable: {}", dir, e))?;

        file.write_all(b"test")
            .map_err(|e| anyhow::anyhow!("Cannot write to directory {}: {}", dir, e))?;

        // Clean up test file
        fs::remove_file(&test_file)
            .map_err(|e| anyhow::anyhow!("Failed to remove test file {}: {}", test_file, e))?;
    }

    Ok(format!("✅ Directories verified: {}", dirs.join(", ")))
}

/// Check disk space for logs directory
fn check_disk_space(path: &str) -> Result<u8> {
    use std::process::Command;

    let output = Command::new("df")
        .arg("-h")
        .arg(path)
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to run df command: {}", e))?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("df command failed"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    if lines.len() < 2 {
        return Err(anyhow::anyhow!("Unexpected df output format"));
    }

    // Parse usage percentage from second line (e.g., "26%")
    let usage_line = lines[1];
    let parts: Vec<&str> = usage_line.split_whitespace().collect();

    // Usage percentage is typically the 5th column (index 4)
    if parts.len() < 5 {
        return Err(anyhow::anyhow!("Cannot parse df output: {}", usage_line));
    }

    let usage_str = parts[4].trim_end_matches('%');
    let usage = usage_str
        .parse::<u8>()
        .map_err(|_| anyhow::anyhow!("Cannot parse usage percentage: {}", usage_str))?;

    Ok(usage)
}

/// Kill old process_monitor instances to prevent conflicts
fn kill_old_instances() -> Result<()> {
    use std::process::Command;

    let process_name = "process_monitor";
    let mut killed_any = false;

    // Use pgrep to find PIDs, excluding current process
    let output = Command::new("pgrep")
        .arg("-f")
        .arg(process_name)
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to run pgrep: {}", e))?;

    if output.status.success() && !output.stdout.is_empty() {
        let pids_str = String::from_utf8_lossy(&output.stdout);
        let current_pid = std::process::id();

        for line in pids_str.lines() {
            if let Ok(pid) = line.trim().parse::<u32>() {
                // Skip current process
                if pid == current_pid {
                    continue;
                }

                // Try SIGTERM first (graceful)
                let term_result = Command::new("kill")
                    .arg("-15") // SIGTERM
                    .arg(pid.to_string())
                    .output();

                if term_result.is_ok() {
                    info!("🔪 Killed old {} process (PID: {})", process_name, pid);
                    killed_any = true;

                    // Wait 100ms for graceful shutdown
                    std::thread::sleep(std::time::Duration::from_millis(100));

                    // Check if still alive, use SIGKILL if needed
                    let check = Command::new("kill")
                        .arg("-0") // Check if process exists
                        .arg(pid.to_string())
                        .output();

                    if check.is_ok() && check.unwrap().status.success() {
                        // Process still alive, force kill
                        let _ = Command::new("kill")
                            .arg("-9") // SIGKILL
                            .arg(pid.to_string())
                            .output();
                        warn!(
                            "⚡ Force killed stubborn {} process (PID: {})",
                            process_name, pid
                        );
                    }
                }
            }
        }
    }

    if !killed_any {
        info!("✅ No old process_monitor instances found");
    }

    Ok(())
}

/// Check system resources (CPU, memory, load)
fn check_system_health() -> Result<String> {
    use std::fs;

    // Read load average
    let loadavg = fs::read_to_string("/proc/loadavg")
        .map_err(|e| anyhow::anyhow!("Failed to read /proc/loadavg: {}", e))?;

    let load_parts: Vec<&str> = loadavg.split_whitespace().collect();
    let load_1min = load_parts
        .first()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);

    // Read meminfo
    let meminfo = fs::read_to_string("/proc/meminfo")
        .map_err(|e| anyhow::anyhow!("Failed to read /proc/meminfo: {}", e))?;

    let mut mem_total = 0u64;
    let mut mem_available = 0u64;

    for line in meminfo.lines() {
        if line.starts_with("MemTotal:") {
            mem_total = line
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
        } else if line.starts_with("MemAvailable:") {
            mem_available = line
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
        }
    }

    let mem_used_pct = if mem_total > 0 {
        ((mem_total - mem_available) as f64 / mem_total as f64) * 100.0
    } else {
        0.0
    };

    Ok(format!(
        "Load: {:.2}, Memory: {:.1}% used",
        load_1min, mem_used_pct
    ))
}

/// Get log statistics from logs directory
fn get_log_stats() -> Result<(usize, String, usize, String)> {
    use std::process::Command;

    // Count active logs and get size
    let active_output = Command::new("sh")
        .arg("-c")
        .arg("find logs/ -maxdepth 1 -name '*.log.*' -type f 2>/dev/null | wc -l")
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to count active logs: {}", e))?;

    let active_count = String::from_utf8_lossy(&active_output.stdout)
        .trim()
        .parse::<usize>()
        .unwrap_or(0);

    let active_size_output = Command::new("sh")
        .arg("-c")
        .arg("du -sh logs/ 2>/dev/null | cut -f1")
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to get active logs size: {}", e))?;

    let active_size = String::from_utf8_lossy(&active_size_output.stdout)
        .trim()
        .to_string();

    // Count archive logs and get size
    let archive_output = Command::new("sh")
        .arg("-c")
        .arg("find logs/archive/ -name '*.gz' -type f 2>/dev/null | wc -l")
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to count archive logs: {}", e))?;

    let archive_count = String::from_utf8_lossy(&archive_output.stdout)
        .trim()
        .parse::<usize>()
        .unwrap_or(0);

    let archive_size_output = Command::new("sh")
        .arg("-c")
        .arg("du -sh logs/archive/ 2>/dev/null | cut -f1")
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to get archive logs size: {}", e))?;

    let archive_size = String::from_utf8_lossy(&archive_size_output.stdout)
        .trim()
        .to_string();

    Ok((active_count, active_size, archive_count, archive_size))
}

/// Display startup banner with system info
fn display_startup_banner(config: &Config) {
    info!("╔═══════════════════════════════════════════════════════════╗");
    info!("║          FreezR Process Monitor v{}              ║", env!("CARGO_PKG_VERSION"));
    info!("╚═══════════════════════════════════════════════════════════╝");
    info!("");
    info!("📊 Monitoring Configuration:");
    info!("   └─ KESL: CPU {:.1}%, Memory {}MB (max {} violations)",
          config.kesl.cpu_threshold, config.kesl.memory_threshold_mb, config.kesl.max_violations);

    if config.node.enabled {
        info!("   └─ Node.js: CPU {:.1}%, Auto-kill: {}",
              config.node.cpu_threshold, config.node.auto_kill);
    }

    if config.snap.enabled {
        info!("   └─ Snap: CPU {:.1}%, Action: {}, Nice: {}",
              config.snap.cpu_threshold, config.snap.action, config.snap.nice_level);
    }

    if config.firefox.enabled {
        info!("   └─ Firefox: Freeze@{:.1}%, Kill@{:.1}%",
              config.firefox.cpu_threshold_freeze, config.firefox.cpu_threshold_kill);
    }

    if config.brave.enabled {
        info!("   └─ Brave: Freeze@{:.1}%, Kill@{:.1}%",
              config.brave.cpu_threshold_freeze, config.brave.cpu_threshold_kill);
    }

    if config.telegram.enabled {
        info!("   └─ Telegram: Freeze@{:.1}%, Kill@{:.1}%",
              config.telegram.cpu_threshold_freeze, config.telegram.cpu_threshold_kill);
    }

    if config.memory_pressure.enabled {
        info!("   └─ Memory Pressure: some {:.1}%/{:.1}%, full {:.1}%/{:.1}% ({}|{})",
              config.memory_pressure.some_threshold_warning,
              config.memory_pressure.some_threshold_critical,
              config.memory_pressure.full_threshold_warning,
              config.memory_pressure.full_threshold_critical,
              config.memory_pressure.action_warning,
              config.memory_pressure.action_critical);
    }

    info!("   └─ Check interval: {}s", config.monitoring.check_interval_secs);
    info!("");
}

/// Clear screen and move cursor to top
/// Export statistics to JSON file for dashboard consumption
fn export_stats_to_file(stats: &freezr_daemon::MonitorStats) -> Result<()> {
    const STATS_FILE: &str = "/tmp/freezr-stats.json";

    let json = serde_json::to_string_pretty(stats)?;
    std::fs::write(STATS_FILE, json)?;

    Ok(())
}

fn clear_screen() {
    print!("\x1B[2J\x1B[1;1H");
    use std::io::{self, Write};
    io::stdout().flush().unwrap();
}

/// Run monitoring with periodic statistics reporting
async fn run_with_stats(config: Config, report_interval: u64) -> Result<()> {
    use std::time::Duration;
    use tokio::time::{interval, sleep};

    info!("📊 Extended statistics mode enabled");
    info!("   └─ Report interval: {}s", report_interval);
    info!("   └─ Live dashboard mode: ON (screen will refresh)");
    info!("");

    let mut monitor = ResourceMonitor::new(
        &config.kesl.service_name,
        config.kesl.cpu_threshold,
        config.kesl.memory_threshold_mb,
        config.kesl.max_violations,
        config.monitoring.min_restart_interval_secs,
    );

    if config.node.enabled {
        monitor.enable_node_monitoring(config.node.cpu_threshold, config.node.auto_kill);
    }

    if config.snap.enabled {
        monitor.enable_snap_monitoring(
            config.snap.cpu_threshold,
            config.snap.action.clone(),
            config.snap.nice_level,
            config.snap.freeze_duration_secs,
            config.snap.max_violations,
        );
    }

    if config.firefox.enabled {
        monitor.enable_firefox_monitoring(
            config.firefox.cpu_threshold_freeze,
            config.firefox.cpu_threshold_kill,
            config.firefox.freeze_duration_secs,
            config.firefox.max_violations_freeze,
            config.firefox.max_violations_kill,
        );
    }

    if config.brave.enabled {
        monitor.enable_brave_monitoring(
            config.brave.cpu_threshold_freeze,
            config.brave.cpu_threshold_kill,
            config.brave.freeze_duration_secs,
            config.brave.max_violations_freeze,
            config.brave.max_violations_kill,
        );
    }

    if config.telegram.enabled {
        monitor.enable_telegram_monitoring(
            config.telegram.cpu_threshold_freeze,
            config.telegram.cpu_threshold_kill,
            config.telegram.freeze_duration_secs,
            config.telegram.max_violations_freeze,
            config.telegram.max_violations_kill,
        );
    }

    if config.memory_pressure.enabled {
        monitor.enable_memory_pressure_monitoring(
            config.memory_pressure.some_threshold_warning,
            config.memory_pressure.some_threshold_critical,
            config.memory_pressure.full_threshold_warning,
            config.memory_pressure.full_threshold_critical,
            config.memory_pressure.action_warning.clone(),
            config.memory_pressure.action_critical.clone(),
            config.memory_pressure.check_interval_secs,
        );
    }

    let check_interval = Duration::from_secs(config.monitoring.check_interval_secs);
    let mut report_timer = interval(Duration::from_secs(report_interval));

    let start_time = std::time::Instant::now();

    // Wait 3 seconds before first dashboard render
    sleep(Duration::from_secs(3)).await;

    loop {
        tokio::select! {
            _ = sleep(check_interval) => {
                // Perform monitoring check (silently, no logs to stdout)
                if let Err(e) = monitor.check() {
                    // Only log errors to file, not stdout
                    tracing::error!("Monitoring check failed: {}", e);
                }

                // Export stats to file for dashboard
                let uptime = start_time.elapsed().as_secs();
                let stats = monitor.export_stats(uptime);
                if let Err(e) = export_stats_to_file(&stats) {
                    tracing::error!("Failed to export stats: {}", e);
                }
            }
            _ = report_timer.tick() => {
                // Clear screen and display live dashboard
                clear_screen();

                let stats = monitor.stats();
                let (cpu_viol, mem_viol) = monitor.violations();
                let uptime = start_time.elapsed().as_secs();

                // Header
                println!("╔═══════════════════════════════════════════════════════════╗");
                println!("║          FreezR Process Monitor - Live Dashboard         ║");
                println!("╚═══════════════════════════════════════════════════════════╝");
                println!();

                // Runtime info
                println!("📈 Runtime: {}h {}m {}s", uptime / 3600, (uptime % 3600) / 60, uptime % 60);
                println!("📊 Total checks: {} (every {}s)", stats.total_checks, config.monitoring.check_interval_secs);
                println!();

                // Current KESL status
                println!("╔═══════════════════════════════════════════════════════════╗");
                println!("║                    KESL Process Status                    ║");
                println!("╚═══════════════════════════════════════════════════════════╝");

                // Get current KESL status from monitor
                let (kesl_cpu, kesl_mem) = monitor.get_kesl_status().unwrap_or((0.0, 0));

                println!("   PID: {} (current)", if kesl_cpu > 0.0 { "detected" } else { "not found" });
                println!("   CPU: {:.1}% (threshold: {:.1}%)", kesl_cpu, config.kesl.cpu_threshold);
                println!("   Memory: {}MB (threshold: {}MB)", kesl_mem, config.kesl.memory_threshold_mb);
                println!();

                // Violations summary
                println!("╔═══════════════════════════════════════════════════════════╗");
                println!("║                   Violations Summary                      ║");
                println!("╚═══════════════════════════════════════════════════════════╝");
                println!("   Total:");
                println!("      CPU violations: {}", stats.cpu_violations);
                println!("      Memory violations: {}", stats.memory_violations);
                println!("   Current session:");
                println!("      CPU: {} (need {} for restart)", cpu_viol, config.kesl.max_violations);
                println!("      Memory: {} (need {} for restart)", mem_viol, config.kesl.max_violations);

                // Violation rate
                if stats.total_checks > 0 {
                    let violation_rate = ((stats.cpu_violations + stats.memory_violations) as f64
                                         / stats.total_checks as f64) * 100.0;
                    println!("   Violation rate: {:.2}%", violation_rate);
                }
                println!();

                // Actions summary
                println!("╔═══════════════════════════════════════════════════════════╗");
                println!("║                    Actions Summary                        ║");
                println!("╚═══════════════════════════════════════════════════════════╝");
                println!("   🔄 KESL restarts: {}", stats.total_restarts);
                println!("   🔪 Node.js kills: {}", stats.total_kills);
                if config.snap.enabled {
                    println!("   ⚡ Snap actions: {} ({})", stats.total_kills, config.snap.action);
                }
                if config.firefox.enabled {
                    println!("   🦊 Firefox: Freeze@{:.1}%, Kill@{:.1}%",
                             config.firefox.cpu_threshold_freeze, config.firefox.cpu_threshold_kill);
                }
                if config.brave.enabled {
                    println!("   🦁 Brave: Freeze@{:.1}%, Kill@{:.1}%",
                             config.brave.cpu_threshold_freeze, config.brave.cpu_threshold_kill);
                }
                if config.telegram.enabled {
                    println!("   ✈️  Telegram: Freeze@{:.1}%, Kill@{:.1}%",
                             config.telegram.cpu_threshold_freeze, config.telegram.cpu_threshold_kill);
                }

                // Memory pressure status
                if config.memory_pressure.enabled {
                    if let Some((some_avg, full_avg, status, warn_count, crit_count)) = monitor.get_memory_pressure_status() {
                        let status_icon = match status.as_str() {
                            "CRITICAL" => "🔴",
                            "HIGH" => "🟠",
                            "MEDIUM" => "🟡",
                            "LOW" => "🟢",
                            _ => "⚪",
                        };
                        println!("   {} Memory Pressure: {} (some: {:.1}%, full: {:.1}%, w:{}/c:{})",
                                 status_icon, status, some_avg, full_avg, warn_count, crit_count);
                    }
                }
                println!();

                // System health
                println!("╔═══════════════════════════════════════════════════════════╗");
                println!("║                     System Health                         ║");
                println!("╚═══════════════════════════════════════════════════════════╝");
                if let Ok(health) = check_system_health() {
                    println!("   {}", health);
                }
                println!();

                // Log statistics
                println!("╔═══════════════════════════════════════════════════════════╗");
                println!("║                    Log Statistics                         ║");
                println!("╚═══════════════════════════════════════════════════════════╝");
                if let Ok((active_count, active_size, archive_count, archive_size)) = get_log_stats() {
                    println!("   📄 Active logs: {} files ({})", active_count, active_size);
                    println!("   🗜️  Archive logs: {} files ({})", archive_count, archive_size);
                    println!("   📊 Retention: 7 days active, 30 days archived");
                } else {
                    println!("   ⚠️  Unable to fetch log statistics");
                }
                println!();

                // Footer
                println!("Press Ctrl+C to stop monitoring");
                println!("Next refresh in {}s...", report_interval);
            }
        }
    }
}

/// Initialize logging system
fn init_logging() -> Result<()> {
    use tracing_appender::rolling::{RollingFileAppender, Rotation};
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    std::fs::create_dir_all("logs")?;

    let file_appender = RollingFileAppender::new(Rotation::DAILY, "logs", "process_monitor.log");

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
                .with_writer(file_appender),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_thread_ids(false)
                .with_file(false)
                .with_writer(std::io::stdout),
        )
        .init();

    Ok(())
}

/// Handle subcommands
async fn handle_subcommand(command: &Commands) -> Result<()> {
    match command {
        Commands::InstallService { yes } => install_systemd_service(*yes),
        Commands::UninstallService { yes } => uninstall_systemd_service(*yes),
        Commands::ServiceStatus => show_service_status(),
        Commands::Dashboard { interval } => show_dashboard(*interval).await,
    }
}

/// Install FreezR as systemd service
fn install_systemd_service(skip_confirm: bool) -> Result<()> {
    use std::fs;
    use std::io::{self, Write};
    use std::process::Command;

    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║       FreezR Systemd Service Installation                ║");
    println!("╚═══════════════════════════════════════════════════════════╝");
    println!();

    // Get current executable path
    let exe_path = std::env::current_exe()
        .map_err(|e| anyhow::anyhow!("Failed to get current executable path: {}", e))?;

    let exe_dir = exe_path.parent()
        .ok_or_else(|| anyhow::anyhow!("Failed to get executable directory"))?;

    let project_dir = exe_dir.parent()
        .and_then(|p| p.parent())
        .ok_or_else(|| anyhow::anyhow!("Failed to determine project directory"))?;

    let config_path = project_dir.join("freezr.toml");

    println!("📋 Installation details:");
    println!("   Executable: {}", exe_path.display());
    println!("   Config: {}", config_path.display());
    println!("   Service file: /etc/systemd/system/freezr.service");
    println!();

    // Check if we're root
    let is_root = unsafe { libc::geteuid() } == 0;

    if !is_root {
        return Err(anyhow::anyhow!(
            "This command must be run with sudo:\n   sudo {} install-service",
            exe_path.display()
        ));
    }

    // Check if service already exists
    let service_exists = std::path::Path::new("/etc/systemd/system/freezr.service").exists();

    if service_exists && !skip_confirm {
        print!("⚠️  FreezR service already exists. Overwrite? [y/N]: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("❌ Installation cancelled");
            return Ok(());
        }
    }

    // Stop existing service if running
    if service_exists {
        println!("⏹️  Stopping existing service...");
        let _ = Command::new("systemctl")
            .args(&["stop", "freezr.service"])
            .output();
    }

    // Generate service file content
    let service_content = format!(
        r#"[Unit]
Description=FreezR Process Monitor - Advanced Resource Management
Documentation=https://github.com/yourusername/freezr
After=network.target multi-user.target

[Service]
Type=simple
User={user}
Group={user}
WorkingDirectory={workdir}

# Main process with full monitoring and dashboard
ExecStart={exe} --config {config} --stats --report-interval 60

# Restart policy
Restart=always
RestartSec=10
KillMode=mixed
TimeoutStopSec=30

# Resource limits for the monitor itself
CPUQuota=5%
MemoryMax=50M
MemoryHigh=40M

# Logging
StandardOutput=journal
StandardError=journal
SyslogIdentifier=freezr

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=read-only
ReadWritePaths={workdir}/logs
ReadWritePaths={workdir}/data

# Process capabilities (needed for nice, freeze, kill)
AmbientCapabilities=CAP_SYS_NICE CAP_KILL
CapabilityBoundingSet=CAP_SYS_NICE CAP_KILL CAP_DAC_OVERRIDE

[Install]
WantedBy=multi-user.target
"#,
        user = std::env::var("SUDO_USER").unwrap_or_else(|_| "root".to_string()),
        workdir = project_dir.display(),
        exe = exe_path.display(),
        config = config_path.display(),
    );

    // Write service file
    println!("📝 Writing service file...");
    fs::write("/etc/systemd/system/freezr.service", service_content)
        .map_err(|e| anyhow::anyhow!("Failed to write service file: {}", e))?;

    // Set permissions
    fs::set_permissions(
        "/etc/systemd/system/freezr.service",
        std::fs::Permissions::from_mode(0o644),
    )?;

    // Reload systemd
    println!("🔄 Reloading systemd daemon...");
    let output = Command::new("systemctl")
        .arg("daemon-reload")
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to run systemctl daemon-reload: {}", e))?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("systemctl daemon-reload failed"));
    }

    // Enable service
    println!("✅ Enabling service (auto-start on boot)...");
    let output = Command::new("systemctl")
        .args(&["enable", "freezr.service"])
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to enable service: {}", e))?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("Failed to enable service"));
    }

    // Start service
    println!("🚀 Starting service...");
    let output = Command::new("systemctl")
        .args(&["start", "freezr.service"])
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to start service: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Failed to start service: {}", stderr));
    }

    // Wait a bit
    std::thread::sleep(std::time::Duration::from_secs(2));

    // Check status
    let status_output = Command::new("systemctl")
        .args(&["is-active", "freezr.service"])
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to check service status: {}", e))?;

    let is_active = String::from_utf8_lossy(&status_output.stdout)
        .trim()
        .eq("active");

    println!();
    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║                Installation Complete                      ║");
    println!("╚═══════════════════════════════════════════════════════════╝");
    println!();

    if is_active {
        println!("✅ FreezR service is RUNNING");
        println!();
        println!("📊 Useful commands:");
        println!("   View logs (real-time):  sudo journalctl -u freezr -f");
        println!("   View logs (last 50):    sudo journalctl -u freezr -n 50");
        println!("   Check status:           sudo systemctl status freezr");
        println!("   Restart:                sudo systemctl restart freezr");
        println!("   Stop:                   sudo systemctl stop freezr");
        println!("   Disable auto-start:     sudo systemctl disable freezr");
    } else {
        println!("❌ Service failed to start!");
        println!("   Check logs: sudo journalctl -u freezr -n 50");
        return Err(anyhow::anyhow!("Service is not active"));
    }

    Ok(())
}

/// Uninstall FreezR systemd service
fn uninstall_systemd_service(skip_confirm: bool) -> Result<()> {
    use std::fs;
    use std::io::{self, Write};
    use std::process::Command;

    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║       FreezR Systemd Service Uninstallation              ║");
    println!("╚═══════════════════════════════════════════════════════════╝");
    println!();

    // Check if we're root
    let is_root = unsafe { libc::geteuid() } == 0;

    if !is_root {
        let exe_path = std::env::current_exe()?;
        return Err(anyhow::anyhow!(
            "This command must be run with sudo:\n   sudo {} uninstall-service",
            exe_path.display()
        ));
    }

    // Check if service exists
    if !std::path::Path::new("/etc/systemd/system/freezr.service").exists() {
        println!("ℹ️  FreezR service is not installed");
        return Ok(());
    }

    if !skip_confirm {
        print!("⚠️  Remove FreezR systemd service? [y/N]: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("❌ Uninstallation cancelled");
            return Ok(());
        }
    }

    // Stop service
    println!("⏹️  Stopping service...");
    let _ = Command::new("systemctl")
        .args(&["stop", "freezr.service"])
        .output();

    // Disable service
    println!("🔧 Disabling service...");
    let _ = Command::new("systemctl")
        .args(&["disable", "freezr.service"])
        .output();

    // Remove service file
    println!("🗑️  Removing service file...");
    fs::remove_file("/etc/systemd/system/freezr.service")
        .map_err(|e| anyhow::anyhow!("Failed to remove service file: {}", e))?;

    // Reload systemd
    println!("🔄 Reloading systemd daemon...");
    let _ = Command::new("systemctl")
        .arg("daemon-reload")
        .output();

    println!();
    println!("✅ FreezR service uninstalled successfully");

    Ok(())
}

/// Show live monitoring dashboard (read-only)
async fn show_dashboard(interval_secs: u64) -> Result<()> {
    use freezr_daemon::MonitorStats;
    use std::fs;
    use std::io::{self, Write};
    use std::time::Duration;
    use tokio::time;

    const STATS_FILE: &str = "/tmp/freezr-stats.json";

    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║       FreezR Live Dashboard (Read-Only Mode)             ║");
    println!("╚═══════════════════════════════════════════════════════════╝");
    println!();
    println!("📡 Reading stats from: {}", STATS_FILE);
    println!("🔄 Update interval: {} seconds", interval_secs);
    println!("Press Ctrl+C to exit");
    println!();

    let mut interval = time::interval(Duration::from_secs(interval_secs));

    loop {
        interval.tick().await;

        // Read stats from JSON file
        let stats: MonitorStats = match fs::read_to_string(STATS_FILE) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(s) => s,
                Err(e) => {
                    println!("⚠️  Failed to parse stats: {}", e);
                    println!("   Waiting for service to write stats...");
                    continue;
                }
            },
            Err(e) => {
                println!("⚠️  Stats file not found: {}", e);
                println!("   Is the FreezR service running?");
                println!("   Start with: sudo systemctl start freezr");
                println!();
                time::sleep(Duration::from_secs(2)).await;
                continue;
            }
        };

        // Clear screen and display dashboard
        print!("\x1B[2J\x1B[1;1H"); // Clear screen and move cursor to top
        io::stdout().flush()?;

        // Header
        println!("╔═══════════════════════════════════════════════════════════╗");
        println!("║          FreezR Process Monitor - Live Dashboard         ║");
        println!("╚═══════════════════════════════════════════════════════════╝");
        println!();

        // Runtime
        let hours = stats.runtime_secs / 3600;
        let minutes = (stats.runtime_secs % 3600) / 60;
        let seconds = stats.runtime_secs % 60;
        println!("📈 Runtime: {}h {}m {}s", hours, minutes, seconds);
        println!("📊 Total checks: {} (every 3s)", stats.total_checks);
        println!();

        // KESL Status
        println!("╔═══════════════════════════════════════════════════════════╗");
        println!("║                    KESL Process Status                    ║");
        println!("╚═══════════════════════════════════════════════════════════╝");
        if let Some(pid) = stats.kesl.pid {
            println!("   PID: {} (current)", pid);
        } else {
            println!("   PID: not found (current)");
        }
        println!("   CPU: {:.1}% (threshold: {:.1}%)", stats.kesl.cpu_percent, stats.kesl.cpu_threshold);
        println!("   Memory: {}MB (threshold: {}MB)", stats.kesl.memory_mb, stats.kesl.memory_threshold_mb);
        println!();

        // Violations Summary
        println!("╔═══════════════════════════════════════════════════════════╗");
        println!("║                   Violations Summary                      ║");
        println!("╚═══════════════════════════════════════════════════════════╝");
        println!("   Total:");
        println!("      CPU violations: {}", stats.kesl.total_cpu_violations);
        println!("      Memory violations: {}", stats.kesl.total_memory_violations);
        println!("   Current session:");
        println!("      CPU: {} (need {} for restart)", stats.kesl.current_cpu_violations, stats.kesl.max_violations);
        println!("      Memory: {} (need {} for restart)", stats.kesl.current_memory_violations, stats.kesl.max_violations);
        if stats.total_checks > 0 {
            println!("   Violation rate: {:.2}%", stats.kesl.violation_rate);
        }
        println!();

        // Actions Summary
        println!("╔═══════════════════════════════════════════════════════════╗");
        println!("║                    Actions Summary                        ║");
        println!("╚═══════════════════════════════════════════════════════════╝");
        println!("   🔄 KESL restarts: {}", stats.kesl.total_restarts);
        println!("   🔪 Node.js kills: {}", stats.node.total_kills);
        println!("   ⚡ Snap actions: {} ({})", stats.snap.total_actions, stats.snap.action);
        println!("   🦊 Firefox: Freeze@{:.1}%, Kill@{:.1}%", stats.firefox.freeze_threshold, stats.firefox.kill_threshold);
        println!("   🦁 Brave: Freeze@{:.1}%, Kill@{:.1}%", stats.brave.freeze_threshold, stats.brave.kill_threshold);
        println!("   ✈️  Telegram: Freeze@{:.1}%, Kill@{:.1}%", stats.telegram.freeze_threshold, stats.telegram.kill_threshold);

        // Memory Pressure
        let mp_icon = match stats.memory_pressure.status.as_str() {
            "CRITICAL" => "🔴",
            "HIGH" => "🟠",
            "MEDIUM" => "🟡",
            "LOW" => "🟢",
            _ => "⚪",
        };
        println!("   {} Memory Pressure: {} (some: {:.1}%, full: {:.1}%, w:{}/c:{})",
            mp_icon,
            stats.memory_pressure.status,
            stats.memory_pressure.some_avg10,
            stats.memory_pressure.full_avg10,
            stats.memory_pressure.warning_count,
            stats.memory_pressure.critical_count
        );
        println!();

        // System Health
        println!("╔═══════════════════════════════════════════════════════════╗");
        println!("║                     System Health                         ║");
        println!("╚═══════════════════════════════════════════════════════════╝");
        println!("   Load: {:.2}, Memory: {:.1}% used",
            stats.system_health.load_1min,
            stats.system_health.memory_used_percent
        );
        println!();

        // Log Statistics
        println!("╔═══════════════════════════════════════════════════════════╗");
        println!("║                    Log Statistics                         ║");
        println!("╚═══════════════════════════════════════════════════════════╝");
        println!("   📄 Active logs: {} files ({})", stats.log_stats.active_files, stats.log_stats.active_size);
        println!("   🗜️  Archive logs: {} files ({})", stats.log_stats.archive_files, stats.log_stats.archive_size);
        println!("   📊 Retention: 7 days active, 30 days archived");
        println!();

        println!("Press Ctrl+C to stop monitoring");
        println!("Next refresh in {}s...", interval_secs);
    }
}

/// Show systemd service status
fn show_service_status() -> Result<()> {
    use std::process::Command;

    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║            FreezR Systemd Service Status                 ║");
    println!("╚═══════════════════════════════════════════════════════════╝");
    println!();

    // Check if service file exists
    if !std::path::Path::new("/etc/systemd/system/freezr.service").exists() {
        println!("❌ FreezR service is not installed");
        println!();
        println!("Install with:");
        let exe_path = std::env::current_exe()?;
        println!("   sudo {} install-service", exe_path.display());
        return Ok(());
    }

    // Run systemctl status
    let output = Command::new("systemctl")
        .args(&["status", "freezr.service", "--no-pager", "-l"])
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to run systemctl status: {}", e))?;

    print!("{}", String::from_utf8_lossy(&output.stdout));

    println!();
    println!("📊 Quick commands:");
    println!("   Logs (real-time):  sudo journalctl -u freezr -f");
    println!("   Logs (last 50):    sudo journalctl -u freezr -n 50");
    println!("   Restart:           sudo systemctl restart freezr");
    println!("   Stop:              sudo systemctl stop freezr");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse arguments first (before logging for subcommands)
    let args = Args::parse();

    // Handle subcommands (don't need logging/monitoring for these)
    if let Some(command) = &args.command {
        return handle_subcommand(command).await;
    }

    // Initialize logging first
    init_logging()?;

    info!("🦀 Process Monitor starting...");
    info!("   Rust version: {}", env!("CARGO_PKG_RUST_VERSION"));
    info!("   Package version: {}", env!("CARGO_PKG_VERSION"));

    // Pre-flight checks
    info!("🔍 Running pre-flight checks...");

    // Kill old instances
    if let Err(e) = kill_old_instances() {
        warn!("⚠️  Failed to kill old instances: {}", e);
        warn!("⚠️  Continuing anyway, but conflicts may occur");
    }

    // Ensure directories exist
    match ensure_directories() {
        Ok(msg) => info!("{}", msg),
        Err(e) => {
            error!("❌ Directory setup failed: {}", e);
            error!("❌ Cannot continue without proper directory structure");
            std::process::exit(1);
        }
    }

    // Disk space check
    match check_disk_space("logs/") {
        Ok(usage) => {
            if usage > 95 {
                error!("❌ DISK SPACE CRITICAL: {}% used in logs/ directory", usage);
                error!("❌ Cannot continue - risk of data loss");
                std::process::exit(1);
            } else if usage > 90 {
                warn!("⚠️  Disk space warning: {}% used in logs/ directory", usage);
                warn!("⚠️  Consider cleaning old logs or adding more disk space");
            } else {
                info!("✅ Disk space check passed: {}% used", usage);
            }
        }
        Err(e) => {
            warn!("⚠️  Disk space check failed: {}", e);
            warn!("⚠️  Continuing anyway, but logs may be at risk...");
        }
    }

    // System health check
    match check_system_health() {
        Ok(health) => info!("✅ System health: {}", health),
        Err(e) => warn!("⚠️  System health check failed: {}", e),
    }

    // Time check
    let hour = chrono::Local::now().hour();
    if hour < 6 || hour >= 23 {
        warn!("⚠️  Late night/early morning hours: {} - System may be idle", hour);
    }

    info!("");

    // Load configuration
    let config = if args.config.exists() {
        info!("📋 Loading configuration from: {:?}", args.config);
        Config::load_from_file(args.config.to_str().unwrap())
            .map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?
    } else {
        warn!("⚠️  Configuration file not found: {:?}", args.config);
        warn!("⚠️  Using default configuration");
        Config::default()
    };

    // Validate configuration
    config
        .validate()
        .map_err(|e| anyhow::anyhow!("Configuration validation failed: {}", e))?;

    info!("✅ Configuration validated successfully");

    // Display startup banner
    display_startup_banner(&config);

    // Run monitoring
    if args.stats {
        run_with_stats(config, args.report_interval).await?;
    } else {
        // Normal monitoring mode
        use freezr_daemon::ResourceMonitor;
        use std::time::Duration;
        use tokio::time::sleep;

        let mut monitor = ResourceMonitor::new(
            &config.kesl.service_name,
            config.kesl.cpu_threshold,
            config.kesl.memory_threshold_mb,
            config.kesl.max_violations,
            config.monitoring.min_restart_interval_secs,
        );

        if config.node.enabled {
            monitor.enable_node_monitoring(config.node.cpu_threshold, config.node.auto_kill);
        }

        if config.snap.enabled {
            monitor.enable_snap_monitoring(
                config.snap.cpu_threshold,
                config.snap.action.clone(),
                config.snap.nice_level,
                config.snap.freeze_duration_secs,
                config.snap.max_violations,
            );
        }

        if config.firefox.enabled {
            monitor.enable_firefox_monitoring(
                config.firefox.cpu_threshold_freeze,
                config.firefox.cpu_threshold_kill,
                config.firefox.freeze_duration_secs,
                config.firefox.max_violations_freeze,
                config.firefox.max_violations_kill,
            );
        }

        if config.brave.enabled {
            monitor.enable_brave_monitoring(
                config.brave.cpu_threshold_freeze,
                config.brave.cpu_threshold_kill,
                config.brave.freeze_duration_secs,
                config.brave.max_violations_freeze,
                config.brave.max_violations_kill,
            );
        }

        let check_interval = Duration::from_secs(config.monitoring.check_interval_secs);

        info!("🚀 Starting monitoring loop...");
        info!("");

        loop {
            if let Err(e) = monitor.check() {
                error!("Monitoring check failed: {}", e);
            }

            let stats = monitor.stats();
            let (cpu_viol, mem_viol) = monitor.violations();

            info!(
                "Stats: checks={}, violations={}/{}, restarts={}, kills={}",
                stats.total_checks, cpu_viol, mem_viol, stats.total_restarts, stats.total_kills
            );

            sleep(check_interval).await;
        }
    }

    Ok(())
}
