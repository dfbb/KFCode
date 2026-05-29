#![allow(dead_code)]

use kfcode_command::{Command, CommandContext, CommandRegistry, CommandSource};
use std::path::PathBuf;
use tempfile::TempDir;

pub fn fresh_registry() -> CommandRegistry {
    CommandRegistry::new()
}

pub fn make_ctx(cwd: PathBuf) -> CommandContext {
    CommandContext::new(cwd)
}

pub fn fresh_workspace() -> TempDir {
    TempDir::new().expect("tempdir")
}

pub fn make_file_command(name: &str, template: &str, path: PathBuf) -> Command {
    Command {
        name: name.into(),
        description: format!("Test command {name}"),
        template: template.into(),
        source: CommandSource::File(path),
    }
}
