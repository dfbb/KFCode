//! Event types and the in-process event bus used to drive the TUI loop.

use crossterm::event::{KeyEvent, MouseEvent};

/// All events that can be dispatched through the TUI event loop.
#[derive(Clone, Debug)]
pub enum Event {
    /// A keyboard event from crossterm.
    Key(KeyEvent),
    /// A mouse event from crossterm.
    Mouse(MouseEvent),
    /// Terminal resize to the given (width, height).
    Resize(u16, u16),
    /// Periodic tick emitted at the configured frame rate.
    Tick,
    /// Terminal window gained focus.
    FocusGained,
    /// Terminal window lost focus.
    FocusLost,
    /// Bracketed paste content.
    Paste(String),
    /// Application-level custom event.
    Custom(CustomEvent),
}

/// Application-level events that carry structured data beyond raw input.
#[derive(Clone, Debug)]
pub enum CustomEvent {
    /// A complete message string received from the server.
    Message(String),
    /// An incremental streaming chunk from the LLM.
    StreamChunk(String),
    /// The LLM stream finished successfully.
    StreamComplete,
    /// The LLM stream terminated with an error.
    StreamError(String),
    /// A tool call has started.
    ToolCallStart { id: String, name: String },
    /// A tool call has completed with a result.
    ToolCallComplete { id: String, result: String },
    /// A state-change notification from the server event stream.
    StateChanged(StateChange),
}

/// Discrete state transitions broadcast from the server event stream.
#[derive(Clone, Debug)]
pub enum StateChange {
    /// A new session was created with the given ID.
    SessionCreated(String),
    /// An existing session's metadata was updated.
    SessionUpdated(String),
    /// A session transitioned to the busy/running state.
    SessionStatusBusy(String),
    /// A session transitioned back to idle.
    SessionStatusIdle(String),
    /// A session is retrying after a transient error.
    SessionStatusRetrying {
        session_id: String,
        attempt: u32,
        message: String,
        next: i64,
    },
    /// A session was deleted.
    SessionDeleted(String),
    /// The active model changed.
    ModelChanged(String),
    /// The active agent changed.
    AgentChanged(String),
    /// A provider connection was established.
    ProviderConnected(String),
    /// A provider connection was dropped.
    ProviderDisconnected(String),
    /// An MCP server's connection status changed.
    McpServerStatusChanged {
        name: String,
        status: String,
    },
    /// The todo list was updated.
    TodoUpdated,
    /// The diff view was updated.
    DiffUpdated,
}

/// Cloneable sender wrapper used to publish events from any thread.
pub struct EventBus {
    tx: std::sync::mpsc::Sender<Event>,
}

impl Clone for EventBus {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}

impl EventBus {
    /// Create a new event bus with a disconnected channel (receiver is dropped).
    pub fn new() -> Self {
        let (tx, _rx) = std::sync::mpsc::channel();
        Self { tx }
    }

    /// Clone the underlying sender so other threads can publish events.
    pub fn sender(&self) -> std::sync::mpsc::Sender<Event> {
        self.tx.clone()
    }

    /// Send an event, silently ignoring send errors.
    pub fn send(&self, event: Event) {
        let _ = self.tx.send(event);
    }

    /// Wrap a `CustomEvent` and send it, silently ignoring send errors.
    pub fn send_custom(&self, event: CustomEvent) {
        let _ = self.tx.send(Event::Custom(event));
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
