mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};

fn patch_req(body: &str) -> Request<Body> {
    Request::builder()
        .uri("/config")
        .method("PATCH")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

// ── SF7-1: PATCH with a safe field updates CONFIG_STATE ──────────────────────

#[tokio::test]
async fn patch_safe_field_returns_200_and_updates_config() {
    let state = common::fresh_state_in_memory();
    let app = common::app_with(state);

    let res = common::oneshot_call(app, patch_req(r#"{"model":"gpt-4o"}"#)).await;
    assert_eq!(res.status(), StatusCode::OK);

    let body = common::body_to_bytes(res).await;
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["model"], "gpt-4o");
}

// ── SF7-2: PATCH with provider.api_key → 422 (deny_unknown_fields) ───────────

#[tokio::test]
async fn patch_with_provider_field_returns_422() {
    let state = common::fresh_state_in_memory();
    let app = common::app_with(state);

    let res = common::oneshot_call(
        app,
        patch_req(r#"{"provider":{"openai":{"api_key":"evil"}}}"#),
    )
    .await;
    // serde deny_unknown_fields causes a deserialization error → 422
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

// ── SF7-3: PATCH with unknown top-level field → 422 ─────────────────────────

#[tokio::test]
async fn patch_with_unknown_field_returns_422() {
    let state = common::fresh_state_in_memory();
    let app = common::app_with(state);

    let res = common::oneshot_call(app, patch_req(r#"{"experimental":{"foo":true}}"#)).await;
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
