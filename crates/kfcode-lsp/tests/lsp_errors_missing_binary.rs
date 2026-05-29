use kfcode_lsp::{LspClient, LspError, LspServerConfig};
use tempfile::TempDir;

#[tokio::test]
async fn start_fails_when_binary_missing() {
    let root = TempDir::new().unwrap();
    let cfg = LspServerConfig {
        id: "missing".into(),
        command: "/path/that/does/not/exist".into(),
        args: vec![],
        initialization_options: None,
    };
    let res = LspClient::start(cfg, root.path().to_path_buf()).await;
    assert!(res.is_err(), "expected an error but got Ok");
    let err = res.err().unwrap();
    match err {
        LspError::ServerStartError(_) => {}
        other => panic!("expected ServerStartError, got {other:?}"),
    }
}
