#![allow(dead_code)]

use kfcode_lsp::{LspClient, LspError, LspServerConfig};
use tempfile::TempDir;

pub fn stub_path() -> &'static str {
    env!("CARGO_BIN_EXE_lsp-test-stub")
}

pub async fn start_default_stub() -> (LspClient, TempDir) {
    let root_dir = TempDir::new().unwrap();
    let cfg = LspServerConfig {
        id: "stub".into(),
        command: stub_path().into(),
        args: vec![],
        initialization_options: None,
    };
    let client = LspClient::start(cfg, root_dir.path().to_path_buf())
        .await
        .expect("start stub");
    (client, root_dir)
}

pub async fn start_stub_with_mode(mode: &str) -> Result<(LspClient, TempDir), LspError> {
    std::env::set_var("STUB_MODE", mode);
    let root_dir = TempDir::new().unwrap();
    let cfg = LspServerConfig {
        id: "stub".into(),
        command: stub_path().into(),
        args: vec![],
        initialization_options: None,
    };
    let client = LspClient::start(cfg, root_dir.path().to_path_buf()).await?;
    Ok((client, root_dir))
}
