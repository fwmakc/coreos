//! Filesystem-backed storage implementation.
//!
//! [`FsBackend`] implements [`StorageBackend`](super::StorageBackend) using
//! `std::fs`. Writes are atomic (temp file + rename) and return a BLAKE3
//! content hash for dedup preparation.

use std::io::Write;
use std::path::{Path, PathBuf};

use fs4::fs_std::FileExt;

use super::{ContentHash, DirEntry, Metadata, StorageBackend, StorageError};

const TEMP_SUFFIX: &str = ".wtmp";

/// Filesystem storage backend rooted at a workspace directory.
///
/// All relative paths are resolved against `workspace_root`. Absolute paths
/// are used as-is (the caller or [`PathSandbox`](super::sandbox::PathSandbox)
/// is responsible for access control).
pub struct FsBackend {
    workspace_root: PathBuf,
}

impl FsBackend {
    /// Create a new backend rooted at `workspace_root`.
    ///
    /// The directory is created recursively if it does not exist.
    pub fn new(workspace_root: PathBuf) -> Result<Self, StorageError> {
        std::fs::create_dir_all(&workspace_root)?;
        Ok(Self { workspace_root })
    }

    /// Return the workspace root path.
    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    /// Resolve `path` against the workspace root if it is relative.
    fn resolve(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.workspace_root.join(path)
        }
    }
}

impl StorageBackend for FsBackend {
    fn read(&self, path: &Path) -> Result<Vec<u8>, StorageError> {
        let resolved = self.resolve(path);
        std::fs::read(&resolved).map_err(|e| map_io_error(e, &resolved))
    }

    fn write(&self, path: &Path, data: &[u8]) -> Result<ContentHash, StorageError> {
        let resolved = self.resolve(path);
        atomic_write(&resolved, data)?;
        let hash = blake3::hash(data);
        Ok(ContentHash(hash.into()))
    }

    fn delete(&self, path: &Path) -> Result<(), StorageError> {
        let resolved = self.resolve(path);
        if resolved.is_dir() {
            std::fs::remove_dir(&resolved).map_err(|e| map_io_error(e, &resolved))
        } else {
            std::fs::remove_file(&resolved).map_err(|e| map_io_error(e, &resolved))
        }
    }

    fn exists(&self, path: &Path) -> bool {
        self.resolve(path).exists()
    }

    fn metadata(&self, path: &Path) -> Result<Metadata, StorageError> {
        let resolved = self.resolve(path);
        let meta = std::fs::symlink_metadata(&resolved).map_err(|e| map_io_error(e, &resolved))?;
        Ok(Metadata::from_fs(&meta))
    }

    fn list_dir(&self, path: &Path) -> Result<Vec<DirEntry>, StorageError> {
        let resolved = self.resolve(path);
        let rd = std::fs::read_dir(&resolved).map_err(|e| map_io_error(e, &resolved))?;
        let mut entries = Vec::new();
        for entry in rd {
            let entry = entry.map_err(StorageError::Io)?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = entry.path();
            let meta = entry.metadata().map_err(|e| map_io_error(e, &path))?;
            entries.push(DirEntry {
                name,
                path,
                metadata: Metadata::from_fs(&meta),
            });
        }
        Ok(entries)
    }

    fn create_dir(&self, path: &Path) -> Result<(), StorageError> {
        let resolved = self.resolve(path);
        std::fs::create_dir_all(&resolved).map_err(|e| map_io_error(e, &resolved))
    }

    fn copy(&self, from: &Path, to: &Path) -> Result<(), StorageError> {
        let src = self.resolve(from);
        let dst = self.resolve(to);
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(&src, &dst).map_err(|e| map_io_error(e, &src))?;
        Ok(())
    }

    fn r#move(&self, from: &Path, to: &Path) -> Result<(), StorageError> {
        let src = self.resolve(from);
        let dst = self.resolve(to);
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::rename(&src, &dst).map_err(|e| map_io_error(e, &src))
    }
}

