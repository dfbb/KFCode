mod common;

#[tokio::test]
async fn stub_starts_and_completes_initialize() {
    let (_client, _root) = common::start_default_stub().await;
}
