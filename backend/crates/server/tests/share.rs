//! End-to-end test of anonymous share links: POST raw (encrypted) bytes to
//! `/api/v2/post/`, get an `{ "id": ... }` back, and GET the identical bytes from
//! `/api/v2/{id}` — the contract upstream Excalidraw's export-to-link flow relies on.

use oxydraw_core::config::Config;

mod common;
use common::{spawn_app as spawn_with_config, test_config};

async fn spawn() -> std::net::SocketAddr {
    spawn_with_config(test_config()).await
}

#[tokio::test]
async fn share_link_round_trips_raw_bytes() {
    let addr = spawn().await;
    let base = format!("http://{addr}");
    let client = reqwest::Client::new();

    // The client uploads an opaque binary blob (encrypted + compressed scene).
    let payload: Vec<u8> = (0..=255u8).collect();
    let r = client
        .post(format!("{base}/api/v2/post/"))
        .body(payload.clone())
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let v: serde_json::Value = serde_json::from_str(&r.text().await.unwrap()).unwrap();
    let id = v["id"].as_str().expect("response carries an id");

    // Reading it back returns the identical bytes.
    let r = client
        .get(format!("{base}/api/v2/{id}"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    assert_eq!(r.bytes().await.unwrap().as_ref(), payload.as_slice());

    // Unknown ids are 404.
    let r = client
        .get(format!("{base}/api/v2/does-not-exist"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 404);
}

/// SEC-19: anonymous capability-URL access is the contract — a stored document must be
/// readable with no session or credentials even when the deployment has auth configured.
/// Share links would break if `get_document` ever grew an auth requirement.
#[tokio::test]
async fn share_link_is_readable_without_any_session_or_auth() {
    let addr = spawn_with_config(Config {
        ext_password: Some("hunter2-hunter2".into()),
        ..test_config()
    })
    .await;
    let base = format!("http://{addr}");
    let client = reqwest::Client::new();

    let r = client
        .post(format!("{base}/api/v2/post/"))
        .body(b"opaque encrypted scene".to_vec())
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let v: serde_json::Value = serde_json::from_str(&r.text().await.unwrap()).unwrap();
    let id = v["id"].as_str().expect("response carries an id");

    // No cookies, no Authorization header — the UUID alone grants read access.
    let r = client
        .get(format!("{base}/api/v2/{id}"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    assert_eq!(r.bytes().await.unwrap().as_ref(), b"opaque encrypted scene");
}

#[tokio::test]
async fn share_link_rejects_uploads_past_the_document_quota() {
    let addr = spawn_with_config(Config {
        max_documents_bytes: 10,
        ..test_config()
    })
    .await;
    let base = format!("http://{addr}");
    let client = reqwest::Client::new();

    // First share fits under the 10-byte quota.
    let r = client
        .post(format!("{base}/api/v2/post/"))
        .body(vec![0u8; 8])
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);

    // The next one would exceed it and is rejected.
    let r = client
        .post(format!("{base}/api/v2/post/"))
        .body(vec![0u8; 8])
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 507);
}
