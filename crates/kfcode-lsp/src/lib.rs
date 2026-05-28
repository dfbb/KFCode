//! LSP client library for kfcode.
//! Provides an async JSON-RPC client that spawns an LSP server process and exposes
//! language-intelligence operations such as diagnostics, completion, and navigation.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::str::FromStr;
use std::sync::Arc;

use lsp_types::{
    ClientCapabilities, Diagnostic, DidChangeTextDocumentParams, DidOpenTextDocumentParams,
    InitializeParams, Range, TextDocumentContentChangeEvent, TextDocumentIdentifier,
    TextDocumentItem, VersionedTextDocumentIdentifier, WorkspaceFolder,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, Command};
use tokio::sync::{broadcast, Mutex, RwLock};
use tracing::{debug, error};
use url::Url;

/// A single node in a call hierarchy tree, representing a callable symbol in a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallHierarchyItem {
    pub name: String,
    pub kind: lsp_types::SymbolKind,
    #[serde(default)]
    pub tags: Option<Vec<lsp_types::SymbolTag>>,
    #[serde(default)]
    pub detail: Option<String>,
    pub uri: lsp_types::Uri,
    pub range: Range,
    pub selection_range: Range,
    #[serde(default)]
    pub data: Option<Value>,
}

/// Parameters for the `textDocument/prepareCallHierarchy` request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallHierarchyPrepareParams {
    #[serde(flatten)]
    pub text_document_position_params: lsp_types::TextDocumentPositionParams,
    #[serde(flatten)]
    pub work_done_progress_params: lsp_types::WorkDoneProgressParams,
}

/// Parameters for the `callHierarchy/incomingCalls` request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallHierarchyIncomingCallsParams {
    pub item: CallHierarchyItem,
    #[serde(flatten)]
    pub work_done_progress_params: lsp_types::WorkDoneProgressParams,
    #[serde(flatten)]
    pub partial_result_params: lsp_types::PartialResultParams,
}

/// Parameters for the `callHierarchy/outgoingCalls` request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallHierarchyOutgoingCallsParams {
    pub item: CallHierarchyItem,
    #[serde(flatten)]
    pub work_done_progress_params: lsp_types::WorkDoneProgressParams,
    #[serde(flatten)]
    pub partial_result_params: lsp_types::PartialResultParams,
}

/// One incoming call edge: the caller symbol and the ranges within it that reference the callee.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallHierarchyIncomingCall {
    pub from: CallHierarchyItem,
    pub from_ranges: Vec<Range>,
}

/// One outgoing call edge: the callee symbol and the ranges within the caller that invoke it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallHierarchyOutgoingCall {
    pub to: CallHierarchyItem,
    pub from_ranges: Vec<Range>,
}

fn path_to_uri(path: &Path) -> Result<lsp_types::Uri, LspError> {
    let url = Url::from_file_path(path)
        .map_err(|_| LspError::InitializeError("Invalid file path".to_string()))?;
    lsp_types::Uri::from_str(url.as_str())
        .map_err(|e| LspError::InitializeError(format!("Invalid URI: {}", e)))
}

fn uri_to_path(uri: &lsp_types::Uri) -> PathBuf {
    Url::parse(&uri.to_string())
        .ok()
        .and_then(|u| u.to_file_path().ok())
        .unwrap_or_default()
}

/// Errors that can occur while communicating with an LSP server.
#[derive(Debug, Error)]
pub enum LspError {
    /// The LSP server process could not be spawned.
    #[error("Failed to start LSP server: {0}")]
    ServerStartError(String),

    /// The LSP `initialize` handshake failed.
    #[error("Failed to initialize LSP: {0}")]
    InitializeError(String),

    /// The server returned a JSON-RPC error object.
    #[error("JSON-RPC error: {0}")]
    JsonRpcError(String),

    /// An underlying I/O error occurred.
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// A JSON serialization or deserialization error occurred.
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// A request was sent before the server was initialized.
    #[error("Server not initialized")]
    NotInitialized,

    /// The response channel was dropped before a reply arrived.
    #[error("Timeout waiting for response")]
    Timeout,
}

