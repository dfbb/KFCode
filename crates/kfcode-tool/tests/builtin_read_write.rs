mod common;

#[tokio::test]
async fn write_then_read_round_trips_text() {
    let r = common::fresh_default_registry().await;
    let ws = common::fresh_workspace();
    let dir = ws.path().to_str().unwrap();

    let target = ws.path().join("hello.txt");
    r.execute(
        "write",
        serde_json::json!({
            "file_path": target.to_str().unwrap(),
            "content": "hello world"
        }),
        common::make_ctx(dir),
    )
    .await
    .expect("write ok");

    let on_disk = std::fs::read_to_string(&target).expect("read file");
    assert_eq!(on_disk, "hello world");

    let read_res = r
        .execute(
            "read",
            serde_json::json!({
                "file_path": target.to_str().unwrap()
            }),
            common::make_ctx(dir),
        )
        .await
        .expect("read ok");

    let read_text = serde_json::to_string(&read_res).unwrap();
    assert!(read_text.contains("hello world"), "read result missing content: {read_text}");
}

#[tokio::test]
async fn read_returns_error_for_missing_file() {
    let r = common::fresh_default_registry().await;
    let ws = common::fresh_workspace();
    let missing = ws.path().join("never-existed.txt");
    let res = r
        .execute(
            "read",
            serde_json::json!({"file_path": missing.to_str().unwrap()}),
            common::make_ctx(ws.path().to_str().unwrap()),
        )
        .await;
    assert!(res.is_err(), "expected error for missing file");
}
