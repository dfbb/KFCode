#![allow(dead_code)]

use kfcode_watcher::{FileWatcher, WatcherConfig};
use std::sync::Arc;
use tempfile::TempDir;

pub fn fresh_watcher() -> Arc<FileWatcher> {
    Arc::new(FileWatcher::new(WatcherConfig::default()).expect("watcher init"))
}

pub fn fresh_tempdir() -> TempDir {
    TempDir::new().expect("tempdir")
}

/// On macOS, TempDir paths are symlinks (/var -> /private/var).
/// notify resolves them to canonical paths, so we must do the same
/// when constructing expected paths for comparison.
/// Canonicalize the directory, then join the filename — avoids the
/// "file doesn't exist yet" problem with canonicalizing the full path.
pub fn canonical_dir(dir: &std::path::Path) -> std::path::PathBuf {
    dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf())
}
