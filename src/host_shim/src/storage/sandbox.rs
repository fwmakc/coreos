//! Path sandboxing for application file access.
//!
//! Each application is restricted to its own subdirectory under the
//! workspace root. Access outside the sandbox requires an explicit
//! capability (implemented in Phase 20).

use std::path::{Component, Path, PathBuf};

use super::StorageError;

const APPS_DIR: &str = "apps";

/// Restricts file system access to a subtree of the workspace root.
///
/// Use [`PathSandbox::validate`] to check an arbitrary path, or
/// [`PathSandbox::validate_app`] to enforce per-app isolation.
pub struct PathSandbox {
    root: PathBuf,
}

impl PathSandbox {
    /// Create a sandbox rooted at `root`.
    ///
    /// The path is canonicalized so that symlink-based escapes are detected.
    pub fn new(root: PathBuf) -> Result<Self, StorageError> {
        if !root.exists() {
            std::fs::create_dir_all(&root)?;
        }
        let canonical = root
            .canonicalize()
            .map_err(|_| StorageError::InvalidPath(root))?;
        Ok(Self { root: canonical })
    }

    /// Return the sandbox root (canonicalized).
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Return the per-app directory for `app_id`.
    ///
    /// The directory is not created automatically; the caller should use
    /// `StorageBackend::create_dir` to materialize it.
    pub fn app_dir(&self, app_id: &str) -> PathBuf {
        self.root.join(APPS_DIR).join(sanitize_app_id(app_id))
    }

    /// Validate that `path` is inside the sandbox root.
    ///
    /// Returns [`StorageError::SandboxViolation`] if the path escapes the
    /// root, or [`StorageError::NotFound`] if the path does not exist and
    /// cannot be validated.
    pub fn validate(&self, path: &Path) -> Result<PathBuf, StorageError> {
        let resolved = self.resolve_against(path, &self.root);

        if let Ok(canonical) = resolved.canonicalize() {
            if canonical.starts_with(&self.root) {
                return Ok(canonical);
            }
            return Err(StorageError::SandboxViolation(canonical));
        }

        if !is_within(&resolved, &self.root) {
            return Err(StorageError::SandboxViolation(resolved));
        }

        Ok(resolved)
    }

    /// Validate that `path` is inside the per-app sandbox for `app_id`.
    ///
    /// This is the primary access check for application code.
    pub fn validate_app(&self, app_id: &str, path: &Path) -> Result<PathBuf, StorageError> {
        let app_root = self.app_dir(app_id);
        let resolved = self.resolve_against(path, &app_root);

        if let Ok(canonical) = resolved.canonicalize() {
            let app_canonical = app_root.canonicalize().unwrap_or(app_root.clone());
            if canonical.starts_with(&app_canonical) {
                return Ok(canonical);
            }
            return Err(StorageError::SandboxViolation(canonical));
        }

        if !is_within(&resolved, &app_root) {
            return Err(StorageError::SandboxViolation(resolved));
        }

        Ok(resolved)
    }

    fn resolve_against(&self, path: &Path, base: &Path) -> PathBuf {
        if path.is_absolute() {
            normalize_path(path)
        } else {
            normalize_path(&base.join(path))
        }
    }
}

/// Normalize a path by resolving `.` and `..` without touching the filesystem.
fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                components.pop();
            }
            c => components.push(c),
        }
    }
    components.iter().collect()
}

/// Check whether `child` is within `parent` using normalized paths.
fn is_within(child: &Path, parent: &Path) -> bool {
    child == parent || child.starts_with(parent)
}

