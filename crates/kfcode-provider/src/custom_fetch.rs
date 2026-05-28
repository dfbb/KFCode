//! Custom HTTP fetch proxy abstraction, allowing callers to intercept provider requests.
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use futures::Stream;
use once_cell::sync::Lazy;

use crate::provider::ProviderError;

/// A pinned, boxed stream of raw SSE chunk strings from a custom fetch proxy.
pub type CustomFetchChunkStream = Pin<Box<dyn Stream<Item = Result<String, ProviderError>> + Send>>;

/// An outgoing HTTP request passed to a `CustomFetchProxy`.
#[derive(Debug, Clone)]
pub struct CustomFetchRequest {
    pub url: String,
    pub method: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

/// A non-streaming HTTP response returned by a `CustomFetchProxy`.
#[derive(Debug, Clone)]
pub struct CustomFetchResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

/// A streaming HTTP response returned by a `CustomFetchProxy`.
pub struct CustomFetchStreamResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub stream: CustomFetchChunkStream,
}

/// Trait for intercepting provider HTTP requests, used for testing or proxying.
#[async_trait]
pub trait CustomFetchProxy: Send + Sync {
    /// Execute a non-streaming HTTP request and return the full response.
    async fn fetch(
        &self,
        request: CustomFetchRequest,
    ) -> Result<CustomFetchResponse, ProviderError>;

    /// Execute a streaming HTTP request and return a chunk stream.
    async fn fetch_stream(
        &self,
        request: CustomFetchRequest,
    ) -> Result<CustomFetchStreamResponse, ProviderError>;
}

static CUSTOM_FETCH_PROXIES: Lazy<RwLock<HashMap<String, Arc<dyn CustomFetchProxy>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Register a custom fetch proxy for the given provider ID, replacing any existing one.
pub fn register_custom_fetch_proxy(
    provider_id: impl Into<String>,
    proxy: Arc<dyn CustomFetchProxy>,
) {
    if let Ok(mut guard) = CUSTOM_FETCH_PROXIES.write() {
        guard.insert(provider_id.into(), proxy);
    }
}

/// Remove the custom fetch proxy registered for the given provider ID.
pub fn unregister_custom_fetch_proxy(provider_id: &str) {
    if let Ok(mut guard) = CUSTOM_FETCH_PROXIES.write() {
        guard.remove(provider_id);
    }
}

/// Remove all registered custom fetch proxies.
pub fn clear_custom_fetch_proxies() {
    if let Ok(mut guard) = CUSTOM_FETCH_PROXIES.write() {
        guard.clear();
    }
}

/// Return the custom fetch proxy registered for the given provider ID, or `None`.
pub fn get_custom_fetch_proxy(provider_id: &str) -> Option<Arc<dyn CustomFetchProxy>> {
    CUSTOM_FETCH_PROXIES
        .read()
        .ok()
        .and_then(|guard| guard.get(provider_id).cloned())
}
