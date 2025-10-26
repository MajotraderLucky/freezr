//! Utility functions for cgroup operations

use std::fs;
use std::path::Path;

use super::error::{CgroupError, Result};

/// Convert CPU percentage to cgroup quota/period microseconds
///
/// Formula: quota = (percent / 100) * period
/// Standard period = 100,000 microseconds (100ms)
///
/// Examples:
/// - 30% -> (30000, 100000)
/// - 100% -> (100000, 100000)
/// - 200% -> (200000, 100000) [2 cores]
pub fn convert_percent_to_quota(percent: f64) -> (u64, u64) {
    const PERIOD_US: u64 = 100_000; // 100ms in microseconds

    let quota_us = ((percent / 100.0) * PERIOD_US as f64) as u64;
    (quota_us, PERIOD_US)
}

/// Convert cgroup quota/period to CPU percentage
pub fn convert_quota_to_percent(quota: u64, period: u64) -> f64 {
    if period == 0 {
        return 0.0;
    }
    (quota as f64 / period as f64) * 100.0
}

/// Parse CPU stat file
///
/// Format:
/// ```text
/// usage_usec 1234567890
/// user_usec 1000000
/// system_usec 234567890
/// nr_periods 1000
/// nr_throttled 100
/// throttled_usec 50000000
/// ```
pub fn parse_cpu_stat(content: &str) -> Result<CpuStatValues> {
    let mut values = CpuStatValues::default();

    for line in content.lines() {
        let mut parts = line.split_whitespace();
        let key = parts.next().ok_or_else(|| {
            CgroupError::ParseError(format!("Invalid cpu.stat line: {}", line))
        })?;
        let value = parts
            .next()
            .ok_or_else(|| CgroupError::ParseError(format!("Missing value for: {}", key)))?
            .parse::<u64>()
            .map_err(|e| CgroupError::ParseError(format!("Parse error for {}: {}", key, e)))?;

        match key {
            "usage_usec" => values.usage_usec = value,
            "user_usec" => values.user_usec = value,
            "system_usec" => values.system_usec = value,
            "nr_periods" => values.nr_periods = value,
            "nr_throttled" => values.nr_throttled = value,
            "throttled_usec" => values.throttled_usec = value,
            _ => {} // Ignore unknown keys
        }
    }

    Ok(values)
}

#[derive(Debug, Default, Clone)]
pub struct CpuStatValues {
    pub usage_usec: u64,
    pub user_usec: u64,
    pub system_usec: u64,
    pub nr_periods: u64,
    pub nr_throttled: u64,
    pub throttled_usec: u64,
}

impl CpuStatValues {
    /// Calculate throttle percentage
    pub fn throttle_percentage(&self) -> f64 {
        if self.nr_periods == 0 {
            return 0.0;
        }
        (self.nr_throttled as f64 / self.nr_periods as f64) * 100.0
    }
}

/// Parse memory stat file
pub fn parse_memory_stat(content: &str) -> Result<MemoryStatValues> {
    let mut values = MemoryStatValues::default();

    for line in content.lines() {
        let mut parts = line.split_whitespace();
        let key = parts.next().ok_or_else(|| {
            CgroupError::ParseError(format!("Invalid memory.stat line: {}", line))
        })?;
        let value = parts
            .next()
            .ok_or_else(|| CgroupError::ParseError(format!("Missing value for: {}", key)))?
            .parse::<u64>()
            .map_err(|e| CgroupError::ParseError(format!("Parse error for {}: {}", key, e)))?;

        match key {
            "anon" => values.anon = value,
            "file" => values.file = value,
            "kernel_stack" => values.kernel_stack = value,
            "slab" => values.slab = value,
            "sock" => values.sock = value,
            "shmem" => values.shmem = value,
            "file_mapped" => values.file_mapped = value,
            "file_dirty" => values.file_dirty = value,
            "file_writeback" => values.file_writeback = value,
            _ => {} // Ignore other keys
        }
    }

    Ok(values)
}

#[derive(Debug, Default, Clone)]
pub struct MemoryStatValues {
    pub anon: u64,
    pub file: u64,
    pub kernel_stack: u64,
    pub slab: u64,
    pub sock: u64,
    pub shmem: u64,
    pub file_mapped: u64,
    pub file_dirty: u64,
    pub file_writeback: u64,
}

