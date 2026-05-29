//! Simple stack-based router that tracks the current TUI view.

use serde::{Deserialize, Serialize};

/// The set of top-level views the TUI can display.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Route {
    /// The home/welcome screen shown before any session is active.
    Home,
    /// An active conversation session identified by its ID.
    Session { session_id: String },
    /// The settings view.
    Settings,
    /// The help overlay.
    Help,
}

impl Default for Route {
    fn default() -> Self {
        Self::Home
    }
}

impl std::fmt::Display for Route {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Route::Home => write!(f, "Home"),
            Route::Session { session_id } => write!(f, "Session: {}", session_id),
            Route::Settings => write!(f, "Settings"),
            Route::Help => write!(f, "Help"),
        }
    }
}

/// Stack-based router that supports forward navigation and single-level back.
#[derive(Clone, Debug, Default)]
pub struct Router {
    history: Vec<Route>,
    current: Route,
}

impl Router {
    /// Create a router starting at the Home route.
    pub fn new() -> Self {
        Self {
            history: vec![Route::Home],
            current: Route::Home,
        }
    }

    /// Return a reference to the currently active route.
    pub fn current(&self) -> &Route {
        &self.current
    }

    /// Push `route` onto the history stack and make it current; no-op if already current.
    pub fn navigate(&mut self, route: Route) {
        if self.current != route {
            self.history.push(self.current.clone());
            self.current = route;
        }
    }

    /// Pop the history stack and return `true` if navigation succeeded.
    pub fn go_back(&mut self) -> bool {
        if self.history.len() > 1 {
            self.history.pop();
            if let Some(prev) = self.history.last() {
                self.current = prev.clone();
                return true;
            }
        }
        false
    }

    /// Return `true` if the current route is `Home`.
    pub fn is_home(&self) -> bool {
        matches!(self.current, Route::Home)
    }

    /// Return `true` if the current route is a `Session`.
    pub fn is_session(&self) -> bool {
        matches!(self.current, Route::Session { .. })
    }

    /// Return the session ID if the current route is a `Session`, otherwise `None`.
    pub fn session_id(&self) -> Option<&str> {
        match &self.current {
            Route::Session { session_id } => Some(session_id),
            _ => None,
        }
    }
}
