//! Structured logging types and tracing initialisation for the application.

use chrono::Local;
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Severity level for log messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    /// Verbose diagnostic information.
    Debug,
    /// General informational messages.
    Info,
    /// Potentially harmful situations that do not stop execution.
    Warn,
    /// Error conditions that may require attention.
    Error,
}

impl LogLevel {
    fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "DEBUG" => LogLevel::Debug,
            "INFO" => LogLevel::Info,
            "WARN" => LogLevel::Warn,
            "ERROR" => LogLevel::Error,
            _ => LogLevel::Info,
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

static LAST_TIMESTAMP: Mutex<i64> = Mutex::new(0);
static LOGGERS: LazyLock<Mutex<HashMap<String, CachedLogger>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

struct CachedLogger {
    tags: HashMap<String, String>,
    level: LogLevel,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
}

/// A logger that writes timestamped, tagged messages to a shared writer.
pub struct Logger {
    tags: HashMap<String, String>,
    level: LogLevel,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
}

impl Clone for Logger {
    fn clone(&self) -> Self {
        Self {
            tags: self.tags.clone(),
            level: self.level,
            writer: Arc::clone(&self.writer),
        }
    }
}

impl Logger {
    fn new(tags: HashMap<String, String>, writer: Arc<Mutex<Box<dyn Write + Send>>>) -> Self {
        let level = tags
            .get("level")
            .map(|l| LogLevel::from_str(l))
            .unwrap_or(LogLevel::Info);
        Self {
            tags,
            level,
            writer,
        }
    }

    fn should_log(&self, level: LogLevel) -> bool {
        let priority = match level {
            LogLevel::Debug => 0,
            LogLevel::Info => 1,
            LogLevel::Warn => 2,
            LogLevel::Error => 3,
        };
        let my_priority = match self.level {
            LogLevel::Debug => 0,
            LogLevel::Info => 1,
            LogLevel::Warn => 2,
            LogLevel::Error => 3,
        };
        priority >= my_priority
    }

    fn build_message(
        &self,
        level: LogLevel,
        message: &str,
        extra: Option<&HashMap<String, String>>,
    ) -> String {
        let mut last = LAST_TIMESTAMP.lock().unwrap();
        let now = Local::now();
        let timestamp = now.format("%Y-%m-%dT%H:%M:%S").to_string();
        let diff = now.timestamp_millis() - *last;
        *last = now.timestamp_millis();
        drop(last);

        let mut parts = vec![timestamp, format!("+{}ms", diff)];

        for (key, value) in &self.tags {
            parts.push(format!("{}={}", key, value));
        }

        if let Some(extra) = extra {
            for (key, value) in extra {
                parts.push(format!("{}={}", key, value));
            }
        }

        parts.push(message.to_string());

        format!("{} {}\n", level, parts.join(" "))
    }

    /// Emits a debug-level message with optional extra key-value pairs.
    pub fn debug(&self, message: &str, extra: Option<HashMap<String, String>>) {
        if self.should_log(LogLevel::Debug) {
            let msg = self.build_message(LogLevel::Debug, message, extra.as_ref());
            if let Ok(mut writer) = self.writer.lock() {
                let _ = writer.write_all(msg.as_bytes());
                let _ = writer.flush();
            }
        }
    }

    /// Emits an info-level message with optional extra key-value pairs.
    pub fn info(&self, message: &str, extra: Option<HashMap<String, String>>) {
        if self.should_log(LogLevel::Info) {
            let msg = self.build_message(LogLevel::Info, message, extra.as_ref());
            if let Ok(mut writer) = self.writer.lock() {
                let _ = writer.write_all(msg.as_bytes());
                let _ = writer.flush();
            }
        }
    }

    /// Emits a warn-level message with optional extra key-value pairs.
    pub fn warn(&self, message: &str, extra: Option<HashMap<String, String>>) {
        if self.should_log(LogLevel::Warn) {
            let msg = self.build_message(LogLevel::Warn, message, extra.as_ref());
            if let Ok(mut writer) = self.writer.lock() {
                let _ = writer.write_all(msg.as_bytes());
                let _ = writer.flush();
            }
        }
    }

