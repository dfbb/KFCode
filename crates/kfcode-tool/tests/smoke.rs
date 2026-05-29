mod common;

#[tokio::test]
async fn default_registry_has_builtin_tools() {
    let r = common::fresh_default_registry().await;
    let ids = r.list_ids().await;
    assert!(!ids.is_empty(), "default registry must have tools");
    for id in ["read", "write", "bash"] {
        assert!(ids.contains(&id.to_string()), "missing tool: {id}; got {ids:?}");
    }
}
