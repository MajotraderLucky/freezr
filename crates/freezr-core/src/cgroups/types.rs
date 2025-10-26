//! Data types for cgroup management

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use super::error::{CgroupError, Result};

/// Cgroup management strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CgroupStrategy {
    /// Only use pre-configured static cgroups
    Static,
    /// Only create dynamic cgroups at runtime
    Dynamic,
    /// Use both static and dynamic (recommended)
    Hybrid,
}

/// Cgroup type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CgroupType {
    /// Pre-configured, persistent cgroup
    Static,
    /// Temporary cgroup for runtime throttling
    Dynamic,
}

/// Resource limits to apply to a cgroup
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// CPU limit (percentage, 0-100 for single core, >100 for multi-core)
    pub cpu_limit_percent: Option<f64>,

    /// Hard memory limit (bytes)
    pub memory_max: Option<u64>,

    /// Soft memory limit (bytes)
    pub memory_high: Option<u64>,
}

impl ResourceLimits {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_cpu_limit(mut self, percent: f64) -> Self {
        self.cpu_limit_percent = Some(percent);
        self
    }

    pub fn with_memory_max(mut self, bytes: u64) -> Self {
        self.memory_max = Some(bytes);
        self
    }

    pub fn with_memory_high(mut self, bytes: u64) -> Self {
        self.memory_high = Some(bytes);
        self
    }

    /// Validate limits are reasonable
    pub fn validate(&self) -> Result<()> {
        if let Some(cpu) = self.cpu_limit_percent {
            if cpu < 0.0 || cpu > 1000.0 {
                return Err(CgroupError::InvalidLimit(format!(
                    "CPU limit must be between 0-1000%, got {}%",
                    cpu
                )));
            }
        }

        if let Some(mem_max) = self.memory_max {
            if mem_max == 0 {
                return Err(CgroupError::InvalidLimit(
                    "Memory max limit cannot be 0".to_string(),
                ));
            }
        }

        if let (Some(high), Some(max)) = (self.memory_high, self.memory_max) {
            if high > max {
                return Err(CgroupError::InvalidLimit(
                    "Memory high limit cannot exceed max limit".to_string(),
                ));
            }
        }

        Ok(())
    }
}

/// Configuration for static cgroup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticCgroupConfig {
    /// Cgroup name (e.g., "kesl")
    pub name: String,

    /// Process name patterns to assign to this cgroup
    pub process_patterns: Vec<String>,

    /// Resource limits
    #[serde(flatten)]
    pub limits: ResourceLimits,

    /// CPU limit in percent (for TOML convenience)
    #[serde(rename = "cpu_limit_percent")]
    pub cpu_limit_percent_compat: Option<f64>,

    /// Memory max in MB (for TOML convenience)
    #[serde(rename = "memory_max_mb")]
    pub memory_max_mb: Option<u64>,

    /// Memory high in MB (for TOML convenience)
    #[serde(rename = "memory_high_mb")]
    pub memory_high_mb: Option<u64>,
}

impl StaticCgroupConfig {
    /// Get resource limits (handles TOML convenience fields)
    pub fn get_limits(&self) -> ResourceLimits {
        let mut limits = self.limits.clone();

        // Override with convenience fields if present
        if let Some(cpu) = self.cpu_limit_percent_compat {
            limits.cpu_limit_percent = Some(cpu);
        }
        if let Some(mem_mb) = self.memory_max_mb {
            limits.memory_max = Some(mem_mb * 1024 * 1024);
        }
        if let Some(mem_mb) = self.memory_high_mb {
            limits.memory_high = Some(mem_mb * 1024 * 1024);
        }

        limits
    }
}

/// Dynamic cgroup settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicCgroupSettings {
    /// Maximum number of dynamic cgroups
    pub max_dynamic_cgroups: usize,

    /// Auto-cleanup after this duration (seconds) if process exited
    pub cleanup_timeout_secs: u64,

    /// Default CPU limit for dynamic cgroups (%)
    pub default_cpu_limit: f64,

    /// Default memory limit for dynamic cgroups (MB)
    pub default_memory_limit_mb: u64,
}

