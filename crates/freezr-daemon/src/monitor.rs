use freezr_core::{
    error::{Error, Result},
    executor::ProcessExecutor,
    memory_pressure::MemoryPressure,
    scanner::ProcessScanner,
    systemd::SystemdService,
    types::MonitorStats,
    CgroupManager,
};
use tracing::{debug, error, info, warn};
use std::time::{Duration, Instant};

/// Resource monitor with violation tracking
///
/// Monitors KESL and Node.js processes, tracks resource violations,
/// and executes actions (restart service, kill processes) when limits are exceeded.
pub struct ResourceMonitor {
    scanner: ProcessScanner,
    kesl_service: SystemdService,
    stats: MonitorStats,

    // Cgroup integration (optional)
    cgroup_manager: Option<CgroupManager>,

    // Violation counters
    cpu_violations: u32,
    memory_violations: u32,

    // Configuration thresholds
    cpu_threshold: f64,
    memory_threshold_mb: u64,
    max_violations: u32,

    // Minimum restart interval (used by SystemdService internally)
    #[allow(dead_code)]
    min_restart_interval_secs: u64,

    // Node.js monitoring
    node_enabled: bool,
    node_cpu_threshold: f64,
    node_auto_kill: bool,

    // Snap/snapd monitoring
    snap_enabled: bool,
    snap_cpu_threshold: f64,
    snap_action: String,
    snap_nice_level: i32,
    snap_freeze_duration_secs: u64,
    snap_violations: u32,
    snap_max_violations: u32,

    // Firefox monitoring (two-tier strategy)
    firefox_enabled: bool,
    firefox_cpu_threshold_freeze: f64,
    firefox_cpu_threshold_kill: f64,
    firefox_freeze_duration_secs: u64,
    firefox_violations_freeze: u32,
    firefox_violations_kill: u32,
    firefox_max_violations_freeze: u32,
    firefox_max_violations_kill: u32,

    // Brave monitoring (two-tier strategy)
    brave_enabled: bool,
    brave_cpu_threshold_freeze: f64,
    brave_cpu_threshold_kill: f64,
    brave_freeze_duration_secs: u64,
    brave_violations_freeze: u32,
    brave_violations_kill: u32,
    brave_max_violations_freeze: u32,
    brave_max_violations_kill: u32,

    // Telegram monitoring (two-tier strategy)
    telegram_enabled: bool,
    telegram_cpu_threshold_freeze: f64,
    telegram_cpu_threshold_kill: f64,
    telegram_freeze_duration_secs: u64,
    telegram_violations_freeze: u32,
    telegram_violations_kill: u32,
    telegram_max_violations_freeze: u32,
    telegram_max_violations_kill: u32,

    // Memory pressure monitoring (PSI - Pressure Stall Information)
    memory_pressure_enabled: bool,
    memory_pressure_some_threshold_warning: f64,
    memory_pressure_some_threshold_critical: f64,
    memory_pressure_full_threshold_warning: f64,
    memory_pressure_full_threshold_critical: f64,
    memory_pressure_action_warning: String,
    memory_pressure_action_critical: String,
    memory_pressure_check_interval: Duration,
    memory_pressure_last_check: Instant,
    memory_pressure_warning_count: u32,
    memory_pressure_critical_count: u32,
}

impl ResourceMonitor {
    /// Create new resource monitor
    ///
    /// # Arguments
    /// * `service_name` - Name of the systemd service to monitor (e.g., "kesl")
    /// * `cpu_threshold` - CPU threshold in percent (e.g., 30.0)
    /// * `memory_threshold_mb` - Memory threshold in MB (e.g., 600)
    /// * `max_violations` - Maximum violations before restart (e.g., 3)
    /// * `min_restart_interval_secs` - Minimum interval between restarts (e.g., 100)
    pub fn new(
        service_name: &str,
        cpu_threshold: f64,
        memory_threshold_mb: u64,
        max_violations: u32,
        min_restart_interval_secs: u64,
    ) -> Self {
        Self {
            scanner: ProcessScanner::new(),
            kesl_service: SystemdService::new(service_name),
            stats: MonitorStats::new(),
            cgroup_manager: None,  // Initialized later if enabled

            cpu_violations: 0,
            memory_violations: 0,

            cpu_threshold,
            memory_threshold_mb,
            max_violations,
            min_restart_interval_secs,

            node_enabled: false,
            node_cpu_threshold: 80.0,
            node_auto_kill: false,

            snap_enabled: false,
            snap_cpu_threshold: 300.0,
            snap_action: "nice".to_string(),
            snap_nice_level: 15,
            snap_freeze_duration_secs: 5,
            snap_violations: 0,
            snap_max_violations: 3,

            firefox_enabled: false,
            firefox_cpu_threshold_freeze: 80.0,
            firefox_cpu_threshold_kill: 95.0,
            firefox_freeze_duration_secs: 5,
            firefox_violations_freeze: 0,
            firefox_violations_kill: 0,
            firefox_max_violations_freeze: 2,
            firefox_max_violations_kill: 3,

            brave_enabled: false,
            brave_cpu_threshold_freeze: 80.0,
            brave_cpu_threshold_kill: 95.0,
            brave_freeze_duration_secs: 5,
            brave_violations_freeze: 0,
            brave_violations_kill: 0,
            brave_max_violations_freeze: 2,
            brave_max_violations_kill: 3,

            telegram_enabled: false,
            telegram_cpu_threshold_freeze: 80.0,
            telegram_cpu_threshold_kill: 95.0,
            telegram_freeze_duration_secs: 5,
            telegram_violations_freeze: 0,
            telegram_violations_kill: 0,
            telegram_max_violations_freeze: 2,
            telegram_max_violations_kill: 3,

            memory_pressure_enabled: false,
            memory_pressure_some_threshold_warning: 10.0,
            memory_pressure_some_threshold_critical: 30.0,
            memory_pressure_full_threshold_warning: 5.0,
            memory_pressure_full_threshold_critical: 15.0,
            memory_pressure_action_warning: "log".to_string(),
            memory_pressure_action_critical: "freeze".to_string(),
            memory_pressure_check_interval: Duration::from_secs(5),
            memory_pressure_last_check: Instant::now(),
            memory_pressure_warning_count: 0,
            memory_pressure_critical_count: 0,
        }
    }

    /// Initialize cgroup manager (call after construction if enabled)
    pub fn initialize_cgroups(&mut self, config: freezr_core::CgroupConfig) -> Result<()> {
        if !config.enabled {
            return Ok(());
        }

        let mut manager = CgroupManager::new(config)?;
        manager.initialize()?;

        info!("Cgroup integration initialized");
        self.cgroup_manager = Some(manager);
        Ok(())
    }

    /// Cleanup cgroups on shutdown
    pub fn cleanup_cgroups(&mut self) -> Result<()> {
        if let Some(manager) = &mut self.cgroup_manager {
            manager.on_service_stop()?;
        }
        Ok(())
    }

    /// Enable Node.js process monitoring
    ///
    /// # Arguments
    /// * `cpu_threshold` - CPU threshold for Node.js processes (e.g., 80.0)
    /// * `auto_kill` - Automatically kill high-CPU Node.js processes
    pub fn enable_node_monitoring(&mut self, cpu_threshold: f64, auto_kill: bool) {
        self.node_enabled = true;
        self.node_cpu_threshold = cpu_threshold;
        self.node_auto_kill = auto_kill;
        info!(
            "Node.js monitoring enabled: CPU threshold {:.1}%, auto-kill: {}",
            cpu_threshold, auto_kill
        );
    }

