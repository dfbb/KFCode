mod common;

#[tokio::test]
async fn mock_server_starts_and_stops() {
    let server = common::fresh_mock_server().await;
    let url = server.uri();
    assert!(url.starts_with("http://"));
    drop(server);
}

#[tokio::test]
async fn registry_constructs() {
    let _r = common::fresh_registry();
}
