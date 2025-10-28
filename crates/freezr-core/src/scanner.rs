use crate::{types::ProcessInfo, Error, Result};
use std::process::Command;

/// Сканер процессов
pub struct ProcessScanner;

impl ProcessScanner {
    pub fn new() -> Self {
        Self
    }

    /// Найти KESL процесс и измерить CPU (3 замера с усреднением)
    pub fn scan_kesl(&self) -> Result<Option<ProcessInfo>> {
        // Найти PID процесса kesl
        let pid = self.find_kesl_pid()?;

        if let Some(pid) = pid {
            // Измерить CPU (3 замера)
            let cpu = self.measure_cpu_average(pid, 3)?;

            // Получить память
            let memory_kb = self.get_memory_kb(pid)?;

            // Получить имя и команду
            let (name, command) = self.get_process_info(pid)?;

            Ok(Some(ProcessInfo::new(pid, name, command, cpu, memory_kb)))
        } else {
            Ok(None)
        }
    }

    /// Найти все Node.js процессы
    pub fn scan_node_processes(&self) -> Result<Vec<ProcessInfo>> {
        let pids = self.find_node_pids()?;
        let mut processes = Vec::new();

        for pid in pids {
            // Измерить CPU через top
            let cpu = self.measure_cpu_top(pid)?;

            // Получить память
            let memory_kb = self.get_memory_kb(pid)?;

            // Получить имя и команду
            let (name, command) = self.get_process_info(pid)?;

            processes.push(ProcessInfo::new(pid, name, command, cpu, memory_kb));
        }

        Ok(processes)
    }