/// Strip characters that are unsafe in directory names.
fn sanitize_app_id(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn unique_temp_dir(name: &str) -> PathBuf {
        let tid = format!("{:?}", std::thread::current().id());
        let dir = std::env::temp_dir().join(format!("w-sandbox-test-{name}-{tid}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn sandbox_new_creates_root() {
        let root = std::env::temp_dir().join("w-sandbox-new-test");
        let _ = fs::remove_dir_all(&root);
        let sandbox = PathSandbox::new(root.clone()).unwrap();
        assert!(root.is_dir());
        assert_eq!(sandbox.root(), &root.canonicalize().unwrap());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn validate_inside_root() {
        let dir = unique_temp_dir("inside");
        let file = dir.join("inside.txt");
        fs::write(&file, b"data").unwrap();
        let sandbox = PathSandbox::new(dir).unwrap();
        let result = sandbox.validate(&file);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_outside_root_is_violation() {
        let dir = unique_temp_dir("outside");
        let sandbox = PathSandbox::new(dir).unwrap();
        let outside = sandbox.root().parent().unwrap().join("forbidden.txt");
        let result = sandbox.validate(&outside);
        assert!(
            matches!(result, Err(StorageError::SandboxViolation(_))),
            "expected SandboxViolation, got {:?}",
            result
        );
    }

    #[test]
    fn validate_dotdot_escape_is_violation() {
        let dir = unique_temp_dir("dotdot");
        let sandbox = PathSandbox::new(dir).unwrap();
        let escape = Path::new("../../../etc/passwd");
        let result = sandbox.validate(escape);
        assert!(
            matches!(result, Err(StorageError::SandboxViolation(_))),
            "expected SandboxViolation for .. escape, got {:?}",
            result
        );
    }

    #[test]
    fn validate_nonexistent_inside_root_ok() {
        let dir = unique_temp_dir("nonexist");
        let sandbox = PathSandbox::new(dir).unwrap();
        let result = sandbox.validate(Path::new("future.txt"));
        assert!(
            result.is_ok(),
            "non-existent path inside root should validate"
        );
    }

    #[test]
    fn app_dir_path() {
        let dir = unique_temp_dir("appdir");
        let sandbox = PathSandbox::new(dir).unwrap();
        let app_dir = sandbox.app_dir("com.example.app");
        assert!(app_dir.to_str().unwrap().contains("apps"));
        assert!(app_dir.to_str().unwrap().contains("com.example.app"));
    }

    #[test]
    fn validate_app_inside_sandbox() {
        let dir = unique_temp_dir("appinside");
        let sandbox = PathSandbox::new(dir.clone()).unwrap();
        let app_dir = sandbox.app_dir("myapp");
        fs::create_dir_all(&app_dir).unwrap();
        let file = app_dir.join("data.txt");
        fs::write(&file, b"ok").unwrap();
        let result = sandbox.validate_app("myapp", &file);
        assert!(result.is_ok(), "expected Ok, got {:?}", result);
    }

    #[test]
    fn validate_app_outside_sandbox_is_violation() {
        let dir = unique_temp_dir("appoutside");
        let sandbox = PathSandbox::new(dir.clone()).unwrap();
        let app_dir = sandbox.app_dir("myapp");
        fs::create_dir_all(&app_dir).unwrap();
        let other = dir.join("other.txt");
        fs::write(&other, b"nope").unwrap();
        let result = sandbox.validate_app("myapp", &other);
        assert!(
            matches!(result, Err(StorageError::SandboxViolation(_))),
            "app should not access files outside its sandbox, got {:?}",
            result
        );
    }

    #[test]
    fn sanitize_app_id_strips_special_chars() {
        assert_eq!(sanitize_app_id("com.example/app"), "com.example_app");
        assert_eq!(sanitize_app_id("a!@#b"), "a___b");
        assert_eq!(sanitize_app_id("normal-app_v2.1"), "normal-app_v2.1");
    }

    #[test]
    fn validate_relative_path() {
        let dir = unique_temp_dir("relative");
        let file = dir.join("relative.txt");
        fs::write(&file, b"rel").unwrap();
        let sandbox = PathSandbox::new(dir).unwrap();
        let result = sandbox.validate(Path::new("relative.txt"));
        assert!(result.is_ok());
    }

    #[test]
    fn normalize_path_strips_dot_and_dotdot() {
        assert_eq!(
            normalize_path(Path::new("/a/b/../c/./d")),
            PathBuf::from("/a/c/d")
        );
        assert_eq!(normalize_path(Path::new("a/./b/")), PathBuf::from("a/b"));
    }
}
