//! Minimal LSP stub server for integration tests.
use std::io::{BufRead, BufReader, Read, Write};

fn main() {
    let mode = std::env::var("STUB_MODE").unwrap_or_else(|_| "default".to_string());
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    if mode == "no-response" {
        let mut sink = String::new();
        let _ = stdin.lock().read_to_string(&mut sink);
        return;
    }

    let mut reader = BufReader::new(stdin.lock());
    loop {
        let mut content_length = 0usize;
        loop {
            let mut header = String::new();
            if reader.read_line(&mut header).unwrap_or(0) == 0 {
                return;
            }
            if header == "\r\n" || header.is_empty() {
                break;
            }
            if let Some(rest) = header.trim_end().strip_prefix("Content-Length: ") {
                content_length = rest.parse().unwrap_or(0);
            }
        }
        if content_length == 0 {
            continue;
        }
        let mut buf = vec![0u8; content_length];
        if reader.read_exact(&mut buf).is_err() {
            return;
        }
        let body = String::from_utf8(buf).unwrap_or_default();
        let req: serde_json::Value = match serde_json::from_str(&body) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let id = req.get("id").cloned();
        let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");

        if id.is_none() {
            continue;
        }

        let response: serde_json::Value = if mode == "always-error" {
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {"code": -32601, "message": format!("stub: forced error for {method}")}
            })
        } else {
            match method {
                "initialize" => serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "capabilities": {
                            "textDocumentSync": 1,
                            "definitionProvider": true,
                            "hoverProvider": true,
                            "referencesProvider": true,
                        },
                        "serverInfo": {"name": "lsp-test-stub", "version": "0"}
                    }
                }),
                "shutdown" => serde_json::json!({"jsonrpc":"2.0","id":id,"result":null}),
                _ => serde_json::json!({"jsonrpc":"2.0","id":id,"result":null}),
            }
        };

        let body = response.to_string();
        write!(stdout, "Content-Length: {}\r\n\r\n{}\n", body.len(), body).unwrap();
        stdout.flush().unwrap();
    }
}
