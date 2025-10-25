use crate::{Error, Result};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct SystemdService {
    service_name: String,
    last_restart_time: u64,
    min_restart_interval: u64,
}

impl SystemdService {
    /// Create a new instance for managing a systemd service
    pub fn new(name: &str) -> Self {
        Self {
            service_name: name.to_string(),
            last_restart_time: 0,
            min_restart_interval: 100,
        }
    }

    /// Get current UNIX timestamp in seconds
    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs()
    }

    /// Check if enough time has passed since the last restart
    fn can_restart(&self) -> bool {
        // First restart - always allow
        if self.last_restart_time == 0 {
            return true;
        }

        let current_time = Self::current_timestamp();
        let time_since_last = current_time - self.last_restart_time;

        // Check if minimum interval has passed
        time_since_last >= self.min_restart_interval
    }

    /// Execute systemd daemon-reload
    fn daemon_reload(&self) -> Result<()> {
        let output = Command::new("sudo")
            .arg("systemctl")
            .arg("daemon-reload")
            .output()
            .map_err(|e| Error::Systemd(format!("Failed to execute daemon-reload: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::Systemd(format!("daemon-reload failed: {}", stderr)));
        }

        Ok(())
    }

    /// Restart the systemd service
    fn restart_service(&self) -> Result<()> {
        let output = Command::new("sudo")
            .arg("systemctl")
            .arg("restart")
            .arg(&self.service_name)
            .output()
            .map_err(|e| Error::Systemd(format!("Failed to execute restart: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::Systemd(format!(
                "restart {} failed: {}",
                self.service_name, stderr
            )));
        }

        Ok(())
    }

    /// Проверить, активна ли служба
    pub fn is_active(&self) -> Result<bool> {
        let output = Command::new("systemctl")
            .arg("is-active")
            .arg(&self.service_name)
            .output()
            .map_err(|e| Error::Systemd(format!("Failed to check service status: {}", e)))?;

        Ok(output.status.success())
    }

    /// Полный перезапуск с daemon-reload
    pub fn restart_with_reload(&mut self) -> Result<()> {
        // Проверка минимального интервала
        if !self.can_restart() {
            let current_time = Self::current_timestamp();
            let time_since_last = current_time - self.last_restart_time;
            return Err(Error::Systemd(format!(
                "Too soon to restart. Only {} seconds passed (minimum: {})",
                time_since_last, self.min_restart_interval
            )));
        }

        // Reload конфигурации
        self.daemon_reload()?;

        // Рестарт службы
        self.restart_service()?;

        // Обновить timestamp
        self.last_restart_time = Self::current_timestamp();

        Ok(())
    }

    /// Получить свойства службы (CPUQuota, MemoryMax, Nice)
    pub fn get_properties(&self) -> Result<String> {
        let output = Command::new("systemctl")
            .arg("show")
            .arg(&self.service_name)
            .arg("--property=CPUQuota,MemoryMax,Nice")
            .output()
            .map_err(|e| Error::Systemd(format!("Failed to get properties: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::Systemd(format!(
                "Failed to get properties: {}",
                stderr
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Время с последнего рестарта (в секундах)
    pub fn time_since_last_restart(&self) -> u64 {
        if self.last_restart_time == 0 {
            return u64::MAX; // Никогда не перезапускался
        }
        let current_time = Self::current_timestamp();
        current_time - self.last_restart_time
    }

    // Методы для интеграционных тестов
    // NOTE: Эти методы публичные для возможности тестирования,
    // но не должны использоваться в production коде

    /// Получить timestamp последнего рестарта
    ///
    /// ⚠️ ТОЛЬКО ДЛЯ ТЕСТИРОВАНИЯ
    #[doc(hidden)]
    pub fn get_last_restart_time(&self) -> u64 {
        self.last_restart_time
    }

    /// Установить timestamp последнего рестарта
    ///
    /// ⚠️ ТОЛЬКО ДЛЯ ТЕСТИРОВАНИЯ
    #[doc(hidden)]
    pub fn set_last_restart_time(&mut self, timestamp: u64) {
        self.last_restart_time = timestamp;
    }

    /// Установить минимальный интервал
    ///
    /// ⚠️ ТОЛЬКО ДЛЯ ТЕСТИРОВАНИЯ
    #[doc(hidden)]
    pub fn set_min_restart_interval(&mut self, interval: u64) {
        self.min_restart_interval = interval;
    }

    /// Получить текущий timestamp
    ///
    /// ⚠️ ТОЛЬКО ДЛЯ ТЕСТИРОВАНИЯ
    #[doc(hidden)]
    pub fn current_timestamp_public() -> u64 {
        Self::current_timestamp()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_systemd_service_creation() {
        let service = SystemdService::new("test-service");
        assert_eq!(service.service_name, "test-service");
        assert_eq!(service.last_restart_time, 0);
        assert_eq!(service.min_restart_interval, 100);
    }

    #[test]
    fn test_can_restart_first_time() {
        let service = SystemdService::new("test");
        // First restart should always be allowed
        assert!(service.can_restart());
    }

    #[test]
    fn test_time_since_last_restart_never_restarted() {
        let service = SystemdService::new("test");
        // Never restarted should return u64::MAX
        assert_eq!(service.time_since_last_restart(), u64::MAX);
    }

    #[test]
    fn test_time_since_last_restart_with_value() {
        let mut service = SystemdService::new("test");

        // Simulate restart at timestamp 1000
        service.last_restart_time = 1000;

        // Current time will be greater, so we should get a positive value
        let time_since = service.time_since_last_restart();
        assert!(time_since > 0);
        assert!(time_since < u64::MAX);
    }

    #[test]
    fn test_current_timestamp() {
        let ts1 = SystemdService::current_timestamp();
        std::thread::sleep(std::time::Duration::from_secs(1));
        let ts2 = SystemdService::current_timestamp();

        // Second timestamp should be greater (after 1 second sleep)
        assert!(ts2 > ts1);
        // Difference should be at least 1 second
        assert!(ts2 >= ts1 + 1);
    }

    // Note: Integration tests for actual systemd operations
    // (restart_with_reload, is_active, etc.) should be in tests/ directory
    // and run only on systems with systemd and appropriate permissions.
}
