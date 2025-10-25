use serde::{Deserialize, Serialize};

/// Информация о процессе
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub command: String,
    pub cpu_percent: f64,
    pub memory_mb: u64,
    pub memory_kb: u64,
}

impl ProcessInfo {
    pub fn new(pid: u32, name: String, command: String, cpu_percent: f64, memory_kb: u64) -> Self {
        Self {
            pid,
            name,
            command,
            cpu_percent,
            memory_mb: memory_kb / 1024,
            memory_kb,
        }
    }

    // Проверка: это KESL процесс?
    pub fn is_kesl(&self) -> bool {
        self.command.contains("/opt/kaspersky/kesl/libexec/kesl")
    }

    // Проверка: это Node.js процесс?
    pub fn is_node(&self) -> bool {
        self.name == "node" || self.name.ends_with("/node") || self.command.contains("node ")
    }

    // CPU превышает порог?
    pub fn cpu_exceeds(&self, threshold: f64) -> bool {
        self.cpu_percent > threshold
    }

    // Память превышает порог?
    pub fn memory_exceeds(&self, threshold_mb: u64) -> bool {
        self.memory_mb > threshold_mb
    }
}

/// Статистика мониторинга
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MonitorStats {
    pub total_checks: u64,
    pub total_violations: u64,
    pub total_kills: u64,
    pub total_restarts: u64,
    pub cpu_violations: u32,
    pub memory_violations: u32,
    pub last_check_timestamp: u64,
}

impl MonitorStats {
    pub fn new() -> Self {
        Self::default()
    }

    // Сбросить счетчики нарушений (после рестарта)
    pub fn reset_violations(&mut self) {
        self.cpu_violations = 0;
        self.memory_violations = 0;
    }

    // Увеличить счетчик CPU нарушений
    pub fn increment_cpu_violation(&mut self) {
        self.cpu_violations += 1;
        self.total_violations += 1;
    }

    // Увеличить счетчик памяти нарушений
    pub fn increment_memory_violation(&mut self) {
        self.memory_violations += 1;
        self.total_violations += 1;
    }

    // Зафиксировать kill процесса
    pub fn record_kill(&mut self) {
        self.total_kills += 1;
    }

    // Зафиксировать рестарт службы
    pub fn record_restart(&mut self) {
        self.total_restarts += 1;
        self.reset_violations();
    }

    // Increment total checks counter
    pub fn increment_checks(&mut self) {
        self.total_checks += 1;
    }

