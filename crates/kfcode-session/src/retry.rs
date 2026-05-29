//! Retry delay calculation and cancellable sleep for LLM API calls.
//!
//! Implements exponential back-off with optional `Retry-After` header support,
//! mirroring the TypeScript retry logic in the session processor.

use std::time::Duration;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

/// Initial retry delay in milliseconds.
pub const RETRY_INITIAL_DELAY: u64 = 2000;
/// Multiplier applied to the delay on each successive attempt.
pub const RETRY_BACKOFF_FACTOR: u64 = 2;
/// Maximum delay when no `Retry-After` header is present.
pub const RETRY_MAX_DELAY_NO_HEADERS: u64 = 30_000;
/// Absolute maximum delay (i32::MAX ms, matching the TS constant).
pub const RETRY_MAX_DELAY: u64 = 2_147_483_647;

/// Error returned when a cancellable sleep is interrupted.
#[derive(Debug, thiserror::Error)]
#[error("Sleep cancelled")]
pub struct SleepCancelled;

/// Sleep for `ms` milliseconds, returning early if `cancel` is triggered.
///
/// # Errors
/// Returns `SleepCancelled` if the token is cancelled before the sleep completes.
pub async fn sleep_with_cancel(ms: u64, cancel: CancellationToken) -> Result<(), SleepCancelled> {
    let duration = Duration::from_millis(ms.min(RETRY_MAX_DELAY));

    tokio::select! {
        _ = sleep(duration) => Ok(()),
        _ = cancel.cancelled() => Err(SleepCancelled),
    }
}

/// Compute the delay in milliseconds before the next retry attempt.
///
/// Respects `Retry-After-Ms` and `Retry-After` response headers when present;
/// otherwise applies exponential back-off capped at `RETRY_MAX_DELAY_NO_HEADERS`.
pub fn delay(attempt: u32, error: Option<&ApiErrorInfo>) -> u64 {
    if let Some(err) = error {
        if let Some(ref headers) = err.response_headers {
            if let Some(retry_after_ms) = headers.get("retry-after-ms") {
                if let Ok(parsed_ms) = retry_after_ms.parse::<f64>() {
                    return parsed_ms as u64;
                }
            }

            if let Some(retry_after) = headers.get("retry-after") {
                if let Ok(parsed_seconds) = retry_after.parse::<f64>() {
                    return (parsed_seconds * 1000.0).ceil() as u64;
                }

                if let Ok(parsed_date) = chrono::DateTime::parse_from_rfc2822(retry_after) {
                    let now = chrono::Utc::now();
                    let diff = parsed_date.with_timezone(&chrono::Utc) - now;
                    if diff.num_milliseconds() > 0 {
                        return diff.num_milliseconds() as u64;
                    }
                }
            }

            return RETRY_INITIAL_DELAY * RETRY_BACKOFF_FACTOR.pow(attempt - 1);
        }
    }

    let calculated = RETRY_INITIAL_DELAY * RETRY_BACKOFF_FACTOR.pow(attempt - 1);
    calculated.min(RETRY_MAX_DELAY_NO_HEADERS)
}

/// Structured error information extracted from a failed API response.
#[derive(Debug, Clone)]
pub struct ApiErrorInfo {
    /// Human-readable error message.
    pub message: String,
    /// Whether the caller should retry this request.
    pub is_retryable: bool,
    /// HTTP response headers, used to extract `Retry-After` values.
    pub response_headers: Option<std::collections::HashMap<String, String>>,
    /// Raw response body, used to detect specific error codes.
    pub response_body: Option<String>,
}

/// Determine whether a `MessageError` is retryable and return a display message.
///
/// Returns `Some(message)` if the error should be retried, `None` otherwise.
pub fn retryable(error: &crate::MessageError) -> Option<String> {
    match error {
        crate::MessageError::ContextOverflowError { .. } => None,
        crate::MessageError::ApiError {
            message,
            is_retryable,
            response_body,
            ..
        } => {
            if !is_retryable {
                return None;
            }

            if let Some(body) = response_body {
                if body.contains("FreeUsageLimitError") {
                    return Some(
                        "Free usage exceeded, add credits https://kfcode.ai/zen".to_string(),
                    );
                }
            }

            if message.contains("Overloaded") {
                Some("Provider is overloaded".to_string())
            } else {
                Some(message.clone())
            }
        }
        crate::MessageError::Unknown { message } => {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(message) {
                if let Some(obj) = json.as_object() {
                    let code = obj.get("code").and_then(|c| c.as_str()).unwrap_or("");

                    if obj.get("type").and_then(|t| t.as_str()) == Some("error") {
                        if let Some(error_obj) = obj.get("error").and_then(|e| e.as_object()) {
                            if error_obj.get("type").and_then(|t| t.as_str())
                                == Some("too_many_requests")
                            {
                                return Some("Too Many Requests".to_string());
                            }
                            if error_obj
                                .get("code")
                                .and_then(|c| c.as_str())
                                .map(|c| c.contains("rate_limit"))
                                .unwrap_or(false)
                            {
                                return Some("Rate Limited".to_string());
                            }
                        }
                    }

                    if code.contains("exhausted") || code.contains("unavailable") {
                        return Some("Provider is overloaded".to_string());
                    }

                    return Some(message.clone());
                }
            }
            None
        }
        _ => None,
    }
}

/// Tracks the current retry attempt count and last error for a request loop.
pub struct RetryState {
    /// Current attempt number (0 = not yet attempted).
    pub attempt: u32,
    /// Maximum number of attempts before giving up.
    pub max_attempts: u32,
    /// Error message from the most recent failed attempt.
    pub last_error: Option<String>,
}

impl RetryState {
    /// Create a new state with zero attempts and the given maximum.
    pub fn new(max_attempts: u32) -> Self {
        Self {
            attempt: 0,
            max_attempts,
            last_error: None,
        }
    }

    /// Return true if another attempt is allowed.
    pub fn should_retry(&self) -> bool {
        self.attempt < self.max_attempts
    }

    /// Increment the attempt counter and record the error message.
    pub fn increment(&mut self, error: String) {
        self.attempt += 1;
        self.last_error = Some(error);
    }

    /// Compute the delay in milliseconds before the next attempt.
    pub fn next_delay(&self, error_info: Option<&ApiErrorInfo>) -> u64 {
        delay(self.attempt, error_info)
    }
}
