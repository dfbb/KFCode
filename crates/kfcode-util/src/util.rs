//! Miscellaneous utility sub-modules: wildcard matching, ANSI color helpers, async timeout,
//! deferred cleanup, async locks, token estimation, formatting, git operations, and abort control.

/// Glob-pattern matching helpers.
pub mod wildcard {
    use glob::Pattern;

    /// Returns `true` if `text` matches the glob `pattern`.
    pub fn matches(pattern: &str, text: &str) -> bool {
        Pattern::new(pattern)
            .map(|p| p.matches(text))
            .unwrap_or(false)
    }

    /// Returns `true` if `text` matches any of the provided glob `patterns`.
    pub fn matches_any(patterns: &[&str], text: &str) -> bool {
        patterns.iter().any(|p| matches(p, text))
    }

    /// Returns the subset of `items` that match the glob `pattern`.
    pub fn filter<'a>(pattern: &str, items: &'a [&str]) -> Vec<&'a str> {
        items
            .iter()
            .filter(|s| matches(pattern, s))
            .copied()
            .collect()
    }
}

/// ANSI terminal color and escape-sequence helpers.
pub mod color {
    /// Removes ANSI SGR escape sequences from `s`, returning plain text.
    pub fn strip_ansi(s: &str) -> String {
        let re = regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap();
        re.replace_all(s, "").to_string()
    }

    /// Returns the visible character length of `s` after stripping ANSI escape sequences.
    pub fn ansi_length(s: &str) -> usize {
        strip_ansi(s).len()
    }
}

/// Async timeout wrapper.
pub mod timeout {
    use std::time::Duration;
    use tokio::time::timeout;

    /// Runs `future` and returns its output wrapped in `Some`, or `None` if `duration` elapses first.
    pub async fn with_timeout<T, F>(duration: Duration, future: F) -> Option<T>
    where
        F: std::future::Future<Output = T>,
    {
        timeout(duration, future).await.ok()
    }
}

/// RAII deferred execution — runs a closure when the guard is dropped.
pub mod defer {
    /// A guard that calls its closure exactly once when it goes out of scope.
    pub struct Defer<F: FnOnce()> {
        f: Option<F>,
    }

    impl<F: FnOnce()> Defer<F> {
        /// Creates a new `Defer` guard that will call `f` on drop.
        pub fn new(f: F) -> Self {
            Self { f: Some(f) }
        }
    }

    impl<F: FnOnce()> Drop for Defer<F> {
        fn drop(&mut self) {
            if let Some(f) = self.f.take() {
                f();
            }
        }
    }

    #[macro_export]
    macro_rules! defer {
        ($($body:expr),*) => {
            let _guard = $crate::defer::Defer::new(move || { $($body);* });
        };
    }
}

/// Async mutex helpers built on `tokio::sync::Mutex`.
pub mod lock {
    use std::sync::Arc;
    use tokio::sync::Mutex;

    /// A reference-counted async mutex wrapping a value of type `T`.
    pub type AsyncLock<T> = Arc<Mutex<T>>;

    /// Wraps `value` in an `AsyncLock`, making it ready for shared async access.
    pub fn new<T: Send + 'static>(value: T) -> AsyncLock<T> {
        Arc::new(Mutex::new(value))
    }
}

/// Rough token-count estimation for LLM context budgeting.
pub mod token {
    /// Assumed average number of UTF-8 bytes per token.
    const CHARS_PER_TOKEN: usize = 4;

    /// Estimates the number of tokens in `input` using a fixed bytes-per-token ratio.
    pub fn estimate(input: &str) -> usize {
        if input.is_empty() {
            return 0;
        }
        input.len() / CHARS_PER_TOKEN
    }

    /// Estimates the total token count across a slice of message strings.
    pub fn estimate_messages(messages: &[&str]) -> usize {
        messages.iter().map(|m| estimate(m)).sum()
    }
}

/// Human-readable formatting for durations, byte sizes, and large numbers.
pub mod format {
    /// Formats a duration in seconds as a compact human-readable string (e.g. `"1h 30m"`, `"~2 days"`).
    pub fn format_duration(secs: u64) -> String {
        if secs == 0 {
            return String::new();
        }
        if secs < 60 {
            return format!("{}s", secs);
        }
        if secs < 3600 {
            let mins = secs / 60;
            let remaining = secs % 60;
            if remaining > 0 {
                format!("{}m {}s", mins, remaining)
            } else {
                format!("{}m", mins)
            }
        } else if secs < 86400 {
            let hours = secs / 3600;
            let remaining = (secs % 3600) / 60;
            if remaining > 0 {
                format!("{}h {}m", hours, remaining)
            } else {
                format!("{}h", hours)
            }
        } else if secs < 604800 {
            let days = secs / 86400;
            if days == 1 {
                "~1 day".to_string()
            } else {
                format!("~{} days", days)
            }
        } else {
            let weeks = secs / 604800;
            if weeks == 1 {
                "~1 week".to_string()
            } else {
                format!("~{} weeks", weeks)
            }
        }
    }

    /// Formats a byte count as a human-readable string with appropriate unit (B, KB, MB, GB).
    pub fn format_bytes(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.1} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.1} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.1} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }

    /// Formats a large integer as a compact string with K or M suffix (e.g. `"1.5K"`, `"2.3M"`).
    pub fn format_number(n: u64) -> String {
        if n >= 1_000_000 {
            format!("{:.1}M", n as f64 / 1_000_000.0)
        } else if n >= 1_000 {
            format!("{:.1}K", n as f64 / 1_000.0)
        } else {
            n.to_string()
        }
    }
}

