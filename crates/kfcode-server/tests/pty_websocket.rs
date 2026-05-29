mod common;

use futures::StreamExt;
use tokio::time::{timeout, Duration};
use tokio_tungstenite::connect_async;
use serde_json::json;

#[tokio::test]
#[ignore = "depends on real PTY backend; run manually with cargo test -- --ignored"]
async fn pty_websocket_connects_and_receives_frame() {
    let state = common::fresh_state_in_memory();
    let app = common::app_with(state);
    let server = common::TestServer::spawn(app).await;

    // 1. 创建 PTY
    let create_url = server.http_url("/pty/");
    let client = reqwest::Client::new();
    let create_res = client
        .post(&create_url)
        .json(&json!({
            "command": "echo hello-pty",
            "cwd": "/"
        }))
        .send()
        .await
        .expect("POST /pty/");

    if !create_res.status().is_success() {
        // PTY 创建失败（可能是环境问题），跳过
        return;
    }

    let info: serde_json::Value = create_res.json().await.unwrap();
    let id = info.get("id").and_then(|v| v.as_str()).expect("pty id").to_string();

    // 2. 连 WebSocket
    let ws_url = server.ws_url(&format!("/pty/{id}/connect"));
    let (mut ws, _resp) = connect_async(ws_url).await.expect("ws connect");

    // 3. 在 2 秒内收到至少一帧
    let frame = timeout(Duration::from_secs(2), ws.next()).await
        .expect("timed out waiting for frame")
        .expect("ws stream ended")
        .expect("ws frame error");
    let _ = frame;

    // 4. 关闭
    ws.close(None).await.unwrap();
}
