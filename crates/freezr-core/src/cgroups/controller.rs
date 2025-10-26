//! Controller-specific operations for CPU and Memory

use std::path::Path;

use super::error::Result;
use super::utils::{
    convert_percent_to_quota, convert_quota_to_percent, parse_cpu_stat, parse_memory_stat,
    read_cgroup_file, write_cgroup_file,
};

/// CPU controller operations
pub struct CpuController;

impl CpuController {
    /// Set CPU quota (percentage -> microseconds)
    ///
    /// # Arguments
    /// * `cgroup_path` - Path to cgroup directory
    /// * `percent` - CPU limit percentage (0-100 for single core, >100 for multi-core)
    ///
    /// # Examples
    /// ```ignore
    /// // Limit to 30% of one core
    /// CpuController::set_quota(path, 30.0)?;
    ///
    /// // Allow 200% (2 cores)
    /// CpuController::set_quota(path, 200.0)?;
    /// ```
    pub fn set_quota(cgroup_path: &Path, percent: f64) -> Result<()> {
        let (quota, period) = convert_percent_to_quota(percent);
        let cpu_max_file = cgroup_path.join("cpu.max");

        let content = if percent >= 100.0 {
            format!("{} {}", quota, period)
        } else {
            format!("{} {}", quota, period)
        };

        write_cgroup_file(&cpu_max_file, &content)?;
        Ok(())
    }

    /// Get current CPU quota
    pub fn get_quota(cgroup_path: &Path) -> Result<Option<f64>> {
        let cpu_max_file = cgroup_path.join("cpu.max");
        let content = read_cgroup_file(&cpu_max_file)?;

        let parts: Vec<&str> = content.trim().split_whitespace().collect();
        if parts.len() != 2 {
            return Ok(None);
        }

        if parts[0] == "max" {
            return Ok(None); // Unlimited
        }

        let quota: u64 = parts[0].parse().map_err(|_| {
            super::error::CgroupError::ParseError(format!("Invalid quota: {}", parts[0]))
        })?;

        let period: u64 = parts[1].parse().map_err(|_| {
            super::error::CgroupError::ParseError(format!("Invalid period: {}", parts[1]))
        })?;

        Ok(Some(convert_quota_to_percent(quota, period)))
    }

    /// Remove CPU quota (set to unlimited)
    pub fn remove_quota(cgroup_path: &Path) -> Result<()> {
        let cpu_max_file = cgroup_path.join("cpu.max");
        write_cgroup_file(&cpu_max_file, "max 100000")?;
        Ok(())
    }

    /// Set CPU weight (relative share, 1-10000)
    ///
    /// Higher weight = more CPU time when there's contention
    /// Default weight = 100
    pub fn set_weight(cgroup_path: &Path, weight: u32) -> Result<()> {
        if weight < 1 || weight > 10000 {
            return Err(super::error::CgroupError::InvalidLimit(format!(
                "CPU weight must be 1-10000, got {}",
                weight
            )));
        }

        let cpu_weight_file = cgroup_path.join("cpu.weight");
        write_cgroup_file(&cpu_weight_file, &weight.to_string())?;
        Ok(())
    }

    /// Get CPU statistics
    pub fn get_stats(cgroup_path: &Path) -> Result<CpuStats> {
        let cpu_stat_file = cgroup_path.join("cpu.stat");
        let content = read_cgroup_file(&cpu_stat_file)?;
        let values = parse_cpu_stat(&content)?;

        Ok(CpuStats {
            usage_usec: values.usage_usec,
            user_usec: values.user_usec,
            system_usec: values.system_usec,
            nr_periods: values.nr_periods,
            nr_throttled: values.nr_throttled,
            throttled_usec: values.throttled_usec,
        })
    }
}

/// CPU statistics
#[derive(Debug, Clone, Default)]
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
    pub fn throttle_percentage(&self) -> f64 {
        if self.nr_periods == 0 {
            return 0.0;
        }
        (self.nr_throttled as f64 / self.nr_periods as f64) * 100.0
    }

    /// Check if being throttled
    pub fn is_throttled(&self) -> bool {
        self.nr_throttled > 0
    }
}

/// Memory controller operations
pub struct MemoryController;