    /// Enable Snap/snapd process monitoring
    ///
    /// # Arguments
    /// * `cpu_threshold` - CPU threshold for snap processes (e.g., 300.0)
    /// * `action` - Action to take: "freeze", "nice", or "kill"
    /// * `nice_level` - Nice level for "nice" action (0-19)
    /// * `freeze_duration_secs` - Freeze duration for "freeze" action
    /// * `max_violations` - Maximum violations before action
    pub fn enable_snap_monitoring(
        &mut self,
        cpu_threshold: f64,
        action: String,
        nice_level: i32,
        freeze_duration_secs: u64,
        max_violations: u32,
    ) {
        self.snap_enabled = true;
        self.snap_cpu_threshold = cpu_threshold;
        self.snap_action = action.clone();
        self.snap_nice_level = nice_level;
        self.snap_freeze_duration_secs = freeze_duration_secs;
        self.snap_max_violations = max_violations;
        info!(
            "Snap monitoring enabled: CPU threshold {:.1}%, action: {}, nice: {}, max violations: {}",
            cpu_threshold, action, nice_level, max_violations
        );
    }

    /// Enable Firefox process monitoring (two-tier strategy)
    ///
    /// # Arguments
    /// * `cpu_threshold_freeze` - CPU threshold for freezing (e.g., 80.0)
    /// * `cpu_threshold_kill` - CPU threshold for killing (e.g., 95.0)
    /// * `freeze_duration_secs` - Freeze duration in seconds
    /// * `max_violations_freeze` - Maximum violations before freeze
    /// * `max_violations_kill` - Maximum violations before kill
    pub fn enable_firefox_monitoring(
        &mut self,
        cpu_threshold_freeze: f64,
        cpu_threshold_kill: f64,
        freeze_duration_secs: u64,
        max_violations_freeze: u32,
        max_violations_kill: u32,
    ) {
        self.firefox_enabled = true;
        self.firefox_cpu_threshold_freeze = cpu_threshold_freeze;
        self.firefox_cpu_threshold_kill = cpu_threshold_kill;
        self.firefox_freeze_duration_secs = freeze_duration_secs;
        self.firefox_max_violations_freeze = max_violations_freeze;
        self.firefox_max_violations_kill = max_violations_kill;
        info!(
            "Firefox monitoring enabled: freeze at {:.1}% ({} violations), kill at {:.1}% ({} violations)",
            cpu_threshold_freeze, max_violations_freeze, cpu_threshold_kill, max_violations_kill
        );
    }

    /// Enable Brave browser process monitoring (two-tier strategy)
    ///
    /// # Arguments
    /// * `cpu_threshold_freeze` - CPU threshold for freezing (e.g., 80.0)
    /// * `cpu_threshold_kill` - CPU threshold for killing (e.g., 95.0)
    /// * `freeze_duration_secs` - Freeze duration in seconds
    /// * `max_violations_freeze` - Maximum violations before freeze
    /// * `max_violations_kill` - Maximum violations before kill
    pub fn enable_brave_monitoring(
        &mut self,
        cpu_threshold_freeze: f64,
        cpu_threshold_kill: f64,
        freeze_duration_secs: u64,
        max_violations_freeze: u32,
        max_violations_kill: u32,
    ) {
        self.brave_enabled = true;
        self.brave_cpu_threshold_freeze = cpu_threshold_freeze;
        self.brave_cpu_threshold_kill = cpu_threshold_kill;
        self.brave_freeze_duration_secs = freeze_duration_secs;
        self.brave_max_violations_freeze = max_violations_freeze;
        self.brave_max_violations_kill = max_violations_kill;
        info!(
            "Brave monitoring enabled: freeze at {:.1}% ({} violations), kill at {:.1}% ({} violations)",
            cpu_threshold_freeze, max_violations_freeze, cpu_threshold_kill, max_violations_kill
        );
    }

    /// Enable Telegram messenger process monitoring (two-tier strategy)
    ///
    /// # Arguments
    /// * `cpu_threshold_freeze` - CPU threshold for freezing (e.g., 80.0)
    /// * `cpu_threshold_kill` - CPU threshold for killing (e.g., 95.0)
    /// * `freeze_duration_secs` - Freeze duration in seconds
    /// * `max_violations_freeze` - Maximum violations before freeze
    /// * `max_violations_kill` - Maximum violations before kill
    pub fn enable_telegram_monitoring(
        &mut self,
        cpu_threshold_freeze: f64,
        cpu_threshold_kill: f64,
        freeze_duration_secs: u64,
        max_violations_freeze: u32,
        max_violations_kill: u32,
    ) {
        self.telegram_enabled = true;
        self.telegram_cpu_threshold_freeze = cpu_threshold_freeze;
        self.telegram_cpu_threshold_kill = cpu_threshold_kill;
        self.telegram_freeze_duration_secs = freeze_duration_secs;
        self.telegram_max_violations_freeze = max_violations_freeze;
        self.telegram_max_violations_kill = max_violations_kill;
        info!(
            "Telegram monitoring enabled: freeze at {:.1}% ({} violations), kill at {:.1}% ({} violations)",
            cpu_threshold_freeze, max_violations_freeze, cpu_threshold_kill, max_violations_kill
        );
    }

    /// Enable memory pressure monitoring (PSI - Pressure Stall Information)
    ///
    /// # Arguments
    /// * `some_threshold_warning` - Warning threshold for 'some' metric (% time processes waiting)
    /// * `some_threshold_critical` - Critical threshold for 'some' metric
    /// * `full_threshold_warning` - Warning threshold for 'full' metric (% time all blocked)
    /// * `full_threshold_critical` - Critical threshold for 'full' metric
    /// * `action_warning` - Action for warning level ("log", "nice", "freeze", "kill")
    /// * `action_critical` - Action for critical level
    /// * `check_interval_secs` - Check interval in seconds
    pub fn enable_memory_pressure_monitoring(
        &mut self,
        some_threshold_warning: f64,
        some_threshold_critical: f64,
        full_threshold_warning: f64,
        full_threshold_critical: f64,
        action_warning: String,
        action_critical: String,
        check_interval_secs: u64,
    ) {
        self.memory_pressure_enabled = true;
        self.memory_pressure_some_threshold_warning = some_threshold_warning;
        self.memory_pressure_some_threshold_critical = some_threshold_critical;
        self.memory_pressure_full_threshold_warning = full_threshold_warning;
        self.memory_pressure_full_threshold_critical = full_threshold_critical;
        self.memory_pressure_action_warning = action_warning.clone();
        self.memory_pressure_action_critical = action_critical.clone();
        self.memory_pressure_check_interval = Duration::from_secs(check_interval_secs);
        info!(
            "Memory pressure monitoring enabled: some {:.1}%/{:.1}%, full {:.1}%/{:.1}%, actions: {}/{}",
            some_threshold_warning, some_threshold_critical,
            full_threshold_warning, full_threshold_critical,
            action_warning, action_critical
        );
    }

