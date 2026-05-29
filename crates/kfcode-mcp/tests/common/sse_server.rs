#![allow(dead_code)]

use axum::{
    response::sse::{Event, KeepAlive, Sse},
    routing::get,
    Router,
};
use futures::stream;
use std::convert::Infallible;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

pub struct TestSseServer {
    pub addr: SocketAddr,
    handle: JoinHandle<()>,
}

impl TestSseServer {
    pub async fn spawn_with_messages(messages: Vec<&'static str>) -> Self {
        let app = Router::new().route(
            "/sse",
            get(move || async move {
                let stream = stream::iter(messages.into_iter().map(|m| {
                    Ok::<_, Infallible>(Event::default().data(m))
                }));
                Sse::new(stream).keep_alive(KeepAlive::default())
            }),
        );

        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let handle = tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        Self { addr, handle }
    }

    pub fn url(&self) -> String {
        format!("http://{}/sse", self.addr)
    }
}

impl Drop for TestSseServer {
    fn drop(&mut self) {
        self.handle.abort();
    }
}
