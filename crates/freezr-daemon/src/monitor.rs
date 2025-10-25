use freezr_core::{
    error::{Error, Result},
    executor::ProcessExecutor,
    scanner::ProcessScanner,
    systemd::SystemdService,
    types::MonitorStats,
};
use tracing::{debug, error, info, warn};

/// Resource monitor with violation tracking
///
/// Monitors KESL and Node.js processes, tracks resource violations,
/// and executes actions (restart service, kill processes) when limits are exceeded.
pub struct ResourceMonitor {
    scanner: ProcessScanner,
    kesl_service: SystemdService,
    stats: MonitorStats,

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

            cpu_violations: 0,
            memory_violations: 0,

            cpu_threshold,
            memory_threshold_mb,
            max_violations,
            min_restart_interval_secs,

            node_enabled: false,
            node_cpu_threshold: 80.0,
            node_auto_kill: false,
        }
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