    /// Perform single monitoring check
    ///
    /// This is the main monitoring loop that:
    /// 1. Scans KESL process
    /// 2. Checks CPU and memory thresholds
    /// 3. Tracks violations
    /// 4. Restarts service if max violations reached
    /// 5. Scans and kills high-CPU Node.js processes (if enabled)
    pub fn check(&mut self) -> Result<()> {
        self.stats.increment_checks();
        debug!("Starting monitoring check #{}", self.stats.total_checks);

        // Monitor KESL process
        if let Err(e) = self.check_kesl() {
            error!("KESL monitoring error: {}", e);
        }

        // Monitor Node.js processes
        if self.node_enabled {
            if let Err(e) = self.check_node_processes() {
                error!("Node.js monitoring error: {}", e);
            }
        }

        // Monitor Snap/snapd processes
        if self.snap_enabled {
            if let Err(e) = self.check_snap_processes() {
                error!("Snap monitoring error: {}", e);
            }
        }

        // Monitor Firefox processes
        if self.firefox_enabled {
            if let Err(e) = self.check_firefox_processes() {
                error!("Firefox monitoring error: {}", e);
            }
        }

        // Monitor Brave processes
        if self.brave_enabled {
            if let Err(e) = self.check_brave_processes() {
                error!("Brave monitoring error: {}", e);
            }
        }

        // Monitor Telegram processes
        if self.telegram_enabled {
            if let Err(e) = self.check_telegram_processes() {
                error!("Telegram monitoring error: {}", e);
            }
        }

        // Monitor memory pressure (PSI)
        if self.memory_pressure_enabled {
            // Check if enough time has passed since last check
            let now = Instant::now();
            if now.duration_since(self.memory_pressure_last_check) >= self.memory_pressure_check_interval {
                if let Err(e) = self.check_memory_pressure() {
                    error!("Memory pressure monitoring error: {}", e);
                }
                self.memory_pressure_last_check = now;
            }
        }

        Ok(())
    }

    /// Monitor KESL process
    fn check_kesl(&mut self) -> Result<()> {
        // Scan KESL process
        let process = match self.scanner.scan_kesl()? {
            Some(p) => p,
            None => {
                warn!("KESL process not found");
                return Ok(());
            }
        };

        info!(
            "KESL process: PID {}, CPU {:.1}%, Memory {}MB",
            process.pid, process.cpu_percent, process.memory_mb
        );

        // Check CPU threshold
        let cpu_violation = process.cpu_percent > self.cpu_threshold;
        if cpu_violation {
            self.cpu_violations += 1;
            self.stats.increment_cpu_violation();
            warn!(
                "CPU violation #{}: {:.1}% > {:.1}%",
                self.cpu_violations, process.cpu_percent, self.cpu_threshold
            );
        } else {
            // Reset CPU violations on success
            if self.cpu_violations > 0 {
                debug!(
                    "CPU back to normal: {:.1}% <= {:.1}%, resetting {} violations",
                    process.cpu_percent, self.cpu_threshold, self.cpu_violations
                );
                self.cpu_violations = 0;
            }
        }

        // Check memory threshold
        let memory_violation = process.memory_mb > self.memory_threshold_mb;
        if memory_violation {
            self.memory_violations += 1;
            self.stats.increment_memory_violation();
            warn!(
                "Memory violation #{}: {}MB > {}MB",
                self.memory_violations, process.memory_mb, self.memory_threshold_mb
            );
        } else {
            // Reset memory violations on success
            if self.memory_violations > 0 {
                debug!(
                    "Memory back to normal: {}MB <= {}MB, resetting {} violations",
                    process.memory_mb, self.memory_threshold_mb, self.memory_violations
                );
                self.memory_violations = 0;
            }
        }

        // Check if max violations reached
        if self.cpu_violations >= self.max_violations || self.memory_violations >= self.max_violations
        {
            error!(
                "Max violations reached (CPU: {}, Memory: {}), restarting service",
                self.cpu_violations, self.memory_violations
            );
            self.restart_kesl_service()?;
        }

        Ok(())
    }

    /// Monitor Node.js processes
    fn check_node_processes(&mut self) -> Result<()> {
        let processes = self.scanner.scan_node_processes()?;

        if processes.is_empty() {
            debug!("No Node.js processes found");
            return Ok(());
        }

        debug!("Found {} Node.js processes", processes.len());

        for process in processes {
            if process.cpu_percent > self.node_cpu_threshold {
                warn!(
                    "High-CPU Node.js process: PID {}, CPU {:.1}%, Command: {}",
                    process.pid, process.cpu_percent, process.command
                );

                if self.node_auto_kill {
                    info!(
                        "Auto-killing Node.js process PID {} (CPU {:.1}%)",
                        process.pid, process.cpu_percent
                    );
                    if let Err(e) = ProcessExecutor::kill_process(process.pid) {
                        error!("Failed to kill Node.js process {}: {}", process.pid, e);
                    } else {
                        self.stats.record_kill();
                        info!("Successfully killed Node.js process {}", process.pid);
                    }
                }
            }
        }

        Ok(())
    }

    /// Monitor Snap/snapd processes
    fn check_snap_processes(&mut self) -> Result<()> {
        use std::thread;
        use std::time::Duration;

        let processes = self.scanner.scan_snap_processes()?;

        if processes.is_empty() {
            debug!("No Snap processes found");
            return Ok(());
        }

        debug!("Found {} Snap processes", processes.len());

        // Find high-CPU snap processes
        let high_cpu_processes: Vec<_> = processes
            .iter()
            .filter(|p| p.cpu_percent > self.snap_cpu_threshold)
            .collect();

        if high_cpu_processes.is_empty() {
            // Reset violations if no high-CPU processes
            if self.snap_violations > 0 {
                debug!(
                    "Snap CPU back to normal, resetting {} violations",
                    self.snap_violations
                );
                self.snap_violations = 0;
            }
            return Ok(());
        }

        // Increment violations
        self.snap_violations += 1;
        warn!(
            "Snap CPU violation #{}: {} processes exceed {:.1}%",
            self.snap_violations,
            high_cpu_processes.len(),
            self.snap_cpu_threshold
        );

        for process in &high_cpu_processes {
            warn!(
                "High-CPU Snap process: PID {}, CPU {:.1}%, Command: {}",
                process.pid, process.cpu_percent, process.command
            );
        }

        // Take action if max violations reached
        if self.snap_violations >= self.snap_max_violations {
            error!(
                "Snap max violations ({}) reached, taking action: {}",
                self.snap_max_violations, self.snap_action
            );

            for process in high_cpu_processes {
                match self.snap_action.as_str() {
                    "nice" => {
                        info!(
                            "Setting nice level {} for snap process PID {}",
                            self.snap_nice_level, process.pid
                        );
                        if let Err(e) =
                            ProcessExecutor::renice_process(process.pid, self.snap_nice_level)
                        {
                            error!("Failed to renice snap process {}: {}", process.pid, e);
                        } else {
                            info!(
                                "Successfully set nice level {} for snap process {}",
                                self.snap_nice_level, process.pid
                            );
                        }
                    }
                    "freeze" => {
                        info!(
                            "Freezing snap process PID {} for {} seconds",
                            process.pid, self.snap_freeze_duration_secs
                        );
                        if let Err(e) = ProcessExecutor::freeze_process(process.pid) {
                            error!("Failed to freeze snap process {}: {}", process.pid, e);
                        } else {
                            info!("Snap process {} frozen, waiting...", process.pid);
                            thread::sleep(Duration::from_secs(self.snap_freeze_duration_secs));

                            if let Err(e) = ProcessExecutor::unfreeze_process(process.pid) {
                                error!("Failed to unfreeze snap process {}: {}", process.pid, e);
                            } else {
                                info!("Snap process {} unfrozen", process.pid);
                            }
                        }
                    }
                    "kill" => {
                        info!("Killing snap process PID {}", process.pid);
                        if let Err(e) = ProcessExecutor::kill_process(process.pid) {
                            error!("Failed to kill snap process {}: {}", process.pid, e);
                        } else {
                            self.stats.record_kill();
                            info!("Successfully killed snap process {}", process.pid);
                        }
                    }
                    _ => {
                        warn!("Unknown snap action: {}", self.snap_action);
                    }
                }
            }

            // Reset violations after taking action
            self.snap_violations = 0;
        }

        Ok(())
    }