impl Default for DynamicCgroupSettings {
    fn default() -> Self {
        Self {
            max_dynamic_cgroups: 50,
            cleanup_timeout_secs: 300,
            default_cpu_limit: 50.0,
            default_memory_limit_mb: 1024,
        }
    }
}

/// Cgroup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CgroupConfig {
    /// Enable cgroup integration
    #[serde(default)]
    pub enabled: bool,

    /// Root path for FreezR cgroups
    #[serde(default = "default_root_path")]
    pub root_path: PathBuf,

    /// Strategy
    #[serde(default = "default_strategy")]
    pub strategy: CgroupStrategy,

    /// Static cgroup configurations
    #[serde(default)]
    pub static_groups: Vec<StaticCgroupConfig>,

    /// Dynamic cgroup settings
    #[serde(default)]
    pub dynamic_settings: DynamicCgroupSettings,

    /// Auto-cleanup on service stop
    #[serde(default = "default_true")]
    pub auto_cleanup_on_stop: bool,

    /// Restore processes to original cgroups on stop
    #[serde(default = "default_true")]
    pub restore_processes_on_stop: bool,
}

fn default_root_path() -> PathBuf {
    PathBuf::from("/sys/fs/cgroup/freezr.slice")
}

fn default_strategy() -> CgroupStrategy {
    CgroupStrategy::Hybrid
}

fn default_true() -> bool {
    true
}

impl Default for CgroupConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            root_path: default_root_path(),
            strategy: default_strategy(),
            static_groups: Vec::new(),
            dynamic_settings: DynamicCgroupSettings::default(),
            auto_cleanup_on_stop: true,
            restore_processes_on_stop: true,
        }
    }
}

/// Represents a single cgroup
#[derive(Debug, Clone)]
pub struct Cgroup {
    /// Cgroup name
    pub name: String,

    /// Full path to cgroup
    pub path: PathBuf,

    /// Type (static or dynamic)
    pub cgroup_type: CgroupType,

    /// Current resource limits
    pub limits: ResourceLimits,

    /// PIDs currently in this cgroup
    pub pids: Vec<u32>,
}

impl Cgroup {
    pub fn new(name: String, path: PathBuf, cgroup_type: CgroupType) -> Self {
        Self {
            name,
            path,
            cgroup_type,
            limits: ResourceLimits::default(),
            pids: Vec::new(),
        }
    }

    /// Check if cgroup directory exists
    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    /// Get list of processes in this cgroup
    pub fn get_processes(&self) -> Result<Vec<u32>> {
        use std::fs;

        let procs_file = self.path.join("cgroup.procs");
        let content = fs::read_to_string(&procs_file)?;

        let pids = content
            .lines()
            .filter_map(|line| line.trim().parse::<u32>().ok())
            .collect();

        Ok(pids)
    }

    /// Reload process list
    pub fn reload_processes(&mut self) -> Result<()> {
        self.pids = self.get_processes()?;
        Ok(())
    }
}

/// Cgroup manager
pub struct CgroupManager {
    /// Root path for FreezR cgroups
    root_path: PathBuf,

    /// Strategy
    strategy: CgroupStrategy,

    /// Active cgroups (name -> Cgroup)
    cgroups: HashMap<String, Cgroup>,

    /// Static cgroup configurations
    static_configs: Vec<StaticCgroupConfig>,

    /// Dynamic cgroup settings
    dynamic_settings: DynamicCgroupSettings,

    /// Config
    config: CgroupConfig,
}

