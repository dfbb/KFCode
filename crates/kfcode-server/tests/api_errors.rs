mod common;

use axum::body::to_bytes;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use kfcode_server::ApiError;
use serde_json::Value;

async fn status_and_type(err: ApiError) -> (StatusCode, String) {
    let res = err.into_response();
    let status = res.status();
    let body_bytes = to_bytes(res.into_body(), 1024 * 1024).await.unwrap();
    let v: Value = serde_json::from_slice(&body_bytes).unwrap();
    let t = v.get("error")
        .and_then(|e| e.get("type"))
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();
    (status, t)
}

#[tokio::test]
async fn session_not_found_maps_to_404() {
    let (s, t) = status_and_type(ApiError::SessionNotFound("x".into())).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    assert_eq!(t, "session_not_found");
}

#[tokio::test]
async fn not_found_maps_to_404() {
    let (s, t) = status_and_type(ApiError::NotFound("nope".into())).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    assert_eq!(t, "not_found");
}

#[tokio::test]
async fn bad_request_maps_to_400() {
    let (s, t) = status_and_type(ApiError::BadRequest("bad".into())).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    assert_eq!(t, "bad_request");
}

#[tokio::test]
async fn invalid_request_maps_to_400() {
    let (s, t) = status_and_type(ApiError::InvalidRequest("invalid".into())).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    assert_eq!(t, "invalid_request");
}

#[tokio::test]
async fn provider_error_maps_to_502() {
    let (s, t) = status_and_type(ApiError::ProviderError("upstream".into())).await;
    assert_eq!(s, StatusCode::BAD_GATEWAY);
    assert_eq!(t, "provider_error");
}

#[tokio::test]
async fn internal_error_maps_to_500() {
    let (s, t) = status_and_type(ApiError::InternalError("boom".into())).await;
    assert_eq!(s, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(t, "internal_error");
}