/// A JSON-RPC 2.0 request message sent to the LSP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    pub params: Value,
}

/// A JSON-RPC 2.0 response message received from the LSP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: u64,
    pub result: Option<Value>,
    pub error: Option<JsonRpcError>,
}

/// A JSON-RPC 2.0 notification message sent to or received from the LSP server (no `id` field).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<Value>,
}

/// The error object embedded in a failed JSON-RPC response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}

/// Configuration required to spawn and identify a single LSP server process.
pub struct LspServerConfig {
    pub id: String,
    pub command: String,
    pub args: Vec<String>,
    pub initialization_options: Option<Value>,
}

/// An async client connected to a running LSP server process.
///
/// Manages the JSON-RPC message loop, pending-response tracking, per-file
/// version counters, and a broadcast channel for diagnostic events.
pub struct LspClient {
    root: PathBuf,
    stdin: Arc<Mutex<ChildStdin>>,
    request_id: Arc<Mutex<u64>>,
    pending_responses:
        Arc<RwLock<HashMap<u64, tokio::sync::oneshot::Sender<Result<Value, LspError>>>>>,
    diagnostics: Arc<RwLock<HashMap<PathBuf, Vec<Diagnostic>>>>,
    file_versions: Arc<RwLock<HashMap<PathBuf, u32>>>,
    event_tx: broadcast::Sender<LspEvent>,
}

/// Events broadcast by an `LspClient` to interested subscribers.
#[derive(Debug, Clone)]
pub enum LspEvent {
    /// The server published a new set of diagnostics for the given file.
    Diagnostics { path: PathBuf, server_id: String },
}

