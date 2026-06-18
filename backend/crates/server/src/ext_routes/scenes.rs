//! The scene-library REST resource (`/api/ext/scenes`) and the
//! `/api/ext/me` principal view. The session gate that populates [`CurrentUser`] lives
//! in [`super::auth`]; this module only reads it.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use oxydraw_core::model::{Scene, Timestamp};
use oxydraw_core::store::{FolderId, OrgId, Permission, Store, StoreError};
use serde::{Deserialize, Serialize};
use tracing::{error, warn};

use crate::routes::{internal_error, log_err};
use crate::AppState;

use super::folders::{double_option, validated_name};
use super::{require_folder_access, CurrentUser};

#[derive(Serialize)]
struct MeView {
    user: MeUser,
    org: MeOrg,
}

#[derive(Serialize)]
struct MeUser {
    name: Option<String>,
    email: Option<String>,
    avatar_url: Option<String>,
}

#[derive(Serialize)]
struct MeOrg {
    id: String,
    name: String,
}

/// Who am I? 401 (from the middleware) when not signed in; in open mode reports the
/// anonymous default-org principal so the client can skip its login view.
pub(crate) async fn me(
    State(state): State<AppState>,
    Extension(current): Extension<CurrentUser>,
) -> Response {
    let user = match &current.user_id {
        Some(user_id) => match state.store.find_user(user_id).await {
            Ok(user) => MeUser {
                name: user.name,
                email: user.email,
                avatar_url: user.avatar_url,
            },
            Err(e) => {
                error!(error = log_err(&e), "failed to load current user");
                return internal_error();
            }
        },
        None => MeUser {
            name: None,
            email: None,
            avatar_url: None,
        },
    };
    // The session middleware already resolved the org (id + name); no second lookup.
    let org = MeOrg {
        id: current.org_id,
        name: current.org_name,
    };
    Json(MeView { user, org }).into_response()
}

/// What the client sees for each saved scene. Includes the AES key on purpose: the
/// share-link `#json=<document_id>,<key>` open path needs it client-side.
#[derive(Serialize)]
struct SceneView {
    id: String,
    name: String,
    document_id: String,
    key: String,
    updated_at: String,
}

impl From<Scene> for SceneView {
    fn from(s: Scene) -> Self {
        Self {
            id: s.id,
            name: s.name,
            document_id: s.document_id,
            key: s.key,
            updated_at: s.updated_at.into(),
        }
    }
}

#[derive(Deserialize)]
pub(crate) struct CreateScene {
    name: String,
    /// The `/api/v2/post/` id holding the encrypted blob.
    document_id: String,
    /// Client-generated scene AES key (JWK `.k`, base64url).
    key: String,
    /// Folder to save into; omitted (or `null`) saves at the root.
    #[serde(default)]
    folder_id: Option<String>,
}

/// Bounds on the scene library (SEC-33), mirroring the quota model every other
/// persistent write path already has (`max_documents_bytes`, `max_files_bytes`, the
/// emulators' `BoundedMap`): generous for legitimate use — names are human-typed,
/// `document_id` is a UUID, `key` is a base64url AES-128 JWK — while keeping the one
/// otherwise-unbounded table from growing without limit.
const MAX_SCENE_NAME_BYTES: usize = 256;
const MAX_SCENE_FIELD_BYTES: usize = 512;
const MAX_SCENES_PER_OWNER: u64 = 4096;

pub(crate) async fn create_scene(
    State(state): State<AppState>,
    Extension(current): Extension<CurrentUser>,
    Json(req): Json<CreateScene>,
) -> Response {
    // The name shares the folder name rule (trim/empty/length); `document_id`/`key` are
    // opaque tokens, only byte-bounded.
    let name = match validated_name(&req.name, MAX_SCENE_NAME_BYTES, "scene name") {
        Ok(name) => name,
        Err(response) => return *response,
    };
    if req.document_id.len() > MAX_SCENE_FIELD_BYTES || req.key.len() > MAX_SCENE_FIELD_BYTES {
        warn!(
            owner = %current.org_id,
            document_id_len = req.document_id.len(),
            key_len = req.key.len(),
            "scene create rejected: field length bound exceeded"
        );
        return (StatusCode::BAD_REQUEST, "scene field too large").into_response();
    }
    // Saving into a folder requires edit access to it; omitted saves at the root.
    if let Some(folder) = &req.folder_id {
        if let Err(response) =
            require_folder_access(state.store.as_ref(), &current, folder, Permission::Editor).await
        {
            return response;
        }
    }
    // Check-then-insert is racy, but the overshoot is bounded by one row per in-flight
    // request — same accepted trade-off as `persist_durable_file`'s byte quota.
    match state.store.count_scenes(&current.org_id).await {
        Ok(count) if count >= MAX_SCENES_PER_OWNER => {
            warn!(owner = %current.org_id, count, "scene create rejected: scene quota exhausted");
            return (StatusCode::INSUFFICIENT_STORAGE, "scene quota exhausted").into_response();
        }
        Ok(_) => {}
        Err(e) => {
            error!(error = log_err(&e), "failed to count scenes for quota");
            return internal_error();
        }
    }
    let now = Timestamp::now();
    let scene = Scene {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        document_id: req.document_id,
        key: req.key,
        owner: current.org_id,
        // `folder_id` places the scene in the tree (`None` = the org root); `owner_user_id`
        // records the creator for the future per-user/permission features (`None` in open
        // mode), and is not yet used as a filter.
        folder_id: req.folder_id,
        owner_user_id: current.user_id.clone(),
        created_at: now.clone(),
        updated_at: now,
    };
    let view = SceneView::from(scene.clone());
    match state.store.create_scene(scene).await {
        Ok(()) => (StatusCode::CREATED, Json(view)).into_response(),
        Err(e) => {
            error!(error = log_err(&e), "failed to create scene");
            internal_error()
        }
    }
}

