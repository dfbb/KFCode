mod common;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header::CONTENT_TYPE};
use serde_json::Value;

#[tokio::test]
async fn create_session_returns_200_with_id() {
    let (state, _dir) = common::state_with_temp_db().await;
    let app = common::app_with(state);
    let res = common::oneshot_call(
        app,
        Request::builder()
            .method(Method::POST)
            .uri("/session")
            .header(CONTENT_TYPE, "application/json")
            .header("x-kfcode-directory", "/tmp/test")
            .body(Body::from(r#"{}"#))
            .unwrap(),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let body = common::body_to_bytes(res).await;
    let v: Value = serde_json::from_slice(&body).unwrap();
    assert!(v.get("id").is_some(), "response missing id: {v}");
}

#[tokio::test]
async fn get_session_returns_404_for_missing() {
    let (state, _dir) = common::state_with_temp_db().await;
    let app = common::app_with(state);
    let res = common::oneshot_call(
        app,
        Request::builder()
            .uri("/session/does-not-exist")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn list_sessions_returns_200() {
    let (state, _dir) = common::state_with_temp_db().await;
    let app = common::app_with(state);
    let res = common::oneshot_call(
        app,
        Request::builder().uri("/session").body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn each_test_has_isolated_storage() {
    let (state_a, _da) = common::state_with_temp_db().await;
    let (state_b, _db) = common::state_with_temp_db().await;
    let app_a = common::app_with(state_a.clone());
    let app_b = common::app_with(state_b.clone());

    let res_a = common::oneshot_call(
        app_a,
        Request::builder()
            .method(Method::POST)
            .uri("/session")
            .header(CONTENT_TYPE, "application/json")
            .header("x-kfcode-directory", "/tmp/a")
            .body(Body::from(r#"{}"#))
            .unwrap(),
    )
    .await;
    assert_eq!(res_a.status(), StatusCode::OK);

    let res_b = common::oneshot_call(
        app_b,
        Request::builder().uri("/session").body(Body::empty()).unwrap(),
    )
    .await;
    let body_b = common::body_to_bytes(res_b).await;
    let v: Value = serde_json::from_slice(&body_b).unwrap();
    let arr = v.as_array().expect("array");
    assert!(arr.is_empty(), "state_b should not see state_a sessions; got {v}");
}
