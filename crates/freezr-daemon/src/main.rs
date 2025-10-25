use anyhow::Result;
use clap::{Parser, Subcommand};
use freezr_core::VERSION;
use freezr_daemon::{Config, ResourceMonitor};
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// FreezR - System resource guardian daemon
///
/// Prevents system freezes by monitoring and managing runaway processes
#[derive(Parser, Debug)]
#[command(name = "freezr-daemon")]
#[command(version = VERSION)]
#[command(about = "FreezR Daemon - System resource guardian", long_about = None)]
struct Cli {
    /// Path to configuration file
    #[arg(short, long, default_value = "/etc/freezr/config.toml")]
    config: PathBuf,

    /// Subcommand to execute
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Show current status (single check)
    Monitor,

    /// Continuous monitoring with automatic actions
    Watch,

    /// Force restart KESL service with limits
    ForceRestart,

    /// Generate default configuration file
    GenerateConfig {
        /// Output path for config file
        #[arg(short, long, default_value = "config.toml")]
        output: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging first
    init_logging()?;

    info!("FreezR Daemon v{} starting...", VERSION);

    // Load configuration
    let config = load_config(&cli.config)?;

    // Validate configuration
    config
        .validate()
        .map_err(|e| anyhow::anyhow!("Configuration validation failed: {}", e))?;

    info!("Configuration loaded and validated successfully");

    // Execute command
    match cli.command {
        Some(Commands::Monitor) => run_monitor_once(config).await?,
        Some(Commands::Watch) => run_watch_loop(config).await?,
        Some(Commands::ForceRestart) => run_force_restart(config).await?,
        Some(Commands::GenerateConfig { output }) => generate_config(output)?,
        None => {
            // Default: run watch loop
            info!("No command specified, running watch loop by default");
            run_watch_loop(config).await?
        }
    }

    Ok(())
}

/// Run single monitoring check and display status
async fn run_monitor_once(config: Config) -> Result<()> {
    info!("Running single monitoring check...");

    let mut monitor = create_monitor(&config);

    // Perform single check
    monitor.check()?;

    // Display statistics
    let stats = monitor.stats();
    let (cpu_violations, mem_violations) = monitor.violations();

    info!("=== Monitoring Status ===");
    info!("Total checks: {}", stats.total_checks);
    info!(
        "CPU violations: {} (current session: {})",
        stats.cpu_violations, cpu_violations
    );
    info!(
        "Memory violations: {} (current session: {})",
        stats.memory_violations, mem_violations
    );
    info!("Total restarts: {}", stats.total_restarts);
    info!("Total kills: {}", stats.total_kills);

    info!("Single check completed");
    Ok(())
}

/// Run continuous monitoring loop
async fn run_watch_loop(config: Config) -> Result<()> {
    info!("Starting continuous monitoring loop...");
    info!(
        "Check interval: {}s, Max violations: {}",
        config.monitoring.check_interval_secs, config.kesl.max_violations
    );

    if config.node.enabled {
        warn!(
            "Node.js monitoring enabled: CPU threshold {:.1}%, auto-kill: {}",
            config.node.cpu_threshold, config.node.auto_kill
        );
    }

    let mut monitor = create_monitor(&config);

    let check_interval = Duration::from_secs(config.monitoring.check_interval_secs);

    loop {
        // Perform monitoring check
        if let Err(e) = monitor.check() {
            error!("Monitoring check failed: {}", e);
        }

        // Display current status
        let stats = monitor.stats();
        let (cpu_violations, mem_violations) = monitor.violations();

        info!(
            "Stats: checks={}, violations={}/{}, restarts={}, kills={}",
            stats.total_checks, cpu_violations, mem_violations, stats.total_restarts, stats.total_kills
        );

        // Sleep until next check
        sleep(check_interval).await;
    }
}

/// Force restart KESL service
async fn run_force_restart(config: Config) -> Result<()> {
    info!("Forcing KESL service restart...");

    use freezr_core::systemd::SystemdService;

    let mut service = SystemdService::new(&config.kesl.service_name);

    // Check if service is active
    if !service.is_active()? {
        error!("KESL service is not active, cannot restart");
        return Err(anyhow::anyhow!("Service is not active"));
    }

    // Restart with daemon-reload
    service.restart_with_reload()?;

    info!("KESL service restarted successfully");
    Ok(())
}

/// Generate default configuration file
fn generate_config(output: PathBuf) -> Result<()> {
    info!("Generating default configuration file: {:?}", output);

    let config = Config::default();
    config
        .save_to_file(output.to_str().unwrap())
        .map_err(|e| anyhow::anyhow!("Failed to save configuration file: {}", e))?;

    info!("Configuration file generated successfully");
    Ok(())
}

/// Create ResourceMonitor from configuration
fn create_monitor(config: &Config) -> ResourceMonitor {
    let mut monitor = ResourceMonitor::new(
        &config.kesl.service_name,
        config.kesl.cpu_threshold,
        config.kesl.memory_threshold_mb,
        config.kesl.max_violations,
        config.monitoring.min_restart_interval_secs,
    );

    // Enable Node.js monitoring if configured
    if config.node.enabled {
        monitor.enable_node_monitoring(config.node.cpu_threshold, config.node.auto_kill);
    }

    monitor
}

/// Load configuration from file or use defaults
fn load_config(path: &PathBuf) -> Result<Config> {
    if path.exists() {
        info!("Loading configuration from: {:?}", path);
        Config::load_from_file(path.to_str().unwrap())
            .map_err(|e| anyhow::anyhow!("Failed to load configuration file: {}", e))
    } else {
        warn!(
            "Configuration file not found: {:?}, using defaults",
            path
        );
        Ok(Config::default())
    }
}

/// Initialize logging with file and stdout output
fn init_logging() -> Result<()> {
    // Create logs directory if it doesn't exist
    std::fs::create_dir_all("./logs")?;

    // File appender for daemon logs
    let file_appender = tracing_appender::rolling::daily("./logs", "freezr-daemon.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Setup tracing subscriber with both stdout and file output
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stdout)
                .with_ansi(true)
                .with_target(false),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_target(true),
        )
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    // Prevent _guard from being dropped
    std::mem::forget(_guard);

    info!("Logging initialized");

    Ok(())
}
