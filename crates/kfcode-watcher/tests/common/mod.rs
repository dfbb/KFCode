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
