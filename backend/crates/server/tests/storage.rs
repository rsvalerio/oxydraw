//! End-to-end test of the clean file-blob API: upload an opaque file under its
//! content-addressed id (`PUT /api/files/{id}`), then fetch it back (`GET /api/files/{id}`),
//! the way the editor / collaboration client does.

use std::sync::Arc;

use oxydraw_core::config::Config;
use oxydraw_storage::MemoryStore;

mod common;
use common::{spawn_app_with_store as spawn_with, test_config};

async fn spawn() -> std::net::SocketAddr {
    spawn_with_store(Arc::new(MemoryStore::new())).await
}

async fn spawn_with_store(store: Arc<MemoryStore>) -> std::net::SocketAddr {
    spawn_with(test_config(), store).await
}

async fn put_status(
    client: &reqwest::Client,
    base: &str,
    id: &str,
    content_type: &str,
    payload: &[u8],
) -> reqwest::StatusCode {
    client
        .put(format!("{base}/api/files/{id}"))
        .header("content-type", content_type)
        .body(payload.to_vec())
        .send()
        .await
        .unwrap()
        .status()
}

async fn put(client: &reqwest::Client, base: &str, id: &str, payload: &[u8]) {
    assert_eq!(
        put_status(client, base, id, "application/octet-stream", payload).await,
        200
    );
}

#[tokio::test]
async fn files_round_trip_by_id() {
    let addr = spawn().await;
    let base = format!("http://{addr}");
    let client = reqwest::Client::new();

    // Id as Excalidraw builds it: a content hash, a single path segment.
    let id = "file-abc123";
    let payload: Vec<u8> = (0..=255u8).collect();
    put(&client, &base, id, &payload).await;

    let r = client
        .get(format!("{base}/api/files/{id}"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    assert_eq!(
        r.headers()["content-type"].to_str().unwrap(),
        "application/octet-stream"
    );
    assert_eq!(r.bytes().await.unwrap().as_ref(), payload.as_slice());

    // Unknown ids are 404.
    let r = client
        .get(format!("{base}/api/files/missing"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 404);
}

#[tokio::test]
async fn files_survive_restart() {
    let store = Arc::new(MemoryStore::new());
    let addr = spawn_with_store(store.clone()).await;
    let base = format!("http://{addr}");
    let client = reqwest::Client::new();

    put(&client, &base, "file-1", b"keep me").await;

    // "Restart": a fresh server sharing the same durable store.
    let addr = spawn_with_store(store).await;
    let base = format!("http://{addr}");

    let r = client
        .get(format!("{base}/api/files/file-1"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    assert_eq!(r.bytes().await.unwrap().as_ref(), b"keep me");
}

#[tokio::test]
async fn uploads_past_the_file_quota_are_rejected() {
    let config = Config {
        max_files_bytes: 10,
        ..test_config()
    };
    let addr = spawn_with(config, Arc::new(MemoryStore::new())).await;
    let base = format!("http://{addr}");
    let client = reqwest::Client::new();

    // First upload fits under the 10-byte quota.
    put(&client, &base, "f1", b"12345678").await;

    // The next would exceed it and is rejected with 507.
    let status = put_status(
        &client,
        &base,
        "f2",
        "application/octet-stream",
        b"12345678",
    )
    .await;
    assert_eq!(status, 507);
}

/// SEC-33: the row cap rejects uploads with 507 once the table holds `max_files_count` rows,
/// even when the byte quota is nowhere near tripping.
#[tokio::test]
async fn uploads_past_the_row_cap_are_rejected() {
    let config = Config {
        max_files_count: 2,
        ..test_config()
    };
    let addr = spawn_with(config, Arc::new(MemoryStore::new())).await;
    let base = format!("http://{addr}");
    let client = reqwest::Client::new();

    // Under the cap: rows 1 and 2 are accepted.
    put(&client, &base, "f1", b"a").await;
    put(&client, &base, "f2", b"b").await;

    // At the cap: the next upload is rejected.
    let status = put_status(&client, &base, "f3", "application/octet-stream", b"c").await;
    assert_eq!(status, 507);
}

/// SEC-33: the byte quota charges only payload bytes, so an over-long id (which would
/// otherwise be stored free) is rejected with 400 before reaching the store.
#[tokio::test]
async fn oversized_ids_are_rejected() {
    let addr = spawn().await;
    let base = format!("http://{addr}");
    let client = reqwest::Client::new();

    // An id well past the 256-byte bound, tiny payload — the unbounded-growth vector.
    let long = "x".repeat(300);
    assert_eq!(
        put_status(&client, &base, &long, "application/octet-stream", b"1").await,
        400
    );

    // A normal-length id still succeeds.
    put(&client, &base, "f1", b"1").await;
}

/// SEC-11: an attacker uploading an HTML payload with `Content-Type: text/html` must not get
/// it served back as an active on-origin HTML document — the type is replaced with
/// `application/octet-stream` and the response is marked `Content-Disposition: attachment`.
#[tokio::test]
async fn downloads_never_reflect_active_content_types() {
    let addr = spawn().await;
    let base = format!("http://{addr}");
    let client = reqwest::Client::new();

    put_status(
        &client,
        &base,
        "evil",
        "text/html",
        b"<script>alert(document.cookie)</script>",
    )
    .await;

    let r = client
        .get(format!("{base}/api/files/evil"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    assert_eq!(
        r.headers()["content-type"].to_str().unwrap(),
        "application/octet-stream"
    );
    assert_eq!(
        r.headers()["content-disposition"].to_str().unwrap(),
        "attachment"
    );

    // Inert image types are still reflected (with the attachment disposition).
    put_status(&client, &base, "img", "image/png", b"\x89PNG").await;
    let r = client
        .get(format!("{base}/api/files/img"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.headers()["content-type"].to_str().unwrap(), "image/png");
    assert_eq!(
        r.headers()["content-disposition"].to_str().unwrap(),
        "attachment"
    );
}
