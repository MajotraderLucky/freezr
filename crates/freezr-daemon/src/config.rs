use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main configuration for FreezR daemon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// KESL monitoring configuration
    pub kesl: KeslConfig,

    /// Node.js monitoring configuration
    pub node: NodeConfig,

    /// Logging configuration
    pub logging: LogConfig,

    /// General monitoring settings
    pub monitoring: MonitoringConfig,
}

/// KESL process monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeslConfig {
    /// CPU threshold in percent (default: 30.0)
    /// Matches CPUQuota=30% systemd limit
    pub cpu_threshold: f64,

    /// Memory threshold in MB (default: 600)
    /// Warning threshold above the hard limit of 512MB
    pub memory_threshold_mb: u64,

    /// Maximum violations before restart (default: 3)
    pub max_violations: u32,

    /// Systemd service name (default: "kesl")
    pub service_name: String,

    /// Enable KESL monitoring (default: true)
    pub enabled: bool,
}

/// Node.js process monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    /// CPU threshold for Node.js processes (default: 80.0)
    /// Processes above this are considered hung
    pub cpu_threshold: f64,

    /// Enable Node.js monitoring (default: true)
    pub enabled: bool,

    /// Automatically kill high-CPU Node.js processes (default: true)
    pub auto_kill: bool,

    /// Require confirmation before killing (default: false)
    /// Only works in interactive mode
    pub confirm_kill: bool,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    /// Log directory path (default: ./logs/)
    pub log_dir: PathBuf,

    /// KESL monitor log file name (default: kesl-monitor.log)
    pub kesl_log: String,

    /// Node monitor log file name (default: node-monitor.log)
    pub node_log: String,

    /// Actions log file name (default: actions.log)
    pub actions_log: String,

    /// Maximum log file size in MB before rotation (default: 10)
    pub max_file_size_mb: u64,

    /// Number of rotated log files to keep (default: 5)
    pub rotate_count: u32,
}

/// General monitoring settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Check interval in seconds (default: 3)
    pub check_interval_secs: u64,

    /// Minimum restart interval in seconds (default: 100)
    /// Prevents restart loops
    pub min_restart_interval_secs: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            kesl: KeslConfig::default(),
            node: NodeConfig::default(),
            logging: LogConfig::default(),
            monitoring: MonitoringConfig::default(),
        }
    }
}

impl Default for KeslConfig {
    fn default() -> Self {
        Self {
            cpu_threshold: 30.0,
            memory_threshold_mb: 600,
            max_violations: 3,
            service_name: "kesl".to_string(),
            enabled: true,
        }
    }
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            cpu_threshold: 80.0,
            enabled: true,
            auto_kill: true,
            confirm_kill: false,
        }
    }
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            log_dir: PathBuf::from("./logs"),
            kesl_log: "kesl-monitor.log".to_string(),
            node_log: "node-monitor.log".to_string(),
            actions_log: "actions.log".to_string(),
            max_file_size_mb: 10,
            rotate_count: 5,
        }
    }
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            check_interval_secs: 3,
            min_restart_interval_secs: 100,
        }
    }
}

impl Config {
    /// Load configuration from TOML file
    ///
    /// # Arguments
    /// * `path` - Path to the TOML configuration file
    ///
    /// # Example
    /// ```no_run
    /// use freezr_daemon::config::Config;
    ///
    /// let config = Config::load_from_file("/etc/freezr/config.toml").unwrap();
    /// println!("KESL CPU threshold: {}", config.kesl.cpu_threshold);
    /// ```
    pub fn load_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to TOML file
    ///
    /// # Arguments
    /// * `path` - Path where to save the configuration
    pub fn save_to_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Validate configuration values
    ///
    /// Checks that all thresholds and intervals are within reasonable ranges
    pub fn validate(&self) -> Result<(), String> {
        // Validate KESL config
        if self.kesl.cpu_threshold < 0.0 || self.kesl.cpu_threshold > 100.0 {
            return Err(format!(
                "KESL CPU threshold must be 0-100, got: {}",
                self.kesl.cpu_threshold
            ));
        }

        if self.kesl.memory_threshold_mb == 0 {
            return Err("KESL memory threshold must be > 0".to_string());
        }

        if self.kesl.max_violations == 0 {
            return Err("KESL max violations must be > 0".to_string());
        }

        // Validate Node config
        if self.node.cpu_threshold < 0.0 || self.node.cpu_threshold > 100.0 {
            return Err(format!(
                "Node CPU threshold must be 0-100, got: {}",
                self.node.cpu_threshold
            ));
        }

        // Validate monitoring config
        if self.monitoring.check_interval_secs == 0 {
            return Err("Check interval must be > 0".to_string());
        }

        if self.monitoring.min_restart_interval_secs == 0 {
            return Err("Min restart interval must be > 0".to_string());
        }

        // Validate logging config
        if self.logging.max_file_size_mb == 0 {
            return Err("Max log file size must be > 0".to_string());
        }

        if self.logging.rotate_count == 0 {
            return Err("Log rotate count must be > 0".to_string());
        }

        Ok(())
    }