/// Thin wrappers around `git` CLI commands for repository inspection.
pub mod git {
    use std::path::Path;
    use std::process::Command;

    /// The output of a `git` command invocation.
    pub struct GitResult {
        /// The process exit code; `0` indicates success.
        pub exit_code: i32,
        /// Captured standard output as a UTF-8 string.
        pub stdout: String,
        /// Captured standard error as a UTF-8 string.
        pub stderr: String,
    }

    impl GitResult {
        /// Returns the stdout output as a string slice.
        pub fn text(&self) -> &str {
            &self.stdout
        }

        /// Returns `true` if the command exited with code `0`.
        pub fn success(&self) -> bool {
            self.exit_code == 0
        }
    }

    /// Runs a `git` command with the given `args` in `cwd` and returns the captured output.
    pub fn run(args: &[&str], cwd: &Path) -> GitResult {
        let output = Command::new("git").args(args).current_dir(cwd).output();

        match output {
            Ok(output) => GitResult {
                exit_code: output.status.code().unwrap_or(1),
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            },
            Err(e) => GitResult {
                exit_code: 1,
                stdout: String::new(),
                stderr: e.to_string(),
            },
        }
    }

    /// Returns `true` if `path` contains a `.git` directory, indicating a git repository root.
    pub fn is_repo(path: &Path) -> bool {
        path.join(".git").exists()
    }

    /// Returns the absolute path of the repository root containing `path`, or `None` if not in a repo.
    pub fn get_root(path: &Path) -> Option<std::path::PathBuf> {
        let result = run(&["rev-parse", "--show-toplevel"], path);
        if result.success() {
            Some(std::path::PathBuf::from(result.stdout.trim()))
        } else {
            None
        }
    }

    /// Returns the name of the currently checked-out branch, or `None` if in detached HEAD state.
    pub fn get_current_branch(path: &Path) -> Option<String> {
        let result = run(&["branch", "--show-current"], path);
        if result.success() {
            Some(result.stdout.trim().to_string())
        } else {
            None
        }
    }

    /// Returns the fetch URL of the `origin` remote, or `None` if no remote is configured.
    pub fn get_remote_url(path: &Path) -> Option<String> {
        let result = run(&["remote", "get-url", "origin"], path);
        if result.success() {
            Some(result.stdout.trim().to_string())
        } else {
            None
        }
    }

    /// Returns the full SHA of the current HEAD commit, or `None` on failure.
    pub fn get_head_commit(path: &Path) -> Option<String> {
        let result = run(&["rev-parse", "HEAD"], path);
        if result.success() {
            Some(result.stdout.trim().to_string())
        } else {
            None
        }
    }

    /// Returns the porcelain status lines for the working tree, or an empty vec on failure.
    pub fn get_status(path: &Path) -> Vec<String> {
        let result = run(&["status", "--porcelain"], path);
        if result.success() {
            result.stdout.lines().map(|s| s.to_string()).collect()
        } else {
            Vec::new()
        }
    }

    /// Returns `true` if the working tree has any uncommitted changes.
    pub fn has_uncommitted_changes(path: &Path) -> bool {
        !get_status(path).is_empty()
    }
}

/// Cooperative cancellation via a shared atomic flag.
pub mod abort {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    /// A cloneable cancellation token that can be signalled from any thread.
    #[derive(Clone)]
    pub struct AbortController {
        cancelled: Arc<AtomicBool>,
    }

    impl AbortController {
        /// Creates a new, unsignalled `AbortController`.
        pub fn new() -> Self {
            Self {
                cancelled: Arc::new(AtomicBool::new(false)),
            }
        }

        /// Signals cancellation; all clones of this controller will observe the change.
        pub fn abort(&self) {
            self.cancelled.store(true, Ordering::SeqCst);
        }

        /// Returns `true` if `abort` has been called on any clone of this controller.
        pub fn is_cancelled(&self) -> bool {
            self.cancelled.load(Ordering::SeqCst)
        }
    }

    impl Default for AbortController {
        fn default() -> Self {
            Self::new()
        }
    }

    /// Returns `true` if the given `controller` has been cancelled.
    pub fn aborted(controller: &AbortController) -> bool {
        controller.is_cancelled()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_estimate() {
        assert_eq!(token::estimate(""), 0);
        assert_eq!(token::estimate("hello"), 1);
        assert_eq!(token::estimate("hello world"), 2);
        assert_eq!(token::estimate("a".repeat(100).as_str()), 25);
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format::format_duration(0), "");
        assert_eq!(format::format_duration(30), "30s");
        assert_eq!(format::format_duration(90), "1m 30s");
        assert_eq!(format::format_duration(3600), "1h");
        assert_eq!(format::format_duration(3661), "1h 1m");
        assert_eq!(format::format_duration(86400), "~1 day");
        assert_eq!(format::format_duration(172800), "~2 days");
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format::format_bytes(500), "500 B");
        assert_eq!(format::format_bytes(1024), "1.0 KB");
        assert_eq!(format::format_bytes(1048576), "1.0 MB");
        assert_eq!(format::format_bytes(1073741824), "1.0 GB");
    }

    #[test]
    fn test_wildcard() {
        assert!(wildcard::matches("*.rs", "main.rs"));
        assert!(!wildcard::matches("*.rs", "main.ts"));
        assert!(wildcard::matches_any(&["*.rs", "*.ts"], "main.ts"));
    }

    #[test]
    fn test_color() {
        let input = "\x1b[32mhello\x1b[0m";
        assert_eq!(color::strip_ansi(input), "hello");
        assert_eq!(color::ansi_length(input), 5);
    }
}
