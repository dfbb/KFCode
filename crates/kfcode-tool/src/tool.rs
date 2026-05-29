//! Core types shared across all tools: `Tool` trait, `ToolContext`, `ToolResult`, and `ToolError`.
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::ToolRegistry;

#[cfg(feature = "lsp")]
use kfcode_lsp::LspClientRegistry;

/// Convenience alias for a map of arbitrary JSON metadata attached to a `ToolResult`.
pub type Metadata = HashMap<String, serde_json::Value>;

static FILE_LOCKS: std::sync::OnceLock<Arc<std::sync::Mutex<HashMap<String, Arc<Mutex<()>>>>>> =
    std::sync::OnceLock::new();

fn get_file_locks() -> Arc<std::sync::Mutex<HashMap<String, Arc<Mutex<()>>>>> {
    FILE_LOCKS
        .get_or_init(|| Arc::new(std::sync::Mutex::new(HashMap::new())))
        .clone()
}

/// Acquires a per-file async mutex, runs `f`, then releases the lock.
pub async fn with_file_lock<F, Fut, T>(filepath: &str, f: F) -> T
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = T>,
{
    let lock = {
        let locks = get_file_locks();
        let mut locks_guard = locks.lock().unwrap();
        locks_guard
            .entry(filepath.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    };

    let _guard = lock.lock().await;
    f().await
}

/// Definition of a single question presented to the user, with optional multiple-choice options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionDef {
    pub question: String,
    #[serde(default)]
    pub header: Option<String>,
    #[serde(default)]
    pub options: Vec<QuestionOption>,
    #[serde(default)]
    pub multiple: bool,
}

/// A selectable option within a `QuestionDef`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionOption {
    pub label: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// Async callback used to request a permission check before a tool action.
pub type AskCallback = Arc<
    dyn (Fn(
            PermissionRequest,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), ToolError>> + Send>>)
        + Send
        + Sync,
>;