impl MemoryController {
    /// Set hard memory limit
    ///
    /// Process will be killed (OOM) if it exceeds this limit
    pub fn set_max(cgroup_path: &Path, bytes: u64) -> Result<()> {
        let memory_max_file = cgroup_path.join("memory.max");
        write_cgroup_file(&memory_max_file, &bytes.to_string())?;
        Ok(())
    }

    /// Get hard memory limit
    pub fn get_max(cgroup_path: &Path) -> Result<Option<u64>> {
        let memory_max_file = cgroup_path.join("memory.max");
        let content = read_cgroup_file(&memory_max_file)?;

        if content.trim() == "max" {
            return Ok(None); // Unlimited
        }

        let bytes: u64 = content.trim().parse().map_err(|_| {
            super::error::CgroupError::ParseError(format!("Invalid memory.max: {}", content))
        })?;

        Ok(Some(bytes))
    }

    /// Remove memory limit (set to unlimited)
    pub fn remove_max(cgroup_path: &Path) -> Result<()> {
        let memory_max_file = cgroup_path.join("memory.max");
        write_cgroup_file(&memory_max_file, "max")?;
        Ok(())
    }

    /// Set soft memory limit
    ///
    /// Process will be throttled (slowed down) if it exceeds this limit,
    /// but not killed
    pub fn set_high(cgroup_path: &Path, bytes: u64) -> Result<()> {
        let memory_high_file = cgroup_path.join("memory.high");
        write_cgroup_file(&memory_high_file, &bytes.to_string())?;
        Ok(())
    }

    /// Get current memory usage
    pub fn get_current(cgroup_path: &Path) -> Result<u64> {
        let memory_current_file = cgroup_path.join("memory.current");
        let content = read_cgroup_file(&memory_current_file)?;

        let bytes: u64 = content.trim().parse().map_err(|_| {
            super::error::CgroupError::ParseError(format!("Invalid memory.current: {}", content))
        })?;

        Ok(bytes)
    }

    /// Get memory statistics
    pub fn get_stats(cgroup_path: &Path) -> Result<MemoryStats> {
        let current = Self::get_current(cgroup_path)?;

        // Read memory.stat for detailed breakdown
        let memory_stat_file = cgroup_path.join("memory.stat");
        let content = read_cgroup_file(&memory_stat_file)?;
        let values = parse_memory_stat(&content)?;

        // Read peak usage if available
        let peak = Self::get_peak(cgroup_path).unwrap_or(current);

        Ok(MemoryStats {
            current,
            peak,
            anon: values.anon,
            file: values.file,
            kernel_stack: values.kernel_stack,
            slab: values.slab,
        })
    }

    /// Get peak memory usage
    fn get_peak(cgroup_path: &Path) -> Result<u64> {
        let memory_peak_file = cgroup_path.join("memory.peak");
        let content = read_cgroup_file(&memory_peak_file)?;

        let bytes: u64 = content.trim().parse().map_err(|_| {
            super::error::CgroupError::ParseError(format!("Invalid memory.peak: {}", content))
        })?;

        Ok(bytes)
    }

    /// Get memory pressure (PSI)
    pub fn get_pressure(cgroup_path: &Path) -> Result<MemoryPressure> {
        let memory_pressure_file = cgroup_path.join("memory.pressure");
        let content = read_cgroup_file(&memory_pressure_file)?;

        parse_memory_pressure(&content)
    }
}

/// Memory statistics
#[derive(Debug, Clone, Default)]
pub struct MemoryStats {
    /// Current memory usage (bytes)
    pub current: u64,

    /// Peak memory usage (bytes)
    pub peak: u64,

    /// Anonymous memory (bytes)
    pub anon: u64,

    /// File cache (bytes)
    pub file: u64,

    /// Kernel stack (bytes)
    pub kernel_stack: u64,

    /// Slab memory (bytes)
    pub slab: u64,
}

impl MemoryStats {
    /// Get current usage in MB
    pub fn current_mb(&self) -> u64 {
        self.current / 1024 / 1024
    }

    /// Get peak usage in MB
    pub fn peak_mb(&self) -> u64 {
        self.peak / 1024 / 1024
    }
}

/// Memory pressure information (PSI)
#[derive(Debug, Clone, Default)]
pub struct MemoryPressure {
    /// Some: avg10, avg60, avg300
    pub some_avg10: f64,
    pub some_avg60: f64,
    pub some_avg300: f64,

    /// Full: avg10, avg60, avg300
    pub full_avg10: f64,
    pub full_avg60: f64,
    pub full_avg300: f64,
}

