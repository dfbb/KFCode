mod common;

#[tokio::test]
async fn goto_definition_returns_none_for_default_stub() {
    let (client, root) = common::start_default_stub().await;
    let file = root.path().join("a.rs");
    std::fs::write(&file, "fn x() {}\n").unwrap();
    client.open_document(&file, "fn x() {}\n", "rust").await.unwrap();

    let result = client
        .goto_definition(&file, 0, 3)
        .await
        .expect("goto_definition request");
    assert!(result.is_none(), "default stub returns null result");
}

#[tokio::test]
async fn hover_returns_none_for_default_stub() {
    let (client, root) = common::start_default_stub().await;
    let file = root.path().join("a.rs");
    std::fs::write(&file, "fn x() {}\n").unwrap();
    client.open_document(&file, "fn x() {}\n", "rust").await.unwrap();

    let result = client.hover(&file, 0, 3).await.expect("hover request");
    assert!(result.is_none());
}

#[tokio::test]
async fn references_returns_empty_for_default_stub() {
    let (client, root) = common::start_default_stub().await;
    let file = root.path().join("a.rs");
    std::fs::write(&file, "fn x() {}\n").unwrap();
    client.open_document(&file, "fn x() {}\n", "rust").await.unwrap();

    let result = client.references(&file, 0, 3).await.expect("references request");
    assert!(result.is_empty(), "stub returns null -> client deserializes to empty vec");
}