impl CgroupManager {
    /// Create new cgroup manager
    pub fn new(config: CgroupConfig) -> Result<Self> {
        // Validate configuration
        if config.enabled {
            Self::validate_system()?;
        }

        Ok(Self {
            root_path: config.root_path.clone(),
            strategy: config.strategy,
            cgroups: HashMap::new(),
            static_configs: config.static_groups.clone(),
            dynamic_settings: config.dynamic_settings.clone(),
            config,
        })
    }

    /// Validate system supports cgroup v2
    fn validate_system() -> Result<()> {
        use std::fs;

        // Check if cgroup v2 is mounted
        let cgroup_mount = PathBuf::from("/sys/fs/cgroup");
        if !cgroup_mount.exists() {
            return Err(CgroupError::CgroupV2NotAvailable);
        }

        // Check if cgroup.controllers exists (v2 indicator)
        let controllers_file = cgroup_mount.join("cgroup.controllers");
        if !controllers_file.exists() {
            return Err(CgroupError::CgroupV2NotAvailable);
        }

        // Check if we have write permission
        let test_path = cgroup_mount.join("cgroup.subtree_control");
        if let Err(_) = fs::metadata(&test_path) {
            return Err(CgroupError::InsufficientPrivileges);
        }

        Ok(())
    }

    /// Initialize: create root slice, apply static configs (called on service start)
    pub fn initialize(&mut self) -> Result<()> {
        use std::fs;

        // Create root slice directory
        if !self.root_path.exists() {
            fs::create_dir(&self.root_path)?;
            println!("Created cgroup root: {:?}", self.root_path);
        }

        // Enable controllers for root slice
        self.enable_controllers(&self.root_path)?;

        // Create and configure static cgroups
        if matches!(
            self.strategy,
            CgroupStrategy::Static | CgroupStrategy::Hybrid
        ) {
            for config in &self.static_configs.clone() {
                self.create_static_cgroup(config)?;
            }
        }

        println!("Cgroup integration initialized");
        Ok(())
    }

    /// Enable CPU and memory controllers
    fn enable_controllers(&self, path: &PathBuf) -> Result<()> {
        use std::fs;

        let subtree_control = path.join("cgroup.subtree_control");
        fs::write(&subtree_control, "+cpu +memory").map_err(|e| {
            CgroupError::PermissionDenied(format!(
                "Failed to enable controllers at {:?}: {}",
                subtree_control, e
            ))
        })?;

        Ok(())
    }

    /// Create static cgroup from config
    fn create_static_cgroup(&mut self, config: &StaticCgroupConfig) -> Result<()> {
        use std::fs;

        let cgroup_path = self.root_path.join(&config.name);

        // Create directory
        if !cgroup_path.exists() {
            fs::create_dir(&cgroup_path)?;
        }

        // Create Cgroup object
        let mut cgroup = Cgroup::new(config.name.clone(), cgroup_path, CgroupType::Static);
        cgroup.limits = config.get_limits();

        // Apply limits
        self.apply_limits(&cgroup)?;

        // Store in map
        self.cgroups.insert(config.name.clone(), cgroup);

        println!("Created static cgroup: {}", config.name);
        Ok(())
    }

    /// Create dynamic cgroup
    pub fn create_cgroup(&mut self, name: &str) -> Result<Cgroup> {
        use std::fs;

        // Check if already exists
        if self.cgroups.contains_key(name) {
            return Err(CgroupError::AlreadyExists(name.to_string()));
        }

        // Check dynamic cgroup limit
        if self.count_dynamic_cgroups() >= self.dynamic_settings.max_dynamic_cgroups {
            return Err(CgroupError::MaxCgroupsReached(
                self.dynamic_settings.max_dynamic_cgroups,
            ));
        }

        let cgroup_path = self.root_path.join(name);

        // Create directory
        fs::create_dir(&cgroup_path)?;

        // Create Cgroup object
        let cgroup = Cgroup::new(name.to_string(), cgroup_path, CgroupType::Dynamic);

        // Store in map
        self.cgroups.insert(name.to_string(), cgroup.clone());

        println!("Created dynamic cgroup: {}", name);
        Ok(cgroup)
    }