impl MemoryPressure {
    /// Check if under pressure (some avg10 > threshold)
    pub fn is_under_pressure(&self, threshold: f64) -> bool {
        self.some_avg10 > threshold
    }

    /// Check if critical pressure (full avg10 > threshold)
    pub fn is_critical(&self, threshold: f64) -> bool {
        self.full_avg10 > threshold
    }
}

/// Parse memory.pressure file
///
/// Format:
/// ```text
/// some avg10=0.00 avg60=0.00 avg300=0.00 total=0
/// full avg10=0.00 avg60=0.00 avg300=0.00 total=0
/// ```
fn parse_memory_pressure(content: &str) -> Result<MemoryPressure> {
    let mut pressure = MemoryPressure::default();

    for line in content.lines() {
        if line.starts_with("some ") {
            pressure.some_avg10 = extract_avg_value(line, "avg10")?;
            pressure.some_avg60 = extract_avg_value(line, "avg60")?;
            pressure.some_avg300 = extract_avg_value(line, "avg300")?;
        } else if line.starts_with("full ") {
            pressure.full_avg10 = extract_avg_value(line, "avg10")?;
            pressure.full_avg60 = extract_avg_value(line, "avg60")?;
            pressure.full_avg300 = extract_avg_value(line, "avg300")?;
        }
    }

    Ok(pressure)
}

/// Extract average value from PSI line
fn extract_avg_value(line: &str, key: &str) -> Result<f64> {
    let search = format!("{}=", key);
    if let Some(start) = line.find(&search) {
        let start = start + search.len();
        if let Some(end) = line[start..].find(char::is_whitespace) {
            let value_str = &line[start..start + end];
            return value_str.parse::<f64>().map_err(|_| {
                super::error::CgroupError::ParseError(format!(
                    "Failed to parse {}: {}",
                    key, value_str
                ))
            });
        }
    }

    Err(super::error::CgroupError::ParseError(format!(
        "Could not find {} in line: {}",
        key, line
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_cpu_stats_throttle_percentage() {
        let stats = CpuStats {
            nr_periods: 1000,
            nr_throttled: 100,
            ..Default::default()
        };

        assert_eq!(stats.throttle_percentage(), 10.0);
        assert!(stats.is_throttled());

        let stats = CpuStats::default();
        assert_eq!(stats.throttle_percentage(), 0.0);
        assert!(!stats.is_throttled());
    }

    #[test]
    fn test_memory_stats_mb_conversion() {
        let stats = MemoryStats {
            current: 1024 * 1024 * 500, // 500MB
            peak: 1024 * 1024 * 1024,   // 1GB
            ..Default::default()
        };

        assert_eq!(stats.current_mb(), 500);
        assert_eq!(stats.peak_mb(), 1024);
    }

    #[test]
    fn test_parse_memory_pressure() {
        let content = r#"some avg10=12.50 avg60=8.33 avg300=3.14 total=123456
full avg10=5.00 avg60=2.50 avg300=1.00 total=654321"#;

        let pressure = parse_memory_pressure(content).unwrap();
        assert_eq!(pressure.some_avg10, 12.50);
        assert_eq!(pressure.some_avg60, 8.33);
        assert_eq!(pressure.some_avg300, 3.14);
        assert_eq!(pressure.full_avg10, 5.00);
        assert_eq!(pressure.full_avg60, 2.50);
        assert_eq!(pressure.full_avg300, 1.00);
    }

    #[test]
    fn test_memory_pressure_thresholds() {
        let pressure = MemoryPressure {
            some_avg10: 15.0,
            full_avg10: 8.0,
            ..Default::default()
        };

        assert!(pressure.is_under_pressure(10.0));
        assert!(!pressure.is_under_pressure(20.0));

        assert!(pressure.is_critical(5.0));
        assert!(!pressure.is_critical(10.0));
    }

    #[test]
    fn test_extract_avg_value() {
        let line = "some avg10=12.50 avg60=8.33 avg300=3.14 total=123456";

        assert_eq!(extract_avg_value(line, "avg10").unwrap(), 12.50);
        assert_eq!(extract_avg_value(line, "avg60").unwrap(), 8.33);
        assert_eq!(extract_avg_value(line, "avg300").unwrap(), 3.14);

        // Missing key
        assert!(extract_avg_value(line, "avg999").is_err());
    }
}