    /// Monitor Firefox processes (two-tier strategy: freeze then kill)
    fn check_firefox_processes(&mut self) -> Result<()> {
        use std::thread;
        use std::time::Duration;

        let processes = self.scanner.scan_firefox_processes()?;

        if processes.is_empty() {
            debug!("No Firefox processes found");
            // Reset violations if no processes
            if self.firefox_violations_freeze > 0 || self.firefox_violations_kill > 0 {
                debug!("Firefox process ended, resetting violations");
                self.firefox_violations_freeze = 0;
                self.firefox_violations_kill = 0;
            }
            return Ok(());
        }

        debug!("Found {} Firefox processes", processes.len());

        // Check for critical CPU (>kill threshold)
        let critical_processes: Vec<_> = processes
            .iter()
            .filter(|p| p.cpu_percent > self.firefox_cpu_threshold_kill)
            .collect();

        // Check for high CPU (>freeze threshold, but <kill threshold)
        let high_cpu_processes: Vec<_> = processes
            .iter()
            .filter(|p| {
                p.cpu_percent > self.firefox_cpu_threshold_freeze
                    && p.cpu_percent <= self.firefox_cpu_threshold_kill
            })
            .collect();

        // Handle critical CPU (kill strategy)
        if !critical_processes.is_empty() {
            self.firefox_violations_kill += 1;
            warn!(
                "Firefox CRITICAL CPU violation #{}: {} processes exceed {:.1}%",
                self.firefox_violations_kill,
                critical_processes.len(),
                self.firefox_cpu_threshold_kill
            );

            for process in &critical_processes {
                warn!(
                    "CRITICAL Firefox process: PID {}, CPU {:.1}%, Command: {}",
                    process.pid, process.cpu_percent, process.command
                );
            }

            if self.firefox_violations_kill >= self.firefox_max_violations_kill {
                error!(
                    "Firefox critical violations ({}) reached, KILLING processes",
                    self.firefox_max_violations_kill
                );

                for process in critical_processes {
                    info!("Killing Firefox process PID {} (CPU {:.1}%)", process.pid, process.cpu_percent);
                    if let Err(e) = ProcessExecutor::kill_process(process.pid) {
                        error!("Failed to kill Firefox process {}: {}", process.pid, e);
                    } else {
                        self.stats.record_kill();
                        info!("Successfully killed Firefox process {}", process.pid);
                    }
                }

                self.firefox_violations_kill = 0;
                self.firefox_violations_freeze = 0; // Reset freeze violations too
            }
        } else if !high_cpu_processes.is_empty() {
            // Handle high CPU (freeze strategy)
            self.firefox_violations_freeze += 1;
            self.firefox_violations_kill = 0; // Reset kill violations

            warn!(
                "Firefox high CPU violation #{}: {} processes exceed {:.1}%",
                self.firefox_violations_freeze,
                high_cpu_processes.len(),
                self.firefox_cpu_threshold_freeze
            );

            for process in &high_cpu_processes {
                warn!(
                    "High-CPU Firefox process: PID {}, CPU {:.1}%, Command: {}",
                    process.pid, process.cpu_percent, process.command
                );
            }

            if self.firefox_violations_freeze >= self.firefox_max_violations_freeze {
                warn!(
                    "Firefox freeze violations ({}) reached, FREEZING processes",
                    self.firefox_max_violations_freeze
                );

                for process in high_cpu_processes {
                    info!(
                        "Freezing Firefox process PID {} for {} seconds (CPU {:.1}%)",
                        process.pid, self.firefox_freeze_duration_secs, process.cpu_percent
                    );

                    if let Err(e) = ProcessExecutor::freeze_process(process.pid) {
                        error!("Failed to freeze Firefox process {}: {}", process.pid, e);
                    } else {
                        info!("Firefox process {} frozen, waiting...", process.pid);
                        thread::sleep(Duration::from_secs(self.firefox_freeze_duration_secs));

                        if let Err(e) = ProcessExecutor::unfreeze_process(process.pid) {
                            error!("Failed to unfreeze Firefox process {}: {}", process.pid, e);
                        } else {
                            info!("Firefox process {} unfrozen", process.pid);
                        }
                    }
                }

                self.firefox_violations_freeze = 0;
            }
        } else {
            // CPU back to normal, reset violations
            if self.firefox_violations_freeze > 0 || self.firefox_violations_kill > 0 {
                debug!(
                    "Firefox CPU back to normal, resetting violations (freeze: {}, kill: {})",
                    self.firefox_violations_freeze, self.firefox_violations_kill
                );
                self.firefox_violations_freeze = 0;
                self.firefox_violations_kill = 0;
            }
        }

        Ok(())
    }

    /// Monitor Brave browser processes (two-tier strategy: freeze then kill)
    fn check_brave_processes(&mut self) -> Result<()> {
        use std::thread;
        use std::time::Duration;

        let processes = self.scanner.scan_brave_processes()?;

        if processes.is_empty() {
            debug!("No Brave processes found");
            // Reset violations if no processes
            if self.brave_violations_freeze > 0 || self.brave_violations_kill > 0 {
                debug!("Brave process ended, resetting violations");
                self.brave_violations_freeze = 0;
                self.brave_violations_kill = 0;
            }
            return Ok(());
        }

        debug!("Found {} Brave processes", processes.len());

        // Check for critical CPU (>kill threshold)
        let critical_processes: Vec<_> = processes
            .iter()
            .filter(|p| p.cpu_percent > self.brave_cpu_threshold_kill)
            .collect();

        // Check for high CPU (>freeze threshold, but <kill threshold)
        let high_cpu_processes: Vec<_> = processes
            .iter()
            .filter(|p| {
                p.cpu_percent > self.brave_cpu_threshold_freeze
                    && p.cpu_percent <= self.brave_cpu_threshold_kill
            })
            .collect();

        // Handle critical CPU (kill strategy)
        if !critical_processes.is_empty() {
            self.brave_violations_kill += 1;
            warn!(
                "Brave CRITICAL CPU violation #{}: {} processes exceed {:.1}%",
                self.brave_violations_kill,
                critical_processes.len(),
                self.brave_cpu_threshold_kill
            );

            for process in &critical_processes {
                warn!(
                    "CRITICAL Brave process: PID {}, CPU {:.1}%, Command: {}",
                    process.pid, process.cpu_percent, process.command
                );
            }

            if self.brave_violations_kill >= self.brave_max_violations_kill {
                error!(
                    "Brave critical violations ({}) reached, KILLING processes",
                    self.brave_max_violations_kill
                );

                for process in critical_processes {
                    info!("Killing Brave process PID {} (CPU {:.1}%)", process.pid, process.cpu_percent);
                    if let Err(e) = ProcessExecutor::kill_process(process.pid) {
                        error!("Failed to kill Brave process {}: {}", process.pid, e);
                    } else {
                        self.stats.record_kill();
                        info!("Successfully killed Brave process {}", process.pid);
                    }
                }

                self.brave_violations_kill = 0;
                self.brave_violations_freeze = 0; // Reset freeze violations too
            }
        } else if !high_cpu_processes.is_empty() {
            // Handle high CPU (freeze strategy)
            self.brave_violations_freeze += 1;
            self.brave_violations_kill = 0; // Reset kill violations

            warn!(
                "Brave high CPU violation #{}: {} processes exceed {:.1}%",
                self.brave_violations_freeze,
                high_cpu_processes.len(),
                self.brave_cpu_threshold_freeze
            );

            for process in &high_cpu_processes {
                warn!(
                    "High-CPU Brave process: PID {}, CPU {:.1}%, Command: {}",
                    process.pid, process.cpu_percent, process.command
                );
            }

            if self.brave_violations_freeze >= self.brave_max_violations_freeze {
                warn!(
                    "Brave freeze violations ({}) reached, FREEZING processes",
                    self.brave_max_violations_freeze
                );

                for process in high_cpu_processes {
                    info!(
                        "Freezing Brave process PID {} for {} seconds (CPU {:.1}%)",
                        process.pid, self.brave_freeze_duration_secs, process.cpu_percent
                    );

                    if let Err(e) = ProcessExecutor::freeze_process(process.pid) {
                        error!("Failed to freeze Brave process {}: {}", process.pid, e);
                    } else {
                        info!("Brave process {} frozen, waiting...", process.pid);
                        thread::sleep(Duration::from_secs(self.brave_freeze_duration_secs));

                        if let Err(e) = ProcessExecutor::unfreeze_process(process.pid) {
                            error!("Failed to unfreeze Brave process {}: {}", process.pid, e);
                        } else {
                            info!("Brave process {} unfrozen", process.pid);
                        }
                    }
                }

                self.brave_violations_freeze = 0;
            }
        } else {
            // CPU back to normal, reset violations
            if self.brave_violations_freeze > 0 || self.brave_violations_kill > 0 {
                debug!(
                    "Brave CPU back to normal, resetting violations (freeze: {}, kill: {})",
                    self.brave_violations_freeze, self.brave_violations_kill
                );
                self.brave_violations_freeze = 0;
                self.brave_violations_kill = 0;
            }
        }

        Ok(())
    }

