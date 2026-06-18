//! Shared bootstrap for the collab integration-test binaries: build the Socket.IO layer,
//! mount it on an axum router, serve on an ephemeral port, and return the base URL.

/// Spawn the relay server exactly as production mounts it; returns `http://<addr>`.
pub async fn spawn_server() -> String {
    let (layer, _io) = oxydraw_collab::build();
    let app = axum::Router::new().layer(layer);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{addr}")
}
