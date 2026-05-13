//! Cross-platform storage subsystem.
//!
//! Provides file I/O, directory operations, file watching, sandboxing,
//! and removable media detection across all supported platforms.
//!
//! All paths are either absolute or relative to `WORKSPACE_ROOT` (the
//! workspace's working directory on the host OS). The [`StorageBackend`]
//! trait defines the core operations; [`FsBackend`] is the default
//! implementation backed by `std::fs`.

pub mod backend;
pub mod removable;
pub mod sandbox;
pub mod watcher;

use std::path::PathBuf;

/// Storage subsystem errors.
#[derive(Debug)]
pub enum StorageError {
    /// The requested path does not exist.
    NotFound(PathBuf),
    /// Permission denied by the OS or sandbox policy.
    PermissionDenied(PathBuf),
    /// A file or directory already exists at the target path.
    AlreadyExists(PathBuf),
    /// Path is outside the allowed sandbox.
    SandboxViolation(PathBuf),
    /// The file watcher failed to start or deliver events.
    WatcherFailed(String),
    /// The path is malformed or contains invalid components.
    InvalidPath(PathBuf),
    /// An underlying I/O error occurred.
    Io(std::io::Error),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(p) => write!(f, "not found: {}", p.display()),
            Self::PermissionDenied(p) => write!(f, "permission denied: {}", p.display()),
            Self::AlreadyExists(p) => write!(f, "already exists: {}", p.display()),
            Self::SandboxViolation(p) => write!(f, "sandbox violation: {}", p.display()),
            Self::WatcherFailed(msg) => write!(f, "watcher failed: {msg}"),
            Self::InvalidPath(p) => write!(f, "invalid path: {}", p.display()),
            Self::Io(e) => write!(f, "I/O error: {e}"),
        }
    }
}

impl std::error::Error for StorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for StorageError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// BLAKE3 content hash produced on every write for dedup preparation.
///
/// Passed to the Micro-Kernel VFS (Phase 12) for content-addressable
/// deduplication. The hash covers the complete file bytes written.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentHash(pub [u8; 32]);

impl ContentHash {
    /// All-zero hash for testing.
    pub const ZERO: Self = Self([0u8; 32]);
}

impl std::fmt::Display for ContentHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in &self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// File or directory metadata.
#[derive(Debug, Clone)]
pub struct Metadata {
    /// Total size in bytes.
    pub size: u64,
    /// Last modification time.
    pub modified: std::time::SystemTime,
    /// Entry is a directory.
    pub is_dir: bool,
    /// Entry is a regular file.
    pub is_file: bool,
    /// Entry is a symbolic link.
    pub is_symlink: bool,
    /// Whether the entry is read-only.
    pub readonly: bool,
}

/// A single entry returned by directory listing.
#[derive(Debug, Clone)]
pub struct DirEntry {
    /// File or directory name (last component).
    pub name: String,
    /// Full path to the entry.
    pub path: PathBuf,
    /// Cached metadata.
    pub metadata: Metadata,
}

/// Core storage operations.
///
/// All methods accept paths that are either absolute or relative to the
/// implementation's workspace root. Implementations must resolve relative
/// paths before performing I/O.
pub trait StorageBackend {
    /// Read the entire contents of a file.
    fn read(&self, path: &std::path::Path) -> Result<Vec<u8>, StorageError>;

    /// Atomically write data to a file and return its content hash.
    ///
    /// Uses a temporary file + rename to ensure the target file is never
    /// in a partially-written state.
    fn write(&self, path: &std::path::Path, data: &[u8]) -> Result<ContentHash, StorageError>;

    /// Delete a file or empty directory.
    fn delete(&self, path: &std::path::Path) -> Result<(), StorageError>;

    /// Check whether a path exists.
    fn exists(&self, path: &std::path::Path) -> bool;

    /// Retrieve metadata for a file or directory.
    fn metadata(&self, path: &std::path::Path) -> Result<Metadata, StorageError>;

    /// List the contents of a directory.
    fn list_dir(&self, path: &std::path::Path) -> Result<Vec<DirEntry>, StorageError>;

    /// Recursively create a directory and all parent components.
    fn create_dir(&self, path: &std::path::Path) -> Result<(), StorageError>;

    /// Copy a file from `from` to `to`.
    fn copy(&self, from: &std::path::Path, to: &std::path::Path) -> Result<(), StorageError>;

    /// Move (rename) a file or directory.
    fn r#move(&self, from: &std::path::Path, to: &std::path::Path) -> Result<(), StorageError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn storage_error_display_variants() {
        assert!(StorageError::NotFound("foo".into())
            .to_string()
            .contains("not found"));
        assert!(StorageError::PermissionDenied("bar".into())
            .to_string()
            .contains("permission denied"));
        assert!(StorageError::AlreadyExists("baz".into())
            .to_string()
            .contains("already exists"));
        assert!(StorageError::SandboxViolation("/etc".into())
            .to_string()
            .contains("sandbox violation"));
        assert!(StorageError::WatcherFailed("crash".into())
            .to_string()
            .contains("watcher failed"));
        assert!(StorageError::InvalidPath("..".into())
            .to_string()
            .contains("invalid path"));
        assert!(StorageError::Io(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "pipe broke"
        ))
        .to_string()
        .contains("I/O error"));
    }

    #[test]
    fn storage_error_is_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(StorageError::NotFound("x".into()));
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn storage_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
        let storage_err = StorageError::from(io_err);
        assert!(matches!(storage_err, StorageError::Io(_)));
    }

    #[test]
    fn storage_error_source() {
        let io_err = std::io::Error::new(std::io::ErrorKind::TimedOut, "timeout");
        let storage_err = StorageError::Io(io_err);
        assert!(storage_err.source().is_some());

        let no_source = StorageError::NotFound("x".into());
        assert!(no_source.source().is_none());
    }

    #[test]
    fn content_hash_display() {
        let hash = ContentHash([0xAB; 32]);
        let s = hash.to_string();
        assert_eq!(s.len(), 64);
        assert!(s.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn content_hash_zero() {
        assert_eq!(ContentHash::ZERO.0, [0u8; 32]);
    }

    #[test]
    fn content_hash_equality() {
        let a = ContentHash([1u8; 32]);
        let b = ContentHash([1u8; 32]);
        let c = ContentHash([2u8; 32]);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn dir_entry_fields() {
        let entry = DirEntry {
            name: "test.txt".into(),
            path: PathBuf::from("/tmp/test.txt"),
            metadata: Metadata {
                size: 42,
                modified: std::time::SystemTime::UNIX_EPOCH,
                is_dir: false,
                is_file: true,
                is_symlink: false,
                readonly: false,
            },
        };
        assert_eq!(entry.name, "test.txt");
        assert!(entry.metadata.is_file);
        assert!(!entry.metadata.is_dir);
    }
}