    /// Emits an error-level message with optional extra key-value pairs.
    pub fn error(&self, message: &str, extra: Option<HashMap<String, String>>) {
        if self.should_log(LogLevel::Error) {
            let msg = self.build_message(LogLevel::Error, message, extra.as_ref());
            if let Ok(mut writer) = self.writer.lock() {
                let _ = writer.write_all(msg.as_bytes());
                let _ = writer.flush();
            }
        }
    }

    /// Returns a new `TaggedLogger` with an additional key-value tag attached to every message.
    pub fn tag(&self, key: &str, value: &str) -> TaggedLogger {
        let mut tags = self.tags.clone();
        tags.insert(key.to_string(), value.to_string());
        TaggedLogger {
            tags,
            writer: Arc::clone(&self.writer),
            level: self.level,
        }
    }
}

/// A `Logger` variant that carries a fixed set of extra key-value tags on every message.
pub struct TaggedLogger {
    tags: HashMap<String, String>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    level: LogLevel,
}

impl Clone for TaggedLogger {
    fn clone(&self) -> Self {
        Self {
            tags: self.tags.clone(),
            writer: Arc::clone(&self.writer),
            level: self.level,
        }
    }
}

impl TaggedLogger {
    /// Emits a debug-level message with optional extra key-value pairs.
    pub fn debug(&self, message: &str, extra: Option<HashMap<String, String>>) {
        if self.should_log(LogLevel::Debug) {
            let msg = self.build_message(LogLevel::Debug, message, extra.as_ref());
            if let Ok(mut writer) = self.writer.lock() {
                let _ = writer.write_all(msg.as_bytes());
                let _ = writer.flush();
            }
        }
    }

    /// Emits an info-level message with optional extra key-value pairs.
    pub fn info(&self, message: &str, extra: Option<HashMap<String, String>>) {
        if self.should_log(LogLevel::Info) {
            let msg = self.build_message(LogLevel::Info, message, extra.as_ref());
            if let Ok(mut writer) = self.writer.lock() {
                let _ = writer.write_all(msg.as_bytes());
                let _ = writer.flush();
            }
        }
    }

    /// Emits a warn-level message with optional extra key-value pairs.
    pub fn warn(&self, message: &str, extra: Option<HashMap<String, String>>) {
        if self.should_log(LogLevel::Warn) {
            let msg = self.build_message(LogLevel::Warn, message, extra.as_ref());
            if let Ok(mut writer) = self.writer.lock() {
                let _ = writer.write_all(msg.as_bytes());
                let _ = writer.flush();
            }
        }
    }

    /// Emits an error-level message with optional extra key-value pairs.
    pub fn error(&self, message: &str, extra: Option<HashMap<String, String>>) {
        if self.should_log(LogLevel::Error) {
            let msg = self.build_message(LogLevel::Error, message, extra.as_ref());
            if let Ok(mut writer) = self.writer.lock() {
                let _ = writer.write_all(msg.as_bytes());
                let _ = writer.flush();
            }
        }
    }

    fn should_log(&self, level: LogLevel) -> bool {
        let priority = match level {
            LogLevel::Debug => 0,
            LogLevel::Info => 1,
            LogLevel::Warn => 2,
            LogLevel::Error => 3,
        };
        let my_priority = match self.level {
            LogLevel::Debug => 0,
            LogLevel::Info => 1,
            LogLevel::Warn => 2,
            LogLevel::Error => 3,
        };
        priority >= my_priority
    }

