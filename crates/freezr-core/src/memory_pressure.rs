use crate::error::{Error, Result};
use std::fs;

/// Memory pressure metrics from PSI (Pressure Stall Information)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MemoryPressure {
    /// "some" metric: percentage of time at least one process is waiting for memory
    pub some_avg10: f64,
    pub some_avg60: f64,
    pub some_avg300: f64,
    pub some_total: u64,

    /// "full" metric: percentage of time ALL processes are waiting for memory
    pub full_avg10: f64,
    pub full_avg60: f64,
    pub full_avg300: f64,
    pub full_total: u64,
}

impl MemoryPressure {
    /// Read memory pressure from /proc/pressure/memory
    ///
    /// Format:
    /// ```text
    /// some avg10=0.00 avg60=0.00 avg300=0.00 total=634678
    /// full avg10=0.00 avg60=0.00 avg300=0.00 total=583219
    /// ```
    pub fn read() -> Result<Self> {
        let content = fs::read_to_string("/proc/pressure/memory")
            .map_err(|e| Error::Other(format!("Failed to read /proc/pressure/memory: {}", e)))?;

        Self::parse(&content)
    }

    /// Parse PSI format
    fn parse(content: &str) -> Result<Self> {
        let lines: Vec<&str> = content.lines().collect();
        if lines.len() < 2 {
            return Err(Error::Parse(
                "Invalid PSI format: expected 2 lines".to_string(),
            ));
        }

        let some_line = lines[0];
        let full_line = lines[1];

        let some = Self::parse_line(some_line, "some")?;
        let full = Self::parse_line(full_line, "full")?;

        Ok(Self {
            some_avg10: some.0,
            some_avg60: some.1,
            some_avg300: some.2,
            some_total: some.3,
            full_avg10: full.0,
            full_avg60: full.1,
            full_avg300: full.2,
            full_total: full.3,
        })
    }

    /// Parse single line: "some avg10=0.00 avg60=0.00 avg300=0.00 total=634678"
    /// Returns: (avg10, avg60, avg300, total)
    fn parse_line(line: &str, expected_prefix: &str) -> Result<(f64, f64, f64, u64)> {
        if !line.starts_with(expected_prefix) {
            return Err(Error::Parse(format!(
                "Line should start with '{}', got: {}",
                expected_prefix, line
            )));
        }

        // Split by whitespace and parse key=value pairs
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() != 5 {
            return Err(Error::Parse(format!(
                "Expected 5 parts, got {}: {}",
                parts.len(),
                line
            )));
        }

        let avg10 = Self::parse_value(parts[1], "avg10=")?;
        let avg60 = Self::parse_value(parts[2], "avg60=")?;
        let avg300 = Self::parse_value(parts[3], "avg300=")?;
        let total = Self::parse_int_value(parts[4], "total=")?;

        Ok((avg10, avg60, avg300, total))
    }

    /// Parse "key=value" to f64
    fn parse_value(part: &str, expected_key: &str) -> Result<f64> {
        if !part.starts_with(expected_key) {
            return Err(Error::Parse(format!(
                "Expected key '{}', got: {}",
                expected_key, part
            )));
        }

        let value_str = &part[expected_key.len()..];
        value_str
            .parse::<f64>()
            .map_err(|e| Error::Parse(format!("Failed to parse float '{}': {}", value_str, e)))
    }

    /// Parse "key=value" to u64
    fn parse_int_value(part: &str, expected_key: &str) -> Result<u64> {
        if !part.starts_with(expected_key) {
            return Err(Error::Parse(format!(
                "Expected key '{}', got: {}",
                expected_key, part
            )));
        }

        let value_str = &part[expected_key.len()..];
        value_str
            .parse::<u64>()
            .map_err(|e| Error::Parse(format!("Failed to parse int '{}': {}", value_str, e)))
    }

    /// Check if memory pressure is at warning level
    pub fn is_warning(&self, some_threshold: f64, full_threshold: f64) -> bool {
        self.some_avg10 >= some_threshold || self.full_avg10 >= full_threshold
    }

    /// Check if memory pressure is at critical level
    pub fn is_critical(&self, some_threshold: f64, full_threshold: f64) -> bool {
        self.some_avg10 >= some_threshold || self.full_avg10 >= full_threshold
    }

    /// Get human-readable status
    pub fn status(&self) -> &'static str {
        if self.full_avg10 > 0.0 {
            "CRITICAL" // Full stall = all processes blocked
        } else if self.some_avg10 > 10.0 {
            "HIGH"
        } else if self.some_avg10 > 5.0 {
            "MEDIUM"
        } else if self.some_avg10 > 0.0 {
            "LOW"
        } else {
            "NONE"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_psi_format() {
        let content = "some avg10=0.00 avg60=0.00 avg300=0.00 total=634678\n\
                       full avg10=0.00 avg60=0.00 avg300=0.00 total=583219\n";

        let pressure = MemoryPressure::parse(content).unwrap();
        assert_eq!(pressure.some_avg10, 0.0);
        assert_eq!(pressure.some_total, 634678);
        assert_eq!(pressure.full_avg10, 0.0);
        assert_eq!(pressure.full_total, 583219);
    }

    #[test]
    fn test_parse_with_pressure() {
        let content = "some avg10=12.50 avg60=8.32 avg300=5.12 total=1234567\n\
                       full avg10=3.21 avg60=2.11 avg300=1.05 total=654321\n";

        let pressure = MemoryPressure::parse(content).unwrap();
        assert_eq!(pressure.some_avg10, 12.50);
        assert_eq!(pressure.some_avg60, 8.32);
        assert_eq!(pressure.full_avg10, 3.21);
        assert!((pressure.full_avg300 - 1.05).abs() < 0.001);
    }

    #[test]
    fn test_is_warning() {
        let pressure = MemoryPressure {
            some_avg10: 15.0,
            some_avg60: 10.0,
            some_avg300: 5.0,
            some_total: 1000,
            full_avg10: 2.0,
            full_avg60: 1.0,
            full_avg300: 0.5,
            full_total: 500,
        };

        assert!(pressure.is_warning(10.0, 5.0)); // some_avg10 (15.0) > 10.0
        assert!(!pressure.is_warning(20.0, 5.0)); // some_avg10 (15.0) < 20.0
    }

    #[test]
    fn test_status() {
        let no_pressure = MemoryPressure {
            some_avg10: 0.0,
            some_avg60: 0.0,
            some_avg300: 0.0,
            some_total: 0,
            full_avg10: 0.0,
            full_avg60: 0.0,
            full_avg300: 0.0,
            full_total: 0,
        };
        assert_eq!(no_pressure.status(), "NONE");

        let critical_pressure = MemoryPressure {
            some_avg10: 50.0,
            some_avg60: 40.0,
            some_avg300: 30.0,
            some_total: 10000,
            full_avg10: 10.0, // Full stall!
            full_avg60: 8.0,
            full_avg300: 5.0,
            full_total: 5000,
        };
        assert_eq!(critical_pressure.status(), "CRITICAL");
    }
}
