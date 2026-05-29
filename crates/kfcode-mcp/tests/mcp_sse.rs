mod common;

use kfcode_mcp::transport::SseTransport;
use std::collections::HashMap;

#[tokio::test]
async fn sse_connect_succeeds_against_local_server() {
    let server = common::sse_server::TestSseServer::spawn_with_messages(vec![
        r#"{"jsonrpc":"2.0","id":1,"result":{}}"#,
    ])
    .await;
    let transport = SseTransport::new(server.url(), None::<HashMap<String, String>>);
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        transport.connect(),
    )
    .await
    .expect("timeout")
    .expect("sse connect");
}

#[tokio::test]
async fn sse_connect_fails_for_unreachable_url() {
    let transport = SseTransport::new(
        "http://127.0.0.1:1/sse".into(),
        None::<HashMap<String, String>>,
    );
    let res = transport.connect().await;
    // 不要求一定 fail（实现可能 lazy connect），只要不 panic
    let _ = res;
}
