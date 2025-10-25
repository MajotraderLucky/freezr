use crate::{Error, Result};
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use std::thread;
use std::time::Duration;

/// Process action executor
pub struct ProcessExecutor;

impl ProcessExecutor {
    pub fn new() -> Self {
        Self
    }

    /// Kill a process (SIGTERM, then SIGKILL if needed)
    ///
    /// Algorithm:
    /// 1. Check if process exists
    /// 2. Send SIGTERM (graceful termination)
    /// 3. Wait 2 seconds
    /// 4. If process still alive, send SIGKILL (forced termination)
    pub fn kill_process(pid: u32) -> Result<()> {
        let process_pid = Pid::from_raw(pid as i32);

        // Check if process exists (send signal 0)
        if !Self::process_exists(pid)? {
            return Err(Error::Executor(format!(
                "Process {} does not exist",
                pid
            )));
        }

        // Step 1: Try graceful termination (SIGTERM)
        kill(process_pid, Signal::SIGTERM).map_err(|e| {
            Error::Executor(format!("Failed to send SIGTERM to process {}: {}", pid, e))
        })?;

        // Wait 2 seconds for termination
        thread::sleep(Duration::from_secs(2));

        // Check if process terminated
        if !Self::process_exists(pid)? {
            // Process successfully terminated after SIGTERM
            return Ok(());
        }

        // Step 2: Force termination (SIGKILL)
        kill(process_pid, Signal::SIGKILL).map_err(|e| {
            Error::Executor(format!("Failed to send SIGKILL to process {}: {}", pid, e))
        })?;

        // Final check
        thread::sleep(Duration::from_millis(500));

        if Self::process_exists(pid)? {
            return Err(Error::Executor(format!(
                "Failed to kill process {} even with SIGKILL",
                pid
            )));
        }

        Ok(())
    }

    /// Check if process exists
    ///
    /// Uses kill(pid, 0) - doesn't kill the process, only checks existence
    fn process_exists(pid: u32) -> Result<bool> {
        let process_pid = Pid::from_raw(pid as i32);

        match kill(process_pid, None) {
            Ok(_) => Ok(true), // Process exists
            Err(nix::errno::Errno::ESRCH) => Ok(false), // Process not found
            Err(nix::errno::Errno::EPERM) => Ok(true), // No permission, but process exists
            Err(e) => Err(Error::Executor(format!(
                "Failed to check process {}: {}",
                pid, e
            ))),
        }
    }

    /// Freeze process (SIGSTOP)
    ///
    /// For future use in FreezR - temporary process suspension
    pub fn freeze_process(pid: u32) -> Result<()> {
        let process_pid = Pid::from_raw(pid as i32);

        if !Self::process_exists(pid)? {
            return Err(Error::Executor(format!(
                "Process {} does not exist",
                pid
            )));
        }

        kill(process_pid, Signal::SIGSTOP).map_err(|e| {
            Error::Executor(format!("Failed to freeze process {}: {}", pid, e))
        })?;

        Ok(())
    }

    /// Unfreeze process (SIGCONT)
    ///
    /// For future use in FreezR - resume suspended process
    pub fn unfreeze_process(pid: u32) -> Result<()> {
        let process_pid = Pid::from_raw(pid as i32);

        if !Self::process_exists(pid)? {
            return Err(Error::Executor(format!(
                "Process {} does not exist",
                pid
            )));
        }

        kill(process_pid, Signal::SIGCONT).map_err(|e| {
            Error::Executor(format!("Failed to unfreeze process {}: {}", pid, e))
        })?;

        Ok(())
    }

    /// Set process nice level (priority)
    ///
    /// Nice values: -20 (highest priority) to 19 (lowest priority)
    /// Higher nice = lower priority
    pub fn renice_process(pid: u32, nice_level: i32) -> Result<()> {
        use std::process::Command;

        if !Self::process_exists(pid)? {
            return Err(Error::Executor(format!(
                "Process {} does not exist",
                pid
            )));
        }

        // Validate nice level
        if nice_level < -20 || nice_level > 19 {
            return Err(Error::Executor(format!(
                "Invalid nice level {}, must be -20 to 19",
                nice_level
            )));
        }

        // Use renice command to change priority
        let output = Command::new("sudo")
            .arg("renice")
            .arg("-n")
            .arg(nice_level.to_string())
            .arg("-p")
            .arg(pid.to_string())
            .output()
            .map_err(|e| Error::Executor(format!("Failed to run renice: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::Executor(format!(
                "Failed to renice process {}: {}",
                pid, stderr
            )));
        }

        Ok(())
    }
}

// Unit tests
#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn test_executor_creation() {
        let _executor = ProcessExecutor::new();
        assert!(true);
    }

    #[test]
    fn test_process_exists_invalid_pid() {
        // PID 999999 most likely doesn't exist
        let exists = ProcessExecutor::process_exists(999999).unwrap_or(true);
        assert!(!exists);
    }

    #[test]
    fn test_process_exists_init_process() {
        // PID 1 (init/systemd) always exists
        let exists = ProcessExecutor::process_exists(1).unwrap_or(false);
        assert!(exists);
    }

    #[test]
    #[ignore] // Requires spawning test process
    fn test_kill_process_workflow() {
        // Spawn test process (sleep 60)
        let child = Command::new("sleep")
            .arg("60")
            .spawn()
            .expect("Failed to spawn test process");

        let pid = child.id();

        // Check that process exists
        assert!(ProcessExecutor::process_exists(pid).unwrap());

        // Kill process
        ProcessExecutor::kill_process(pid).expect("Failed to kill process");

        // Check that process no longer exists
        thread::sleep(Duration::from_millis(100));
        assert!(!ProcessExecutor::process_exists(pid).unwrap());
    }

    #[test]
    #[ignore] // Requires spawning test process
    fn test_freeze_unfreeze_workflow() {
        // Spawn test process
        let child = Command::new("sleep")
            .arg("60")
            .spawn()
            .expect("Failed to spawn test process");

        let pid = child.id();

        // Freeze process
        ProcessExecutor::freeze_process(pid).expect("Failed to freeze");

        // Process should exist but be stopped
        assert!(ProcessExecutor::process_exists(pid).unwrap());

        // Unfreeze process
        ProcessExecutor::unfreeze_process(pid).expect("Failed to unfreeze");

        // Kill process for cleanup
        ProcessExecutor::kill_process(pid).expect("Failed to kill");
    }

    #[test]
    fn test_kill_nonexistent_process() {
        // Try to kill nonexistent process
        let result = ProcessExecutor::kill_process(999999);
        assert!(result.is_err());

        if let Err(e) = result {
            let error_msg = format!("{}", e);
            assert!(error_msg.contains("does not exist"));
        }
    }
}