    /// Apply resource limits to a cgroup
    pub fn apply_limits(&self, cgroup: &Cgroup) -> Result<()> {
        use super::controller::{CpuController, MemoryController};

        // Validate limits
        cgroup.limits.validate()?;

        // Apply CPU limit
        if let Some(cpu_percent) = cgroup.limits.cpu_limit_percent {
            CpuController::set_quota(&cgroup.path, cpu_percent)?;
            println!(
                "Applied CPU limit {}% to cgroup {}",
                cpu_percent, cgroup.name
            );
        }

        // Apply memory max limit
        if let Some(mem_max) = cgroup.limits.memory_max {
            MemoryController::set_max(&cgroup.path, mem_max)?;
            println!(
                "Applied memory max {} MB to cgroup {}",
                mem_max / 1024 / 1024,
                cgroup.name
            );
        }

        // Apply memory high limit
        if let Some(mem_high) = cgroup.limits.memory_high {
            MemoryController::set_high(&cgroup.path, mem_high)?;
        }

        Ok(())
    }

    /// Assign process to cgroup
    pub fn assign_process(&self, cgroup: &Cgroup, pid: u32) -> Result<()> {
        use std::fs;

        // Check process exists
        if !PathBuf::from(format!("/proc/{}", pid)).exists() {
            return Err(CgroupError::ProcessNotFound(pid));
        }

        // Write PID to cgroup.procs
        let procs_file = cgroup.path.join("cgroup.procs");
        fs::write(&procs_file, pid.to_string())?;

        println!("Assigned process {} to cgroup {}", pid, cgroup.name);
        Ok(())
    }

    /// Remove cgroup
    pub fn remove_cgroup(&mut self, name: &str) -> Result<()> {
        use std::fs;

        let cgroup = self
            .cgroups
            .remove(name)
            .ok_or_else(|| CgroupError::NotFound(name.to_string()))?;

        // Move processes out first
        let pids = cgroup.get_processes()?;
        for pid in pids {
            // Move back to root cgroup
            let root_procs = PathBuf::from("/sys/fs/cgroup/cgroup.procs");
            if let Ok(_) = fs::write(&root_procs, pid.to_string()) {
                println!("Moved process {} back to root cgroup", pid);
            }
        }

        // Remove directory
        fs::remove_dir(&cgroup.path)?;

        println!("Removed cgroup: {}", name);
        Ok(())
    }

    /// Called when systemd service stops (CRITICAL)
    pub fn on_service_stop(&mut self) -> Result<()> {
        println!("Cleaning up cgroups on service stop...");

        // Restore all processes
        if self.config.restore_processes_on_stop {
            self.restore_all_processes()?;
        }

        // Remove all cgroups
        if self.config.auto_cleanup_on_stop {
            self.remove_all_cgroups()?;
            self.cleanup_root_slice()?;
        }

        println!("✅ Cgroup cleanup complete - system restored");
        Ok(())
    }

    /// Restore all processes to their original cgroups
    fn restore_all_processes(&self) -> Result<()> {
        use std::fs;

        for cgroup in self.cgroups.values() {
            let pids = cgroup.get_processes()?;
            for pid in pids {
                let root_procs = PathBuf::from("/sys/fs/cgroup/cgroup.procs");
                if let Ok(_) = fs::write(&root_procs, pid.to_string()) {
                    println!("Restored process {} to root cgroup", pid);
                }
            }
        }

        Ok(())
    }

    /// Remove all FreezR cgroups (static + dynamic)
    fn remove_all_cgroups(&mut self) -> Result<()> {
        use std::fs;

        let cgroup_names: Vec<String> = self.cgroups.keys().cloned().collect();

        for name in cgroup_names {
            if let Some(cgroup) = self.cgroups.remove(&name) {
                if cgroup.path.exists() {
                    let _ = fs::remove_dir(&cgroup.path);
                }
            }
        }

        Ok(())
    }