    fn build_message(
        &self,
        level: LogLevel,
        message: &str,
        extra: Option<&HashMap<String, String>>,
    ) -> String {
        let mut last = LAST_TIMESTAMP.lock().unwrap();
        let now = Local::now();
        let timestamp = now.format("%Y-%m-%dT%H:%M:%S").to_string();
        let diff = now.timestamp_millis() - *last;
        *last = now.timestamp_millis();
        drop(last);

        let mut parts = vec![timestamp, format!("+{}ms", diff)];

        for (key, value) in &self.tags {
            parts.push(format!("{}={}", key, value));
        }

        if let Some(extra) = extra {
            for (key, value) in extra {
                parts.push(format!("{}={}", key, value));
            }
        }

        parts.push(message.to_string());

        format!("{} {}\n", level, parts.join(" "))
    }
}

/// Factory and lifecycle manager for named `Logger` instances.
pub struct Log;

impl Log {
    /// Creates or retrieves a cached `Logger` for the given tags.
    ///
    /// If a `"service"` tag is present and a logger for that service already exists,
    /// the cached instance is returned instead of creating a new one.
    pub fn create(tags: Option<HashMap<String, String>>) -> Logger {
        let tags = tags.unwrap_or_default();

        if let Some(service) = tags.get("service") {
            if let Ok(loggers) = LOGGERS.lock() {
                if let Some(cached) = loggers.get(service) {
                    return Logger {
                        tags: cached.tags.clone(),
                        level: cached.level,
                        writer: Arc::clone(&cached.writer),
                    };
                }
            }
        }

        let writer: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(Box::new(std::io::stderr())));
        let logger = Logger::new(tags.clone(), Arc::clone(&writer));

        if let Some(service) = tags.get("service") {
            if let Ok(mut loggers) = LOGGERS.lock() {
                loggers.insert(
                    service.to_string(),
                    CachedLogger {
                        tags: tags.clone(),
                        level: logger.level,
                        writer: Arc::clone(&writer),
                    },
                );
            }
        }

        logger
    }

    /// Returns the shared logger for the `"default"` service, creating it if necessary.
    pub fn default() -> Logger {
        let mut tags = HashMap::new();
        tags.insert("service".to_string(), "default".to_string());
        Self::create(Some(tags))
    }

    /// Initialises file-based logging, creating a timestamped log file in `log_dir`.
    ///
    /// When `print` is `true`, logging is suppressed and `None` is returned.
    /// Returns the path of the created log file on success.
    pub fn init(_level: Option<LogLevel>, log_dir: Option<PathBuf>, print: bool) -> Option<PathBuf> {
        if print {
            return None;
        }

        let log_dir = log_dir.unwrap_or_else(|| PathBuf::from("."));
        let timestamp = Local::now().format("%Y-%m-%dT%H%M%S").to_string();
        let log_path = log_dir.join(format!("{}.log", timestamp));

        if let Some(parent) = log_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&log_path)
            .ok();

        if let Some(file) = file {
            let writer: Arc<Mutex<Box<dyn Write + Send>>> = Arc::new(Mutex::new(Box::new(file)));
            if let Ok(mut loggers) = LOGGERS.lock() {
                for (_, cached) in loggers.iter_mut() {
                    cached.writer = Arc::clone(&writer);
                }
            }
        }

        Some(log_path)
    }
}

/// Initialises the `tracing` subscriber, directing output to a file or stderr.
///
/// When `print` is `true`, logs are written to stderr and `None` is returned.
/// Otherwise a timestamped log file is created in `log_dir` and its path is returned.
pub fn init_tracing(_level: Option<&str>, log_dir: Option<PathBuf>, print: bool) -> Option<PathBuf> {
    if print {
        let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer())
            .init();
        return None;
    }

    let log_dir = log_dir.unwrap_or_else(|| PathBuf::from("."));
    let timestamp = Local::now().format("%Y-%m-%dT%H%M%S").to_string();
    let log_path = log_dir.join(format!("{}.log", timestamp));

    if let Some(parent) = log_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_path)
        .ok();

    if let Some(file) = file {
        let (non_blocking, _guard) = tracing_appender::non_blocking(file);
        let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().with_writer(non_blocking))
            .init();
    }

    Some(log_path)
}