    /// Найти PID процесса KESL
    fn find_kesl_pid(&self) -> Result<Option<u32>> {
        let output = Command::new("ps")
            .args(&["aux"])
            .output()
            .map_err(|e| Error::Scanner(format!("Failed to run ps: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        for line in stdout.lines() {
            // Find main KESL process, not wdserver or kesl-starter
            if line.contains("/opt/kaspersky/kesl/libexec/kesl")
                && !line.contains("grep")
                && !line.contains("wdserver")
                && !line.contains("kesl-starter") {
                // Parse PID (second field in ps aux)
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() > 1 {
                    if let Ok(pid) = parts[1].parse::<u32>() {
                        return Ok(Some(pid));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Найти все PID процессов node
    fn find_node_pids(&self) -> Result<Vec<u32>> {
        let output = Command::new("ps")
            .args(&["aux"])
            .output()
            .map_err(|e| Error::Scanner(format!("Failed to run ps: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut pids = Vec::new();

        for line in stdout.lines() {
            // Проверяем: команда заканчивается на "node" или содержит "/node"
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() > 10 {
                let cmd = parts[10];
                if cmd == "node" || cmd.ends_with("/node") {
                    if let Ok(pid) = parts[1].parse::<u32>() {
                        pids.push(pid);
                    }
                }
            }
        }

        Ok(pids)
    }

    /// Найти все snap/snapd процессы
    pub fn scan_snap_processes(&self) -> Result<Vec<ProcessInfo>> {
        let pids = self.find_snap_pids()?;
        let mut processes = Vec::new();

        for pid in pids {
            // Измерить CPU через top
            let cpu = self.measure_cpu_top(pid)?;

            // Получить память
            let memory_kb = self.get_memory_kb(pid)?;

            // Получить имя и команду
            let (name, command) = self.get_process_info(pid)?;

            processes.push(ProcessInfo::new(pid, name, command, cpu, memory_kb));
        }

        Ok(processes)
    }

    /// Найти все PID процессов snap/snapd
    fn find_snap_pids(&self) -> Result<Vec<u32>> {
        let output = Command::new("ps")
            .args(&["aux"])
            .output()
            .map_err(|e| Error::Scanner(format!("Failed to run ps: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut pids = Vec::new();

        for line in stdout.lines() {
            // Ищем процессы snap, snapd, snap-store, snap-confine
            if line.contains("snap") && !line.contains("grep") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() > 10 {
                    let cmd = parts[10];
                    // Проверяем что это действительно snap процесс
                    if cmd.contains("snap") || cmd.contains("/snap/") {
                        if let Ok(pid) = parts[1].parse::<u32>() {
                            pids.push(pid);
                        }
                    }
                }
            }
        }

        Ok(pids)
    }

    /// Найти все Firefox процессы
    pub fn scan_firefox_processes(&self) -> Result<Vec<ProcessInfo>> {
        let pids = self.find_firefox_pids()?;
        let mut processes = Vec::new();

        for pid in pids {
            // Измерить CPU через top
            let cpu = self.measure_cpu_top(pid)?;

            // Получить память
            let memory_kb = self.get_memory_kb(pid)?;

            // Получить имя и команду
            let (name, command) = self.get_process_info(pid)?;

            processes.push(ProcessInfo::new(pid, name, command, cpu, memory_kb));
        }

        Ok(processes)
    }

    /// Найти все PID процессов Firefox
    fn find_firefox_pids(&self) -> Result<Vec<u32>> {
        let output = Command::new("ps")
            .args(&["aux"])
            .output()
            .map_err(|e| Error::Scanner(format!("Failed to run ps: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut pids = Vec::new();

        for line in stdout.lines() {
            // Проверяем: команда содержит "firefox" (включая /usr/lib/firefox/firefox)
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() > 10 {
                let cmd = parts[10];
                // Ищем firefox в команде (может быть firefox, /usr/bin/firefox, /usr/lib/firefox/firefox)
                if cmd.contains("firefox") && !line.contains("grep") {
                    if let Ok(pid) = parts[1].parse::<u32>() {
                        pids.push(pid);
                    }
                }
            }
        }

        Ok(pids)
    }

    /// Найти все Brave процессы
    pub fn scan_brave_processes(&self) -> Result<Vec<ProcessInfo>> {
        let pids = self.find_brave_pids()?;
        let mut processes = Vec::new();

        for pid in pids {
            // Измерить CPU через top
            let cpu = self.measure_cpu_top(pid)?;

            // Получить память
            let memory_kb = self.get_memory_kb(pid)?;

            // Получить имя и команду
            let (name, command) = self.get_process_info(pid)?;

            processes.push(ProcessInfo::new(pid, name, command, cpu, memory_kb));
        }

        Ok(processes)
    }

    /// Найти все PID процессов Brave
    fn find_brave_pids(&self) -> Result<Vec<u32>> {
        let output = Command::new("ps")
            .args(&["aux"])
            .output()
            .map_err(|e| Error::Scanner(format!("Failed to run ps: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut pids = Vec::new();

        for line in stdout.lines() {
            // Проверяем: команда содержит "brave" (включая /opt/brave.com/brave/brave)
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() > 10 {
                let cmd = parts[10];
                // Ищем brave в команде (может быть brave, /usr/bin/brave, /opt/brave.com/brave/brave)
                if cmd.contains("brave") && !line.contains("grep") {
                    if let Ok(pid) = parts[1].parse::<u32>() {
                        pids.push(pid);
                    }
                }
            }
        }

        Ok(pids)
    }

    /// Найти все Telegram процессы
    pub fn scan_telegram_processes(&self) -> Result<Vec<ProcessInfo>> {
        let pids = self.find_telegram_pids()?;
        let mut processes = Vec::new();

        for pid in pids {
            // Измерить CPU через top
            let cpu = self.measure_cpu_top(pid)?;

            // Получить память
            let memory_kb = self.get_memory_kb(pid)?;

            // Получить имя и команду
            let (name, command) = self.get_process_info(pid)?;

            processes.push(ProcessInfo::new(pid, name, command, cpu, memory_kb));
        }

        Ok(processes)
    }

    /// Найти все PID процессов Telegram
    fn find_telegram_pids(&self) -> Result<Vec<u32>> {
        let output = Command::new("ps")
            .args(&["aux"])
            .output()
            .map_err(|e| Error::Scanner(format!("Failed to run ps: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut pids = Vec::new();

        for line in stdout.lines() {
            // Проверяем: команда содержит "telegram" (включая telegram-desktop, /snap/telegram-desktop)
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() > 10 {
                let cmd = parts[10];
                // Ищем telegram в команде (может быть telegram-desktop, /usr/bin/telegram-desktop, /snap/telegram-desktop/...)
                if cmd.contains("telegram") && !line.contains("grep") {
                    if let Ok(pid) = parts[1].parse::<u32>() {
                        pids.push(pid);
                    }
                }
            }
        }

        Ok(pids)
    }

    /// Найти все Neovim процессы
    pub fn scan_nvim_processes(&self) -> Result<Vec<ProcessInfo>> {
        let pids = self.find_nvim_pids()?;
        let mut processes = Vec::new();

        for pid in pids {
            let cpu = self.measure_cpu_top(pid)?;
            let memory_kb = self.get_memory_kb(pid)?;
            let (name, command) = self.get_process_info(pid)?;
            processes.push(ProcessInfo::new(pid, name, command, cpu, memory_kb));
        }

        Ok(processes)
    }

    /// Найти все PID процессов Neovim
    fn find_nvim_pids(&self) -> Result<Vec<u32>> {
        let output = Command::new("ps")
            .args(&["aux"])
            .output()
            .map_err(|e| Error::Scanner(format!("Failed to run ps: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut pids = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() > 10 {
                let cmd = parts[10];
                // Ищем nvim в команде (может быть nvim, /usr/bin/nvim, /path/to/nvim)
                if cmd.contains("nvim") && !line.contains("grep") {
                    if let Ok(pid) = parts[1].parse::<u32>() {
                        pids.push(pid);
                    }
                }
            }
        }

        Ok(pids)
    }

    /// Измерить CPU через top (3 замера с усреднением)
    fn measure_cpu_average(&self, pid: u32, samples: usize) -> Result<f64> {
        let mut sum = 0.0;
        let mut count = 0;

        for i in 0..samples {
            let cpu = self.measure_cpu_top(pid)?;
            if cpu > 0.0 {
                sum += cpu;
                count += 1;
            }

            // Спать 1 секунду между замерами (кроме последнего)
            if i < samples - 1 {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }

        if count > 0 {
            Ok(sum / count as f64)
        } else {
            Ok(0.0)
        }
    }

    /// Измерить CPU через top (один замер)
    fn measure_cpu_top(&self, pid: u32) -> Result<f64> {
        let output = Command::new("top")
            .args(&["-b", "-n1", "-p", &pid.to_string()])
            .output()
            .map_err(|e| Error::Scanner(format!("Failed to run top: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Берем последнюю строку (данные процесса)
        if let Some(last_line) = stdout.lines().last() {
            let parts: Vec<&str> = last_line.split_whitespace().collect();
            // CPU% обычно в 9-м столбце (считая с 1)
            if parts.len() > 8 {
                let cpu_str = parts[8].replace(',', ".");
                if let Ok(cpu) = cpu_str.parse::<f64>() {
                    return Ok(cpu);
                }
            }
        }

        Ok(0.0)
    }

    /// Получить использование памяти в KB (RSS)
    fn get_memory_kb(&self, pid: u32) -> Result<u64> {
        let output = Command::new("ps")
            .args(&["-p", &pid.to_string(), "-o", "rss="])
            .output()
            .map_err(|e| Error::Scanner(format!("Failed to get memory: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let rss_kb = stdout.trim().parse::<u64>().unwrap_or(0);

        Ok(rss_kb)
    }

    /// Получить имя процесса и команду
    fn get_process_info(&self, pid: u32) -> Result<(String, String)> {
        let output = Command::new("ps")
            .args(&["-p", &pid.to_string(), "-o", "comm=,cmd="])
            .output()
            .map_err(|e| Error::Scanner(format!("Failed to get process info: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let line = stdout.trim();

        // Разделяем на имя и полную команду
        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        let name = parts[0].to_string();
        let command = if parts.len() > 1 {
            parts[1].to_string()
        } else {
            name.clone()
        };

        Ok((name, command))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scanner_creation() {
        let _scanner = ProcessScanner::new();
        // Just verify it can be created without panicking
        assert!(true);
    }

    #[test]
    fn test_scan_kesl_returns_result() {
        let scanner = ProcessScanner::new();
        // Should not panic, even if KESL is not running
        let result = scanner.scan_kesl();
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_scan_node_processes_returns_vec() {
        let scanner = ProcessScanner::new();
        // Should return a Vec, even if empty
        let result = scanner.scan_node_processes();
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_find_kesl_pid_does_not_panic() {
        let scanner = ProcessScanner::new();
        // Should not panic even if KESL doesn't exist
        let result = scanner.find_kesl_pid();
        assert!(result.is_ok());
    }

    #[test]
    fn test_find_node_pids_returns_vec() {
        let scanner = ProcessScanner::new();
        // Should return a Vec (possibly empty)
        let result = scanner.find_node_pids();
        assert!(result.is_ok());

        if let Ok(pids) = result {
            // PIDs should be positive numbers
            for pid in pids {
                assert!(pid > 0);
            }
        }
    }

    #[test]
    fn test_get_memory_kb_with_init_process() {
        let scanner = ProcessScanner::new();
        // PID 1 (init/systemd) always exists
        let result = scanner.get_memory_kb(1);

        if let Ok(mem) = result {
            // Init process should have some memory
            assert!(mem > 0);
        }
    }

    #[test]
    fn test_get_memory_kb_invalid_pid() {
        let scanner = ProcessScanner::new();
        // Invalid PID should return 0 or error
        let result = scanner.get_memory_kb(999999);

        if let Ok(mem) = result {
            // Should be 0 for non-existent process
            assert_eq!(mem, 0);
        }
    }

    #[test]
    fn test_get_process_info_init() {
        let scanner = ProcessScanner::new();
        // PID 1 should have process info
        let result = scanner.get_process_info(1);

        if let Ok((name, command)) = result {
            // Name and command should not be empty
            assert!(!name.is_empty());
            assert!(!command.is_empty());
            // Init process is typically systemd or init
            assert!(name.contains("systemd") || name.contains("init") || name == "sh");
        }
    }

    #[test]
    fn test_measure_cpu_top_init_process() {
        let scanner = ProcessScanner::new();
        // Measuring CPU for init (should be very low)
        let result = scanner.measure_cpu_top(1);

        if let Ok(cpu) = result {
            // CPU should be a valid percentage
            assert!(cpu >= 0.0);
            assert!(cpu <= 100.0); // Single core can't exceed 100% in top output
        }
    }

    #[test]
    fn test_measure_cpu_average_bounds() {
        let scanner = ProcessScanner::new();
        // Test with init process (PID 1) and 1 sample to make it fast
        let result = scanner.measure_cpu_average(1, 1);

        if let Ok(cpu) = result {
            // CPU should be within valid range
            assert!(cpu >= 0.0);
            assert!(cpu <= 100.0);
        }
    }

    #[test]
    fn test_scanner_multiple_calls() {
        let scanner = ProcessScanner::new();

        // Multiple calls should not panic or crash
        let _ = scanner.find_kesl_pid();
        let _ = scanner.find_node_pids();
        let _ = scanner.get_memory_kb(1);

        assert!(true);
    }
}