#[derive(Deserialize)]
pub(crate) struct ListQuery {
    /// List scenes directly inside this folder; omitted lists the root ("Unfiled") scenes.
    folder: Option<String>,
}

pub(crate) async fn list_scenes(
    State(state): State<AppState>,
    Extension(current): Extension<CurrentUser>,
    Query(query): Query<ListQuery>,
) -> Response {
    // Listing a folder's scenes requires read access to it; the root needs no folder.
    if let Some(folder) = &query.folder {
        if let Err(response) =
            require_folder_access(state.store.as_ref(), &current, folder, Permission::Viewer).await
        {
            return response;
        }
    }
    let folder = query.folder.as_deref().map(FolderId);
    match state
        .store
        .list_scenes_in_folder(&current.org_id, folder)
        .await
    {
        Ok(scenes) => {
            Json(scenes.into_iter().map(SceneView::from).collect::<Vec<_>>()).into_response()
        }
        Err(e) => {
            error!(error = log_err(&e), "failed to list scenes");
            internal_error()
        }
    }
}

#[derive(Deserialize)]
pub(crate) struct UpdateScene {
    #[serde(default)]
    name: Option<String>,
    /// Present to move the scene: `null` moves it to the root, a value into that folder.
    /// Absent leaves the folder unchanged (hence the double `Option`).
    #[serde(default, deserialize_with = "double_option")]
    folder_id: Option<Option<String>>,
}

/// `PATCH /api/ext/scenes/{id}` — rename and/or move a scene. Reads as validate → authorize
/// everything → mutate → reload: every authorization check runs before any write, so a
/// rejected PATCH (e.g. a move into a forbidden folder) leaves no field mutated (SEC-31).
pub(crate) async fn update_scene(
    State(state): State<AppState>,
    Extension(current): Extension<CurrentUser>,
    Path(id): Path<String>,
    Json(req): Json<UpdateScene>,
) -> Response {
    // 1. Validate inputs — same trim/empty/length rule as create and as folder names.
    let name = match req
        .name
        .as_deref()
        .map(|n| validated_name(n, MAX_SCENE_NAME_BYTES, "scene name"))
        .transpose()
    {
        Ok(name) => name,
        Err(response) => return *response,
    };
    // 2. Authorize the scene and (for a non-root move) the destination — before any mutation.
    if let Err(response) = require_scene_access(state.store.as_ref(), &current, &id).await {
        return response;
    }
    if let Some(Some(folder)) = &req.folder_id {
        if let Err(response) =
            require_folder_access(state.store.as_ref(), &current, folder, Permission::Editor).await
        {
            return response;
        }
    }
    // 3. Mutate — each operation is now fully pre-authorized.
    if let Some(name) = &name {
        if let Err(response) = apply_scene_rename(&state, &id, name).await {
            return response;
        }
    }
    if let Some(target) = &req.folder_id {
        if let Err(response) =
            apply_scene_move(&state, &id, &current.org_id, target.as_deref()).await
        {
            return response;
        }
    }
    // 4. Reload the updated row.
    match state.store.find_scene(&id).await {
        Ok(scene) => Json(SceneView::from(scene)).into_response(),
        Err(e) => {
            error!(error = log_err(&e), "failed to reload scene after update");
            internal_error()
        }
    }
}