/// Write `data` to `path` atomically via temp file + rename.
///
/// The temp file is created in the same directory as `path` to guarantee
/// the rename stays on the same mount point. An exclusive file lock is
/// held during the write to prevent concurrent temp-file collisions.
fn atomic_write(target: &Path, data: &[u8]) -> Result<(), StorageError> {
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let temp_path = append_temp_suffix(target);
    let mut file = std::fs::File::create(&temp_path).map_err(|e| map_io_error(e, &temp_path))?;

    file.lock_exclusive().map_err(StorageError::Io)?;

    let write_result = file.write_all(data).and_then(|()| file.sync_data());
    drop(file);

    if let Err(e) = write_result {
        let _ = std::fs::remove_file(&temp_path);
        return Err(StorageError::Io(e));
    }

    std::fs::rename(&temp_path, target).map_err(|e| {
        let _ = std::fs::remove_file(&temp_path);
        StorageError::Io(e)
    })
}

fn append_temp_suffix(path: &Path) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(TEMP_SUFFIX);
    PathBuf::from(s)
}

fn map_io_error(e: std::io::Error, path: &Path) -> StorageError {
    match e.kind() {
        std::io::ErrorKind::NotFound => StorageError::NotFound(path.to_path_buf()),
        std::io::ErrorKind::PermissionDenied => StorageError::PermissionDenied(path.to_path_buf()),
        std::io::ErrorKind::AlreadyExists => StorageError::AlreadyExists(path.to_path_buf()),
        _ => StorageError::Io(e),
    }
}

