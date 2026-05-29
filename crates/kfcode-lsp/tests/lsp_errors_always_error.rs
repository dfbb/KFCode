#[path = "common/mod.rs"]
mod common;

use kfcode_lsp::LspError;

#[tokio::test]
async fn initialize_fails_when_stub_returns_error() {
    let res = common::start_stub_with_mode("always-error").await;
    assert!(res.is_err(), "initialize should propagate JSON-RPC error");
    let err = res.err().unwrap();
    match err {
        LspError::InitializeError(_) | LspError::JsonRpcError(_) => {}
        other => panic!("expected InitializeError/JsonRpcError, got {other:?}"),
    }
}
