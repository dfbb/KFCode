mod common;

use kfcode_watcher::{FileEvent, WatcherError};
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn watch_multiple_paths_emits_events_from_each() {
    let dir_a = common::fresh_tempdir();
    let dir_b = common::fresh_tempdir();
    let base_a = common::canonical_dir(dir_a.path());
    let base_b = common::canonical_dir(dir_b.path());
    let w = common::fresh_watcher();
    w.watch(&base_a).expect("watch a");
    w.watch(&base_b).expect("watch b");

    let mut rx = w.subscribe();

    let fa = base_a.join("a.txt");
    let fb = base_b.join("b.txt");
    std::fs::write(&fa, "a").unwrap();
    std::fs::write(&fb, "b").unwrap();

    let mut got_a = false;
    let mut got_b = false;
    let deadline = Duration::from_secs(5);
    let start = std::time::Instant::now();
    while !(got_a && got_b) {
        let remaining = deadline.checked_sub(start.elapsed()).unwrap_or(Duration::ZERO);
        let ev = timeout(remaining, rx.recv()).await
            .expect("timed out")
            .expect("recv ok");
        if ev.file == fa && ev.event == FileEvent::Add { got_a = true; }
        if ev.file == fb && ev.event == FileEvent::Add { got_b = true; }
    }
}

#[tokio::test]
async fn watch_returns_error_for_nonexistent_path() {
    let w = common::fresh_watcher();
    let bogus = PathBuf::from("/path/does/not/exist/at/all");
    let err = w.watch(&bogus).expect_err("expected PathNotFound");
    match err {
        WatcherError::PathNotFound(_) | WatcherError::WatchError(_) => {}
        other => panic!("expected PathNotFound/WatchError, got {other:?}"),
    }
}

#[tokio::test]
async fn watch_returns_already_watching_on_duplicate() {
    let dir = common::fresh_tempdir();
    let base = common::canonical_dir(dir.path());
    let w = common::fresh_watcher();
    w.watch(&base).expect("first ok");
    let err = w.watch(&base).expect_err("expected AlreadyWatching");
    match err {
        WatcherError::AlreadyWatching(_) => {}
        other => panic!("expected AlreadyWatching, got {other:?}"),
    }
}

#[tokio::test]
async fn unwatch_stops_emitting_events() {
    let dir = common::fresh_tempdir();
    let base = common::canonical_dir(dir.path());
    let w = common::fresh_watcher();
    w.watch(&base).expect("watch");
    let mut rx = w.subscribe();

    w.unwatch(&base).expect("unwatch");

    let f = base.join("after-unwatch.txt");
    std::fs::write(&f, "x").unwrap();

    let res = timeout(Duration::from_millis(1000), async {
        loop {
            let ev = rx.recv().await.unwrap();
            if ev.file == f { return ev; }
        }
    })
    .await;
    assert!(res.is_err(), "no events after unwatch");
}

#[tokio::test]
async fn watched_paths_lists_active_paths() {
    let dir_a = common::fresh_tempdir();
    let dir_b = common::fresh_tempdir();
    let base_a = common::canonical_dir(dir_a.path());
    let base_b = common::canonical_dir(dir_b.path());
    let w = common::fresh_watcher();
    w.watch(&base_a).unwrap();
    w.watch(&base_b).unwrap();

    let paths = w.watched_paths();
    assert!(paths.iter().any(|p| p == &base_a));
    assert!(paths.iter().any(|p| p == &base_b));
    assert!(w.is_watching(&base_a));
}