impl Metadata {
    fn from_fs(meta: &std::fs::Metadata) -> Self {
        Self {
            size: meta.len(),
            modified: meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH),
            is_dir: meta.is_dir(),
            is_file: meta.is_file(),
            is_symlink: meta.is_symlink(),
            readonly: meta.permissions().readonly(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_dir() -> PathBuf {
        let tid = format!("{:?}", std::thread::current().id());
        let dir = std::env::temp_dir().join(format!("w-storage-test-{tid}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn fs_backend_new_creates_root() {
        let root = std::env::temp_dir().join("w-test-fs-backend-new");
        let _ = std::fs::remove_dir_all(&root);
        let backend = FsBackend::new(root.clone()).unwrap();
        assert!(root.is_dir());
        assert_eq!(backend.workspace_root(), root);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn write_and_read_roundtrip() {
        let dir = temp_dir();
        let backend = FsBackend::new(dir.clone()).unwrap();
        let data = b"hello workspace";
        let hash = backend.write(Path::new("test.txt"), data).unwrap();
        assert_ne!(hash, ContentHash::ZERO);
        let read = backend.read(Path::new("test.txt")).unwrap();
        assert_eq!(read, data);
    }

    #[test]
    fn write_returns_blake3_hash() {
        let dir = temp_dir();
        let backend = FsBackend::new(dir).unwrap();
        let data = b"hash me";
        let hash = backend.write(Path::new("hash.txt"), data).unwrap();
        let expected: [u8; 32] = blake3::hash(data).into();
        assert_eq!(hash.0, expected);
    }

    #[test]
    fn atomic_write_no_leftover_temp() {
        let dir = temp_dir();
        let backend = FsBackend::new(dir.clone()).unwrap();
        backend.write(Path::new("atomic.txt"), b"data").unwrap();
        let temp_path = dir.join("atomic.txt").with_extension("txt.wtmp");
        assert!(!temp_path.exists(), "temp file should be cleaned up");
    }

    #[test]
    fn exists_check() {
        let dir = temp_dir();
        let backend = FsBackend::new(dir).unwrap();
        assert!(!backend.exists(Path::new("nope.txt")));
        backend.write(Path::new("yep.txt"), b"").unwrap();
        assert!(backend.exists(Path::new("yep.txt")));
    }

    #[test]
    fn delete_file() {
        let dir = temp_dir();
        let backend = FsBackend::new(dir).unwrap();
        backend.write(Path::new("del.txt"), b"x").unwrap();
        backend.delete(Path::new("del.txt")).unwrap();
        assert!(!backend.exists(Path::new("del.txt")));
    }

    #[test]
    fn delete_nonexistent_is_error() {
        let dir = temp_dir();
        let backend = FsBackend::new(dir).unwrap();
        let result = backend.delete(Path::new("ghost.txt"));
        assert!(result.is_err());
    }

    #[test]
    fn metadata_for_file() {
        let dir = temp_dir();
        let backend = FsBackend::new(dir).unwrap();
        backend.write(Path::new("meta.txt"), b"12345").unwrap();
        let meta = backend.metadata(Path::new("meta.txt")).unwrap();
        assert_eq!(meta.size, 5);
        assert!(meta.is_file);
        assert!(!meta.is_dir);
    }

    #[test]
    fn create_dir_and_list() {
        let dir = temp_dir();
        let backend = FsBackend::new(dir).unwrap();
        backend.create_dir(Path::new("subdir")).unwrap();
        backend.write(Path::new("subdir/a.txt"), b"a").unwrap();
        let entries = backend.list_dir(Path::new("subdir")).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "a.txt");
    }

    #[test]
    fn copy_file() {
        let dir = temp_dir();
        let backend = FsBackend::new(dir).unwrap();
        backend.write(Path::new("src.txt"), b"copy me").unwrap();
        backend
            .copy(Path::new("src.txt"), Path::new("dst.txt"))
            .unwrap();
        assert_eq!(backend.read(Path::new("dst.txt")).unwrap(), b"copy me");
    }

    #[test]
    fn move_file() {
        let dir = temp_dir();
        let backend = FsBackend::new(dir).unwrap();
        backend.write(Path::new("old.txt"), b"move me").unwrap();
        backend
            .r#move(Path::new("old.txt"), Path::new("new.txt"))
            .unwrap();
        assert!(!backend.exists(Path::new("old.txt")));
        assert_eq!(backend.read(Path::new("new.txt")).unwrap(), b"move me");
    }

    #[test]
    fn read_nonexistent_is_not_found() {
        let dir = temp_dir();
        let backend = FsBackend::new(dir).unwrap();
        let err = backend.read(Path::new("missing.txt")).unwrap_err();
        assert!(matches!(err, StorageError::NotFound(_)));
    }

    #[test]
    fn absolute_path_bypasses_root() {
        let dir = temp_dir();
        let backend = FsBackend::new(dir).unwrap();
        let abs = std::env::temp_dir().join("w-abs-test.txt");
        backend.write(&abs, b"abs").unwrap();
        assert_eq!(backend.read(&abs).unwrap(), b"abs");
        let _ = std::fs::remove_file(&abs);
    }

    #[test]
    fn write_creates_parent_dirs() {
        let dir = temp_dir();
        let backend = FsBackend::new(dir).unwrap();
        backend
            .write(Path::new("deep/nested/file.txt"), b"deep")
            .unwrap();
        assert!(backend.exists(Path::new("deep/nested/file.txt")));
    }

    #[test]
    fn map_io_error_kinds() {
        let p = Path::new("x");
        assert!(matches!(
            map_io_error(std::io::Error::from(std::io::ErrorKind::NotFound), p),
            StorageError::NotFound(_)
        ));
        assert!(matches!(
            map_io_error(
                std::io::Error::from(std::io::ErrorKind::PermissionDenied),
                p
            ),
            StorageError::PermissionDenied(_)
        ));
        assert!(matches!(
            map_io_error(std::io::Error::from(std::io::ErrorKind::AlreadyExists), p),
            StorageError::AlreadyExists(_)
        ));
        assert!(matches!(
            map_io_error(std::io::Error::from(std::io::ErrorKind::BrokenPipe), p),
            StorageError::Io(_)
        ));
    }

    #[test]
    fn append_temp_suffix_format() {
        let p = Path::new("/tmp/file.txt");
        let suffixed = append_temp_suffix(p);
        assert_eq!(suffixed.to_str().unwrap(), "/tmp/file.txt.wtmp");
    }
}
