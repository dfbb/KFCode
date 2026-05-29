//! Persistent storage for MCP OAuth credentials.
//!
//! Mirrors the TypeScript `McpAuth` namespace – stores tokens, client info,
//! code verifiers and OAuth state in a JSON file inside the user data directory.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;

/// OAuth tokens obtained from the authorization server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokens {
    pub access_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    /// Unix timestamp (seconds) at which the access token expires.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

/// Dynamically-registered (or pre-configured) client information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthClientInfo {
    pub client_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id_issued_at: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_secret_expires_at: Option<f64>,
}

/// A single entry in the auth store, keyed by MCP server name.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuthEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens: Option<OAuthTokens>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_info: Option<OAuthClientInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_verifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth_state: Option<String>,
    /// The server URL these credentials belong to – invalidated on URL change.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_url: Option<String>,
}

// ---------------------------------------------------------------------------
// AuthStore – path-injectable primitive
// ---------------------------------------------------------------------------

/// Path-injectable auth store. Use `AuthStore::new(path)` in tests;
/// use `AuthStore::default_user_store()` (or the free functions below) in
/// production code.
#[derive(Debug, Clone)]
pub struct AuthStore {
    path: PathBuf,
}

impl AuthStore {
    /// Create a store backed by the given file path.
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Create a store backed by the default user data directory.
    pub fn default_user_store() -> Self {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("kfcode");
        Self::new(data_dir.join("mcp-auth.json"))
    }

    /// The file path this store reads/writes.
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    // -----------------------------------------------------------------------
    // Internal IO helpers
    // -----------------------------------------------------------------------

    async fn read_all(&self) -> HashMap<String, AuthEntry> {
        match fs::read_to_string(&self.path).await {
            Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
            Err(_) => HashMap::new(),
        }
    }

