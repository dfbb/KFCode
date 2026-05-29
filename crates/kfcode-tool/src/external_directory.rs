//! Utilities for detecting and requesting permission to access paths outside the project root.

use crate::{PermissionRequest, ToolContext, ToolError};
use std::path::Path;

/// Options controlling how an external directory permission check is performed.
#[derive(Debug, Clone)]
pub struct ExternalDirectoryOptions {
    /// When `true`, skip the permission check entirely.
    pub bypass: bool,
    /// Whether the target path is a file or a directory.
    pub kind: ExternalDirectoryKind,
}

impl Default for ExternalDirectoryOptions {
    fn default() -> Self {
        Self {
            bypass: false,
            kind: ExternalDirectoryKind::File,
        }
    }
}

/// Distinguishes whether a target path refers to a file or a directory.
#[derive(Debug, Clone, Copy)]
pub enum ExternalDirectoryKind {
    /// The target is a regular file.
    File,
    /// The target is a directory.
    Directory,
}

/// Checks whether `target` is outside the project root and, if so, requests permission.
pub async fn assert_external_directory(
    ctx: &ToolContext,
    target: Option<&str>,
    options: ExternalDirectoryOptions,
) -> Result<(), ToolError> {
    let target = match target {
        Some(t) => t,
        None => return Ok(()),
    };

    if options.bypass {
        return Ok(());
    }

    if is_within_project(target, &ctx.project_root) {
        return Ok(());
    }

    let parent_dir = match options.kind {
        ExternalDirectoryKind::Directory => target.to_string(),
        ExternalDirectoryKind::File => Path::new(target)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| target.to_string()),
    };

    let glob_pattern = format!("{}/*", parent_dir);

    let request = PermissionRequest::new("external_directory")
        .with_pattern(&glob_pattern)
        .with_metadata("filepath", serde_json::json!(target))
        .with_metadata("parentDir", serde_json::json!(&parent_dir));

    ctx.ask_permission(request).await
}

fn is_within_project(target: &str, project_root: &str) -> bool {
    let target_path = Path::new(target);

    if target_path.is_absolute() {
        return target_path.starts_with(project_root);
    }

    if target.starts_with("./") || target.starts_with("../") {
        return true;
    }

    if !target.starts_with('/') && !target.contains(':') {
        return true;
    }

    false
}

/// Returns the parent directory of `target` according to its kind.
pub fn get_parent_directory(target: &str, kind: ExternalDirectoryKind) -> String {
    match kind {
        ExternalDirectoryKind::Directory => target.to_string(),
        ExternalDirectoryKind::File => Path::new(target)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| target.to_string()),
    }
}

/// Builds a glob pattern that matches all direct children of `parent_dir`.
pub fn make_glob_pattern(parent_dir: &str) -> String {
    format!("{}/*", parent_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_within_project_relative() {
        assert!(is_within_project("./src/main.rs", "/home/user/project"));
        assert!(is_within_project("../other/file.txt", "/home/user/project"));
        assert!(is_within_project("src/lib.rs", "/home/user/project"));
    }

    #[test]
    fn test_is_within_project_absolute() {
        assert!(is_within_project(
            "/home/user/project/src/main.rs",
            "/home/user/project"
        ));
        assert!(!is_within_project(
            "/home/other/file.txt",
            "/home/user/project"
        ));
    }

    #[test]
    fn test_get_parent_directory_file() {
        let parent =
            get_parent_directory("/home/user/project/file.txt", ExternalDirectoryKind::File);
        assert_eq!(parent, "/home/user/project");
    }

    #[test]
    fn test_get_parent_directory_dir() {
        let parent = get_parent_directory("/home/user/project", ExternalDirectoryKind::Directory);
        assert_eq!(parent, "/home/user/project");
    }

    #[test]
    fn test_make_glob_pattern() {
        let pattern = make_glob_pattern("/home/user/external");
        assert_eq!(pattern, "/home/user/external/*");
    }
}
