//! The `/api/ext/folders` tree API and the scene move/rename/delete surface (M1). Runs in
//! open mode (no auth), so the principal is the anonymous default-org user — exactly the
//! deployment where the org (tenant) boundary is the only access check.

use std::sync::Arc;

use oxydraw_core::model::{Folder, Timestamp};
use oxydraw_core::store::{FolderStore, SceneStore};
use oxydraw_storage::MemoryStore;
use serde_json::{json, Value};

mod common;
use common::{spawn_app_with_store, test_config};

async fn spawn() -> (String, Arc<MemoryStore>, reqwest::Client) {
    let store = Arc::new(MemoryStore::new());
    let addr = spawn_app_with_store(test_config(), store.clone()).await;
    (format!("http://{addr}"), store, reqwest::Client::new())
}

/// Create a folder and return its id.
async fn make_folder(base: &str, client: &reqwest::Client, body: Value) -> String {
    let r = client
        .post(format!("{base}/api/ext/folders"))
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 201, "folder create should succeed");
    let v: Value = r.json().await.unwrap();
    v["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn folder_tree_create_nest_list_and_breadcrumb() {
    let (base, _store, client) = spawn().await;

    // The root view starts empty, with no breadcrumb.
    let r = client
        .get(format!("{base}/api/ext/folders"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let v: Value = r.json().await.unwrap();
    assert_eq!(v["folders"].as_array().unwrap().len(), 0);
    assert_eq!(v["breadcrumb"].as_array().unwrap().len(), 0);

    // Create a top-level folder and a nested child.
    let parent = make_folder(&base, &client, json!({ "name": "Projects" })).await;
    let child = make_folder(
        &base,
        &client,
        json!({ "name": "2026", "parent_id": parent }),
    )
    .await;

    // The root listing now shows the top-level folder (parent_id null).
    let v: Value = client
        .get(format!("{base}/api/ext/folders"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let folders = v["folders"].as_array().unwrap();
    assert_eq!(folders.len(), 1);
    assert_eq!(folders[0]["name"], "Projects");
    assert!(folders[0]["parent_id"].is_null());

    // Listing the parent shows the child and a breadcrumb ending at the parent.
    let v: Value = client
        .get(format!("{base}/api/ext/folders?parent={parent}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(v["folders"].as_array().unwrap().len(), 1);
    assert_eq!(v["folders"][0]["id"], child);
    let crumbs = v["breadcrumb"].as_array().unwrap();
    assert_eq!(crumbs.len(), 1);
    assert_eq!(crumbs[0]["id"], parent);

    // The child's view has a two-level breadcrumb, root-first.
    let v: Value = client
        .get(format!("{base}/api/ext/folders?parent={child}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let crumbs = v["breadcrumb"].as_array().unwrap();
    assert_eq!(
        crumbs
            .iter()
            .map(|c| c["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [parent.as_str(), child.as_str()]
    );
}

#[tokio::test]
async fn renaming_a_folder_persists_the_new_name() {
    let (base, _store, client) = spawn().await;
    let folder = make_folder(&base, &client, json!({ "name": "Drafts" })).await;

    // The PATCH echoes the new name in its response body...
    let r = client
        .patch(format!("{base}/api/ext/folders/{folder}"))
        .json(&json!({ "name": "Final" }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    assert_eq!(r.json::<Value>().await.unwrap()["name"], "Final");

    // ...and a follow-up listing confirms the rename was persisted, not just echoed.
    let v: Value = client
        .get(format!("{base}/api/ext/folders"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let folders = v["folders"].as_array().unwrap();
    assert_eq!(folders.len(), 1);
    assert_eq!(folders[0]["id"], folder);
    assert_eq!(folders[0]["name"], "Final");
}

#[tokio::test]
async fn root_listing_returns_all_top_level_folders() {
    let (base, _store, client) = spawn().await;
    let a = make_folder(&base, &client, json!({ "name": "Alpha" })).await;
    let b = make_folder(&base, &client, json!({ "name": "Beta" })).await;
    let c = make_folder(&base, &client, json!({ "name": "Gamma" })).await;

    let v: Value = client
        .get(format!("{base}/api/ext/folders"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let mut ids = v["folders"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["id"].as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    ids.sort();
    let mut expected = vec![a, b, c];
    expected.sort();
    assert_eq!(
        ids, expected,
        "root listing should return every top-level folder"
    );
}

#[tokio::test]
async fn scenes_save_into_list_by_and_move_between_folders() {
    let (base, _store, client) = spawn().await;
    let folder = make_folder(&base, &client, json!({ "name": "Inbox" })).await;

    // Save a scene directly into the folder.
    let r = client
        .post(format!("{base}/api/ext/scenes"))
        .json(&json!({
            "name": "diagram",
            "document_id": "doc-1",
            "key": "k1",
            "folder_id": folder,
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 201);
    let scene_id = r.json::<Value>().await.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // It shows under the folder, not at the root.
    let in_folder: Value = client
        .get(format!("{base}/api/ext/scenes?folder={folder}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(in_folder.as_array().unwrap().len(), 1);
    assert_eq!(in_folder[0]["id"], scene_id);
    let at_root: Value = client
        .get(format!("{base}/api/ext/scenes"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(at_root.as_array().unwrap().len(), 0);

    // Rename it.
    let r = client
        .patch(format!("{base}/api/ext/scenes/{scene_id}"))
        .json(&json!({ "name": "renamed" }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    assert_eq!(r.json::<Value>().await.unwrap()["name"], "renamed");

    // Move it to the root (folder_id: null).
    let r = client
        .patch(format!("{base}/api/ext/scenes/{scene_id}"))
        .json(&json!({ "folder_id": null }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let at_root: Value = client
        .get(format!("{base}/api/ext/scenes"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(at_root.as_array().unwrap().len(), 1);
    assert_eq!(at_root[0]["id"], scene_id);

    // Delete it.
    let r = client
        .delete(format!("{base}/api/ext/scenes/{scene_id}"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 204);
    let at_root: Value = client
        .get(format!("{base}/api/ext/scenes"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(at_root.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn deleting_a_folder_cascades_its_scenes() {
    let (base, store, client) = spawn().await;
    let folder = make_folder(&base, &client, json!({ "name": "Temp" })).await;
    client
        .post(format!("{base}/api/ext/scenes"))
        .json(&json!({
            "name": "doomed",
            "document_id": "doc-1",
            "key": "k1",
            "folder_id": folder,
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(store.list_scenes("default").await.unwrap().len(), 1);

    let r = client
        .delete(format!("{base}/api/ext/folders/{folder}"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 204);
    // The folder's scene is gone with it.
    assert_eq!(store.list_scenes("default").await.unwrap().len(), 0);
}

#[tokio::test]
async fn moving_a_folder_into_its_own_descendant_is_rejected() {
    let (base, _store, client) = spawn().await;
    let a = make_folder(&base, &client, json!({ "name": "A" })).await;
    let b = make_folder(&base, &client, json!({ "name": "B", "parent_id": a })).await;

    // Reparenting A under its descendant B would create a cycle → 409.
    let r = client
        .patch(format!("{base}/api/ext/folders/{a}"))
        .json(&json!({ "parent_id": b }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 409);

    // A valid reparent (B to the root) succeeds.
    let r = client
        .patch(format!("{base}/api/ext/folders/{b}"))
        .json(&json!({ "parent_id": null }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    assert!(r.json::<Value>().await.unwrap()["parent_id"].is_null());
}

#[tokio::test]
async fn moving_a_folder_under_a_deep_descendant_is_rejected() {
    let (base, _store, client) = spawn().await;
    // A -> B -> C: reparenting the grandparent A under its depth-3 descendant C is a cycle.
    // A depth-2-only check that stopped one ancestor short would miss this and let it through.
    let a = make_folder(&base, &client, json!({ "name": "A" })).await;
    let b = make_folder(&base, &client, json!({ "name": "B", "parent_id": a })).await;
    let c = make_folder(&base, &client, json!({ "name": "C", "parent_id": b })).await;

    let r = client
        .patch(format!("{base}/api/ext/folders/{a}"))
        .json(&json!({ "parent_id": c }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 409);
}

#[tokio::test]
async fn moving_a_folder_under_itself_is_rejected() {
    let (base, _store, client) = spawn().await;
    let a = make_folder(&base, &client, json!({ "name": "A" })).await;

    // Setting a folder's parent to its own id is the degenerate cycle → 409.
    let r = client
        .patch(format!("{base}/api/ext/folders/{a}"))
        .json(&json!({ "parent_id": a }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 409);
}

#[tokio::test]
async fn unknown_folders_and_scenes_are_404_and_name_is_required() {
    let (base, _store, client) = spawn().await;

    for (method, path) in [
        ("GET", "/api/ext/folders?parent=nope"),
        ("DELETE", "/api/ext/folders/nope"),
        ("DELETE", "/api/ext/scenes/nope"),
    ] {
        let req = match method {
            "GET" => client.get(format!("{base}{path}")),
            "DELETE" => client.delete(format!("{base}{path}")),
            _ => unreachable!(),
        };
        let r = req.send().await.unwrap();
        assert_eq!(r.status(), 404, "{method} {path} should be 404");
    }

    // PATCH on a missing folder/scene is 404.
    let r = client
        .patch(format!("{base}/api/ext/folders/nope"))
        .json(&json!({ "name": "x" }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 404);

    // Saving a scene into a non-existent folder is 404.
    let r = client
        .post(format!("{base}/api/ext/scenes"))
        .json(&json!({
            "name": "s", "document_id": "d", "key": "k", "folder_id": "nope",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 404);

    // A blank folder name is rejected.
    let r = client
        .post(format!("{base}/api/ext/folders"))
        .json(&json!({ "name": "  " }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 400);
}

/// SEC-31: a PATCH that renames *and* moves into a folder the caller cannot reach is
/// rejected whole — the rename must not persist as a side effect of the failed move.
#[tokio::test]
async fn rejected_scene_move_does_not_persist_the_rename() {
    let (base, _store, client) = spawn().await;
    let folder = make_folder(&base, &client, json!({ "name": "Home" })).await;
    let r = client
        .post(format!("{base}/api/ext/scenes"))
        .json(&json!({
            "name": "original", "document_id": "d", "key": "k", "folder_id": folder,
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 201);
    let scene_id = r.json::<Value>().await.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Rename + move into a non-existent (unreachable) folder: the move is rejected 404.
    let r = client
        .patch(format!("{base}/api/ext/scenes/{scene_id}"))
        .json(&json!({ "name": "renamed", "folder_id": "nope" }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 404);

    // The scene is unchanged — still named "original", still in its folder.
    let in_folder: Value = client
        .get(format!("{base}/api/ext/scenes?folder={folder}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(in_folder.as_array().unwrap().len(), 1);
    assert_eq!(in_folder[0]["id"], scene_id);
    assert_eq!(
        in_folder[0]["name"], "original",
        "rename must not persist when the paired move is rejected"
    );
}

/// SEC-31 for folders: a PATCH renaming a folder while moving it under an unreachable parent
/// is rejected whole — the rename does not persist.
#[tokio::test]
async fn rejected_folder_move_does_not_persist_the_rename() {
    let (base, _store, client) = spawn().await;
    let folder = make_folder(&base, &client, json!({ "name": "original" })).await;

    let r = client
        .patch(format!("{base}/api/ext/folders/{folder}"))
        .json(&json!({ "name": "renamed", "parent_id": "nope" }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 404);

    // Listing the root shows the folder still named "original".
    let v: Value = client
        .get(format!("{base}/api/ext/folders"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let folders = v["folders"].as_array().unwrap();
    assert_eq!(folders.len(), 1);
    assert_eq!(
        folders[0]["name"], "original",
        "rename must not persist when the paired move is rejected"
    );
}

/// SEC-19: the breadcrumb walk re-asserts the org boundary on every ancestor, so a folder
/// whose `parent_id` points (via corruption or a future cross-org move bug) at another org's
/// folder never surfaces that folder's name/id in the trail.
#[tokio::test]
async fn breadcrumb_never_crosses_into_another_org() {
    let (base, store, client) = spawn().await;
    let now = Timestamp::parse("2026-07-01T00:00:00Z").unwrap();
    let seed = |id: &str, name: &str, parent: Option<&str>, org: &str| Folder {
        id: id.to_string(),
        name: name.to_string(),
        parent_id: parent.map(str::to_string),
        org_id: org.to_string(),
        owner_user_id: None,
        created_at: now.clone(),
        updated_at: now.clone(),
    };
    // A folder in another org, and a default-org leaf whose parent dangles into it.
    store
        .create_folder(seed("foreign", "Secret", None, "other-org"))
        .await
        .unwrap();
    store
        .create_folder(seed("leaf", "Leaf", Some("foreign"), "default"))
        .await
        .unwrap();

    let v: Value = client
        .get(format!("{base}/api/ext/folders?parent=leaf"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let crumbs = v["breadcrumb"].as_array().unwrap();
    // The trail stops at the leaf — the cross-org parent is never revealed.
    assert_eq!(
        crumbs
            .iter()
            .map(|c| c["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["leaf"]
    );
    assert!(
        crumbs.iter().all(|c| c["name"] != "Secret"),
        "breadcrumb leaked a folder from another org"
    );
}