    /// Check and manage Telegram messenger processes (two-tier strategy: freeze/kill)
    fn check_telegram_processes(&mut self) -> Result<()> {
        use std::thread;
        use std::time::Duration;

        let processes = self.scanner.scan_telegram_processes()?;

        if processes.is_empty() {
            debug!("No Telegram processes found");
            // Reset violations if no processes
            if self.telegram_violations_freeze > 0 || self.telegram_violations_kill > 0 {
                debug!("Telegram process ended, resetting violations");
                self.telegram_violations_freeze = 0;
                self.telegram_violations_kill = 0;
            }
            return Ok(());
        }

        debug!("Found {} Telegram processes", processes.len());

        // Check for critical CPU (>kill threshold)
        let critical_processes: Vec<_> = processes
            .iter()
            .filter(|p| p.cpu_percent > self.telegram_cpu_threshold_kill)
            .collect();

        // Check for high CPU (>freeze threshold, but <kill threshold)
        let high_cpu_processes: Vec<_> = processes
            .iter()
            .filter(|p| {
                p.cpu_percent > self.telegram_cpu_threshold_freeze
                    && p.cpu_percent <= self.telegram_cpu_threshold_kill
            })
            .collect();

        // Handle critical CPU (kill strategy)
        if !critical_processes.is_empty() {
            self.telegram_violations_kill += 1;
            warn!(
                "Telegram CRITICAL CPU violation #{}: {} processes exceed {:.1}%",
                self.telegram_violations_kill,
                critical_processes.len(),
                self.telegram_cpu_threshold_kill
            );

            for process in &critical_processes {
                warn!(
                    "CRITICAL Telegram process: PID {}, CPU {:.1}%, Command: {}",
                    process.pid, process.cpu_percent, process.command
                );
            }

            if self.telegram_violations_kill >= self.telegram_max_violations_kill {
                error!(
                    "Telegram critical violations ({}) reached, KILLING processes",
                    self.telegram_max_violations_kill
                );

                for process in critical_processes {
                    info!("Killing Telegram process PID {} (CPU {:.1}%)", process.pid, process.cpu_percent);
                    if let Err(e) = ProcessExecutor::kill_process(process.pid) {
                        error!("Failed to kill Telegram process {}: {}", process.pid, e);
                    } else {
                        self.stats.record_kill();
                        info!("Successfully killed Telegram process {}", process.pid);
                    }
                }

                self.telegram_violations_kill = 0;
                self.telegram_violations_freeze = 0; // Reset freeze violations too
            }
        } else if !high_cpu_processes.is_empty() {
            // Handle high CPU (freeze strategy)
            self.telegram_violations_freeze += 1;
            self.telegram_violations_kill = 0; // Reset kill violations

            warn!(
                "Telegram high CPU violation #{}: {} processes exceed {:.1}%",
                self.telegram_violations_freeze,
                high_cpu_processes.len(),
                self.telegram_cpu_threshold_freeze
            );

            for process in &high_cpu_processes {
                warn!(
                    "High-CPU Telegram process: PID {}, CPU {:.1}%, Command: {}",
                    process.pid, process.cpu_percent, process.command
                );
            }

            if self.telegram_violations_freeze >= self.telegram_max_violations_freeze {
                warn!(
                    "Telegram freeze violations ({}) reached, FREEZING processes",
                    self.telegram_max_violations_freeze
                );

                for process in high_cpu_processes {
                    info!(
                        "Freezing Telegram process PID {} for {} seconds (CPU {:.1}%)",
                        process.pid, self.telegram_freeze_duration_secs, process.cpu_percent
                    );

                    if let Err(e) = ProcessExecutor::freeze_process(process.pid) {
                        error!("Failed to freeze Telegram process {}: {}", process.pid, e);
                    } else {
                        info!("Telegram process {} frozen, waiting...", process.pid);
                        thread::sleep(Duration::from_secs(self.telegram_freeze_duration_secs));

                        if let Err(e) = ProcessExecutor::unfreeze_process(process.pid) {
                            error!("Failed to unfreeze Telegram process {}: {}", process.pid, e);
                        } else {
                            info!("Telegram process {} unfrozen", process.pid);
                        }
                    }
                }

                self.telegram_violations_freeze = 0;
            }
        } else {
            // CPU back to normal, reset violations
            if self.telegram_violations_freeze > 0 || self.telegram_violations_kill > 0 {
                debug!(
                    "Telegram CPU back to normal, resetting violations (freeze: {}, kill: {})",
                    self.telegram_violations_freeze, self.telegram_violations_kill
                );
                self.telegram_violations_freeze = 0;
                self.telegram_violations_kill = 0;
            }
        }

        Ok(())
    }

    /// Restart KESL service with protection against frequent restarts
    fn restart_kesl_service(&mut self) -> Result<()> {
        // Check if service is active
        if !self.kesl_service.is_active()? {
            error!("KESL service is not active, cannot restart");
            return Err(Error::Systemd("Service is not active".to_string()));
        }

        // Restart with daemon-reload
        info!("Restarting KESL service with daemon-reload");
        self.kesl_service.restart_with_reload()?;

        // Reset violation counters after successful restart
        self.cpu_violations = 0;
        self.memory_violations = 0;
        self.stats.record_restart();

        info!("KESL service successfully restarted, violations reset");
        Ok(())
    }

    /// Get current monitoring statistics
    pub fn stats(&self) -> &MonitorStats {
        &self.stats
    }

    /// Get current violation counters
    pub fn violations(&self) -> (u32, u32) {
        (self.cpu_violations, self.memory_violations)
    }

    /// Reset violation counters (useful for testing or manual reset)
    pub fn reset_violations(&mut self) {
        self.cpu_violations = 0;
        self.memory_violations = 0;
        debug!("Violation counters manually reset");
    }

    /// Get current KESL CPU and memory status
    /// Returns (cpu_percent, memory_mb)
    pub fn get_kesl_status(&self) -> Option<(f64, u64)> {
        match self.scanner.scan_kesl() {
            Ok(Some(process)) => {
                let memory_mb = process.memory_kb / 1024;
                Some((process.cpu_percent, memory_mb))
            }
            Ok(None) => None,
            Err(_) => None,
        }
    }

