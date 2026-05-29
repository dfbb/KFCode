mod common;

use std::sync::Arc;
use kfcode_tool::tool::{PermissionRequest, ToolError};

fn deny_callback() -> kfcode_tool::tool::AskCallback {
    Arc::new(|_req: PermissionRequest| {
        Box::pin(async move {
            Err::<(), _>(ToolError::PermissionDenied("denied by test".into()))
        })
    })
}

fn allow_callback() -> kfcode_tool::tool::AskCallback {
    Arc::new(|_req: PermissionRequest| {
        Box::pin(async move { Ok::<(), ToolError>(()) })
    })
}

#[tokio::test]
async fn no_callback_defaults_to_allow() {
    let r = common::fresh_default_registry().await;
    let ws = common::fresh_workspace();
    let target = ws.path().join("default.txt");

    let ctx = common::make_ctx(ws.path().to_str().unwrap());
    r.execute(
        "write",
        serde_json::json!({
            "file_path": target.to_str().unwrap(),
            "content": "x"
        }),
        ctx,
    )
    .await
    .expect("default ctx should allow");
    assert!(target.exists());
}

#[tokio::test]
async fn deny_callback_blocks_write() {
    let r = common::fresh_default_registry().await;
    let ws = common::fresh_workspace();
    let target = ws.path().join("denied.txt");

    let mut ctx = common::make_ctx(ws.path().to_str().unwrap());
    ctx.ask = Some(deny_callback());

    let res = r
        .execute(
            "write",
            serde_json::json!({
                "file_path": target.to_str().unwrap(),
                "content": "should not be written"
            }),
            ctx,
        )
        .await;

    assert!(res.is_err(), "deny callback should block write");
    match res.unwrap_err() {
        ToolError::PermissionDenied(msg) => {
            assert!(msg.contains("denied by test"), "got: {msg}");
        }
        other => panic!("expected PermissionDenied, got {other:?}"),
    }
    assert!(!target.exists(), "file should not have been created");
}

#[tokio::test]
async fn allow_callback_permits_write() {
    let r = common::fresh_default_registry().await;
    let ws = common::fresh_workspace();
    let target = ws.path().join("allowed.txt");

    let mut ctx = common::make_ctx(ws.path().to_str().unwrap());
    ctx.ask = Some(allow_callback());

    r.execute(
        "write",
        serde_json::json!({
            "file_path": target.to_str().unwrap(),
            "content": "allowed"
        }),
        ctx,
    )
    .await
    .expect("allow callback should permit write");
    assert!(target.exists());
}
