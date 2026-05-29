//! File system watcher that monitors directories for create, modify, and remove events,
//! filters them against configurable glob ignore patterns, and broadcasts them over a
//! tokio broadcast channel.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use dashmap::DashSet;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

/// The kind of change that occurred on a watched file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileEvent {
    /// A new file was created at the watched path.
    Add,
    /// An existing file was modified.
    Change,
    /// A file was removed from the watched path.
    Unlink,
}

/// A single file-system event emitted by the watcher, pairing a path with its change kind.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatcherEvent {
    /// Absolute path of the file that changed.
    pub file: PathBuf,
    /// The kind of change that was detected.
    pub event: FileEvent,
}

/// Errors that can occur while creating or operating the file watcher.
#[derive(Debug, Error)]
pub enum WatcherError {
    #[error("Failed to create watcher: {0}")]
    CreateError(#[from] notify::Error),

    #[error("Failed to watch path: {0}")]
    WatchError(String),

    #[error("Path does not exist: {0}")]
    PathNotFound(PathBuf),

    #[error("Already watching: {0}")]
    AlreadyWatching(PathBuf),
}

/// Configuration options for a `FileWatcher` instance.
#[derive(Debug, Clone)]
pub struct WatcherConfig {
    /// Glob patterns for paths that should be silently ignored.
    pub ignore_patterns: Vec<String>,
    /// Poll interval in milliseconds used to debounce rapid successive events.
    pub debounce_ms: u64,
    /// Whether to watch directories recursively.
    pub recursive: bool,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            ignore_patterns: vec![
                "**/node_modules/**".to_string(),
                "**/.git/**".to_string(),
                "**/target/**".to_string(),
                "**/.DS_Store".to_string(),
                "**/*.swp".to_string(),
                "**/*.swo".to_string(),
                "**/*~".to_string(),
            ],
            debounce_ms: 100,
            recursive: true,
        }
    }
}

/// A non-blocking file system watcher that broadcasts change events to subscribers.
///
/// Internally wraps a `notify` watcher and filters events through compiled glob patterns
/// before forwarding them on a tokio broadcast channel.
pub struct FileWatcher {
    watcher: RwLock<Option<RecommendedWatcher>>,
    watched_paths: DashSet<PathBuf>,
    ignore_patterns: Vec<glob::Pattern>,
    event_tx: broadcast::Sender<WatcherEvent>,
    debounce_ms: u64,
}

impl FileWatcher {
    /// Create a new file watcher
    pub fn new(config: WatcherConfig) -> Result<Self, WatcherError> {
        let (event_tx, _) = broadcast::channel(1024);

        let ignore_patterns = config
            .ignore_patterns
            .iter()
            .filter_map(|p| glob::Pattern::new(p).ok())
            .collect();

        Ok(Self {
            watcher: RwLock::new(None),
            watched_paths: DashSet::new(),
            ignore_patterns,
            event_tx,
            debounce_ms: config.debounce_ms,
        })
    }

    /// Subscribe to file watcher events
    pub fn subscribe(&self) -> broadcast::Receiver<WatcherEvent> {
        self.event_tx.subscribe()
    }

    /// Start watching a directory
    pub fn watch(&self, path: &Path) -> Result<(), WatcherError> {
        if !path.exists() {
            return Err(WatcherError::PathNotFound(path.to_path_buf()));
        }

        if self.watched_paths.contains(path) {
            return Err(WatcherError::AlreadyWatching(path.to_path_buf()));
        }

        let mut guard = self.watcher.write();

        if guard.is_none() {
            // First watch: create the underlying notify watcher.
            let event_tx = self.event_tx.clone();
            let ignore_patterns = self.ignore_patterns.clone();

            let mut new_watcher =
                notify::recommended_watcher(move |res: Result<Event, notify::Error>| match res {
                    Ok(event) => {
                        let file_event = match event.kind {
                            EventKind::Create(_) => FileEvent::Add,
                            EventKind::Modify(_) => FileEvent::Change,
                            EventKind::Remove(_) => FileEvent::Unlink,
                            _ => return,
                        };

                        for path in &event.paths {
                            let path_str = path.to_string_lossy();
                            let should_ignore =
                                ignore_patterns.iter().any(|p| p.matches(&path_str));

                            if should_ignore {
                                debug!(path = ?path, "Ignoring file event");
                                continue;
                            }

                            let watcher_event = WatcherEvent {
                                file: path.clone(),
                                event: file_event,
                            };

                            if let Err(e) = event_tx.send(watcher_event) {
                                warn!(error = %e, "Failed to send watcher event");
                            }
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "Watcher error");
                    }
                })?;

            new_watcher.configure(
                Config::default().with_poll_interval(Duration::from_millis(self.debounce_ms)),
            )?;

            *guard = Some(new_watcher);
        }

        // Reuse the existing watcher for subsequent paths.
        let w = guard.as_mut().expect("watcher just initialised");
        w.watch(path, RecursiveMode::Recursive)
            .map_err(|e| WatcherError::WatchError(e.to_string()))?;

        info!(path = ?path, "Started watching directory");

        self.watched_paths.insert(path.to_path_buf());

        Ok(())
    }

