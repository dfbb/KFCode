# T30 — watcher 多路径 + 错误路径

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 验证多路径同时监听、`PathNotFound` / `AlreadyWatching` 错误、`unwatch` 后停止接收。

**Architecture:** 复用 T28/T29 helper；不同测试函数独立 `FileWatcher` 实例（避免互相污染 broadcast channel）。

**Tech Stack:** notify / tokio broadcast / tempfile。

**依赖:** T28

---

### Task 4.2：watcher 多路径与错误

**Files:**
- Create: `crates/kfcode-watcher/tests/multi_path_errors.rs`

- [ ] **Step 1: 写测试**

写入 `crates/kfcode-watcher/tests/multi_path_errors.rs`：

```rust
mod common;

use kfcode_watcher::{FileEvent, WatcherError};
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn watch_multiple_paths_emits_events_from_each() {
    let dir_a = common::fresh_tempdir();
    let dir_b = common::fresh_tempdir();
    let w = common::fresh_watcher();
    w.watch(dir_a.path()).expect("watch a");
    w.watch(dir_b.path()).expect("watch b");

    let mut rx = w.subscribe();

    let fa = dir_a.path().join("a.txt");
    let fb = dir_b.path().join("b.txt");
    std::fs::write(&fa, "a").unwrap();
    std::fs::write(&fb, "b").unwrap();

    // 5s 内同时拿到两条 Add（顺序不保证）
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
    let w = common::fresh_watcher();
    w.watch(dir.path()).expect("first ok");
    let err = w.watch(dir.path()).expect_err("expected AlreadyWatching");
    match err {
        WatcherError::AlreadyWatching(_) => {}
        other => panic!("expected AlreadyWatching, got {other:?}"),
    }
}

#[tokio::test]
async fn unwatch_stops_emitting_events() {
    let dir = common::fresh_tempdir();
    let w = common::fresh_watcher();
    w.watch(dir.path()).expect("watch");
    let mut rx = w.subscribe();

    // 立即 unwatch
    w.unwatch(dir.path()).expect("unwatch");

    let f = dir.path().join("after-unwatch.txt");
    std::fs::write(&f, "x").unwrap();

    // 1s 内不应再收到该路径事件
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
    let w = common::fresh_watcher();
    w.watch(dir_a.path()).unwrap();
    w.watch(dir_b.path()).unwrap();

    let paths = w.watched_paths();
    assert!(paths.iter().any(|p| p == dir_a.path()));
    assert!(paths.iter().any(|p| p == dir_b.path()));
    assert!(w.is_watching(dir_a.path()));
}
```

- [ ] **Step 2: 跑测试**

```
cargo test -p kfcode-watcher --test multi_path_errors
```

预期：5 条 pass。

- [ ] **Step 3: 提交**

```bash
git add crates/kfcode-watcher/tests/multi_path_errors.rs
git commit -m "test(watcher): cover multi-path watch, PathNotFound, AlreadyWatching, unwatch"
```
