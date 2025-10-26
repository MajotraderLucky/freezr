//! Error types for cgroup operations

use std::io;
use thiserror::Error;

/// Cgroup operation errors
#[derive(Debug, Error)]
pub enum CgroupError {
    #[error("Cgroup not found: {0}")]
    NotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Invalid limit value: {0}")]
    InvalidLimit(String),

    #[error("Cgroup already exists: {0}")]
    AlreadyExists(String),

    #[error("Failed to parse cgroup file: {0}")]
    ParseError(String),

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Process not found: {0}")]
    ProcessNotFound(u32),

    #[error("Max dynamic cgroups reached (limit: {0})")]
    MaxCgroupsReached(usize),

    #[error("Cgroup validation failed: {0}")]
    ValidationError(String),

    #[error("Systemd not available")]
    SystemdNotAvailable,

    #[error("Cgroup v2 not available")]
    CgroupV2NotAvailable,

    #[error("Insufficient privileges (need root or CAP_SYS_ADMIN)")]
    InsufficientPrivileges,
}

pub type Result<T> = std::result::Result<T, CgroupError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = CgroupError::NotFound("test-group".to_string());
        assert_eq!(err.to_string(), "Cgroup not found: test-group");

        let err = CgroupError::MaxCgroupsReached(50);
        assert_eq!(err.to_string(), "Max dynamic cgroups reached (limit: 50)");
    }

    #[test]
    fn test_error_from_io() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let cgroup_err: CgroupError = io_err.into();
        assert!(matches!(cgroup_err, CgroupError::Io(_)));
    }
}