    /// Stops watching the given path and removes it from the active watch set.
    ///
    /// If no paths remain after removal the underlying watcher is dropped entirely.
    pub fn unwatch(&self, path: &Path) -> Result<(), WatcherError> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

        // First, tell the underlying notify watcher to stop watching this path.
        {
            let mut guard = self.watcher.write();
            if let Some(w) = guard.as_mut() {
                // Ignore errors: the path may already have been removed from the watcher.
                let _ = w.unwatch(&canonical);
            }
        }

        // Remove from our own tracking set (stored as the original path).
        self.watched_paths.remove(path);

        // Drop the watcher entirely when no paths remain.
        if self.watched_paths.is_empty() {
            *self.watcher.write() = None;
            info!("Stopped all watchers");
        }

        Ok(())
    }

    /// Stops all active watchers and clears the watched-path set.
    pub fn stop(&self) {
        self.watched_paths.clear();
        *self.watcher.write() = None;
        info!("Stopped all file watchers");
    }

    /// Returns a snapshot of all paths currently being watched.
    pub fn watched_paths(&self) -> Vec<PathBuf> {
        self.watched_paths.iter().map(|p| p.clone()).collect()
    }

    /// Returns `true` if the given path is currently in the active watch set.
    pub fn is_watching(&self, path: &Path) -> bool {
        self.watched_paths.contains(path)
    }
}

impl Drop for FileWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Process-wide singleton `FileWatcher` initialised on first access.
static FILE_WATCHER: std::sync::OnceLock<Arc<FileWatcher>> = std::sync::OnceLock::new();

/// Returns the process-wide `FileWatcher`, creating it with default config on first call.
///
/// # Panics
///
/// Panics if the underlying `FileWatcher` cannot be constructed (e.g. the OS watcher
/// backend is unavailable).
pub fn get_watcher() -> Arc<FileWatcher> {
    FILE_WATCHER
        .get_or_init(|| {
            Arc::new(
                FileWatcher::new(WatcherConfig::default()).expect("Failed to create file watcher"),
            )
        })
        .clone()
}

/// Initialises the process-wide `FileWatcher` with the supplied config and returns it.
///
/// If the singleton has already been initialised the existing instance is returned and
/// `config` is ignored.
///
/// # Panics
///
/// Panics if the `FileWatcher` cannot be constructed from the given config.
pub fn init_watcher(config: WatcherConfig) -> Arc<FileWatcher> {
    let watcher = Arc::new(FileWatcher::new(config).expect("Failed to create file watcher"));
    FILE_WATCHER.get_or_init(|| watcher.clone()).clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::Duration;
    use tempfile::TempDir;
    use tokio::time::sleep;

    #[test]
    fn test_watcher_config_default() {
        let config = WatcherConfig::default();
        assert!(!config.ignore_patterns.is_empty());
        assert!(config.recursive);
    }

    #[test]
    fn test_file_event_serde() {
        let event = FileEvent::Add;
        let json = serde_json::to_string(&event).unwrap();
        assert_eq!(json, "\"add\"");

        let parsed: FileEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, FileEvent::Add);
    }

    #[tokio::test]
    async fn test_watch_directory() {
        let temp_dir = TempDir::new().unwrap();
        let watcher = FileWatcher::new(WatcherConfig::default()).unwrap();

        let mut rx = watcher.subscribe();

        watcher.watch(temp_dir.path()).unwrap();

        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "hello").unwrap();

        sleep(Duration::from_millis(200)).await;

        let mut found = false;
        loop {
            match rx.try_recv() {
                Ok(event) => {
                    if event.file.ends_with("test.txt") {
                        assert_eq!(event.event, FileEvent::Add);
                        found = true;
                        break;
                    }
                }
                Err(broadcast::error::TryRecvError::Empty) => break,
                Err(e) => panic!("Unexpected error: {}", e),
            }
        }
        assert!(found, "did not observe an Add event for test.txt");
    }

    #[test]
    fn test_watch_nonexistent_path() {
        let watcher = FileWatcher::new(WatcherConfig::default()).unwrap();
        let result = watcher.watch(Path::new("/nonexistent/path"));
        assert!(matches!(result, Err(WatcherError::PathNotFound(_))));
    }

    #[test]
    fn test_watch_same_path_twice() {
        let temp_dir = TempDir::new().unwrap();
        let watcher = FileWatcher::new(WatcherConfig::default()).unwrap();

        watcher.watch(temp_dir.path()).unwrap();
        let result = watcher.watch(temp_dir.path());
        assert!(matches!(result, Err(WatcherError::AlreadyWatching(_))));
    }

    #[test]
    fn test_watched_paths() {
        let temp_dir = TempDir::new().unwrap();
        let watcher = FileWatcher::new(WatcherConfig::default()).unwrap();

        assert!(watcher.watched_paths().is_empty());
        assert!(!watcher.is_watching(temp_dir.path()));

        watcher.watch(temp_dir.path()).unwrap();

        assert_eq!(watcher.watched_paths().len(), 1);
        assert!(watcher.is_watching(temp_dir.path()));
    }
}
