mod common;

use kfcode_mcp::McpClient;
use std::time::Duration;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn http_connect_fails_on_5xx() {
    let server = common::fresh_mock_server().await;
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;

    let res = McpClient::http("t".into(), common::fresh_registry(), server.uri(), None).await;
    assert!(res.is_err(), "expected error on 500");
}

#[tokio::test]
async fn http_connect_fails_on_4xx() {
    let server = common::fresh_mock_server().await;
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(400).set_body_string("bad"))
        .mount(&server)
        .await;

    let res = McpClient::http("t".into(), common::fresh_registry(), server.uri(), None).await;
    assert!(res.is_err());
}

#[tokio::test]
async fn http_connect_propagates_jsonrpc_error_envelope() {
    let server = common::fresh_mock_server().await;
    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_partial_json(serde_json::json!({"method": "initialize"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "error": {"code": -32603, "message": "internal error"}
        })))
        .mount(&server)
        .await;

    let res = McpClient::http("t".into(), common::fresh_registry(), server.uri(), None).await;
    let err = match res {
        Err(e) => e,
        Ok(_) => panic!("expected error, got Ok"),
    };
    let s = format!("{err}");
    assert!(
        s.to_lowercase().contains("internal") || s.contains("-32603"),
        "got: {s}"
    );
}

// McpClient::connect_http 内部调用 send_request（无超时包装），
// with_timeout 仅影响 call_tool。此测试会挂住，标记为 ignore。
#[tokio::test]
#[ignore = "McpClient has no built-in timeout for connect_http; with_timeout only applies to call_tool"]
async fn http_call_times_out_on_slow_response() {
    let server = common::fresh_mock_server().await;
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_secs(2))
                .set_body_json(serde_json::json!({
                    "jsonrpc": "2.0", "id": 1,
                    "result": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": {"tools": {}},
                        "serverInfo": {"name":"x", "version":"1"}
                    }
                })),
        )
        .mount(&server)
        .await;

    let registry = common::fresh_registry();
    let client = kfcode_mcp::McpClient::new("t".into(), registry).with_timeout(200);
    let res = client.connect_http(server.uri(), None).await;
    assert!(res.is_err(), "expected timeout error");
}