    /// Remove root slice
    fn cleanup_root_slice(&self) -> Result<()> {
        use std::fs;

        if self.root_path.exists() {
            fs::remove_dir(&self.root_path)?;
            println!("Removed root cgroup: {:?}", self.root_path);
        }

        Ok(())
    }

    /// Health check
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

        if issues.is_empty() {
            Ok(HealthStatus::Healthy)
        } else {
            Ok(HealthStatus::Degraded(issues))
        }
    }

    /// Get cgroup by name
    pub fn get_cgroup(&self, name: &str) -> Option<&Cgroup> {
        self.cgroups.get(name)
    }

    /// Get mutable cgroup by name
    pub fn get_cgroup_mut(&mut self, name: &str) -> Option<&mut Cgroup> {
        self.cgroups.get_mut(name)
    }

    /// Count dynamic cgroups
    pub fn count_dynamic_cgroups(&self) -> usize {
        self.cgroups
            .values()
            .filter(|c| c.cgroup_type == CgroupType::Dynamic)
            .count()
    }

    /// Cleanup (alias for on_service_stop)
    pub fn cleanup(&mut self) -> Result<()> {
        self.on_service_stop()
    }
}

/// Health status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded(Vec<String>),
    Failed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_limits_builder() {
        let limits = ResourceLimits::new()
            .with_cpu_limit(50.0)
            .with_memory_max(1024 * 1024 * 1024);

        assert_eq!(limits.cpu_limit_percent, Some(50.0));
        assert_eq!(limits.memory_max, Some(1024 * 1024 * 1024));
        assert_eq!(limits.memory_high, None);
    }

    #[test]
    fn test_resource_limits_validation() {
        // Valid limits
        let limits = ResourceLimits::new().with_cpu_limit(50.0);
        assert!(limits.validate().is_ok());

        // Invalid CPU limit
        let limits = ResourceLimits::new().with_cpu_limit(-10.0);
        assert!(limits.validate().is_err());

        let limits = ResourceLimits::new().with_cpu_limit(2000.0);
        assert!(limits.validate().is_err());

        // Invalid memory limits
        let limits = ResourceLimits::new()
            .with_memory_high(2 * 1024 * 1024 * 1024)
            .with_memory_max(1 * 1024 * 1024 * 1024);
        assert!(limits.validate().is_err());
    }

    #[test]
    fn test_cgroup_strategy_serde() {
        let strategy = CgroupStrategy::Hybrid;
        let json = serde_json::to_string(&strategy).unwrap();
        assert_eq!(json, r#""hybrid""#);

        let deserialized: CgroupStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, strategy);
    }

    #[test]
    fn test_cgroup_type() {
        assert_eq!(CgroupType::Static, CgroupType::Static);
        assert_ne!(CgroupType::Static, CgroupType::Dynamic);
    }

    #[test]
    fn test_cgroup_creation() {
        let cgroup = Cgroup::new(
            "test".to_string(),
            PathBuf::from("/sys/fs/cgroup/test"),
            CgroupType::Static,
        );

        assert_eq!(cgroup.name, "test");
        assert_eq!(cgroup.cgroup_type, CgroupType::Static);
        assert!(cgroup.pids.is_empty());
    }

    #[test]
    fn test_default_config() {
        let config = CgroupConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.root_path, PathBuf::from("/sys/fs/cgroup/freezr.slice"));
        assert_eq!(config.strategy, CgroupStrategy::Hybrid);
        assert!(config.auto_cleanup_on_stop);
    }

    #[test]
    fn test_health_status() {
        let healthy = HealthStatus::Healthy;
        assert_eq!(healthy, HealthStatus::Healthy);

        let degraded = HealthStatus::Degraded(vec!["Issue 1".to_string()]);
        assert_ne!(degraded, HealthStatus::Healthy);
    }
}
