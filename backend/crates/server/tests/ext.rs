//! Scene-library surface: the `/api/ext` save/list/auth API.

use std::sync::Arc;

use oxydraw_core::model::{Scene, Timestamp};
use oxydraw_core::store::SceneStore;
use oxydraw_storage::MemoryStore;

mod common;
use common::{spawn_app_with_store, test_config};

async fn spawn() -> (std::net::SocketAddr, Arc<MemoryStore>) {
    let store = Arc::new(MemoryStore::new());
    let addr = spawn_app_with_store(test_config(), store.clone()).await;
    (addr, store)
}

#[tokio::test]
async fn save_flow_records_scene_metadata() {
    let (addr, _store) = spawn().await;
    let base = format!("http://{addr}");
    let client = reqwest::Client::new();

    // The client first uploads the opaque encrypted blob...
    let r = client
        .post(format!("{base}/api/v2/post/"))
        .body(vec![1u8, 2, 3])
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let v: serde_json::Value = serde_json::from_str(&r.text().await.unwrap()).unwrap();
    let document_id = v["id"].as_str().unwrap().to_string();

    // ...then records the library entry with the client-generated key.
    let r = client
        .post(format!("{base}/api/ext/scenes"))
        .header("content-type", "application/json")
        .body(
            serde_json::json!({
                "name": "round trip",
                "document_id": document_id,
                "key": "k123",
            })
            .to_string(),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 201);
    let created: serde_json::Value = serde_json::from_str(&r.text().await.unwrap()).unwrap();
    assert_eq!(created["name"], "round trip");
    assert_eq!(created["document_id"], document_id.as_str());
    assert_eq!(created["key"], "k123");
    assert!(created["id"].as_str().is_some_and(|s| !s.is_empty()));

    // The listing now carries everything needed to rebuild `#json=<id>,<key>`.
    let r = client
        .get(format!("{base}/api/ext/scenes"))
        .send()
        .await
        .unwrap();
    let scenes: serde_json::Value = serde_json::from_str(&r.text().await.unwrap()).unwrap();
    assert_eq!(scenes.as_array().unwrap().len(), 1);
    assert_eq!(scenes[0]["document_id"], document_id.as_str());
    assert_eq!(scenes[0]["key"], "k123");
}

#[tokio::test]
async fn lists_saved_scenes_for_the_default_owner() {
    let (addr, store) = spawn().await;
    let base = format!("http://{addr}");
    let client = reqwest::Client::new();

    let r = client
        .get(format!("{base}/api/ext/scenes"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let empty: serde_json::Value = serde_json::from_str(&r.text().await.unwrap()).unwrap();
    assert_eq!(empty, serde_json::json!([]));

    store
        .create_scene(Scene {
            id: "scene-1".to_string(),
            name: "my drawing".to_string(),
            document_id: "doc-1".to_string(),
            key: "abc123".to_string(),
            owner: "default".to_string(),
            folder_id: None,
            owner_user_id: None,
            created_at: Timestamp::parse("2026-06-01T00:00:00Z").unwrap(),
            updated_at: Timestamp::parse("2026-06-01T00:00:00Z").unwrap(),
        })
        .await
        .unwrap();
    // A foreign owner's scene must not leak into the default listing.
    store
        .create_scene(Scene {
            id: "scene-2".to_string(),
            name: "not mine".to_string(),
            document_id: "doc-2".to_string(),
            key: "zzz".to_string(),
            owner: "someone-else".to_string(),
            folder_id: None,
            owner_user_id: None,
            created_at: Timestamp::parse("2026-06-02T00:00:00Z").unwrap(),
            updated_at: Timestamp::parse("2026-06-02T00:00:00Z").unwrap(),
        })
        .await
        .unwrap();

    let r = client
        .get(format!("{base}/api/ext/scenes"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let scenes: serde_json::Value = serde_json::from_str(&r.text().await.unwrap()).unwrap();
    assert_eq!(
        scenes,
        serde_json::json!([{
            "id": "scene-1",
            "name": "my drawing",
            "document_id": "doc-1",
            "key": "abc123",
            // `Timestamp`'s canonical form: microsecond precision, `Z` suffix.
            "updated_at": "2026-06-01T00:00:00.000000Z",
        }])
    );
}

/// SEC-33: the scene library is quota-bounded like every other persistent write path —
/// oversized fields are rejected outright, and the per-owner row cap stops unbounded
/// table growth.
#[tokio::test]
async fn scene_creation_is_bounded() {
    let (addr, store) = spawn().await;
    let base = format!("http://{addr}");
    let client = reqwest::Client::new();

    // Oversized name (handler bound is 256 bytes) → 400, nothing stored.
    let r = client
        .post(format!("{base}/api/ext/scenes"))
        .header("content-type", "application/json")
        .body(
            serde_json::json!({
                "name": "x".repeat(300),
                "document_id": "doc-1",
                "key": "k123",
            })
            .to_string(),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 400);
    assert!(store.list_scenes("default").await.unwrap().is_empty());

    // Fill the per-owner cap (4096) directly in the store, then the next create is 507.
    for i in 0..4096 {
        store
            .create_scene(Scene {
                id: format!("scene-{i}"),
                name: "filler".to_string(),
                document_id: "doc".to_string(),
                key: "k".to_string(),
                owner: "default".to_string(),
                folder_id: None,
                owner_user_id: None,
                created_at: Timestamp::parse("2026-06-01T00:00:00Z").unwrap(),
                updated_at: Timestamp::parse("2026-06-01T00:00:00Z").unwrap(),
            })
            .await
            .unwrap();
    }
    let r = client
        .post(format!("{base}/api/ext/scenes"))
        .header("content-type", "application/json")
        .body(
            serde_json::json!({
                "name": "one too many",
                "document_id": "doc-x",
                "key": "k123",
            })
            .to_string(),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 507);
}