    /// Get full path to KESL log file
    pub fn kesl_log_path(&self) -> PathBuf {
        self.logging.log_dir.join(&self.logging.kesl_log)
    }

    /// Get full path to Node log file
    pub fn node_log_path(&self) -> PathBuf {
        self.logging.log_dir.join(&self.logging.node_log)
    }

    /// Get full path to actions log file
    pub fn actions_log_path(&self) -> PathBuf {
        self.logging.log_dir.join(&self.logging.actions_log)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();

        assert_eq!(config.kesl.cpu_threshold, 30.0);
        assert_eq!(config.kesl.memory_threshold_mb, 600);
        assert_eq!(config.kesl.max_violations, 3);
        assert_eq!(config.node.cpu_threshold, 80.0);
        assert_eq!(config.monitoring.check_interval_secs, 3);
    }

    #[test]
    fn test_kesl_config_default() {
        let kesl = KeslConfig::default();

        assert_eq!(kesl.cpu_threshold, 30.0);
        assert_eq!(kesl.memory_threshold_mb, 600);
        assert_eq!(kesl.max_violations, 3);
        assert_eq!(kesl.service_name, "kesl");
        assert!(kesl.enabled);
    }

    #[test]
    fn test_node_config_default() {
        let node = NodeConfig::default();

        assert_eq!(node.cpu_threshold, 80.0);
        assert!(node.enabled);
        assert!(node.auto_kill);
        assert!(!node.confirm_kill);
    }

    #[test]
    fn test_logging_config_default() {
        let logging = LogConfig::default();

        assert_eq!(logging.log_dir, PathBuf::from("./logs"));
        assert_eq!(logging.kesl_log, "kesl-monitor.log");
        assert_eq!(logging.node_log, "node-monitor.log");
        assert_eq!(logging.actions_log, "actions.log");
        assert_eq!(logging.max_file_size_mb, 10);
        assert_eq!(logging.rotate_count, 5);
    }

    #[test]
    fn test_monitoring_config_default() {
        let monitoring = MonitoringConfig::default();

        assert_eq!(monitoring.check_interval_secs, 3);
        assert_eq!(monitoring.min_restart_interval_secs, 100);
    }

    #[test]
    fn test_config_validation_valid() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_invalid_cpu() {
        let mut config = Config::default();
        config.kesl.cpu_threshold = 150.0;

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("CPU threshold"));
    }

    #[test]
    fn test_config_validation_zero_memory() {
        let mut config = Config::default();
        config.kesl.memory_threshold_mb = 0;

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("memory threshold"));
    }

    #[test]
    fn test_config_validation_zero_interval() {
        let mut config = Config::default();
        config.monitoring.check_interval_secs = 0;

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Check interval"));
    }

    #[test]
    fn test_log_paths() {
        let config = Config::default();

        assert_eq!(
            config.kesl_log_path(),
            PathBuf::from("./logs/kesl-monitor.log")
        );
        assert_eq!(
            config.node_log_path(),
            PathBuf::from("./logs/node-monitor.log")
        );
        assert_eq!(
            config.actions_log_path(),
            PathBuf::from("./logs/actions.log")
        );
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).unwrap();

        assert!(toml_str.contains("cpu_threshold"));
        assert!(toml_str.contains("memory_threshold_mb"));
        assert!(toml_str.contains("[kesl]"));
        assert!(toml_str.contains("[node]"));
    }

    #[test]
    fn test_config_deserialization() {
        let toml_str = r#"
            [kesl]
            cpu_threshold = 25.0
            memory_threshold_mb = 500
            max_violations = 5
            service_name = "kesl"
            enabled = true

            [node]
            cpu_threshold = 90.0
            enabled = true
            auto_kill = false
            confirm_kill = true

            [logging]
            log_dir = "/var/log/freezr"
            kesl_log = "kesl.log"
            node_log = "node.log"
            actions_log = "actions.log"
            max_file_size_mb = 20
            rotate_count = 10

            [monitoring]
            check_interval_secs = 5
            min_restart_interval_secs = 120
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();

        assert_eq!(config.kesl.cpu_threshold, 25.0);
        assert_eq!(config.node.cpu_threshold, 90.0);
        assert_eq!(config.monitoring.check_interval_secs, 5);
        assert_eq!(config.logging.log_dir, PathBuf::from("/var/log/freezr"));
    }
}