    async fn write_all(&self, data: &HashMap<String, AuthEntry>) -> Result<(), std::io::Error> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).await?;
        }
        let json = serde_json::to_string_pretty(data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        fs::write(&self.path, json).await
    }

    // -----------------------------------------------------------------------
    // Public methods
    // -----------------------------------------------------------------------

    /// Get the auth entry for a given MCP server name.
    pub async fn get(&self, mcp_name: &str) -> Option<AuthEntry> {
        let data = self.read_all().await;
        data.get(mcp_name).cloned()
    }

    /// Get the auth entry only if it was stored for the same `server_url`.
    pub async fn get_for_url(&self, mcp_name: &str, server_url: &str) -> Option<AuthEntry> {
        let entry = self.get(mcp_name).await?;
        match &entry.server_url {
            Some(url) if url == server_url => Some(entry),
            _ => None,
        }
    }

    /// Persist an auth entry.
    pub async fn set(&self, mcp_name: &str, entry: AuthEntry) -> Result<(), std::io::Error> {
        let mut data = self.read_all().await;
        data.insert(mcp_name.to_string(), entry);
        self.write_all(&data).await
    }

    /// Persist an auth entry, optionally overriding the server URL.
    pub async fn set_with_url(
        &self,
        mcp_name: &str,
        entry: AuthEntry,
        server_url: Option<&str>,
    ) -> Result<(), std::io::Error> {
        let mut entry = entry;
        if let Some(url) = server_url {
            entry.server_url = Some(url.to_string());
        }
        self.set(mcp_name, entry).await
    }

    /// Remove all stored auth data for a server.
    pub async fn remove(&self, mcp_name: &str) -> Result<(), std::io::Error> {
        let mut data = self.read_all().await;
        data.remove(mcp_name);
        self.write_all(&data).await
    }

    /// Update only the tokens portion of an entry.
    pub async fn update_tokens(
        &self,
        mcp_name: &str,
        tokens: OAuthTokens,
    ) -> Result<(), std::io::Error> {
        let mut entry = self.get(mcp_name).await.unwrap_or_default();
        entry.tokens = Some(tokens);
        self.set(mcp_name, entry).await
    }

    /// Update only the client info portion of an entry.
    pub async fn update_client_info(
        &self,
        mcp_name: &str,
        info: OAuthClientInfo,
        server_url: Option<&str>,
    ) -> Result<(), std::io::Error> {
        let mut entry = self.get(mcp_name).await.unwrap_or_default();
        entry.client_info = Some(info);
        self.set_with_url(mcp_name, entry, server_url).await
    }

    /// Store the PKCE code verifier.
    pub async fn update_code_verifier(
        &self,
        mcp_name: &str,
        code_verifier: &str,
    ) -> Result<(), std::io::Error> {
        let mut entry = self.get(mcp_name).await.unwrap_or_default();
        entry.code_verifier = Some(code_verifier.to_string());
        self.set(mcp_name, entry).await
    }

    /// Clear the stored code verifier.
    pub async fn clear_code_verifier(&self, mcp_name: &str) -> Result<(), std::io::Error> {
        if let Some(mut entry) = self.get(mcp_name).await {
            entry.code_verifier = None;
            self.set(mcp_name, entry).await?;
        }
        Ok(())
    }

    /// Store the OAuth state parameter.
    pub async fn update_oauth_state(
        &self,
        mcp_name: &str,
        state: &str,
    ) -> Result<(), std::io::Error> {
        let mut entry = self.get(mcp_name).await.unwrap_or_default();
        entry.oauth_state = Some(state.to_string());
        self.set(mcp_name, entry).await
    }

    /// Read the stored OAuth state.
    pub async fn get_oauth_state(&self, mcp_name: &str) -> Option<String> {
        self.get(mcp_name).await.and_then(|e| e.oauth_state)
    }

    /// Clear the stored OAuth state.
    pub async fn clear_oauth_state(&self, mcp_name: &str) -> Result<(), std::io::Error> {
        if let Some(mut entry) = self.get(mcp_name).await {
            entry.oauth_state = None;
            self.set(mcp_name, entry).await?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Public free functions – preserved for backward compatibility.
// Each delegates to AuthStore::default_user_store().
// ---------------------------------------------------------------------------

/// Get the auth entry for a given MCP server name.
pub async fn get(mcp_name: &str) -> Option<AuthEntry> {
    AuthStore::default_user_store().get(mcp_name).await
}

/// Get the auth entry only if it was stored for the same `server_url`.
pub async fn get_for_url(mcp_name: &str, server_url: &str) -> Option<AuthEntry> {
    AuthStore::default_user_store()
        .get_for_url(mcp_name, server_url)
        .await
}

/// Persist an auth entry (optionally updating the server URL).
pub async fn set(
    mcp_name: &str,
    entry: AuthEntry,
    server_url: Option<&str>,
) -> Result<(), std::io::Error> {
    AuthStore::default_user_store()
        .set_with_url(mcp_name, entry, server_url)
        .await
}

/// Remove all stored auth data for a server.
pub async fn remove(mcp_name: &str) -> Result<(), std::io::Error> {
    AuthStore::default_user_store().remove(mcp_name).await
}

/// Update only the tokens portion of an entry.
pub async fn update_tokens(
    mcp_name: &str,
    tokens: OAuthTokens,
    server_url: Option<&str>,
) -> Result<(), std::io::Error> {
    let store = AuthStore::default_user_store();
    let mut entry = store.get(mcp_name).await.unwrap_or_default();
    entry.tokens = Some(tokens);
    store.set_with_url(mcp_name, entry, server_url).await
}

/// Update only the client info portion of an entry.
pub async fn update_client_info(
    mcp_name: &str,
    info: OAuthClientInfo,
    server_url: Option<&str>,
) -> Result<(), std::io::Error> {
    AuthStore::default_user_store()
        .update_client_info(mcp_name, info, server_url)
        .await
}

/// Store the PKCE code verifier.
pub async fn update_code_verifier(
    mcp_name: &str,
    code_verifier: &str,
) -> Result<(), std::io::Error> {
    AuthStore::default_user_store()
        .update_code_verifier(mcp_name, code_verifier)
        .await
}

/// Clear the stored code verifier.
pub async fn clear_code_verifier(mcp_name: &str) -> Result<(), std::io::Error> {
    AuthStore::default_user_store()
        .clear_code_verifier(mcp_name)
        .await
}

/// Store the OAuth state parameter.
pub async fn update_oauth_state(mcp_name: &str, state: &str) -> Result<(), std::io::Error> {
    AuthStore::default_user_store()
        .update_oauth_state(mcp_name, state)
        .await
}

/// Read the stored OAuth state.
pub async fn get_oauth_state(mcp_name: &str) -> Option<String> {
    AuthStore::default_user_store()
        .get_oauth_state(mcp_name)
        .await
}

/// Clear the stored OAuth state.
pub async fn clear_oauth_state(mcp_name: &str) -> Result<(), std::io::Error> {
    AuthStore::default_user_store()
        .clear_oauth_state(mcp_name)
        .await
}

/// Check whether stored tokens are expired.
/// Returns `None` if no tokens exist, `Some(false)` if not expired (or no
/// expiry set), `Some(true)` if expired.
pub fn is_token_expired(entry: &AuthEntry) -> Option<bool> {
    let tokens = entry.tokens.as_ref()?;
    match tokens.expires_at {
        Some(exp) => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64();
            Some(exp < now)
        }
        None => Some(false),
    }
}