    /// Export complete statistics for dashboard
    ///
    /// Creates a comprehensive stats snapshot including all monitored processes,
    /// violations, actions, system health, and memory pressure
    pub fn export_stats(&self, runtime_secs: u64) -> crate::stats::MonitorStats {
        use crate::stats::*;

        // Get current KESL status
        let (kesl_cpu, kesl_mem) = self.get_kesl_status().unwrap_or((0.0, 0));
        let kesl_pid = self.scanner.scan_kesl().ok().flatten().map(|p| p.pid);

        // Read memory pressure (if enabled)
        let (mp_some, mp_full, mp_status) = if self.memory_pressure_enabled {
            match freezr_core::memory_pressure::MemoryPressure::read() {
                Ok(pressure) => {
                    let status = if pressure.full_avg10 >= self.memory_pressure_full_threshold_critical {
                        "CRITICAL".to_string()
                    } else if pressure.some_avg10 >= self.memory_pressure_some_threshold_critical {
                        "WARNING".to_string()
                    } else {
                        "OK".to_string()
                    };
                    (pressure.some_avg10, pressure.full_avg10, status)
                }
                Err(_) => (0.0, 0.0, "UNKNOWN".to_string()),
            }
        } else {
            (0.0, 0.0, "DISABLED".to_string())
        };

        // Read system load and memory
        let (load_1, load_5, load_15) = if let Ok(content) = std::fs::read_to_string("/proc/loadavg") {
            let parts: Vec<&str> = content.split_whitespace().collect();
            if parts.len() >= 3 {
                (
                    parts[0].parse().unwrap_or(0.0),
                    parts[1].parse().unwrap_or(0.0),
                    parts[2].parse().unwrap_or(0.0),
                )
            } else {
                (0.0, 0.0, 0.0)
            }
        } else {
            (0.0, 0.0, 0.0)
        };

        let (mem_total, mem_available, mem_used_pct) = if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
            let mut total_kb = 0u64;
            let mut available_kb = 0u64;
            for line in content.lines() {
                if line.starts_with("MemTotal:") {
                    total_kb = line.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                } else if line.starts_with("MemAvailable:") {
                    available_kb = line.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                }
            }
            let total_mb = total_kb / 1024;
            let available_mb = available_kb / 1024;
            let used_pct = if total_kb > 0 {
                ((total_kb - available_kb) as f64 / total_kb as f64) * 100.0
            } else {
                0.0
            };
            (total_mb, available_mb, used_pct)
        } else {
            (0, 0, 0.0)
        };

        // Log statistics
        let log_stats = LogStats::default(); // TODO: Implement log directory scanning

