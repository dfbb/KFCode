mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};

#[tokio::test]
async fn config_get_returns_200() {
    let state = common::fresh_state_in_memory();
    let app = common::app_with(state);
    let res = common::oneshot_call(
        app,
        Request::builder().uri("/config").body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn config_providers_returns_200() {
    let state = common::fresh_state_in_memory();
    let app = common::app_with(state);
    let res = common::oneshot_call(
        app,
        Request::builder().uri("/config/providers").body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn unknown_path_returns_404() {
    let state = common::fresh_state_in_memory();
    let app = common::app_with(state);
    let res = common::oneshot_call(
        app,
        Request::builder().uri("/no/such/path").body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