    // Обновить время последней проверки
    pub fn update_check_time(&mut self, timestamp: u64) {
        self.last_check_timestamp = timestamp;
        self.total_checks += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== ProcessInfo Tests =====

    #[test]
    fn test_process_info_creation() {
        let proc = ProcessInfo::new(
            1234,
            "test".to_string(),
            "test command".to_string(),
            50.5,
            2048,
        );

        assert_eq!(proc.pid, 1234);
        assert_eq!(proc.name, "test");
        assert_eq!(proc.command, "test command");
        assert_eq!(proc.cpu_percent, 50.5);
        assert_eq!(proc.memory_kb, 2048);
        assert_eq!(proc.memory_mb, 2); // 2048 KB / 1024 = 2 MB
    }

    #[test]
    fn test_process_info_memory_conversion() {
        let proc = ProcessInfo::new(1, "test".to_string(), "cmd".to_string(), 0.0, 1024);
        assert_eq!(proc.memory_mb, 1);

        let proc2 = ProcessInfo::new(1, "test".to_string(), "cmd".to_string(), 0.0, 2560);
        assert_eq!(proc2.memory_mb, 2); // 2560 KB / 1024 = 2 MB (integer division)
    }

    #[test]
    fn test_is_kesl() {
        let kesl_proc = ProcessInfo::new(
            1,
            "kesl".to_string(),
            "/opt/kaspersky/kesl/libexec/kesl --config /etc/kesl.conf".to_string(),
            10.0,
            1024,
        );
        assert!(kesl_proc.is_kesl());

        let other_proc = ProcessInfo::new(
            2,
            "chrome".to_string(),
            "/usr/bin/chrome".to_string(),
            20.0,
            2048,
        );
        assert!(!other_proc.is_kesl());
    }

    #[test]
    fn test_is_node() {
        // Test exact match
        let node1 = ProcessInfo::new(
            1,
            "node".to_string(),
            "node server.js".to_string(),
            0.0,
            1024,
        );
        assert!(node1.is_node());

        // Test path ending with /node
        let node2 = ProcessInfo::new(
            2,
            "/usr/bin/node".to_string(),
            "/usr/bin/node app.js".to_string(),
            0.0,
            1024,
        );
        assert!(node2.is_node());

        // Test command containing "node "
        let node3 = ProcessInfo::new(
            3,
            "npm".to_string(),
            "npm start node script.js".to_string(),
            0.0,
            1024,
        );
        assert!(node3.is_node());

        // Test non-node process
        let other = ProcessInfo::new(
            4,
            "python".to_string(),
            "python script.py".to_string(),
            0.0,
            1024,
        );
        assert!(!other.is_node());
    }

    #[test]
    fn test_cpu_exceeds() {
        let proc = ProcessInfo::new(1, "test".to_string(), "cmd".to_string(), 85.5, 1024);

        assert!(proc.cpu_exceeds(80.0));
        assert!(proc.cpu_exceeds(85.0));
        assert!(!proc.cpu_exceeds(90.0));
        assert!(!proc.cpu_exceeds(85.5)); // Equal is not exceeding
    }

    #[test]
    fn test_memory_exceeds() {
        let proc = ProcessInfo::new(1, "test".to_string(), "cmd".to_string(), 0.0, 1024 * 512); // 512 MB

        assert!(proc.memory_exceeds(500));
        assert!(!proc.memory_exceeds(512)); // Equal is not exceeding
        assert!(!proc.memory_exceeds(600));
    }

    #[test]
    fn test_process_info_serialization() {
        let proc = ProcessInfo::new(
            1234,
            "test".to_string(),
            "test command".to_string(),
            50.5,
            2048,
        );

        // Test JSON serialization
        let json = serde_json::to_string(&proc).expect("Failed to serialize");
        assert!(json.contains("\"pid\":1234"));
        assert!(json.contains("\"name\":\"test\""));

        // Test JSON deserialization
        let deserialized: ProcessInfo = serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(deserialized.pid, proc.pid);
        assert_eq!(deserialized.name, proc.name);
    }

    // ===== MonitorStats Tests =====

    #[test]
    fn test_monitor_stats_creation() {
        let stats = MonitorStats::new();

        assert_eq!(stats.total_checks, 0);
        assert_eq!(stats.total_violations, 0);
        assert_eq!(stats.total_kills, 0);
        assert_eq!(stats.total_restarts, 0);
        assert_eq!(stats.cpu_violations, 0);
        assert_eq!(stats.memory_violations, 0);
        assert_eq!(stats.last_check_timestamp, 0);
    }

    #[test]
    fn test_increment_cpu_violation() {
        let mut stats = MonitorStats::new();

        stats.increment_cpu_violation();
        assert_eq!(stats.cpu_violations, 1);
        assert_eq!(stats.total_violations, 1);

        stats.increment_cpu_violation();
        assert_eq!(stats.cpu_violations, 2);
        assert_eq!(stats.total_violations, 2);
    }

    #[test]
    fn test_increment_memory_violation() {
        let mut stats = MonitorStats::new();

        stats.increment_memory_violation();
        assert_eq!(stats.memory_violations, 1);
        assert_eq!(stats.total_violations, 1);

        stats.increment_memory_violation();
        assert_eq!(stats.memory_violations, 2);
        assert_eq!(stats.total_violations, 2);
    }

    #[test]
    fn test_mixed_violations() {
        let mut stats = MonitorStats::new();

        stats.increment_cpu_violation();
        stats.increment_memory_violation();
        stats.increment_cpu_violation();

        assert_eq!(stats.cpu_violations, 2);
        assert_eq!(stats.memory_violations, 1);
        assert_eq!(stats.total_violations, 3);
    }

    #[test]
    fn test_reset_violations() {
        let mut stats = MonitorStats::new();

        stats.increment_cpu_violation();
        stats.increment_memory_violation();
        stats.increment_cpu_violation();

        assert_eq!(stats.cpu_violations, 2);
        assert_eq!(stats.memory_violations, 1);

        stats.reset_violations();

        assert_eq!(stats.cpu_violations, 0);
        assert_eq!(stats.memory_violations, 0);
        assert_eq!(stats.total_violations, 3); // Total violations NOT reset
    }

    #[test]
    fn test_record_kill() {
        let mut stats = MonitorStats::new();

        stats.record_kill();
        assert_eq!(stats.total_kills, 1);

        stats.record_kill();
        assert_eq!(stats.total_kills, 2);
    }

    #[test]
    fn test_record_restart() {
        let mut stats = MonitorStats::new();

        // Create some violations
        stats.increment_cpu_violation();
        stats.increment_memory_violation();
        assert_eq!(stats.cpu_violations, 1);
        assert_eq!(stats.memory_violations, 1);

        // Record restart - should reset violations
        stats.record_restart();

        assert_eq!(stats.total_restarts, 1);
        assert_eq!(stats.cpu_violations, 0);
        assert_eq!(stats.memory_violations, 0);
        assert_eq!(stats.total_violations, 2); // Total preserved

        // Second restart
        stats.increment_cpu_violation();
        stats.record_restart();

        assert_eq!(stats.total_restarts, 2);
        assert_eq!(stats.cpu_violations, 0);
        assert_eq!(stats.total_violations, 3);
    }

    #[test]
    fn test_update_check_time() {
        let mut stats = MonitorStats::new();

        stats.update_check_time(1000);
        assert_eq!(stats.last_check_timestamp, 1000);
        assert_eq!(stats.total_checks, 1);

        stats.update_check_time(2000);
        assert_eq!(stats.last_check_timestamp, 2000);
        assert_eq!(stats.total_checks, 2);
    }

    #[test]
    fn test_monitor_stats_complete_workflow() {
        let mut stats = MonitorStats::new();

        // Monitoring cycle 1
        stats.update_check_time(1000);
        stats.increment_cpu_violation();
        assert_eq!(stats.total_checks, 1);
        assert_eq!(stats.cpu_violations, 1);

        // Monitoring cycle 2
        stats.update_check_time(2000);
        stats.increment_cpu_violation();
        assert_eq!(stats.total_checks, 2);
        assert_eq!(stats.cpu_violations, 2);

        // Monitoring cycle 3 - threshold reached, restart
        stats.update_check_time(3000);
        stats.increment_cpu_violation();
        stats.record_restart();
        assert_eq!(stats.total_checks, 3);
        assert_eq!(stats.cpu_violations, 0); // Reset after restart
        assert_eq!(stats.total_violations, 3); // History preserved
        assert_eq!(stats.total_restarts, 1);

        // Monitoring cycle 4 - new violation
        stats.update_check_time(4000);
        stats.increment_memory_violation();
        assert_eq!(stats.total_checks, 4);
        assert_eq!(stats.memory_violations, 1);
        assert_eq!(stats.total_violations, 4);
    }

    #[test]
    fn test_monitor_stats_serialization() {
        let mut stats = MonitorStats::new();
        stats.increment_cpu_violation();
        stats.update_check_time(1000);

        // Test JSON serialization
        let json = serde_json::to_string(&stats).expect("Failed to serialize");
        assert!(json.contains("\"total_checks\":1"));
        assert!(json.contains("\"cpu_violations\":1"));

        // Test JSON deserialization
        let deserialized: MonitorStats =
            serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(deserialized.total_checks, stats.total_checks);
        assert_eq!(deserialized.cpu_violations, stats.cpu_violations);
    }
}
