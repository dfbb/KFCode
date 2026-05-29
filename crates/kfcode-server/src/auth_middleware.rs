//! Bearer token authentication middleware for all HTTP and WebSocket routes.
//!
//! When `KFCODE_SERVER_PASSWORD` is set, every request must supply a matching
//! `Authorization: Bearer <token>` header.  WebSocket clients that cannot set
//! arbitrary headers may pass the token as a `?token=<value>` query parameter.
//!
//! When the environment variable is **not** set the middleware emits a `warn!`
//! and passes the request through unchanged, preserving backward compatibility
//! for local no-password setups.

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};

pub async fn require_auth(req: Request, next: Next) -> Result<Response, StatusCode> {
    let expected = std::env::var("KFCODE_SERVER_PASSWORD").ok();

    // No password configured → warn once per request and allow through.
    let expected = match expected {
        Some(p) if !p.is_empty() => p,
        _ => {
            tracing::warn!(
                "KFCODE_SERVER_PASSWORD is not set; all requests are allowed without authentication"
            );
            return Ok(next.run(req).await);
        }
    };

    // Try Authorization: Bearer <token> header first.
    let token_from_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .map(|t| t.to_string());

    // Fall back to ?token=<value> query parameter (for WebSocket clients).
    let token = token_from_header.or_else(|| {
        req.uri().query().and_then(|q| {
            // Simple key=value scan; percent-decoding not needed for bearer tokens.
            q.split('&').find_map(|pair| {
                let mut parts = pair.splitn(2, '=');
                let key = parts.next()?;
                let val = parts.next()?;
                if key == "token" { Some(val.to_string()) } else { None }
            })
        })
    });

    match token {
        Some(t) if t == expected => Ok(next.run(req).await),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}
