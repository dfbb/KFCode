mod common;

use kfcode_mcp::McpClient;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, ResponseTemplate};

/// Mount the three-step MCP handshake: initialize → notifications/initialized → tools/list.
async fn mount_initialize_sequence(server: &wiremock::MockServer) {
    // initialize
    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_partial_json(
            serde_json::json!({"method": "initialize"}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "test-server", "version": "1.0" }
            }
        })))
        .mount(server)
        .await;

    // notifications/initialized — client sends this as a request and ignores the result,
    // but HttpTransport still POSTs and reads the body. Return a valid JSON-RPC response
    // so the channel doesn't stall.
    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_partial_json(
            serde_json::json!({"method": "notifications/initialized"}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "result": {}
        })))
        .mount(server)
        .await;

    // tools/list
    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_partial_json(
            serde_json::json!({"method": "tools/list"}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "result": {
                "tools": [
                    {
                        "name": "echo",
                        "description": "echo input",
                        "inputSchema": { "type": "object" }
                    }
                ]
            }
        })))
        .mount(server)
        .await;
}

#[tokio::test]
async fn http_connect_completes_handshake_and_lists_tools() {
    let server = common::fresh_mock_server().await;
    mount_initialize_sequence(&server).await;

    let registry = common::fresh_registry();
    let client = McpClient::http(
        "test".to_string(),
        registry.clone(),
        server.uri(),
        None,
    )
    .await
    .expect("connect http");

    assert!(client.is_initialized().await, "must be initialized");

    let tools = registry.list().await;
    assert_eq!(tools.len(), 1, "registry must contain exactly one tool");
    assert_eq!(tools[0].name, "echo");
    assert_eq!(tools[0].server_name, "test");
    assert_eq!(tools[0].full_name, "test_echo");
}

#[tokio::test]
async fn http_call_tool_returns_result() {
    let server = common::fresh_mock_server().await;
    mount_initialize_sequence(&server).await;

    // tools/call
    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_partial_json(
            serde_json::json!({"method": "tools/call"}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 4,
            "result": {
                "content": [{"type": "text", "text": "hello"}],
                "isError": false
            }
        })))
        .mount(&server)
        .await;

    let client = McpClient::http(
        "t".to_string(),
        common::fresh_registry(),
        server.uri(),
        None,
    )
    .await
    .unwrap();

    let res = client
        .call_tool("echo", Some(serde_json::json!({"text": "hello"})))
        .await
        .expect("call_tool");

    assert_eq!(res.content.len(), 1);
    assert_eq!(res.content[0].text.as_deref(), Some("hello"));
}
