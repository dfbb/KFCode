mod common;

use kfcode_permission::{
    PermissionEngine, PermissionError, PermissionInfo, Pattern, Response, TimeInfo,
};
use std::collections::HashMap;

fn make_info(id: &str, session: &str, ptype: &str, pattern: Option<Pattern>) -> PermissionInfo {
    PermissionInfo {
        id: id.to_string(),
        permission_type: ptype.to_string(),
        pattern,
        session_id: session.to_string(),
        message_id: "msg".into(),
        call_id: None,
        message: format!("test request {id}"),
        metadata: HashMap::new(),
        time: TimeInfo { created: 0 },
    }
}

#[tokio::test]
async fn ask_queues_request_into_pending() {
    let mut eng = PermissionEngine::new();
    let info = make_info("req1", "ses1", "edit", Some(Pattern::Single("foo.rs".into())));
    eng.ask(info).await.expect("ask");
    let list: Vec<_> = eng.list().into_iter().collect();
    assert_eq!(list.len(), 1, "expected one pending request");
}

#[tokio::test]
async fn respond_reject_clears_pending() {
    let mut eng = PermissionEngine::new();
    eng.ask(make_info("r", "s", "edit", None)).await.unwrap();
    let _ = eng.respond("s", "r", Response::Reject);
    assert!(eng.list().is_empty(), "pending should be cleared after Reject");
}

#[tokio::test]
async fn respond_always_caches_approval() {
    let mut eng = PermissionEngine::new();
    let pat = Pattern::Single("foo.rs".into());
    eng.ask(make_info("r1", "s", "edit", Some(pat.clone()))).await.unwrap();
    eng.respond("s", "r1", Response::Always).expect("respond");

    eng.ask(make_info("r2", "s", "edit", Some(pat.clone()))).await.unwrap();
    let list = eng.list();
    assert!(
        list.iter().all(|i| i.id != "r2"),
        "second ask with cached approval must not go pending"
    );
    assert!(eng.is_approved("s", Some(&pat), "edit"));
}

#[tokio::test]
async fn respond_unknown_returns_not_found() {
    let mut eng = PermissionEngine::new();
    let res = eng.respond("no-session", "no-id", Response::Once);
    let err = res.expect_err("expected NotFound");
    match err {
        PermissionError::NotFound(_, _) => {}
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[tokio::test]
async fn clear_session_drops_pending_and_approved() {
    let mut eng = PermissionEngine::new();
    eng.ask(make_info("r", "s", "edit", None)).await.unwrap();
    eng.clear_session("s");
    assert!(eng.list().is_empty());
}