        MonitorStats {
            timestamp: MonitorStats::current_timestamp(),
            runtime_secs,
            total_checks: self.stats.total_checks,
            kesl: ProcessStats {
                pid: kesl_pid,
                cpu_percent: kesl_cpu,
                memory_mb: kesl_mem,
                cpu_threshold: self.cpu_threshold,
                memory_threshold_mb: self.memory_threshold_mb,
                total_cpu_violations: self.stats.cpu_violations,
                total_memory_violations: self.stats.memory_violations,
                current_cpu_violations: self.cpu_violations,
                current_memory_violations: self.memory_violations,
                max_violations: self.max_violations,
                violation_rate: if self.stats.total_checks > 0 {
                    ((self.stats.cpu_violations + self.stats.memory_violations) as f64 / self.stats.total_checks as f64) * 100.0
                } else {
                    0.0
                },
                total_restarts: self.stats.total_restarts as u32,
            },
            node: NodeStats {
                enabled: self.node_enabled,
                cpu_threshold: self.node_cpu_threshold,
                auto_kill: self.node_auto_kill,
                total_kills: self.stats.total_kills as u32,
            },
            snap: SnapStats {
                enabled: self.snap_enabled,
                cpu_threshold: self.snap_cpu_threshold,
                action: self.snap_action.clone(),
                nice_level: self.snap_nice_level,
                total_actions: 0, // TODO: Track snap actions
            },
            firefox: BrowserStats {
                enabled: self.firefox_enabled,
                freeze_threshold: self.firefox_cpu_threshold_freeze,
                kill_threshold: self.firefox_cpu_threshold_kill,
                freeze_violations: self.firefox_violations_freeze,
                kill_violations: self.firefox_violations_kill,
                max_violations_freeze: self.firefox_max_violations_freeze,
                max_violations_kill: self.firefox_max_violations_kill,
                total_freezes: 0, // TODO: Track freezes
                total_kills: 0,   // TODO: Track kills
            },
            brave: BrowserStats {
                enabled: self.brave_enabled,
                freeze_threshold: self.brave_cpu_threshold_freeze,
                kill_threshold: self.brave_cpu_threshold_kill,
                freeze_violations: self.brave_violations_freeze,
                kill_violations: self.brave_violations_kill,
                max_violations_freeze: self.brave_max_violations_freeze,
                max_violations_kill: self.brave_max_violations_kill,
                total_freezes: 0, // TODO: Track freezes
                total_kills: 0,   // TODO: Track kills
            },
            telegram: BrowserStats {
                enabled: self.telegram_enabled,
                freeze_threshold: self.telegram_cpu_threshold_freeze,
                kill_threshold: self.telegram_cpu_threshold_kill,
                freeze_violations: self.telegram_violations_freeze,
                kill_violations: self.telegram_violations_kill,
                max_violations_freeze: self.telegram_max_violations_freeze,
                max_violations_kill: self.telegram_max_violations_kill,
                total_freezes: 0, // TODO: Track freezes
                total_kills: 0,   // TODO: Track kills
            },
            memory_pressure: MemoryPressureStats {
                enabled: self.memory_pressure_enabled,
                some_avg10: mp_some,
                full_avg10: mp_full,
                status: mp_status,
                warning_count: self.memory_pressure_warning_count,
                critical_count: self.memory_pressure_critical_count,
                some_threshold_warning: self.memory_pressure_some_threshold_warning,
                some_threshold_critical: self.memory_pressure_some_threshold_critical,
                full_threshold_warning: self.memory_pressure_full_threshold_warning,
                full_threshold_critical: self.memory_pressure_full_threshold_critical,
                action_warning: self.memory_pressure_action_warning.clone(),
                action_critical: self.memory_pressure_action_critical.clone(),
            },
            system_health: SystemHealth {
                load_1min: load_1,
                load_5min: load_5,
                load_15min: load_15,
                memory_used_percent: mem_used_pct,
                memory_total_mb: mem_total,
                memory_available_mb: mem_available,
            },
            log_stats,
        }
    }

    /// Monitor memory pressure (PSI - Pressure Stall Information)
    ///
    /// Reads /proc/pressure/memory and takes proactive actions based on thresholds
    fn check_memory_pressure(&mut self) -> Result<()> {
        // Read current memory pressure
        let pressure = match MemoryPressure::read() {
            Ok(p) => p,
            Err(e) => {
                warn!("Failed to read memory pressure: {}", e);
                return Ok(()); // Don't fail monitoring on PSI read error
            }
        };

        debug!(
            "Memory pressure: some {:.2}%, full {:.2}%",
            pressure.some_avg10, pressure.full_avg10
        );

        // Check if pressure is at critical level
        if pressure.is_critical(
            self.memory_pressure_some_threshold_critical,
            self.memory_pressure_full_threshold_critical,
        ) {
            self.memory_pressure_critical_count += 1;
            warn!("╔═══════════════════════════════════════════════════════════╗");
            warn!("║           🚨 CRITICAL MEMORY PRESSURE DETECTED 🚨         ║");
            warn!("╚═══════════════════════════════════════════════════════════╝");
            warn!(
                "PSI Metrics: some={:.2}%, full={:.2}% (thresholds: some={:.1}%, full={:.1}%)",
                pressure.some_avg10,
                pressure.full_avg10,
                self.memory_pressure_some_threshold_critical,
                self.memory_pressure_full_threshold_critical
            );
            warn!(
                "PSI Averages: some(10s/60s/300s)={:.2}/{:.2}/{:.2}%, full(10s/60s/300s)={:.2}/{:.2}/{:.2}%",
                pressure.some_avg10, pressure.some_avg60, pressure.some_avg300,
                pressure.full_avg10, pressure.full_avg60, pressure.full_avg300
            );

            // Get system memory info
            if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
                if let Some(total) = meminfo.lines().find(|l| l.starts_with("MemTotal:")) {
                    if let Some(avail) = meminfo.lines().find(|l| l.starts_with("MemAvailable:")) {
                        warn!("System Memory: {} | {}", total.trim(), avail.trim());
                    }
                }
            }

            // Execute critical action
            self.execute_memory_pressure_action(&self.memory_pressure_action_critical.clone(), "CRITICAL")?;
        }
        // Check if pressure is at warning level
        else if pressure.is_warning(
            self.memory_pressure_some_threshold_warning,
            self.memory_pressure_full_threshold_warning,
        ) {
            self.memory_pressure_warning_count += 1;
            warn!(
                "WARNING memory pressure detected! some={:.2}%, full={:.2}% (thresholds: some={:.1}%, full={:.1}%)",
                pressure.some_avg10,
                pressure.full_avg10,
                self.memory_pressure_some_threshold_warning,
                self.memory_pressure_full_threshold_warning
            );

            // Execute warning action
            self.execute_memory_pressure_action(&self.memory_pressure_action_warning.clone(), "WARNING")?;
        } else {
            // No pressure detected - reset counters
            if self.memory_pressure_warning_count > 0 || self.memory_pressure_critical_count > 0 {
                debug!("Memory pressure normalized (some={:.2}%, full={:.2}%)",
                    pressure.some_avg10, pressure.full_avg10);
                self.memory_pressure_warning_count = 0;
                self.memory_pressure_critical_count = 0;
            }
        }

        Ok(())
    }

    /// Execute action based on memory pressure level
    fn execute_memory_pressure_action(&mut self, action: &str, level: &str) -> Result<()> {
        match action {
            "log" => {
                info!("[Memory Pressure {}] Logging event", level);
                // Already logged in check_memory_pressure
                Ok(())
            }
            "nice" => {
                info!("[Memory Pressure {}] Applying nice to non-critical processes", level);
                // Nice down non-critical processes (Firefox, Brave, Telegram)
                self.nice_non_critical_processes()
            }
            "freeze" => {
                info!("[Memory Pressure {}] Freezing non-critical processes", level);
                // Freeze non-critical processes temporarily
                self.freeze_non_critical_processes()
            }
            "kill" => {
                warn!("[Memory Pressure {}] Killing non-critical processes", level);
                // Kill non-critical processes (most aggressive)
                self.kill_non_critical_processes()
            }
            _ => {
                warn!("Unknown memory pressure action: {}", action);
                Ok(())
            }
        }
    }

    /// Lower priority of non-critical processes
    fn nice_non_critical_processes(&mut self) -> Result<()> {
        let mut niced_count = 0;

        // Nice Firefox processes
        if let Ok(processes) = self.scanner.scan_firefox_processes() {
            for process in processes {
                if let Ok(()) = ProcessExecutor::renice_process(process.pid, 15) {
                    info!("Niced Firefox process {} to priority 15", process.pid);
                    niced_count += 1;
                }
            }
        }

        // Nice Brave processes
        if let Ok(processes) = self.scanner.scan_brave_processes() {
            for process in processes {
                if let Ok(()) = ProcessExecutor::renice_process(process.pid, 15) {
                    info!("Niced Brave process {} to priority 15", process.pid);
                    niced_count += 1;
                }
            }
        }

        // Nice Telegram processes
        if let Ok(processes) = self.scanner.scan_telegram_processes() {
            for process in processes {
                if let Ok(()) = ProcessExecutor::renice_process(process.pid, 15) {
                    info!("Niced Telegram process {} to priority 15", process.pid);
                    niced_count += 1;
                }
            }
        }

        info!("Memory pressure: niced {} non-critical processes", niced_count);
        Ok(())
    }

    /// Freeze non-critical processes temporarily (5 seconds)
    fn freeze_non_critical_processes(&mut self) -> Result<()> {
        let mut frozen_count = 0;

        // Freeze Firefox
        if let Ok(processes) = self.scanner.scan_firefox_processes() {
            for process in processes {
                if let Ok(()) = ProcessExecutor::freeze_process(process.pid) {
                    info!("Froze Firefox process {}", process.pid);
                    frozen_count += 1;
                }
            }
        }

        // Freeze Brave
        if let Ok(processes) = self.scanner.scan_brave_processes() {
            for process in processes {
                if let Ok(()) = ProcessExecutor::freeze_process(process.pid) {
                    info!("Froze Brave process {}", process.pid);
                    frozen_count += 1;
                }
            }
        }

        // Freeze Telegram
        if let Ok(processes) = self.scanner.scan_telegram_processes() {
            for process in processes {
                if let Ok(()) = ProcessExecutor::freeze_process(process.pid) {
                    info!("Froze Telegram process {}", process.pid);
                    frozen_count += 1;
                }
            }
        }

        info!("Memory pressure: froze {} non-critical processes for {} seconds",
            frozen_count, 5);

        // Unfreeze after 5 seconds
        std::thread::sleep(std::time::Duration::from_secs(5));

        // Unfreeze all
        let mut unfrozen_count = 0;
        if let Ok(processes) = self.scanner.scan_firefox_processes() {
            for process in processes {
                if let Ok(()) = ProcessExecutor::unfreeze_process(process.pid) {
                    unfrozen_count += 1;
                }
            }
        }
        if let Ok(processes) = self.scanner.scan_brave_processes() {
            for process in processes {
                if let Ok(()) = ProcessExecutor::unfreeze_process(process.pid) {
                    unfrozen_count += 1;
                }
            }
        }
        if let Ok(processes) = self.scanner.scan_telegram_processes() {
            for process in processes {
                if let Ok(()) = ProcessExecutor::unfreeze_process(process.pid) {
                    unfrozen_count += 1;
                }
            }
        }

        info!("Memory pressure: unfroze {} processes", unfrozen_count);
        Ok(())
    }

    /// Kill non-critical processes (most aggressive action)
    /// Priority: Brave → Telegram → nvim (>1GB)
    fn kill_non_critical_processes(&mut self) -> Result<()> {
        let mut killed_count = 0;
        let mut total_memory_freed = 0u64;

        // Log all potential culprits before killing
        warn!("=== OOM Prevention: Analyzing memory consumers ===");

        // Collect all processes and their memory usage
        let mut all_consumers: Vec<(String, u32, u64, f64, String)> = Vec::new();

        if let Ok(processes) = self.scanner.scan_brave_processes() {
            for p in processes {
                all_consumers.push(("Brave".to_string(), p.pid, p.memory_kb, p.cpu_percent, p.command.clone()));
            }
        }
        if let Ok(processes) = self.scanner.scan_telegram_processes() {
            for p in processes {
                all_consumers.push(("Telegram".to_string(), p.pid, p.memory_kb, p.cpu_percent, p.command.clone()));
            }
        }
        if let Ok(processes) = self.scanner.scan_nvim_processes() {
            for p in processes {
                all_consumers.push(("nvim".to_string(), p.pid, p.memory_kb, p.cpu_percent, p.command.clone()));
            }
        }
        if let Ok(processes) = self.scanner.scan_firefox_processes() {
            for p in processes {
                all_consumers.push(("Firefox".to_string(), p.pid, p.memory_kb, p.cpu_percent, p.command.clone()));
            }
        }

        // Sort by memory consumption (descending)
        all_consumers.sort_by(|a, b| b.2.cmp(&a.2));

        // Log top memory consumers
        warn!("Top memory consumers before OOM prevention:");
        for (idx, (name, pid, mem_kb, cpu, cmd)) in all_consumers.iter().take(10).enumerate() {
            let mem_mb = mem_kb / 1024;
            let cmd_short: String = cmd.chars().take(60).collect();
            warn!("  #{} {} PID:{} RAM:{}MB CPU:{:.1}% CMD:{}",
                  idx + 1, name, pid, mem_mb, cpu, cmd_short);
        }

        // Priority 1: Kill Brave (браузер можно пожертвовать)
        if let Ok(processes) = self.scanner.scan_brave_processes() {
            for process in processes {
                let mem_mb = process.memory_kb / 1024;
                warn!("🔴 [Priority 1] Killing Brave PID:{} RAM:{}MB CPU:{:.1}% CMD:{}",
                      process.pid, mem_mb, process.cpu_percent,
                      process.command.chars().take(60).collect::<String>());
                if let Ok(()) = ProcessExecutor::kill_process(process.pid) {
                    killed_count += 1;
                    total_memory_freed += process.memory_kb;
                }
            }
        }

        // Priority 2: Kill Telegram (мессенджер менее критичен)
        if let Ok(processes) = self.scanner.scan_telegram_processes() {
            for process in processes {
                let mem_mb = process.memory_kb / 1024;
                warn!("🟠 [Priority 2] Killing Telegram PID:{} RAM:{}MB CPU:{:.1}% CMD:{}",
                      process.pid, mem_mb, process.cpu_percent,
                      process.command.chars().take(60).collect::<String>());
                if let Ok(()) = ProcessExecutor::kill_process(process.pid) {
                    killed_count += 1;
                    total_memory_freed += process.memory_kb;
                }
            }
        }

        // Priority 3: Kill nvim if memory > 1GB (крайняя мера)
        if let Ok(processes) = self.scanner.scan_nvim_processes() {
            for process in processes {
                let memory_mb = process.memory_kb / 1024;
                if memory_mb > 1024 {
                    warn!("🟡 [Priority 3] Killing nvim PID:{} RAM:{}MB CPU:{:.1}% CMD:{}",
                          process.pid, memory_mb, process.cpu_percent,
                          process.command.chars().take(60).collect::<String>());
                    if let Ok(()) = ProcessExecutor::kill_process(process.pid) {
                        killed_count += 1;
                        total_memory_freed += process.memory_kb;
                    }
                } else {
                    info!("⚪ Skipping nvim PID:{} ({}MB < 1GB threshold)", process.pid, memory_mb);
                }
            }
        }

        // Priority 4: Kill Firefox (дополнительная защита)
        if let Ok(processes) = self.scanner.scan_firefox_processes() {
            for process in processes {
                let mem_mb = process.memory_kb / 1024;
                warn!("🔵 [Priority 4] Killing Firefox PID:{} RAM:{}MB CPU:{:.1}% CMD:{}",
                      process.pid, mem_mb, process.cpu_percent,
                      process.command.chars().take(60).collect::<String>());
                if let Ok(()) = ProcessExecutor::kill_process(process.pid) {
                    killed_count += 1;
                    total_memory_freed += process.memory_kb;
                }
            }
        }

        let freed_mb = total_memory_freed / 1024;
        warn!("=== OOM Prevention completed: killed {} processes, freed {}MB ===",
              killed_count, freed_mb);
        Ok(())
    }

    /// Get current memory pressure status (for dashboard)
    pub fn get_memory_pressure_status(&self) -> Option<(f64, f64, String, u32, u32)> {
        if !self.memory_pressure_enabled {
            return None;
        }

        match MemoryPressure::read() {
            Ok(pressure) => {
                let status = pressure.status().to_string();
                Some((
                    pressure.some_avg10,
                    pressure.full_avg10,
                    status,
                    self.memory_pressure_warning_count,
                    self.memory_pressure_critical_count,
                ))
            }
            Err(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monitor_creation() {
        let monitor = ResourceMonitor::new("kesl", 30.0, 600, 3, 100);

        assert_eq!(monitor.cpu_threshold, 30.0);
        assert_eq!(monitor.memory_threshold_mb, 600);
        assert_eq!(monitor.max_violations, 3);
        assert_eq!(monitor.min_restart_interval_secs, 100);
        assert_eq!(monitor.cpu_violations, 0);
        assert_eq!(monitor.memory_violations, 0);
        assert!(!monitor.node_enabled);
    }

    #[test]
    fn test_enable_node_monitoring() {
        let mut monitor = ResourceMonitor::new("kesl", 30.0, 600, 3, 100);

        assert!(!monitor.node_enabled);

        monitor.enable_node_monitoring(80.0, true);

        assert!(monitor.node_enabled);
        assert_eq!(monitor.node_cpu_threshold, 80.0);
        assert!(monitor.node_auto_kill);
    }

    #[test]
    fn test_initial_stats() {
        let monitor = ResourceMonitor::new("kesl", 30.0, 600, 3, 100);
        let stats = monitor.stats();

        assert_eq!(stats.total_checks, 0);
        assert_eq!(stats.total_violations, 0);
        assert_eq!(stats.total_kills, 0);
        assert_eq!(stats.total_restarts, 0);
    }

    #[test]
    fn test_violations_getter() {
        let monitor = ResourceMonitor::new("kesl", 30.0, 600, 3, 100);
        let (cpu, mem) = monitor.violations();

        assert_eq!(cpu, 0);
        assert_eq!(mem, 0);
    }

    #[test]
    fn test_reset_violations() {
        let mut monitor = ResourceMonitor::new("kesl", 30.0, 600, 3, 100);

        // Manually set violations (simulating check_kesl behavior)
        monitor.cpu_violations = 2;
        monitor.memory_violations = 1;

        let (cpu, mem) = monitor.violations();
        assert_eq!(cpu, 2);
        assert_eq!(mem, 1);

        monitor.reset_violations();

        let (cpu, mem) = monitor.violations();
        assert_eq!(cpu, 0);
        assert_eq!(mem, 0);
    }

    #[test]
    fn test_monitor_with_default_values() {
        let monitor = ResourceMonitor::new("test-service", 50.0, 1024, 5, 300);

        assert_eq!(monitor.cpu_threshold, 50.0);
        assert_eq!(monitor.memory_threshold_mb, 1024);
        assert_eq!(monitor.max_violations, 5);
        assert_eq!(monitor.min_restart_interval_secs, 300);
    }

    #[test]
    fn test_node_monitoring_disabled_by_default() {
        let monitor = ResourceMonitor::new("kesl", 30.0, 600, 3, 100);

        assert!(!monitor.node_enabled);
        assert_eq!(monitor.node_cpu_threshold, 80.0); // Default value
        assert!(!monitor.node_auto_kill);
    }

    #[test]
    fn test_check_increments_stats() {
        let mut monitor = ResourceMonitor::new("kesl", 30.0, 600, 3, 100);

        assert_eq!(monitor.stats().total_checks, 0);

        // Check will fail to find KESL (not running in test), but should increment checks
        let _ = monitor.check();

        // Stats should be incremented even if process not found
        assert_eq!(monitor.stats().total_checks, 1);
    }
}