impl LspClient {
    /// Spawns the LSP server process described by `config`, performs the `initialize` handshake,
    /// and returns a ready-to-use client rooted at `root`.
    ///
    /// # Errors
    /// Returns `LspError::ServerStartError` if the process cannot be spawned, or
    /// `LspError::InitializeError` if the handshake fails.
    pub async fn start(config: LspServerConfig, root: PathBuf) -> Result<Self, LspError> {
        let mut child = Command::new(&config.command)
            .args(&config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| LspError::ServerStartError(e.to_string()))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| LspError::ServerStartError("Failed to get stdin".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| LspError::ServerStartError("Failed to get stdout".to_string()))?;

        let (event_tx, _) = broadcast::channel(256);

        let client = Self {
            root,
            stdin: Arc::new(Mutex::new(stdin)),
            request_id: Arc::new(Mutex::new(0)),
            pending_responses: Arc::new(RwLock::new(HashMap::new())),
            diagnostics: Arc::new(RwLock::new(HashMap::new())),
            file_versions: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
        };

        let pending = client.pending_responses.clone();
        let diagnostics = client.diagnostics.clone();
        let server_id = config.id.clone();
        let event_tx_clone = client.event_tx.clone();

        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                if line.is_empty() || line.starts_with("Content-Length:") {
                    continue;
                }

                if let Ok(notification) = serde_json::from_str::<JsonRpcNotification>(&line) {
                    if notification.method == "textDocument/publishDiagnostics" {
                        if let Some(params) = notification.params {
                            if let Ok(diag_params) = serde_json::from_value::<
                                lsp_types::PublishDiagnosticsParams,
                            >(params)
                            {
                                let path = uri_to_path(&diag_params.uri);

                                diagnostics
                                    .write()
                                    .await
                                    .insert(path.clone(), diag_params.diagnostics);

                                let _ = event_tx_clone.send(LspEvent::Diagnostics {
                                    path,
                                    server_id: server_id.clone(),
                                });
                            }
                        }
                    }
                } else if let Ok(response) = serde_json::from_str::<JsonRpcResponse>(&line) {
                    if let Some(sender) = pending.write().await.remove(&response.id) {
                        let result = if let Some(error) = response.error {
                            Err(LspError::JsonRpcError(error.message))
                        } else {
                            Ok(response.result.unwrap_or(Value::Null))
                        };
                        let _ = sender.send(result);
                    }
                }
            }
        });

        let mut client = client;
        client.initialize(config.initialization_options).await?;

        Ok(client)
    }

    async fn initialize(&mut self, initialization_options: Option<Value>) -> Result<(), LspError> {
        let workspace_uri = path_to_uri(&self.root)?;

        let params = InitializeParams {
            initialization_options,
            workspace_folders: Some(vec![WorkspaceFolder {
                uri: workspace_uri,
                name: "workspace".to_string(),
            }]),
            capabilities: ClientCapabilities::default(),
            ..Default::default()
        };

        let result = self
            .request("initialize", serde_json::to_value(params)?)
            .await?;
        debug!(?result, "LSP initialized");

        self.notify("initialized", Value::Null).await?;

        Ok(())
    }

    async fn next_id(&self) -> u64 {
        let mut id = self.request_id.lock().await;
        *id += 1;
        *id
    }

    /// Sends a JSON-RPC request and waits for the server's response.
    ///
    /// # Errors
    /// Returns `LspError::Timeout` if the response channel is dropped before a reply arrives.
    pub async fn request(&self, method: &str, params: Value) -> Result<Value, LspError> {
        let id = self.next_id().await;
        let (tx, rx) = tokio::sync::oneshot::channel();

        self.pending_responses.write().await.insert(id, tx);

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params,
        };

        let content = serde_json::to_string(&request)?;
        let message = format!("Content-Length: {}\r\n\r\n{}", content.len(), content);

        let mut stdin = self.stdin.lock().await;
        stdin.write_all(message.as_bytes()).await?;
        stdin.flush().await?;

        rx.await.map_err(|_| LspError::Timeout)?
    }

    /// Sends a JSON-RPC notification (fire-and-forget; no response is expected).
    pub async fn notify(&self, method: &str, params: Value) -> Result<(), LspError> {
        let notification = JsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params: if params.is_null() { None } else { Some(params) },
        };

        let content = serde_json::to_string(&notification)?;
        let message = format!("Content-Length: {}\r\n\r\n{}", content.len(), content);

        let mut stdin = self.stdin.lock().await;
        stdin.write_all(message.as_bytes()).await?;
        stdin.flush().await?;

        Ok(())
    }

    /// Notifies the server that a document has been opened or its content has changed.
    ///
    /// Sends `textDocument/didOpen` on the first call for a path, and
    /// `textDocument/didChange` on subsequent calls, incrementing the version counter.
    pub async fn open_document(
        &self,
        path: &Path,
        content: &str,
        language_id: &str,
    ) -> Result<(), LspError> {
        let uri = path_to_uri(path)?;

        let version = {
            let mut versions = self.file_versions.write().await;
            let v = versions.entry(path.to_path_buf()).or_insert(0);
            *v
        };

        if version > 0 {
            let next_version = version + 1;
            self.file_versions
                .write()
                .await
                .insert(path.to_path_buf(), next_version);

            let params = DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri,
                    version: next_version as i32,
                },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: content.to_string(),
                }],
            };

            self.notify("textDocument/didChange", serde_json::to_value(params)?)
                .await?;
        } else {
            self.file_versions
                .write()
                .await
                .insert(path.to_path_buf(), 0);
            self.diagnostics.write().await.remove(path);

            let params = DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri,
                    language_id: language_id.to_string(),
                    version: 0,
                    text: content.to_string(),
                },
            };

            self.notify("textDocument/didOpen", serde_json::to_value(params)?)
                .await?;
        }

        Ok(())
    }

    /// Returns the most recently received diagnostics for the given file path.
    pub async fn get_diagnostics(&self, path: &Path) -> Vec<Diagnostic> {
        self.diagnostics
            .read()
            .await
            .get(path)
            .cloned()
            .unwrap_or_default()
    }

    /// Returns all diagnostics from all files this LSP server has reported on.
    pub async fn get_all_diagnostics(&self) -> HashMap<PathBuf, Vec<Diagnostic>> {
        self.diagnostics.read().await.clone()
    }

    /// Subscribes to the broadcast channel of `LspEvent`s emitted by this client.
    pub fn subscribe(&self) -> broadcast::Receiver<LspEvent> {
        self.event_tx.subscribe()
    }

    /// Resolves the definition location of the symbol at the given cursor position.
    ///
    /// Returns the first location when the server returns multiple results.
    pub async fn goto_definition(
        &self,
        path: &Path,
        line: u32,
        character: u32,
    ) -> Result<Option<lsp_types::Location>, LspError> {
        let uri = path_to_uri(path)?;

        let params = lsp_types::GotoDefinitionParams {
            text_document_position_params: lsp_types::TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: lsp_types::Position { line, character },
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result = self
            .request("textDocument/definition", serde_json::to_value(params)?)
            .await?;

        if result.is_null() {
            return Ok(None);
        }

        let response: lsp_types::GotoDefinitionResponse = serde_json::from_value(result)?;
        match response {
            lsp_types::GotoDefinitionResponse::Scalar(loc) => Ok(Some(loc)),
            lsp_types::GotoDefinitionResponse::Array(locs) => Ok(locs.into_iter().next()),
            lsp_types::GotoDefinitionResponse::Link(links) => {
                Ok(links.into_iter().next().map(|l| lsp_types::Location {
                    uri: l.target_uri,
                    range: l.target_selection_range,
                }))
            }
        }
    }

    /// Requests completion items at the given cursor position.
    pub async fn completion(
        &self,
        path: &Path,
        line: u32,
        character: u32,
    ) -> Result<Option<Vec<lsp_types::CompletionItem>>, LspError> {
        let uri = path_to_uri(path)?;

        let params = lsp_types::CompletionParams {
            text_document_position: lsp_types::TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: lsp_types::Position { line, character },
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: None,
        };

        let result = self
            .request("textDocument/completion", serde_json::to_value(params)?)
            .await?;

        if result.is_null() {
            return Ok(None);
        }

        let response: lsp_types::CompletionResponse = serde_json::from_value(result)?;
        Ok(Some(match response {
            lsp_types::CompletionResponse::Array(items) => items,
            lsp_types::CompletionResponse::List(list) => list.items,
        }))
    }

    /// Returns all reference locations for the symbol at the given cursor position,
    /// including its declaration.
    pub async fn references(
        &self,
        path: &Path,
        line: u32,
        character: u32,
    ) -> Result<Vec<lsp_types::Location>, LspError> {
        let uri = path_to_uri(path)?;

        let params = lsp_types::ReferenceParams {
            text_document_position: lsp_types::TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: lsp_types::Position { line, character },
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: lsp_types::ReferenceContext {
                include_declaration: true,
            },
        };

        let result = self
            .request("textDocument/references", serde_json::to_value(params)?)
            .await?;

        if result.is_null() {
            return Ok(vec![]);
        }

        let locations: Vec<lsp_types::Location> = serde_json::from_value(result)?;
        Ok(locations)
    }

    /// Requests hover information for the symbol at the given cursor position.
    pub async fn hover(
        &self,
        path: &Path,
        line: u32,
        character: u32,
    ) -> Result<Option<lsp_types::Hover>, LspError> {
        let uri = path_to_uri(path)?;

        let params = lsp_types::HoverParams {
            text_document_position_params: lsp_types::TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: lsp_types::Position { line, character },
            },
            work_done_progress_params: Default::default(),
        };

        let result = self
            .request("textDocument/hover", serde_json::to_value(params)?)
            .await?;

        if result.is_null() {
            return Ok(None);
        }

        let hover: lsp_types::Hover = serde_json::from_value(result)?;
        Ok(Some(hover))
    }

    /// Returns a flat list of all symbols defined in the given document.
    ///
    /// Nested `DocumentSymbol` trees are flattened into `SymbolInformation` entries.
    pub async fn document_symbol(
        &self,
        path: &Path,
    ) -> Result<Vec<lsp_types::SymbolInformation>, LspError> {
        let uri = path_to_uri(path)?;

        let params = lsp_types::DocumentSymbolParams {
            text_document: TextDocumentIdentifier { uri },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result = self
            .request("textDocument/documentSymbol", serde_json::to_value(params)?)
            .await?;

        if result.is_null() {
            return Ok(vec![]);
        }

        let symbols: lsp_types::DocumentSymbolResponse = serde_json::from_value(result)?;
        Ok(match symbols {
            lsp_types::DocumentSymbolResponse::Flat(symbols) => symbols,
            lsp_types::DocumentSymbolResponse::Nested(nested) => nested
                .into_iter()
                .flat_map(|s| flatten_document_symbol(&s))
                .collect(),
        })
    }

    /// Searches the entire workspace for symbols matching `query`.
    pub async fn workspace_symbol(
        &self,
        query: &str,
    ) -> Result<Vec<lsp_types::SymbolInformation>, LspError> {
        let params = lsp_types::WorkspaceSymbolParams {
            query: query.to_string(),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result = self
            .request("workspace/symbol", serde_json::to_value(params)?)
            .await?;

        if result.is_null() {
            return Ok(vec![]);
        }

        let symbols: Vec<lsp_types::SymbolInformation> = serde_json::from_value(result)?;
        Ok(symbols)
    }

    /// Returns all implementation locations for the symbol at the given cursor position.
    pub async fn goto_implementation(
        &self,
        path: &Path,
        line: u32,
        character: u32,
    ) -> Result<Vec<lsp_types::Location>, LspError> {
        let uri = path_to_uri(path)?;

        let params = lsp_types::request::GotoImplementationParams {
            text_document_position_params: lsp_types::TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: lsp_types::Position { line, character },
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result = self
            .request("textDocument/implementation", serde_json::to_value(params)?)
            .await?;

        if result.is_null() {
            return Ok(vec![]);
        }

        let response: lsp_types::GotoDefinitionResponse = serde_json::from_value(result)?;
        Ok(match response {
            lsp_types::GotoDefinitionResponse::Scalar(loc) => vec![loc],
            lsp_types::GotoDefinitionResponse::Array(locs) => locs,
            lsp_types::GotoDefinitionResponse::Link(links) => links
                .into_iter()
                .map(|l| lsp_types::Location {
                    uri: l.target_uri,
                    range: l.target_selection_range,
                })
                .collect(),
        })
    }

    /// Returns all type-definition locations for the symbol at the given cursor position.
    pub async fn type_definition(
        &self,
        path: &Path,
        line: u32,
        character: u32,
    ) -> Result<Vec<lsp_types::Location>, LspError> {
        let uri = path_to_uri(path)?;

        let params = lsp_types::TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri },
            position: lsp_types::Position { line, character },
        };

        let result = self
            .request("textDocument/typeDefinition", serde_json::to_value(params)?)
            .await?;

        if result.is_null() {
            return Ok(vec![]);
        }

        let response: lsp_types::GotoDefinitionResponse = serde_json::from_value(result)?;
        Ok(match response {
            lsp_types::GotoDefinitionResponse::Scalar(loc) => vec![loc],
            lsp_types::GotoDefinitionResponse::Array(locs) => locs,
            lsp_types::GotoDefinitionResponse::Link(links) => links
                .into_iter()
                .map(|l| lsp_types::Location {
                    uri: l.target_uri,
                    range: l.target_selection_range,
                })
                .collect(),
        })
    }

    /// Requests a workspace-wide rename of the symbol at the given cursor position.
    ///
    /// Returns `None` if the server has no edits to apply.
    pub async fn rename(
        &self,
        path: &Path,
        line: u32,
        character: u32,
        new_name: &str,
    ) -> Result<Option<lsp_types::WorkspaceEdit>, LspError> {
        let uri = path_to_uri(path)?;

        let params = lsp_types::RenameParams {
            text_document_position: lsp_types::TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: lsp_types::Position { line, character },
            },
            new_name: new_name.to_string(),
            work_done_progress_params: Default::default(),
        };

        let result = self
            .request("textDocument/rename", serde_json::to_value(params)?)
            .await?;

        if result.is_null() {
            return Ok(None);
        }

        let edit: lsp_types::WorkspaceEdit = serde_json::from_value(result)?;
        Ok(Some(edit))
    }

    /// Prepares a call hierarchy at the given cursor position, returning the candidate items.
    pub async fn prepare_call_hierarchy(
        &self,
        path: &Path,
        line: u32,
        character: u32,
    ) -> Result<Vec<CallHierarchyItem>, LspError> {
        let uri = path_to_uri(path)?;

        let params = CallHierarchyPrepareParams {
            text_document_position_params: lsp_types::TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: lsp_types::Position { line, character },
            },
            work_done_progress_params: Default::default(),
        };

        let result = self
            .request(
                "textDocument/prepareCallHierarchy",
                serde_json::to_value(params)?,
            )
            .await?;

        if result.is_null() {
            return Ok(vec![]);
        }

        let items: Vec<CallHierarchyItem> = serde_json::from_value(result)?;
        Ok(items)
    }

    /// Returns all callers of the symbol at the given cursor position.
    ///
    /// Internally calls `prepare_call_hierarchy` first and uses the first result.
    pub async fn incoming_calls(
        &self,
        path: &Path,
        line: u32,
        character: u32,
    ) -> Result<Vec<CallHierarchyIncomingCall>, LspError> {
        let items = self.prepare_call_hierarchy(path, line, character).await?;

        if items.is_empty() {
            return Ok(vec![]);
        }

        let item = &items[0];
        let params = CallHierarchyIncomingCallsParams {
            item: item.clone(),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result = self
            .request("callHierarchy/incomingCalls", serde_json::to_value(params)?)
            .await?;

        if result.is_null() {
            return Ok(vec![]);
        }

        let calls: Vec<CallHierarchyIncomingCall> = serde_json::from_value(result)?;
        Ok(calls)
    }

    /// Returns all callees invoked by the symbol at the given cursor position.
    ///
    /// Internally calls `prepare_call_hierarchy` first and uses the first result.
    pub async fn outgoing_calls(
        &self,
        path: &Path,
        line: u32,
        character: u32,
    ) -> Result<Vec<CallHierarchyOutgoingCall>, LspError> {
        let items = self.prepare_call_hierarchy(path, line, character).await?;

        if items.is_empty() {
            return Ok(vec![]);
        }

        let item = &items[0];
        let params = CallHierarchyOutgoingCallsParams {
            item: item.clone(),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result = self
            .request("callHierarchy/outgoingCalls", serde_json::to_value(params)?)
            .await?;

        if result.is_null() {
            return Ok(vec![]);
        }

        let calls: Vec<CallHierarchyOutgoingCall> = serde_json::from_value(result)?;
        Ok(calls)
    }
}

#[allow(deprecated)]
fn flatten_document_symbol(
    symbol: &lsp_types::DocumentSymbol,
) -> Vec<lsp_types::SymbolInformation> {
    let mut result = vec![];

    result.push(lsp_types::SymbolInformation {
        name: symbol.name.clone(),
        kind: symbol.kind,
        tags: symbol.tags.clone(),
        deprecated: symbol.deprecated,
        location: lsp_types::Location {
            uri: path_to_uri(&std::path::PathBuf::new())
                .unwrap_or_else(|_| lsp_types::Uri::from_str("file:///").unwrap()),
            range: symbol.selection_range,
        },
        container_name: symbol.detail.clone(),
    });

    if let Some(children) = &symbol.children {
        for child in children {
            result.extend(flatten_document_symbol(child));
        }
    }

    result
}

/// A named collection of `LspClient` instances, keyed by server identifier.
///
/// Allows multiple language servers to be managed together and looked up by id.
pub struct LspClientRegistry {
    clients: RwLock<HashMap<String, Arc<LspClient>>>,
}

impl LspClientRegistry {
    /// Creates an empty registry with no registered clients.
    pub fn new() -> Self {
        Self {
            clients: RwLock::new(HashMap::new()),
        }
    }

    /// Registers a client under the given `id`, replacing any previous entry with the same id.
    pub async fn register(&self, id: String, client: Arc<LspClient>) {
        self.clients.write().await.insert(id, client);
    }

    /// Looks up a client by its server identifier, returning `None` if not found.
    pub async fn get(&self, id: &str) -> Option<Arc<LspClient>> {
        self.clients.read().await.get(id).cloned()
    }

    /// Returns all registered `(id, client)` pairs.
    pub async fn list(&self) -> Vec<(String, Arc<LspClient>)> {
        self.clients
            .read()
            .await
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Returns true if any LSP clients are currently registered.
    /// Mirrors the TS `LSP.hasClients(file)` which checks whether any server
    /// could handle the given file. Here we check if any registered client's
    /// id contains the detected language for the file.
    pub async fn has_clients(&self, path: &Path) -> bool {
        let clients = self.clients.read().await;
        if clients.is_empty() {
            return false;
        }
        let language = detect_language(path);
        clients.keys().any(|id| id.contains(language))
    }

    /// Opens or refreshes a file in all matching LSP clients.
    /// Mirrors the TS `LSP.touchFile(input, waitForDiagnostics)`.
    ///
    /// - Reads the file content from disk
    /// - For each registered client whose id matches the file's language,
    ///   calls `open_document` (which internally handles didOpen vs didChange)
    /// - If `wait_for_diagnostics` is true, waits briefly for diagnostics to arrive
    pub async fn touch_file(
        &self,
        path: &Path,
        wait_for_diagnostics: bool,
    ) -> Result<(), LspError> {
        let language = detect_language(path);
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| LspError::IoError(e))?;

        let clients = self.clients.read().await;
        let matching: Vec<Arc<LspClient>> = clients
            .iter()
            .filter(|(id, _)| id.contains(language))
            .map(|(_, c)| c.clone())
            .collect();
        drop(clients);

        for client in &matching {
            if let Err(e) = client.open_document(path, &content, language).await {
                error!("Failed to touch file {:?} in LSP: {}", path, e);
            }
        }

        if wait_for_diagnostics && !matching.is_empty() {
            // Give LSP servers a moment to produce diagnostics.
            // The TS version uses client.waitForDiagnostics which listens
            // for a publishDiagnostics notification. We approximate this
            // with a short sleep, since the broadcast-based event system
            // already stores diagnostics as they arrive.
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }

        Ok(())
    }
}

impl Default for LspClientRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Maps a file path to its LSP language identifier string based on the file extension.
///
/// Returns `"plaintext"` for unrecognized extensions.
pub fn detect_language(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("rs") => "rust",
        Some("ts") => "typescript",
        Some("tsx") => "typescriptreact",
        Some("js") => "javascript",
        Some("jsx") => "javascriptreact",
        Some("py") => "python",
        Some("go") => "go",
        Some("java") => "java",
        Some("c") => "c",
        Some("cpp") | Some("cc") | Some("cxx") => "cpp",
        Some("h") | Some("hpp") => "cpp",
        Some("rb") => "ruby",
        Some("php") => "php",
        Some("swift") => "swift",
        Some("kt") => "kotlin",
        Some("scala") => "scala",
        Some("lua") => "lua",
        Some("json") => "json",
        Some("yaml") | Some("yml") => "yaml",
        Some("toml") => "toml",
        Some("md") => "markdown",
        Some("html") => "html",
        Some("css") => "css",
        Some("scss") => "scss",
        _ => "plaintext",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language(Path::new("main.rs")), "rust");
        assert_eq!(detect_language(Path::new("index.ts")), "typescript");
        assert_eq!(detect_language(Path::new("app.tsx")), "typescriptreact");
        assert_eq!(detect_language(Path::new("main.py")), "python");
        assert_eq!(detect_language(Path::new("main.go")), "go");
        assert_eq!(detect_language(Path::new("unknown.xyz")), "plaintext");
    }

    #[test]
    fn test_json_rpc_request_serialization() {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "initialize".to_string(),
            params: Value::Null,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"method\":\"initialize\""));
    }

    #[test]
    fn test_json_rpc_response_deserialization() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{"capabilities":{}}}"#;
        let response: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.id, 1);
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_has_clients_empty_registry() {
        let registry = LspClientRegistry::new();
        assert!(!registry.has_clients(Path::new("main.rs")).await);
        assert!(!registry.has_clients(Path::new("index.ts")).await);
    }

    #[tokio::test]
    async fn test_registry_default() {
        let registry = LspClientRegistry::default();
        let clients = registry.list().await;
        assert!(clients.is_empty());
    }
}