/// Persist a pre-authorized scene rename. The caller must have verified scene access and
/// that `name` is within bounds; this only writes (returns the 500 response on failure).
async fn apply_scene_rename(state: &AppState, id: &str, name: &str) -> Result<(), Response> {
    state
        .store
        .rename_scene(id, name, Timestamp::now())
        .await
        .map_err(|e| {
            error!(error = log_err(&e), "failed to rename scene");
            internal_error()
        })
}

/// Persist a pre-authorized scene move. The caller must have verified scene access and (for
/// a non-root `target`) edit access to the destination folder; `owner` is the caller's org,
/// which the store re-checks for tenant isolation (SEC-20).
async fn apply_scene_move(
    state: &AppState,
    id: &str,
    owner: &str,
    target: Option<&str>,
) -> Result<(), Response> {
    state
        .store
        .move_scene(id, OrgId(owner), target.map(FolderId), Timestamp::now())
        .await
        .map_err(|e| {
            error!(error = log_err(&e), "failed to move scene");
            internal_error()
        })
}

/// `DELETE /api/ext/scenes/{id}` — remove a scene's library entry (the encrypted blob is
/// content-addressed and shared with share-links, so it is left untouched).
pub(crate) async fn delete_scene(
    State(state): State<AppState>,
    Extension(current): Extension<CurrentUser>,
    Path(id): Path<String>,
) -> Response {
    if let Err(response) = require_scene_access(state.store.as_ref(), &current, &id).await {
        return response;
    }
    match state.store.delete_scene(&id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(StoreError::NotFound) => (StatusCode::NOT_FOUND, "scene not found").into_response(),
        Err(e) => {
            error!(error = log_err(&e), "failed to delete scene");
            internal_error()
        }
    }
}

/// Resolve a scene and enforce the org (tenant) boundary: a missing scene, or one owned by
/// another org, is reported identically as 404 (no existence disclosure) — the scene-level
/// mirror of [`require_folder_access`], sharing its return convention (resolved [`Scene`] on
/// success, error [`Response`] to return otherwise; documented there). Scenes have no
/// per-scene permission seam, so there is no `required` parameter.
pub(crate) async fn require_scene_access(
    store: &dyn Store,
    current: &CurrentUser,
    id: &str,
) -> Result<Scene, Response> {
    match store.find_scene(id).await {
        Ok(scene) if scene.owner == current.org_id => Ok(scene),
        Ok(_) | Err(StoreError::NotFound) => {
            Err((StatusCode::NOT_FOUND, "scene not found").into_response())
        }
        Err(e) => {
            error!(
                error = log_err(&e),
                "failed to resolve scene for access check"
            );
            Err(internal_error())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ext_routes::test_support::{current_user, FaultyStore};
    use oxydraw_core::store::SceneStore;
    use oxydraw_storage::MemoryStore;

    /// Seed a scene owned by `org` and return it.
    fn scene_in(org: &str) -> Scene {
        let now = Timestamp::now();
        Scene {
            id: "s1".to_string(),
            name: "diagram".to_string(),
            document_id: "doc-1".to_string(),
            key: "k1".to_string(),
            owner: org.to_string(),
            folder_id: None,
            owner_user_id: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn require_scene_access_returns_the_scene_for_a_same_org_id() {
        let store = MemoryStore::new();
        store.create_scene(scene_in("myorg")).await.unwrap();
        let current = current_user("myorg");
        let scene = require_scene_access(&store, &current, "s1")
            .await
            .expect("same-org scene is accessible");
        assert_eq!(scene.id, "s1");
        assert_eq!(scene.owner, "myorg");
    }

    #[tokio::test]
    async fn require_scene_access_404_for_a_missing_id() {
        let store = MemoryStore::new();
        let current = current_user("myorg");
        let err = require_scene_access(&store, &current, "missing")
            .await
            .expect_err("a missing scene is rejected");
        assert_eq!(err.status(), StatusCode::NOT_FOUND);
    }

    /// The non-disclosure invariant: another org's scene is indistinguishable from a missing
    /// one (404, never 403).
    #[tokio::test]
    async fn require_scene_access_404_for_an_other_org_id() {
        let store = MemoryStore::new();
        store.create_scene(scene_in("otherorg")).await.unwrap();
        let current = current_user("myorg");
        let err = require_scene_access(&store, &current, "s1")
            .await
            .expect_err("a cross-org scene is rejected");
        assert_eq!(err.status(), StatusCode::NOT_FOUND);
    }

    /// A backend failure must surface as 500 — never 401/403, which would leak the check as
    /// an authorization decision.
    #[tokio::test]
    async fn require_scene_access_500_on_a_backend_error() {
        let current = current_user("myorg");
        let err = require_scene_access(&FaultyStore, &current, "s1")
            .await
            .expect_err("a backend error is rejected");
        assert_eq!(err.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
