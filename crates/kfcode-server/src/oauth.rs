//! Provider authentication helpers that delegate to plugin auth bridges for OAuth and API-key flows.
use std::collections::HashMap;
use std::sync::Arc;

use kfcode_plugin::subprocess::PluginLoader;
use kfcode_provider::{AuthError, AuthInfo, AuthManager, AuthMethodType, Authorization};
use serde::{Deserialize, Serialize};

/// Describes a single authentication method offered by a provider plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthMethodInfo {
    #[serde(rename = "type")]
    pub method_type: String,
    pub label: String,
}

/// Wraps an `AuthManager` and exposes provider authentication operations backed by plugin bridges.
pub struct ProviderAuth {
    auth_manager: Arc<AuthManager>,
}

impl ProviderAuth {
    /// Creates a new `ProviderAuth` wrapping the given auth manager.
    pub fn new(auth_manager: Arc<AuthManager>) -> Self {
        Self { auth_manager }
    }

    /// Returns all available authentication methods grouped by provider ID.
    pub async fn methods(loader: &PluginLoader) -> HashMap<String, Vec<AuthMethodInfo>> {
        let bridges = loader.auth_bridges().await;
        bridges
            .iter()
            .map(|(provider, bridge)| {
                let methods = bridge
                    .methods()
                    .iter()
                    .map(|method| AuthMethodInfo {
                        method_type: method.method_type.clone(),
                        label: method.label.clone(),
                    })
                    .collect::<Vec<_>>();
                (provider.clone(), methods)
            })
            .collect()
    }

    /// Initiates an authorization flow for the given provider and method index, returning the redirect URL and instructions.
    pub async fn authorize(
        loader: &PluginLoader,
        provider_id: &str,
        method: usize,
        inputs: Option<HashMap<String, String>>,
    ) -> Result<Authorization, AuthError> {
        let bridge = loader
            .auth_bridge(provider_id)
            .await
            .ok_or_else(|| AuthError::OauthMissing(provider_id.to_string()))?;
        let result = bridge
            .authorize(method, inputs)
            .await
            .map_err(|_| AuthError::OauthCallbackFailed)?;

        let method_type = match result.method.as_deref() {
            Some("code") => AuthMethodType::Code,
            _ => AuthMethodType::Auto,
        };

        Ok(Authorization {
            url: result.url.unwrap_or_default(),
            method: method_type,
            instructions: result.instructions.unwrap_or_default(),
        })
    }

    /// Handles the OAuth callback for the given provider, persisting the resulting credentials to the auth manager.
    pub async fn callback(
        &self,
        loader: &PluginLoader,
        provider_id: &str,
        code: Option<&str>,
    ) -> Result<(), AuthError> {
        let bridge = loader
            .auth_bridge(provider_id)
            .await
            .ok_or_else(|| AuthError::OauthMissing(provider_id.to_string()))?;
        let result = bridge
            .callback(code)
            .await
            .map_err(|_| AuthError::OauthCallbackFailed)?;

        let auth_type = result.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if auth_type != "success" {
            return Err(AuthError::OauthCallbackFailed);
        }

        // Plugin callback can override target provider (e.g. copilot enterprise).
        let target_provider = result
            .get("provider")
            .and_then(|v| v.as_str())
            .unwrap_or(provider_id);

        if let Some(key) = result
            .get("key")
            .and_then(|v| v.as_str())
            .or_else(|| result.get("apiKey").and_then(|v| v.as_str()))
            .or_else(|| result.get("token").and_then(|v| v.as_str()))
        {
            self.auth_manager
                .set(
                    target_provider,
                    AuthInfo::Api {
                        key: key.to_string(),
                    },
                )
                .await;
            return Ok(());
        }

        let access = result
            .get("access")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let refresh = result
            .get("refresh")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        if access.is_empty() && refresh.is_empty() {
            return Err(AuthError::OauthCallbackFailed);
        }

        self.auth_manager
            .set(
                target_provider,
                AuthInfo::OAuth {
                    access,
                    refresh,
                    expires: result.get("expires").and_then(|v| v.as_i64()),
                    account_id: result
                        .get("accountId")
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                    enterprise_url: result
                        .get("enterpriseUrl")
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                },
            )
            .await;

        Ok(())
    }

    /// Stores a plain API key for the given provider in the auth manager.
    pub async fn set_api_key(&self, provider_id: &str, key: String) {
        self.auth_manager
            .set(provider_id, AuthInfo::Api { key })
            .await;
    }

    /// Removes all stored credentials for the given provider from the auth manager.
    pub async fn remove(&self, provider_id: &str) {
        self.auth_manager.remove(provider_id).await;
    }
}
