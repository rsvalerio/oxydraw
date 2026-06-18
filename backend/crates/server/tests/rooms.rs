//! End-to-end test of the collab scene-snapshot API: a room with no prior state is 404, a
//! `PUT` stores the opaque snapshot, a `GET` returns it verbatim, and a second `PUT`
//! overwrites (last save wins).

use oxydraw_core::config::Config;

mod common;
use common::spawn_app;

#[tokio::test]
async fn scene_snapshot_round_trips_and_overwrites() {
    let addr = spawn_app(Config::default()).await;
    let base = format!("http://{addr}");
    let client = reqwest::Client::new();
    let url = format!("{base}/api/rooms/room-1/scene");

    // Empty room: nothing stored yet.
    let r = client.get(&url).send().await.unwrap();
    assert_eq!(r.status(), 404);

    // Store an opaque (encrypted) snapshot.
    let r = client
        .put(&url)
        .body(&b"ciphertext-v1"[..])
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 204);

    let r = client.get(&url).send().await.unwrap();
    assert_eq!(r.status(), 200);
    assert_eq!(
        r.headers()["content-type"].to_str().unwrap(),
        "application/octet-stream"
    );
    assert_eq!(r.bytes().await.unwrap().as_ref(), b"ciphertext-v1");

    // A later save overwrites.
    let r = client
        .put(&url)
        .body(&b"ciphertext-v2"[..])
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 204);
    let r = client.get(&url).send().await.unwrap();
    assert_eq!(r.bytes().await.unwrap().as_ref(), b"ciphertext-v2");
}

/// SEC-33: an over-long room id is rejected before it can mint an unbounded map key.
#[tokio::test]
async fn oversized_room_ids_are_rejected() {
    let addr = spawn_app(Config::default()).await;
    let base = format!("http://{addr}");
    let client = reqwest::Client::new();

    let long = "x".repeat(300);
    let r = client
        .put(format!("{base}/api/rooms/{long}/scene"))
        .body(&b"x"[..])
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 400);
}
