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
    // Canonicalize the dir (resolves /var -> /private/var on macOS), then join filename.
    let base = common::canonical_dir(dir.path());
    let w = common::fresh_watcher();
    w.watch(&base).expect("watch");
    let mut rx = w.subscribe();

    let f = base.join("new.txt");
    std::fs::write(&f, "hello").unwrap();

    let ev = drain_until(&mut rx, |e| e.file == f && e.event == FileEvent::Add).await;
    assert_eq!(ev.event, FileEvent::Add);
}

#[tokio::test]
async fn detects_file_modify() {
    let dir = common::fresh_tempdir();
    let base = common::canonical_dir(dir.path());
    let f = base.join("a.txt");
    std::fs::write(&f, "v1").unwrap();

    let w = common::fresh_watcher();
    w.watch(&base).expect("watch");
    let mut rx = w.subscribe();

    tokio::time::sleep(Duration::from_millis(150)).await;
    std::fs::write(&f, "v2").unwrap();

    let ev = drain_until(&mut rx, |e| e.file == f && e.event == FileEvent::Change).await;
    assert_eq!(ev.event, FileEvent::Change);
}

#[tokio::test]
async fn detects_file_delete() {
    let dir = common::fresh_tempdir();
    let base = common::canonical_dir(dir.path());
    let f = base.join("a.txt");
    std::fs::write(&f, "v1").unwrap();

    let w = common::fresh_watcher();
    w.watch(&base).expect("watch");
    let mut rx = w.subscribe();

    tokio::time::sleep(Duration::from_millis(150)).await;
    std::fs::remove_file(&f).unwrap();

    let ev = drain_until(&mut rx, |e| e.file == f && e.event == FileEvent::Unlink).await;
    assert_eq!(ev.event, FileEvent::Unlink);
}

#[tokio::test]
async fn ignored_pattern_filters_event() {
    let dir = common::fresh_tempdir();
    let base = common::canonical_dir(dir.path());
    let w = common::fresh_watcher();
    w.watch(&base).expect("watch");
    let mut rx = w.subscribe();

    let git_dir = base.join(".git");
    std::fs::create_dir(&git_dir).unwrap();
    // HEAD is inside .git/ — matches **/.git/** and must be filtered.
    let head = git_dir.join("HEAD");
    std::fs::write(&head, "ref: refs/heads/main").unwrap();

    let res = timeout(Duration::from_millis(500), async {
        loop {
            let ev = rx.recv().await.unwrap();
            // Only fail if we see an event for a file *inside* .git/
            if ev.file == head {
                return ev;
            }
        }
    })
    .await;
    assert!(res.is_err(), ".git/HEAD event should be filtered by **/.git/** pattern");
}
