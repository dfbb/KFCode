#![allow(dead_code)]

use axum::body::{Body, to_bytes};
use axum::http::{Request, Response};
use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tower::util::ServiceExt;

use kfcode_server::{router, ServerState};

pub fn fresh_state_in_memory() -> Arc<ServerState> {
    Arc::new(ServerState::new())
}

pub fn app_with(state: Arc<ServerState>) -> Router {
    router().with_state(state)
}

pub async fn oneshot_call(app: Router, req: Request<Body>) -> Response<Body> {
    app.oneshot(req).await.expect("oneshot")
}

pub async fn body_to_bytes(res: Response<Body>) -> bytes::Bytes {
    to_bytes(res.into_body(), 1024 * 1024).await.expect("body")
}

pub struct TestServer {
    pub addr: SocketAddr,
    handle: JoinHandle<()>,
}

impl TestServer {
    pub async fn spawn(app: Router) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let handle = tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        Self { addr, handle }
    }

    pub fn http_url(&self, path: &str) -> String {
        format!("http://{}{}", self.addr, path)
    }

    pub fn ws_url(&self, path: &str) -> String {
        format!("ws://{}{}", self.addr, path)
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

use kfcode_storage::Database;

pub async fn state_with_temp_db() -> (Arc<ServerState>, tempfile::TempDir) {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("server-test.db");
    let db = Database::open_at(&path).await.expect("open_at");
    let state = ServerState::new_with_database(db, "http://test".into())
        .await
        .expect("inject");
    (Arc::new(state), dir)
}