/// Validate path is under allowed root
pub fn validate_path_under_root(path: &Path, root: &Path) -> Result<()> {
    let canonical_path = path
        .canonicalize()
        .map_err(|_| CgroupError::ValidationError(format!("Invalid path: {:?}", path)))?;

    let canonical_root = root
        .canonicalize()
        .map_err(|_| CgroupError::ValidationError(format!("Invalid root: {:?}", root)))?;

    if !canonical_path.starts_with(&canonical_root) {
        return Err(CgroupError::ValidationError(format!(
            "Path {:?} is not under root {:?}",
            path, root
        )));
    }

    Ok(())
}

/// Check if process exists
pub fn process_exists(pid: u32) -> bool {
    Path::new(&format!("/proc/{}", pid)).exists()
}

/// Get cgroup path for process
pub fn get_process_cgroup(pid: u32) -> Result<String> {
    let cgroup_file = format!("/proc/{}/cgroup", pid);
    let content = fs::read_to_string(&cgroup_file).map_err(|_| CgroupError::ProcessNotFound(pid))?;

    // Format: 0::/path/to/cgroup
    for line in content.lines() {
        if let Some(path) = line.strip_prefix("0::") {
            return Ok(path.to_string());
        }
    }

    Err(CgroupError::ParseError(format!(
        "Could not parse cgroup for PID {}",
        pid
    )))
}

/// Safe read file to string
pub fn read_cgroup_file(path: &Path) -> Result<String> {
    fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            CgroupError::NotFound(format!("{:?}", path))
        } else if e.kind() == std::io::ErrorKind::PermissionDenied {
            CgroupError::PermissionDenied(format!("{:?}", path))
        } else {
            CgroupError::Io(e)
        }
    })
}

/// Safe write file
pub fn write_cgroup_file(path: &Path, content: &str) -> Result<()> {
    fs::write(path, content).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            CgroupError::PermissionDenied(format!("{:?}", path))
        } else {
            CgroupError::Io(e)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_percent_to_quota() {
        // Single core limits
        assert_eq!(convert_percent_to_quota(30.0), (30_000, 100_000));
        assert_eq!(convert_percent_to_quota(50.0), (50_000, 100_000));
        assert_eq!(convert_percent_to_quota(100.0), (100_000, 100_000));

        // Multi-core limits
        assert_eq!(convert_percent_to_quota(200.0), (200_000, 100_000));
        assert_eq!(convert_percent_to_quota(400.0), (400_000, 100_000));
    }

    #[test]
    fn test_convert_quota_to_percent() {
        assert_eq!(convert_quota_to_percent(30_000, 100_000), 30.0);
        assert_eq!(convert_quota_to_percent(50_000, 100_000), 50.0);
        assert_eq!(convert_quota_to_percent(200_000, 100_000), 200.0);

        // Edge case: period = 0
        assert_eq!(convert_quota_to_percent(50_000, 0), 0.0);
    }

    #[test]
    fn test_parse_cpu_stat() {
        let content = r#"usage_usec 1234567890
user_usec 1000000
system_usec 234567890
nr_periods 1000
nr_throttled 100
throttled_usec 50000000
nr_bursts 0
burst_usec 0"#;

        let values = parse_cpu_stat(content).unwrap();
        assert_eq!(values.usage_usec, 1234567890);
        assert_eq!(values.user_usec, 1000000);
        assert_eq!(values.system_usec, 234567890);
        assert_eq!(values.nr_periods, 1000);
        assert_eq!(values.nr_throttled, 100);
        assert_eq!(values.throttled_usec, 50000000);
    }

    #[test]
    fn test_cpu_stat_throttle_percentage() {
        let values = CpuStatValues {
            nr_periods: 1000,
            nr_throttled: 100,
            ..Default::default()
        };

        assert_eq!(values.throttle_percentage(), 10.0);

        let values = CpuStatValues {
            nr_periods: 0,
            nr_throttled: 0,
            ..Default::default()
        };

        assert_eq!(values.throttle_percentage(), 0.0);
    }

    #[test]
    fn test_parse_memory_stat() {
        let content = r#"anon 1073741824
file 536870912
kernel_stack 65536
slab 131072
sock 8192
shmem 4096
file_mapped 268435456
file_dirty 1024
file_writeback 512"#;

        let values = parse_memory_stat(content).unwrap();
        assert_eq!(values.anon, 1073741824);
        assert_eq!(values.file, 536870912);
        assert_eq!(values.kernel_stack, 65536);
    }

    #[test]
    fn test_process_exists() {
        // PID 1 should always exist (init/systemd)
        assert!(process_exists(1));

        // Very high PID unlikely to exist
        assert!(!process_exists(9999999));
    }
}
