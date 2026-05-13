//! Structured JSON logging setup.

use std::fs;
use std::path::PathBuf;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::prelude::*;

/// Initialize tracing with JSON formatting and file output.
///
/// Environment variables:
/// - `CORE_LOG` — log level filter (default: `info`)
pub fn init_logging() -> Result<(), crate::backend::HostError> {
    let log_dir = log_directory()?;
    fs::create_dir_all(&log_dir).map_err(|e| {
        crate::backend::HostError::WindowCreationFailed(format!("log dir creation failed: {e}"))
    })?;

    let log_file = log_dir.join("coreos.log");
    let file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .map_err(|e| {
            crate::backend::HostError::WindowCreationFailed(format!("log file open failed: {e}"))
        })?;

    let env_filter = tracing_subscriber::EnvFilter::try_from_env("CORE_LOG")
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_span_events(FmtSpan::CLOSE)
        .with_writer(std::sync::Mutex::new(file));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();

    tracing::info!(log_file = %log_file.display(), "Logging initialized");
    Ok(())
}

/// Returns the platform-specific log directory.
fn log_directory() -> Result<PathBuf, crate::backend::HostError> {
    #[cfg(target_os = "windows")]
    {
        let local_app_data = std::env::var("LOCALAPPDATA").map_err(|_| {
            crate::backend::HostError::WindowCreationFailed("LOCALAPPDATA not set".into())
        })?;
        Ok(PathBuf::from(local_app_data).join("CoreOS").join("logs"))
    }

    #[cfg(target_os = "linux")]
    {
        let home = std::env::var("HOME")
            .map_err(|_| crate::backend::HostError::WindowCreationFailed("HOME not set".into()))?;
        Ok(PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("CoreOS")
            .join("logs"))
    }

    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME")
            .map_err(|_| crate::backend::HostError::WindowCreationFailed("HOME not set".into()))?;
        Ok(PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("CoreOS")
            .join("logs"))
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        Err(crate::backend::HostError::PlatformNotSupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_directory_does_not_panic() {
        // We can't call init_logging in tests because tracing_subscriber::init
        // panics if called twice, but we can verify the path logic.
        let path = log_directory();
        assert!(path.is_ok(), "log_directory should not panic");
        let path = path.unwrap();
        assert!(path.to_string_lossy().contains("CoreOS"));
        assert!(path.to_string_lossy().contains("logs"));
    }
}
