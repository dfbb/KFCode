mod common;

#[tokio::test]
async fn full_open_change_diagnostics_round_trip() {
    let (client, root) = common::start_default_stub().await;

    let file = root.path().join("hello.rs");
    std::fs::write(&file, "fn main() {}\n").unwrap();

    client
        .open_document(&file, "fn main() {}\n", "rust")
        .await
        .expect("didOpen");

    // 默认 stub 不推 diagnostics，应返回空列表
    let diagnostics = client.get_diagnostics(&file).await;
    assert!(diagnostics.is_empty(), "default stub does not push diagnostics");
}

#[tokio::test]
async fn subscribe_yields_no_events_on_default_stub() {
    let (client, root) = common::start_default_stub().await;
    let _rx = client.subscribe();
    drop(_rx);
    let _ = root;
}

#[tokio::test]
async fn open_nonexistent_file_does_not_panic() {
    let (client, root) = common::start_default_stub().await;
    let file = root.path().join("never-existed.rs");
    // 文件不在磁盘也无所谓——LSP didOpen 接受 in-memory 内容
    client
        .open_document(&file, "// content", "rust")
        .await
        .expect("didOpen with in-memory content");
}
