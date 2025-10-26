use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Systemd error: {0}")]
    Systemd(String),

    #[error("Scanner error: {0}")]
    Scanner(String),

    #[error("Executor error: {0}")]
    Executor(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Other error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_systemd_error_display() {
        let err = Error::Systemd("daemon-reload failed".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Systemd error"));
        assert!(msg.contains("daemon-reload failed"));
    }

    #[test]
    fn test_scanner_error_display() {
        let err = Error::Scanner("process not found".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Scanner error"));
        assert!(msg.contains("process not found"));
    }

    #[test]
    fn test_executor_error_display() {
        let err = Error::Executor("failed to kill process".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Executor error"));
        assert!(msg.contains("failed to kill process"));
    }

    #[test]
    fn test_other_error_display() {
        let err = Error::Other("unknown error".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Other error"));
        assert!(msg.contains("unknown error"));
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: Error = io_err.into();
        let msg = format!("{}", err);
        assert!(msg.contains("IO error"));
        assert!(msg.contains("file not found"));
    }

    #[test]
    fn test_error_debug() {
        let err = Error::Systemd("test".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("Systemd"));
    }

    #[test]
    fn test_result_type_ok() {
        let result: Result<i32> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_result_type_err() {
        let result: Result<i32> = Err(Error::Other("test error".to_string()));
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(format!("{}", e).contains("test error"));
        }
    }
}
