//! File system watcher using the `notify` crate.
//!
//! Provides cross-platform file change detection (Create, Modify, Delete,
//! Rename) backed by inotify (Linux), FSEvents (macOS), or
//! ReadDirectoryChangesW (Windows).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

use notify::{Config, EventKind, RecursiveMode, Watcher};

use super::StorageError;

/// Opaque handle returned by [`FileWatcher::watch`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WatchId(u64);

/// A single file system change event.
#[derive(Debug, Clone)]
pub enum WatchEvent {
    /// A file or directory was created.
    Created {
        /// Path to the new entry.
        path: PathBuf,
    },
    /// A file or directory was modified.
    Modified {
        /// Path to the modified entry.
        path: PathBuf,
    },
    /// A file or directory was deleted.
    Deleted {
        /// Path to the removed entry.
        path: PathBuf,
    },
    /// A file or directory was renamed.
    Renamed {
        /// Original path.
        from: PathBuf,
        /// New path.
        to: PathBuf,
    },
}

/// Cross-platform file system watcher.
///
/// Wraps `notify::RecommendedWatcher` and translates low-level events
/// into [`WatchEvent`] values. Events are buffered internally and
/// drained via [`FileWatcher::poll`].
pub struct FileWatcher {
    watcher: notify::RecommendedWatcher,
    rx: mpsc::Receiver<Result<notify::Event, notify::Error>>,
    watches: HashMap<WatchId, PathBuf>,
    next_id: u64,
}

impl FileWatcher {
    /// Create a new watcher.
    pub fn new() -> Result<Self, StorageError> {
        let (tx, rx) = mpsc::channel();
        let watcher = notify::RecommendedWatcher::new(
            move |res: Result<notify::Event, notify::Error>| {
                let _ = tx.send(res);
            },
            Config::default(),
        )
        .map_err(|e| StorageError::WatcherFailed(e.to_string()))?;

        Ok(Self {
            watcher,
            rx,
            watches: HashMap::new(),
            next_id: 0,
        })
    }

    /// Start watching `path` for changes.
    ///
    /// If `recursive` is true, changes in subdirectories are also reported.
    /// Returns a [`WatchId`] that can be used to stop the watch later.
    pub fn watch(&mut self, path: &Path, recursive: bool) -> Result<WatchId, StorageError> {
        let mode = if recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };
        self.watcher
            .watch(path, mode)
            .map_err(|e| StorageError::WatcherFailed(e.to_string()))?;

        let id = WatchId(self.next_id);
        self.next_id += 1;
        self.watches.insert(id, path.to_path_buf());
        Ok(id)
    }

    /// Stop watching the path associated with `id`.
    pub fn unwatch(&mut self, id: WatchId) -> Result<(), StorageError> {
        if let Some(path) = self.watches.remove(&id) {
            self.watcher
                .unwatch(&path)
                .map_err(|e| StorageError::WatcherFailed(e.to_string()))?;
        }
        Ok(())
    }

    /// Drain all pending events since the last call.
    ///
    /// Returns an empty vector if no events are available.
    pub fn poll(&mut self) -> Vec<WatchEvent> {
        let mut events = Vec::new();
        while let Ok(result) = self.rx.try_recv() {
            match result {
                Ok(event) => events.extend(convert_event(event)),
                Err(e) => {
                    tracing::warn!("watcher error: {e}");
                }
            }
        }
        events
    }

    /// Number of active watches.
    pub fn watch_count(&self) -> usize {
        self.watches.len()
    }
}

