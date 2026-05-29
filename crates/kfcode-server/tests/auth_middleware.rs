/// Tests for the Bearer token auth middleware.
///
/// These tests mutate `KFCODE_SERVER_PASSWORD` and must run serially to avoid
/// races.  A process-wide `Mutex` serializes all tests in this binary.
mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Helper: build a GET /health request with an optional Authorization header.
fn health_req(auth: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder().uri("/health").method("GET");
    if let Some(value) = auth {
        builder = builder.header("Authorization", value);
    }
    builder.body(Body::empty()).unwrap()
}

/// Helper: build a GET /health request with a ?token= query parameter.
fn health_req_with_token_param(token: &str) -> Request<Body> {
    Request::builder()
        .uri(format!("/health?token={}", token))
        .method("GET")
        .body(Body::empty())
        .unwrap()
}

// ── SF6-1: no password set → 200 (backward compat) ──────────────────────────

#[tokio::test]
async fn no_password_set_allows_all_requests() {
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::remove_var("KFCODE_SERVER_PASSWORD");

    let state = common::fresh_state_in_memory();
    let app = common::app_with(state);
    let res = common::oneshot_call(app, health_req(None)).await;
    assert_eq!(res.status(), StatusCode::OK);
}

// ── SF6-2: password set, no Authorization header → 401 ──────────────────────

#[tokio::test]
async fn password_set_no_auth_header_returns_401() {
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::set_var("KFCODE_SERVER_PASSWORD", "secret");

    let state = common::fresh_state_in_memory();
    let app = common::app_with(state);
    let res = common::oneshot_call(app, health_req(None)).await;

    std::env::remove_var("KFCODE_SERVER_PASSWORD");
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

// ── SF6-3: password set, wrong token → 401 ──────────────────────────────────

#[tokio::test]
async fn password_set_wrong_token_returns_401() {
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::set_var("KFCODE_SERVER_PASSWORD", "secret");

    let state = common::fresh_state_in_memory();
    let app = common::app_with(state);
    let res = common::oneshot_call(app, health_req(Some("Bearer wrong"))).await;

    std::env::remove_var("KFCODE_SERVER_PASSWORD");
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

// ── SF6-4: password set, correct Bearer token → 200 ─────────────────────────

#[tokio::test]
async fn password_set_correct_bearer_returns_200() {
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::set_var("KFCODE_SERVER_PASSWORD", "secret");

    let state = common::fresh_state_in_memory();
    let app = common::app_with(state);
    let res = common::oneshot_call(app, health_req(Some("Bearer secret"))).await;

    std::env::remove_var("KFCODE_SERVER_PASSWORD");
    assert_eq!(res.status(), StatusCode::OK);
}

// ── SF6-5: password set, correct ?token= query param → 200 ──────────────────

#[tokio::test]
async fn password_set_correct_query_token_returns_200() {
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::set_var("KFCODE_SERVER_PASSWORD", "secret");

    let state = common::fresh_state_in_memory();
    let app = common::app_with(state);
    let res = common::oneshot_call(app, health_req_with_token_param("secret")).await;

    std::env::remove_var("KFCODE_SERVER_PASSWORD");
    assert_eq!(res.status(), StatusCode::OK);
}
