mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};

#[tokio::test]
async fn provider_list_returns_200() {
    let state = common::fresh_state_in_memory();
    let app = common::app_with(state);
    let res = common::oneshot_call(
        app,
        Request::builder().uri("/provider").body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn file_content_returns_file_bytes() {
    // The /file/content handler resolves paths relative to current_dir() and
    // rejects paths that escape it. Create a file inside cwd and use a relative path.
    let cwd = std::env::current_dir().unwrap();
    let file = cwd.join("_test_hello.txt");
    std::fs::write(&file, "hello world").unwrap();

    let state = common::fresh_state_in_memory();
    let app = common::app_with(state);

    let res = common::oneshot_call(
        app,
        Request::builder()
            .uri("/file/content?path=_test_hello.txt")
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    let _ = std::fs::remove_file(&file);

    assert_eq!(res.status(), StatusCode::OK);
    let body = common::body_to_bytes(res).await;
    assert!(
        std::str::from_utf8(&body).unwrap_or("").contains("hello world"),
        "expected file body in response"
    );
}

#[tokio::test]
async fn file_content_returns_error_for_missing() {
    let state = common::fresh_state_in_memory();
    let app = common::app_with(state);
    let res = common::oneshot_call(
        app,
        Request::builder()
            .uri("/file/content?path=/no/such/file/here")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_ne!(res.status(), StatusCode::OK, "missing file must not return 200");
}
