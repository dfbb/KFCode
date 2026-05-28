//! Provider implementations and shared types for all LLM backends.
//!
//! Re-exports the public API surface used by the rest of the workspace.

/// Anthropic Claude provider implementation.
pub mod anthropic;
/// Authentication storage and credential management.
pub mod auth;
/// Azure OpenAI Service provider implementation.
pub mod azure;
/// AWS Bedrock provider implementation.
pub mod bedrock;
/// Provider registry bootstrap and configuration loading.
pub mod bootstrap;
/// Cerebras provider implementation.
pub mod cerebras;
/// Cohere provider implementation.
pub mod cohere;
/// Custom HTTP fetch proxy abstraction for provider requests.
pub mod custom_fetch;
/// DeepInfra provider implementation.
pub mod deepinfra;
/// DeepSeek provider implementation.
pub mod deepseek;
/// GitHub Copilot provider implementation.
pub mod github_copilot;
/// GitLab AI provider implementation.
pub mod gitlab;
/// Google Gemini provider implementation.
pub mod google;
/// Groq provider implementation.
pub mod groq;
/// Core chat request and response message types.
pub mod message;
/// Mistral AI provider implementation.
pub mod mistral;
/// models.dev registry types and lookup helpers.
pub mod models;
/// OpenAI provider implementation (chat completions and Responses API).
pub mod openai;
/// OpenRouter provider implementation.
pub mod openrouter;
/// Perplexity provider implementation.
pub mod perplexity;
/// Core `Provider` trait, `ProviderRegistry`, and `ProviderError`.
pub mod provider;
/// OpenAI Responses API types and streaming support.
pub mod responses;
/// Message conversion for the OpenAI Responses API input format.
pub mod responses_convert;
/// Retry logic with exponential backoff and header-based delay.
pub mod retry;
/// Streaming event types and SSE parsing helpers.
pub mod stream;
/// Together AI provider implementation.
pub mod together;
/// Tool definition and preparation types for chat and Responses APIs.
pub mod tools;
/// Message normalization, caching, and provider-option transforms.
pub mod transform;
/// Vercel AI provider implementation.
pub mod vercel;
/// Google Vertex AI provider implementation.
pub mod vertex;
/// xAI (Grok) provider implementation.
pub mod xai;

pub use auth::*;
pub use bootstrap::create_registry_from_env;
pub use bootstrap::create_registry_from_env_with_auth_store;
pub use bootstrap::{
    apply_custom_loaders, bootstrap_config_from_raw, create_registry_from_bootstrap_config,
    filter_models_by_status, BootstrapConfig, ConfigModel, ConfigProvider, CustomLoaderResult,
};
pub use custom_fetch::*;
pub use message::*;
pub use provider::*;
pub use retry::{with_retry, with_retry_and_hook, IsRetryable, RetryConfig};
pub use stream::*;
pub use tools::*;
pub use transform::{
    apply_caching, apply_caching_per_part, extract_reasoning_from_response, max_output_tokens,
    mime_to_modality, normalize_interleaved_thinking, normalize_messages,
    normalize_messages_for_caching, normalize_messages_with_interleaved_field, options,
    provider_options_map, schema, sdk_key, small_options, temperature_for_model, top_k_for_model,
    top_p_for_model, transform_messages, unsupported_parts, variants, Modality, ProviderType,
    OUTPUT_TOKEN_MAX,
};

pub use models::{
    get_model_context_limit, supports_function_calling, supports_vision, ModelCost,
    ModelInfo as ModelsDevInfo, ModelLimit, ModelModalities, ModelsData, ModelsRegistry,
    ProviderInfo as ModelsProviderInfo,
};
