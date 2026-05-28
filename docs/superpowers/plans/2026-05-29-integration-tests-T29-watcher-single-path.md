# T29 — watcher 单路径 create/modify/delete

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 验证 `FileWatcher::watch(dir)` 后，对 `dir` 内文件做 create/modify/delete 能在 broadcast 收到对应 `WatcherEvent`。

**Architecture:** 用 `subscribe()` 拿 `broadcast::Receiver`；用 `tokio::time::timeout` 限 5 秒等事件，避免文件系统时序在 macOS 偶发 flaky（spec §3.2 watcher 切面允许默认不 ignore，跑出 flaky 再标）。

**Tech Stack:** notify / tokio broadcast / tempfile。

**依赖:** T28

---

### Task 4.1：watcher 单路径黄金路径

**Files:**
- Create: `crates/kfcode-watcher/tests/single_path.rs`

- [ ] **Step 1: 写测试**

写入 `crates/kfcode-watcher/tests/single_path.rs`：

```rust
mod common;

use kfcode_watcher::{FileEvent, WatcherEvent};
use std::time::Duration;
use tokio::time::timeout;

async fn drain_until<F>(rx: &mut tokio::sync::broadcast::Receiver<WatcherEvent>, pred: F) -> WatcherEvent
where
    F: Fn(&WatcherEvent) -> bool,
{
    let deadline = Duration::from_secs(5);
    let start = std::time::Instant::now();
    loop {
        let remaining = deadline.checked_sub(start.elapsed()).unwrap_or(Duration::from_millis(0));
        match timeout(remaining, rx.recv()).await {
            Ok(Ok(ev)) if pred(&ev) => return ev,
            Ok(Ok(_)) => continue,
            Ok(Err(_)) | Err(_) => panic!("did not receive expected event in time"),
        }
    }
}

#[tokio::test]
async fn detects_file_create() {
    let dir = common::fresh_tempdir();
    let w = common::fresh_watcher();
    w.watch(dir.path()).expect("watch");
    let mut rx = w.subscribe();

    let f = dir.path().join("new.txt");
    std::fs::write(&f, "hello").unwrap();

    let ev = drain_until(&mut rx, |e| e.file == f && e.event == FileEvent::Add).await;
    assert_eq!(ev.event, FileEvent::Add);
}

#[tokio::test]
async fn detects_file_modify() {
    let dir = common::fresh_tempdir();
    let f = dir.path().join("a.txt");
    std::fs::write(&f, "v1").unwrap();

    let w = common::fresh_watcher();
    w.watch(dir.path()).expect("watch");
    let mut rx = w.subscribe();

    // 给 watcher 一点时间稳定再写
    tokio::time::sleep(Duration::from_millis(150)).await;
    std::fs::write(&f, "v2").unwrap();

    let ev = drain_until(&mut rx, |e| e.file == f && e.event == FileEvent::Change).await;
    assert_eq!(ev.event, FileEvent::Change);
}

#[tokio::test]
async fn detects_file_delete() {
    let dir = common::fresh_tempdir();
    let f = dir.path().join("a.txt");
    std::fs::write(&f, "v1").unwrap();

    let w = common::fresh_watcher();
    w.watch(dir.path()).expect("watch");
    let mut rx = w.subscribe();

    tokio::time::sleep(Duration::from_millis(150)).await;
    std::fs::remove_file(&f).unwrap();

    let ev = drain_until(&mut rx, |e| e.file == f && e.event == FileEvent::Unlink).await;
    assert_eq!(ev.event, FileEvent::Unlink);
}

#[tokio::test]
async fn ignored_pattern_filters_event() {
    let dir = common::fresh_tempdir();
    let w = common::fresh_watcher();
    w.watch(dir.path()).expect("watch");
    let mut rx = w.subscribe();

    let git_dir = dir.path().join(".git");
    std::fs::create_dir(&git_dir).unwrap();
    std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main").unwrap();

    // .git/** 默认在 ignore_patterns 里。500ms 内不应收到该路径事件
    let res = timeout(Duration::from_millis(500), async {
        loop {
            let ev = rx.recv().await.unwrap();
            if ev.file.starts_with(&git_dir) {
                return ev;
            }
        }
    })
    .await;
    assert!(res.is_err(), ".git events should be filtered");
}
```

- [ ] **Step 2: 跑测试**

```
cargo test -p kfcode-watcher --test single_path
```

预期：4 条 pass。**如某条偶发 flaky**（macOS 文件事件时序），按 spec §2.6 标 `#[ignore = "macOS fs event timing flaky on CI; tracked in <issue>"]`。

- [ ] **Step 3: 提交**

```bash
git add crates/kfcode-watcher/tests/single_path.rs
git commit -m "test(watcher): cover single-path create/modify/delete and ignore pattern"
```
