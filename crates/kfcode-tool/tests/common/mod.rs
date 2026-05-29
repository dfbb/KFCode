#![allow(dead_code)]

use kfcode_tool::registry::{create_default_registry, ToolRegistry};
use kfcode_tool::tool::ToolContext;
use tempfile::TempDir;

pub async fn fresh_default_registry() -> ToolRegistry {
    create_default_registry().await
}

pub fn make_ctx(directory: &str) -> ToolContext {
    ToolContext::new(
        "ses-test".into(),
        "msg-test".into(),
        directory.into(),
    )
}

pub fn fresh_workspace() -> TempDir {
    TempDir::new().expect("tempdir")
}
