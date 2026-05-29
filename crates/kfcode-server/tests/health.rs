mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};

#[tokio::test]
async fn health_returns_200_ok() {
    let state = common::fresh_state_in_memory();
    let app = common::app_with(state);

    let res = common::oneshot_call(
        app,
        Request::builder().uri("/health").body(Body::empty()).unwrap(),
    )
    .await;

    assert_eq!(res.status(), StatusCode::OK);
    let body = common::body_to_bytes(res).await;
    let body_str = std::str::from_utf8(&body).unwrap();
    assert!(body_str.contains(r#""status":"ok""#), "got: {body_str}");
}