/// Async callback used to present questions to the user and collect answers.
pub type QuestionCallback = Arc<
    dyn (Fn(
            Vec<QuestionDef>,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<Vec<Vec<String>>, ToolError>> + Send>,
        >) + Send
        + Sync,
>;

/// Async callback used to switch the active agent to a different role.
pub type SwitchAgentCallback = Arc<
    dyn (Fn(
            String,
            Option<String>,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), ToolError>> + Send>>)
        + Send
        + Sync,
>;

/// Async callback used to create a new subagent session and return its ID.
pub type CreateSubsessionCallback = Arc<
    dyn (Fn(
            String,
            Option<String>,
            Option<String>,
            Vec<String>,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, ToolError>> + Send>>)
        + Send
        + Sync,
>;
/// Async callback used to send a prompt to an existing subagent session.
pub type PromptSubsessionCallback = Arc<
    dyn (Fn(
            String,
            String,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, ToolError>> + Send>>)
        + Send
        + Sync,
>;

/// Async callback used to assert that a file has not been modified since it was last read.
pub type FileTimeAssertCallback = Arc<
    dyn (Fn(
            String,
            String,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), ToolError>> + Send>>)
        + Send
        + Sync,
>;

/// Async callback used to record that a file was read at the current time.
pub type FileTimeReadCallback = Arc<
    dyn (Fn(
            String,
            String,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), ToolError>> + Send>>)
        + Send
        + Sync,
>;

/// Async callback used to publish an event to the internal message bus.
pub type PublishBusCallback = Arc<
    dyn (Fn(
            String,
            serde_json::Value,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>)
        + Send
        + Sync,
>;

/// Async callback used to persist a message part to the session store.
pub type UpdatePartCallback = Arc<
    dyn (Fn(
            serde_json::Value,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), ToolError>> + Send>>)
        + Send
        + Sync,
>;

/// Async callback used to persist a message to the session store.
pub type UpdateMessageCallback = Arc<
    dyn (Fn(
            serde_json::Value,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), ToolError>> + Send>>)
        + Send
        + Sync,
>;

/// Async callback used to notify the LSP client that a file was opened or written.
pub type LspTouchFileCallback = Arc<
    dyn (Fn(
            String,
            bool,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), ToolError>> + Send>>)
        + Send
        + Sync,
>;

/// Data for a single todo item transferred between the tool and the session store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItemData {
    pub content: String,
    pub status: String,
    pub priority: String,
}

/// Async callback used to replace the session's todo list.
pub type TodoUpdateCallback = Arc<
    dyn (Fn(
            String,
            Vec<TodoItemData>,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), ToolError>> + Send>>)
        + Send
        + Sync,
>;

/// Async callback used to retrieve the session's current todo list.
pub type TodoGetCallback = Arc<
    dyn (Fn(
            String,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<Vec<TodoItemData>, ToolError>> + Send>,
        >) + Send
        + Sync,
>;

/// Async callback used to retrieve the model identifier last used in the session.
pub type GetLastModelCallback = Arc<
    dyn (Fn(
            String,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<Option<String>, ToolError>> + Send>,
        >) + Send
        + Sync,
>;

/// Async callback used to inject a synthetic user message into the session.
pub type CreateSyntheticMessageCallback = Arc<
    dyn (Fn(
            String,
            Option<String>,
            String,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), ToolError>> + Send>>)
        + Send
        + Sync,
>;

/// Describes a permission check that must be approved before a tool action proceeds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRequest {
    pub permission: String,
    pub patterns: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub always: Vec<String>,
}

impl PermissionRequest {
    /// Creates a new `PermissionRequest` for the given permission name.
    pub fn new(permission: impl Into<String>) -> Self {
        Self {
            permission: permission.into(),
            patterns: Vec::new(),
            metadata: HashMap::new(),
            always: Vec::new(),
        }
    }

    /// Appends a glob pattern to the permission request.
    pub fn with_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.patterns.push(pattern.into());
        self
    }

    /// Replaces all patterns with the given list.
    pub fn with_patterns(mut self, patterns: Vec<String>) -> Self {
        self.patterns = patterns;
        self
    }

    /// Attaches an arbitrary metadata key-value pair to the request.
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Adds a pattern that should always be auto-approved.
    pub fn with_always(mut self, always: impl Into<String>) -> Self {
        self.always.push(always.into());
        self
    }

    /// Marks the request as always auto-approved by adding a wildcard pattern.
    pub fn always_allow(mut self) -> Self {
        self.always.push("*".to_string());
        self
    }
}

/// The output produced by a successful tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub title: String,
    pub output: String,
    pub metadata: Metadata,
    pub truncated: bool,
}

impl ToolResult {
    /// Creates a `ToolResult` with only a title and output, and no metadata.
    pub fn simple(title: impl Into<String>, output: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            output: output.into(),
            metadata: Metadata::new(),
            truncated: false,
        }
    }

    /// Attaches a metadata key-value pair to the result.
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Runtime context passed to every tool execution, carrying session state and host callbacks.
#[derive(Clone)]
pub struct ToolContext {
    pub session_id: String,
    pub message_id: String,
    pub agent: String,
    pub call_id: Option<String>,
    pub directory: String,
    pub worktree: String,
    pub abort: CancellationToken,
    pub extra: HashMap<String, serde_json::Value>,
    pub ask: Option<AskCallback>,
    pub ask_question: Option<QuestionCallback>,
    pub switch_agent: Option<SwitchAgentCallback>,
    pub create_subsession: Option<CreateSubsessionCallback>,
    pub prompt_subsession: Option<PromptSubsessionCallback>,
    pub file_time_assert: Option<FileTimeAssertCallback>,
    pub file_time_read: Option<FileTimeReadCallback>,
    pub publish_bus: Option<PublishBusCallback>,
    pub update_part: Option<UpdatePartCallback>,
    pub update_message: Option<UpdateMessageCallback>,
    pub lsp_touch_file: Option<LspTouchFileCallback>,
    pub todo_update: Option<TodoUpdateCallback>,
    pub todo_get: Option<TodoGetCallback>,
    pub get_last_model: Option<GetLastModelCallback>,
    pub create_synthetic_message: Option<CreateSyntheticMessageCallback>,
    pub project_root: String,
    pub registry: Option<Arc<ToolRegistry>>,
    #[cfg(feature = "lsp")]
    pub lsp_registry: Option<Arc<LspClientRegistry>>,
}

impl ToolContext {
    /// Creates a minimal `ToolContext` with the given session, message, and working directory.
    pub fn new(session_id: String, message_id: String, directory: String) -> Self {
        Self {
            session_id,
            message_id,
            agent: String::new(),
            call_id: None,
            directory: directory.clone(),
            worktree: directory.clone(),
            abort: CancellationToken::new(),
            extra: HashMap::new(),
            ask: None,
            ask_question: None,
            switch_agent: None,
            create_subsession: None,
            prompt_subsession: None,
            file_time_assert: None,
            file_time_read: None,
            publish_bus: None,
            update_part: None,
            update_message: None,
            lsp_touch_file: None,
            todo_update: None,
            todo_get: None,
            get_last_model: None,
            create_synthetic_message: None,
            project_root: directory,
            registry: None,
            #[cfg(feature = "lsp")]
            lsp_registry: None,
        }
    }

    /// Sets the agent name on this context.
    pub fn with_agent(mut self, agent: String) -> Self {
        self.agent = agent;
        self
    }

    /// Sets the cancellation token used to abort long-running operations.
    pub fn with_abort(mut self, abort: CancellationToken) -> Self {
        self.abort = abort;
        self
    }

    /// Attaches a shared `ToolRegistry` so tools can invoke other tools.
    pub fn with_registry(mut self, registry: Arc<ToolRegistry>) -> Self {
        self.registry = Some(registry);
        self
    }

    #[cfg(feature = "lsp")]
    /// Attaches an LSP client registry for post-write diagnostic collection.
    pub fn with_lsp_registry(mut self, lsp_registry: Arc<LspClientRegistry>) -> Self {
        self.lsp_registry = Some(lsp_registry);
        self
    }

    /// Registers the permission-check callback.
    pub fn with_ask<F, Fut>(mut self, callback: F) -> Self
    where
        F: Fn(PermissionRequest) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<(), ToolError>> + Send + 'static,
    {
        self.ask = Some(Arc::new(move |req| Box::pin(callback(req))));
        self
    }

    /// Invokes the permission-check callback, or succeeds silently if none is set.
    pub async fn ask_permission(&self, request: PermissionRequest) -> Result<(), ToolError> {
        if let Some(ref callback) = self.ask {
            callback(request).await
        } else {
            Ok(())
        }
    }

    /// Registers the question callback used to prompt the user.
    pub fn with_ask_question<F, Fut>(mut self, callback: F) -> Self
    where
        F: Fn(Vec<QuestionDef>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<Vec<Vec<String>>, ToolError>> + Send + 'static,
    {
        self.ask_question = Some(Arc::new(move |questions| Box::pin(callback(questions))));
        self
    }

    /// Presents questions to the user via the registered callback.
    pub async fn question(
        &self,
        questions: Vec<QuestionDef>,
    ) -> Result<Vec<Vec<String>>, ToolError> {
        if let Some(ref callback) = self.ask_question {
            callback(questions).await
        } else {
            Err(ToolError::ExecutionError(
                "Question callback not configured".to_string(),
            ))
        }
    }

    /// Registers the agent-switch callback.
    pub fn with_switch_agent<F, Fut>(mut self, callback: F) -> Self
    where
        F: Fn(String, Option<String>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<(), ToolError>> + Send + 'static,
    {
        self.switch_agent = Some(Arc::new(move |agent, model| {
            Box::pin(callback(agent, model))
        }));
        self
    }

    /// Switches the active agent via the registered callback, or succeeds silently if none is set.
    pub async fn do_switch_agent(
        &self,
        agent: String,
        model: Option<String>,
    ) -> Result<(), ToolError> {
        if let Some(ref callback) = self.switch_agent {
            callback(agent, model).await
        } else {
            Ok(())
        }
    }

    /// Registers the subsession-creation callback.
    pub fn with_create_subsession<F, Fut>(mut self, callback: F) -> Self
    where
        F: Fn(String, Option<String>, Option<String>, Vec<String>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<String, ToolError>> + Send + 'static,
    {
        self.create_subsession = Some(Arc::new(move |agent, title, model, disabled_tools| {
            Box::pin(callback(agent, title, model, disabled_tools))
        }));
        self
    }

    /// Creates a new subagent session, returning its ID; generates a local UUID if no callback is set.
    pub async fn do_create_subsession(
        &self,
        agent: String,
        title: Option<String>,
        model: Option<String>,
        disabled_tools: Vec<String>,
    ) -> Result<String, ToolError> {
        if let Some(ref callback) = self.create_subsession {
            callback(agent, title, model, disabled_tools).await
        } else {
            Ok(format!("task_{}_{}", agent, uuid::Uuid::new_v4()))
        }
    }

    /// Registers the subsession-prompt callback.
    pub fn with_prompt_subsession<F, Fut>(mut self, callback: F) -> Self
    where
        F: Fn(String, String) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<String, ToolError>> + Send + 'static,
    {
        self.prompt_subsession = Some(Arc::new(move |session_id, prompt| {
            Box::pin(callback(session_id, prompt))
        }));
        self
    }

    /// Sends a prompt to an existing subagent session and returns its response.
    pub async fn do_prompt_subsession(
        &self,
        session_id: String,
        prompt: String,
    ) -> Result<String, ToolError> {
        if let Some(ref callback) = self.prompt_subsession {
            callback(session_id, prompt).await
        } else {
            Err(ToolError::ExecutionError(
                "Subsession prompt callback not configured".to_string(),
            ))
        }
    }

    /// Registers the file-time-assert callback.
    pub fn with_file_time_assert<F, Fut>(mut self, callback: F) -> Self
    where
        F: Fn(String, String) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<(), ToolError>> + Send + 'static,
    {
        self.file_time_assert = Some(Arc::new(move |session_id, file_path| {
            Box::pin(callback(session_id, file_path))
        }));
        self
    }

    /// Asserts that the file has not changed since it was last read; no-ops if no callback is set.
    pub async fn do_file_time_assert(&self, file_path: String) -> Result<(), ToolError> {
        if let Some(ref callback) = self.file_time_assert {
            callback(self.session_id.clone(), file_path).await
        } else {
            Ok(())
        }
    }

    /// Registers the file-time-read callback.
    pub fn with_file_time_read<F, Fut>(mut self, callback: F) -> Self
    where
        F: Fn(String, String) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<(), ToolError>> + Send + 'static,
    {
        self.file_time_read = Some(Arc::new(move |session_id, file_path| {
            Box::pin(callback(session_id, file_path))
        }));
        self
    }

    /// Records that the file was read at the current time; no-ops if no callback is set.
    pub async fn do_file_time_read(&self, file_path: String) -> Result<(), ToolError> {
        if let Some(ref callback) = self.file_time_read {
            callback(self.session_id.clone(), file_path).await
        } else {
            Ok(())
        }
    }

    /// Registers the bus-publish callback.
    pub fn with_publish_bus<F, Fut>(mut self, callback: F) -> Self
    where
        F: Fn(String, serde_json::Value) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        self.publish_bus = Some(Arc::new(move |event_type, properties| {
            Box::pin(callback(event_type, properties))
        }));
        self
    }

    /// Publishes an event to the internal bus; silently no-ops if no callback is set.
    pub async fn do_publish_bus(&self, event_type: &str, properties: serde_json::Value) {
        if let Some(ref callback) = self.publish_bus {
            callback(event_type.to_string(), properties).await;
        }
    }

    /// Registers the part-update callback.
    pub fn with_update_part<F, Fut>(mut self, callback: F) -> Self
    where
        F: Fn(serde_json::Value) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<(), ToolError>> + Send + 'static,
    {
        self.update_part = Some(Arc::new(move |part| Box::pin(callback(part))));
        self
    }

    /// Persists a message part; no-ops if no callback is set.
    pub async fn do_update_part(&self, part: serde_json::Value) -> Result<(), ToolError> {
        if let Some(ref callback) = self.update_part {
            callback(part).await
        } else {
            Ok(())
        }
    }

    /// Registers the message-update callback.
    pub fn with_update_message<F, Fut>(mut self, callback: F) -> Self
    where
        F: Fn(serde_json::Value) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<(), ToolError>> + Send + 'static,
    {
        self.update_message = Some(Arc::new(move |msg| Box::pin(callback(msg))));
        self
    }

    /// Persists a message; no-ops if no callback is set.
    pub async fn do_update_message(&self, msg: serde_json::Value) -> Result<(), ToolError> {
        if let Some(ref callback) = self.update_message {
            callback(msg).await
        } else {
            Ok(())
        }
    }

    /// Registers the LSP file-touch callback.
    pub fn with_lsp_touch_file<F, Fut>(mut self, callback: F) -> Self
    where
        F: Fn(String, bool) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<(), ToolError>> + Send + 'static,
    {
        self.lsp_touch_file = Some(Arc::new(move |file_path, write| {
            Box::pin(callback(file_path, write))
        }));
        self
    }

    /// Notifies the LSP client that a file was opened or written; no-ops if no callback is set.
    pub async fn do_lsp_touch_file(&self, file_path: String, write: bool) -> Result<(), ToolError> {
        if let Some(ref callback) = self.lsp_touch_file {
            callback(file_path, write).await
        } else {
            Ok(())
        }
    }

    /// Registers the todo-update callback.
    pub fn with_todo_update<F, Fut>(mut self, callback: F) -> Self
    where
        F: Fn(String, Vec<TodoItemData>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<(), ToolError>> + Send + 'static,
    {
        self.todo_update = Some(Arc::new(move |session_id, todos| {
            Box::pin(callback(session_id, todos))
        }));
        self
    }

    /// Replaces the session's todo list; no-ops if no callback is set.
    pub async fn do_todo_update(&self, todos: Vec<TodoItemData>) -> Result<(), ToolError> {
        if let Some(ref callback) = self.todo_update {
            callback(self.session_id.clone(), todos).await
        } else {
            Ok(())
        }
    }

    /// Registers the todo-get callback.
    pub fn with_todo_get<F, Fut>(mut self, callback: F) -> Self
    where
        F: Fn(String) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<Vec<TodoItemData>, ToolError>> + Send + 'static,
    {
        self.todo_get = Some(Arc::new(move |session_id| Box::pin(callback(session_id))));
        self
    }

    /// Returns the session's current todo list; returns an empty list if no callback is set.
    pub async fn do_todo_get(&self) -> Result<Vec<TodoItemData>, ToolError> {
        if let Some(ref callback) = self.todo_get {
            callback(self.session_id.clone()).await
        } else {
            Ok(Vec::new())
        }
    }

    /// Registers the get-last-model callback.
    pub fn with_get_last_model<F, Fut>(mut self, callback: F) -> Self
    where
        F: Fn(String) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<Option<String>, ToolError>> + Send + 'static,
    {
        self.get_last_model = Some(Arc::new(move |session_id| Box::pin(callback(session_id))));
        self
    }

    /// Returns the model last used in this session, or `None` if unavailable.
    pub async fn do_get_last_model(&self) -> Option<String> {
        if let Some(ref callback) = self.get_last_model {
            callback(self.session_id.clone()).await.ok().flatten()
        } else {
            None
        }
    }

    /// Registers the synthetic-message creation callback.
    pub fn with_create_synthetic_message<F, Fut>(mut self, callback: F) -> Self
    where
        F: Fn(String, Option<String>, String) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<(), ToolError>> + Send + 'static,
    {
        self.create_synthetic_message = Some(Arc::new(move |session_id, agent, text| {
            Box::pin(callback(session_id, agent, text))
        }));
        self
    }

    /// Injects a synthetic user message into the session; no-ops if no callback is set.
    pub async fn do_create_synthetic_message(
        &self,
        agent: Option<String>,
        text: String,
    ) -> Result<(), ToolError> {
        if let Some(ref callback) = self.create_synthetic_message {
            callback(self.session_id.clone(), agent, text).await
        } else {
            Ok(())
        }
    }

    /// Returns `true` if the abort token has been cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.abort.is_cancelled()
    }

    /// Returns `true` if the given path falls outside the project root.
    pub fn is_external_path(&self, path: &str) -> bool {
        let abs_path = if std::path::Path::new(path).is_absolute() {
            path.to_string()
        } else {
            format!("{}/{}", self.directory, path)
        };
        !abs_path.starts_with(&self.project_root)
    }
}

impl std::fmt::Debug for ToolContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolContext")
            .field("session_id", &self.session_id)
            .field("message_id", &self.message_id)
            .field("agent", &self.agent)
            .field("directory", &self.directory)
            .field("worktree", &self.worktree)
            .finish()
    }
}

/// Trait that every tool must implement to be registered and executed.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Returns the unique identifier used to look up this tool.
    fn id(&self) -> &str;
    /// Returns a human-readable description of what this tool does.
    fn description(&self) -> &str;
    /// Returns the JSON Schema describing the tool's accepted arguments.
    fn parameters(&self) -> serde_json::Value;

    /// Executes the tool with the given arguments and context.
    async fn execute(
        &self,
        args: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError>;

    /// Validates arguments before execution; returns `Ok(())` by default.
    fn validate(&self, args: &serde_json::Value) -> Result<(), ToolError> {
        let _ = args;
        Ok(())
    }
}

/// Errors that can be returned by tool execution.
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    /// Input failed schema or semantic validation.
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// The tool encountered a runtime failure.
    #[error("Execution error: {0}")]
    ExecutionError(String),

    /// The permission check rejected the action.
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// The user declined to answer a required question.
    #[error("Question rejected: {0}")]
    QuestionRejected(String),

    /// The requested file does not exist.
    #[error("File not found: {0}")]
    FileNotFound(String),

    /// The operation exceeded its time limit.
    #[error("Timeout: {0}")]
    Timeout(String),

    /// The target file is binary and cannot be read as text.
    #[error("Binary file: {0}")]
    BinaryFile(String),

    /// The caller supplied arguments that do not match the tool's schema.
    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),

    /// The operation was cancelled via the abort token.
    #[error("Cancelled")]
    Cancelled,
}

impl ToolError {
    /// Constructs a `FileNotFound` error, appending similar filenames as suggestions when available.
    pub fn with_suggestions(msg: impl Into<String>, suggestions: &[String]) -> Self {
        let msg = msg.into();
        if suggestions.is_empty() {
            ToolError::FileNotFound(msg)
        } else {
            ToolError::FileNotFound(format!(
                "{}\n\nDid you mean one of these?\n{}",
                msg,
                suggestions.join("\n")
            ))
        }
    }
}
