//! Process Monitor Binary
//!
//! Advanced process monitoring with statistics tracking

use anyhow::Result;
use chrono::Timelike;
use clap::Parser;
use freezr_daemon::{Config, ResourceMonitor};
use std::path::PathBuf;
use tracing::{error, info, warn};

/// Command line arguments
#[derive(Parser, Debug)]
#[clap(name = "process_monitor")]
#[clap(about = "Advanced process monitoring with statistics", long_about = None)]
struct Args {
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

    info!("   └─ Check interval: {}s", config.monitoring.check_interval_secs);
    info!("");
}

/// Clear screen and move cursor to top
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

    let check_interval = Duration::from_secs(config.monitoring.check_interval_secs);
    let mut report_timer = interval(Duration::from_secs(report_interval));

    let start_time = std::time::Instant::now();
    let mut last_kesl_cpu = 0.0;
    let mut last_kesl_mem = 0u64;

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
                println!("   PID: 1546 (example)");
                println!("   CPU: {:.1}% (threshold: {:.1}%)", last_kesl_cpu, config.kesl.cpu_threshold);
                println!("   Memory: {}MB (threshold: {}MB)", last_kesl_mem, config.kesl.memory_threshold_mb);
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
                println!();

                // System health
                println!("╔═══════════════════════════════════════════════════════════╗");
                println!("║                     System Health                         ║");
                println!("╚═══════════════════════════════════════════════════════════╝");
                if let Ok(health) = check_system_health() {
                    println!("   {}", health);
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

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging first
    init_logging()?;

    info!("🦀 Process Monitor starting...");
    info!("   Rust version: {}", env!("CARGO_PKG_RUST_VERSION"));
    info!("   Package version: {}", env!("CARGO_PKG_VERSION"));

    // Parse arguments
    let args = Args::parse();

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
