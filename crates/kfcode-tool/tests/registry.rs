mod common;

use kfcode_tool::tool::ToolError;

#[tokio::test]
async fn list_returns_all_registered() {
    let r = common::fresh_default_registry().await;
    let ids = r.list_ids().await;
    let list = r.list().await;
    assert_eq!(list.len(), ids.len());
}

#[tokio::test]
async fn get_returns_some_for_known_tool() {
    let r = common::fresh_default_registry().await;
    assert!(r.get("read").await.is_some());
}

#[tokio::test]
async fn get_returns_none_for_unknown() {
    let r = common::fresh_default_registry().await;
    assert!(r.get("definitely-not-a-tool").await.is_none());
}

#[tokio::test]
async fn list_schemas_includes_known_tools() {
    let r = common::fresh_default_registry().await;
    let schemas = r.list_schemas().await;
    let names: Vec<&str> = schemas.iter().map(|s| s.name.as_str()).collect();
    for n in ["read", "write"] {
        assert!(names.contains(&n), "missing schema: {n}");
    }
}

#[tokio::test]
async fn execute_unknown_tool_returns_invalid_arguments_error() {
    let r = common::fresh_default_registry().await;
    let ws = common::fresh_workspace();
    let ctx = common::make_ctx(ws.path().to_str().unwrap());
    let res = r.execute("definitely-not-a-tool", serde_json::json!({}), ctx).await;
    let err = res.expect_err("expected error for unknown tool");
    match err {
        ToolError::InvalidArguments(msg) => {
            assert!(msg.contains("not found"), "got: {msg}");
        }
        other => panic!("expected InvalidArguments, got {other:?}"),
    }
}

#[tokio::test]
async fn suggest_tools_returns_nonempty_for_typo() {
    let r = common::fresh_default_registry().await;
    let suggestions = r.suggest_tools("reaad").await;
    assert!(!suggestions.is_empty(), "should suggest tools for typo");
}