fn convert_event(event: notify::Event) -> Vec<WatchEvent> {
    match event.kind {
        EventKind::Create(_) => event
            .paths
            .into_iter()
            .map(|path| WatchEvent::Created { path })
            .collect(),
        EventKind::Modify(notify::event::ModifyKind::Name(notify::event::RenameMode::From)) => {
            let mut out = Vec::new();
            if event.paths.len() >= 2 {
                out.push(WatchEvent::Renamed {
                    from: event.paths[0].clone(),
                    to: event.paths[1].clone(),
                });
            }
            out
        }
        EventKind::Modify(_) => event
            .paths
            .into_iter()
            .map(|path| WatchEvent::Modified { path })
            .collect(),
        EventKind::Remove(_) => event
            .paths
            .into_iter()
            .map(|path| WatchEvent::Deleted { path })
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn unique_temp_dir(name: &str) -> PathBuf {
        let tid = format!("{:?}", std::thread::current().id());
        let dir = std::env::temp_dir().join(format!("w-watcher-test-{name}-{tid}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn file_watcher_new() {
        assert!(FileWatcher::new().is_ok());
    }

    #[test]
    fn watch_and_unwatch() {
        let dir = unique_temp_dir("watch");
        let mut watcher = FileWatcher::new().unwrap();
        let id = watcher.watch(&dir, false).unwrap();
        assert_eq!(watcher.watch_count(), 1);
        watcher.unwatch(id).unwrap();
        assert_eq!(watcher.watch_count(), 0);
    }

    #[test]
    #[ignore = "requires file system event timing (run with --ignored --test-threads=1)"]
    fn detect_file_creation() {
        let dir = unique_temp_dir("create");
        let mut watcher = FileWatcher::new().unwrap();
        watcher.watch(&dir, true).unwrap();

        let file_path = dir.join("created.txt");
        fs::write(&file_path, b"hello").unwrap();

        std::thread::sleep(std::time::Duration::from_millis(200));
        let events = watcher.poll();

        let has_create = events.iter().any(|e| {
            matches!(
                e,
                WatchEvent::Created { path } if *path == file_path
            )
        });
        assert!(has_create, "expected Created event for {:?}", file_path);
    }

    #[test]
    #[ignore = "requires file system event timing (run with --ignored --test-threads=1)"]
    fn detect_file_modification() {
        let dir = unique_temp_dir("modify");
        let file_path = dir.join("modify.txt");
        fs::write(&file_path, b"v1").unwrap();

        let mut watcher = FileWatcher::new().unwrap();
        watcher.watch(&dir, true).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(100));
        let _ = watcher.poll();

        fs::write(&file_path, b"v2").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(200));
        let events = watcher.poll();

        let has_modify = events.iter().any(|e| {
            matches!(
                e,
                WatchEvent::Modified { path } if *path == file_path
            )
        });
        assert!(has_modify, "expected Modified event for {:?}", file_path);
    }

    #[test]
    #[ignore = "requires file system event timing (run with --ignored --test-threads=1)"]
    fn detect_file_deletion() {
        let dir = unique_temp_dir("delete");
        let file_path = dir.join("delete.txt");
        fs::write(&file_path, b"bye").unwrap();

        let mut watcher = FileWatcher::new().unwrap();
        watcher.watch(&dir, true).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(100));
        let _ = watcher.poll();

        fs::remove_file(&file_path).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(200));
        let events = watcher.poll();

        let has_delete = events.iter().any(|e| {
            matches!(
                e,
                WatchEvent::Deleted { path } if *path == file_path
            )
        });
        assert!(has_delete, "expected Deleted event for {:?}", file_path);
    }

    #[test]
    #[ignore = "requires file system event timing (run with --ignored --test-threads=1)"]
    fn poll_empty_when_no_events() {
        let dir = unique_temp_dir("empty");
        let mut watcher = FileWatcher::new().unwrap();
        watcher.watch(&dir, false).unwrap();
        let events = watcher.poll();
        assert!(events.is_empty());
    }

    #[test]
    fn watch_id_unique() {
        let dir = unique_temp_dir("unique");
        let sub = dir.join("sub");
        fs::create_dir_all(&sub).unwrap();
        let mut watcher = FileWatcher::new().unwrap();
        let id1 = watcher.watch(&dir, false).unwrap();
        let id2 = watcher.watch(&sub, false).unwrap();
        assert_ne!(id1, id2);
    }

    #[test]
    fn unwatch_nonexistent_id_is_ok() {
        let mut watcher = FileWatcher::new().unwrap();
        let result = watcher.unwatch(WatchId(999));
        assert!(result.is_ok());
    }
}
