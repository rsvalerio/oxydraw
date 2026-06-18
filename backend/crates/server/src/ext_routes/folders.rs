//! The scene-library folder tree REST resource (`/api/ext/folders`). Folders nest via
//! `parent_id` (a `None` parent is a top-level folder — the "root" view), scope to the
//! caller's org, and hold the scenes listed by [`super::scenes`]. Every handler routes
//! its folder lookups through [`super::require_folder_access`], the single authorization
//! seam that part 2 will extend from "same org" to a per-folder permission check.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use oxydraw_core::model::{Folder, Timestamp};
use oxydraw_core::store::{FolderId, FolderMoveError, OrgId, Permission};
use serde::{Deserialize, Serialize};
use tracing::error;

use crate::routes::{internal_error, log_err};
use crate::AppState;

use super::{require_folder_access, CurrentUser};

/// Folder-name byte cap, mirroring the scene-name bound (`MAX_SCENE_NAME_BYTES`): names
/// are human-typed, so this is generous for legitimate use while keeping the column
/// bounded.
const MAX_FOLDER_NAME_BYTES: usize = 256;

/// Per-org folder-count cap, the folder analog of `MAX_SCENES_PER_OWNER`. Bounds the one
/// otherwise-unbounded growth vector on the folder tree; check-then-insert is racy but the
/// overshoot is one row per in-flight request (the same trade-off as scene creation).
const MAX_FOLDERS_PER_OWNER: u64 = 1024;

/// What the client sees for a folder. `parent_id` is `null` for a top-level folder.
#[derive(Serialize)]
struct FolderView {
    id: String,
    name: String,
    parent_id: Option<String>,
    updated_at: String,
}

impl From<Folder> for FolderView {
    fn from(f: Folder) -> Self {
        Self {
            id: f.id,
            name: f.name,
            parent_id: f.parent_id,
            updated_at: f.updated_at.into(),
        }
    }
}

/// The contents of one folder level: its child folders plus the breadcrumb chain from the
/// root down to (and including) the folder being viewed. The breadcrumb is empty at the
/// root view.
#[derive(Serialize)]
struct FolderListView {
    folders: Vec<FolderView>,
    breadcrumb: Vec<FolderView>,
}

#[derive(Deserialize)]
pub(crate) struct ListQuery {
    /// The folder whose children to list; omitted lists the top-level (root) folders.
    parent: Option<String>,
}

/// `GET /api/ext/folders[?parent=<id>]` — child folders of `parent` (or the root level),
/// plus the breadcrumb to `parent`.
pub(crate) async fn list_folders(
    State(state): State<AppState>,
    Extension(current): Extension<CurrentUser>,
    Query(query): Query<ListQuery>,
) -> Response {
    let breadcrumb = match &query.parent {
        Some(parent) => {
            // Viewing a folder's children requires read access to that folder.
            let folder = match require_folder_access(
                state.store.as_ref(),
                &current,
                parent,
                Permission::Viewer,
            )
            .await
            {
                Ok(folder) => folder,
                Err(response) => return response,
            };
            match breadcrumb_to(&state, folder).await {
                Ok(crumbs) => crumbs,
                Err(response) => return *response,
            }
        }
        None => Vec::new(),
    };
    let parent = query.parent.as_deref().map(FolderId);
    match state
        .store
        .list_folders(OrgId(&current.org_id), parent)
        .await
    {
        Ok(folders) => Json(FolderListView {
            folders: folders.into_iter().map(FolderView::from).collect(),
            breadcrumb,
        })
        .into_response(),
        Err(e) => {
            error!(error = log_err(&e), "failed to list folders");
            internal_error()
        }
    }
}

/// Walk `folder`'s ancestor chain to the root, returning the breadcrumb root-first
/// (including `folder` itself). Bounded by the tree depth the store enforces. The error is
/// boxed (oversized `Response` against the small `Ok`).
async fn breadcrumb_to(state: &AppState, folder: Folder) -> Result<Vec<FolderView>, Box<Response>> {
    let org_id = folder.org_id.clone();
    let mut chain = vec![FolderView::from(folder.clone())];
    let mut parent_id = folder.parent_id;
    while let Some(id) = parent_id {
        match state.store.find_folder(FolderId(&id)).await {
            // Re-assert the org (tenant) boundary on every ancestor (SEC-19): only the leaf
            // was org-checked by the caller, so a `parent_id` that crosses into another org
            // (data corruption or a future move bug) must break the trail, never surface a
            // foreign folder's name/id — same treatment as the `NotFound` arm.
            Ok(parent) if parent.org_id == org_id => {
                parent_id = parent.parent_id.clone();
                chain.push(FolderView::from(parent));
            }
            Ok(_) => break,
            // A dangling parent ref (or a row removed mid-walk) just truncates the trail
            // rather than failing the whole listing.
            Err(oxydraw_core::store::StoreError::NotFound) => break,
            Err(e) => {
                error!(error = log_err(&e), "failed to build folder breadcrumb");
                return Err(Box::new(internal_error()));
            }
        }
    }
    chain.reverse();
    Ok(chain)
}

#[derive(Deserialize)]
pub(crate) struct CreateFolder {
    name: String,
    /// Parent folder; omitted (or `null`) creates a top-level folder.
    #[serde(default)]
    parent_id: Option<String>,
}

/// `POST /api/ext/folders` — create a folder under `parent_id` (or at the root).
pub(crate) async fn create_folder(
    State(state): State<AppState>,
    Extension(current): Extension<CurrentUser>,
    Json(req): Json<CreateFolder>,
) -> Response {
    let name = match validated_name(&req.name, MAX_FOLDER_NAME_BYTES, "folder name") {
        Ok(name) => name,
        Err(response) => return *response,
    };
    if let Some(parent) = &req.parent_id {
        // Adding a child requires edit access to the parent.
        if let Err(response) =
            require_folder_access(state.store.as_ref(), &current, parent, Permission::Editor).await
        {
            return response;
        }
    }
    match state.store.count_folders(OrgId(&current.org_id)).await {
        Ok(count) if count >= MAX_FOLDERS_PER_OWNER => {
            return (StatusCode::INSUFFICIENT_STORAGE, "folder quota exhausted").into_response();
        }
        Ok(_) => {}
        Err(e) => {
            error!(error = log_err(&e), "failed to count folders for quota");
            return internal_error();
        }
    }
    let now = Timestamp::now();
    let folder = Folder {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        parent_id: req.parent_id,
        org_id: current.org_id.clone(),
        owner_user_id: current.user_id.clone(),
        created_at: now.clone(),
        updated_at: now,
    };
    let view = FolderView::from(folder.clone());
    match state.store.create_folder(folder).await {
        Ok(()) => (StatusCode::CREATED, Json(view)).into_response(),
        Err(e) => folder_move_error(e),
    }
}

#[derive(Deserialize)]
pub(crate) struct UpdateFolder {
    #[serde(default)]
    name: Option<String>,
    /// Present to move the folder: `null` moves it to the root, a value reparents it.
    /// Absent leaves the parent unchanged (hence the double `Option`).
    #[serde(default, deserialize_with = "double_option")]
    parent_id: Option<Option<String>>,
}

/// `PATCH /api/ext/folders/{id}` — rename and/or move a folder. Reads as validate →
/// authorize everything → mutate → reload: every authorization check runs before any write,
/// so a rejected PATCH (e.g. a move into a forbidden destination) leaves no field mutated
/// (SEC-31).
pub(crate) async fn update_folder(
    State(state): State<AppState>,
    Extension(current): Extension<CurrentUser>,
    Path(id): Path<String>,
    Json(req): Json<UpdateFolder>,
) -> Response {
    // 1. Validate inputs.
    let name = match req
        .name
        .as_deref()
        .map(|n| validated_name(n, MAX_FOLDER_NAME_BYTES, "folder name"))
        .transpose()
    {
        Ok(name) => name,
        Err(response) => return *response,
    };
    // 2. Authorize the folder (admin) and (for a non-root move) the destination (editor) —
    //    before any mutation.
    if let Err(response) =
        require_folder_access(state.store.as_ref(), &current, &id, Permission::Admin).await
    {
        return response;
    }
    if let Some(Some(parent)) = &req.parent_id {
        if let Err(response) =
            require_folder_access(state.store.as_ref(), &current, parent, Permission::Editor).await
        {
            return response;
        }
    }
    // 3. Mutate — each operation is now fully pre-authorized.
    if let Some(name) = &name {
        if let Err(response) = apply_folder_rename(&state, &id, name).await {
            return response;
        }
    }
    if let Some(target) = &req.parent_id {
        if let Err(response) =
            apply_folder_move(&state, &id, &current.org_id, target.as_deref()).await
        {
            return response;
        }
    }
    // 4. Reload the updated row.
    match state.store.find_folder(FolderId(&id)).await {
        Ok(folder) => Json(FolderView::from(folder)).into_response(),
        Err(e) => {
            error!(error = log_err(&e), "failed to reload folder after update");
            internal_error()
        }
    }
}

/// Persist a pre-authorized folder rename. The caller must hold admin on the folder and have
/// validated `name`; this only writes (returns the 500 response on failure).
async fn apply_folder_rename(state: &AppState, id: &str, name: &str) -> Result<(), Response> {
    state
        .store
        .rename_folder(FolderId(id), name, Timestamp::now())
        .await
        .map_err(|e| {
            error!(error = log_err(&e), "failed to rename folder");
            internal_error()
        })
}

/// Persist a pre-authorized folder move. The caller must hold admin on the folder and (for a
/// non-root `target`) edit access to the destination; `org` is the caller's org, which the
/// store re-checks for tenant isolation (SEC-20). Maps a rejected move to its HTTP status.
async fn apply_folder_move(
    state: &AppState,
    id: &str,
    org: &str,
    target: Option<&str>,
) -> Result<(), Response> {
    state
        .store
        .move_folder(
            FolderId(id),
            OrgId(org),
            target.map(FolderId),
            Timestamp::now(),
        )
        .await
        .map_err(folder_move_error)
}

/// `DELETE /api/ext/folders/{id}` — delete the folder and its whole subtree (descendant
/// folders, their scenes, and their ACL grants; content-addressed blobs are untouched).
pub(crate) async fn delete_folder(
    State(state): State<AppState>,
    Extension(current): Extension<CurrentUser>,
    Path(id): Path<String>,
) -> Response {
    // Deleting the folder (and cascading its scenes) is an admin-level operation on it.
    if let Err(response) =
        require_folder_access(state.store.as_ref(), &current, &id, Permission::Admin).await
    {
        return response;
    }
    match state.store.delete_folder(FolderId(&id)).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(oxydraw_core::store::StoreError::NotFound) => {
            (StatusCode::NOT_FOUND, "folder not found").into_response()
        }
        Err(e) => {
            error!(error = log_err(&e), "failed to delete folder");
            internal_error()
        }
    }
}

/// Validate a user-supplied name: trimmed, non-empty, within `max` bytes. `label` (e.g.
/// `"folder name"`, `"scene name"`) names the field in the 400 message. Returns the trimmed
/// name, or the error response to return (boxed — `Response` is large and clippy flags an
/// oversized `Err` against the small `Ok`). Shared by the folder and scene name paths
/// ([`super::scenes`]) so the trim/empty/length rule cannot drift between them (DUP-3).
pub(crate) fn validated_name(raw: &str, max: usize, label: &str) -> Result<String, Box<Response>> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(Box::new(
            (StatusCode::BAD_REQUEST, format!("{label} is required")).into_response(),
        ));
    }
    if trimmed.len() > max {
        return Err(Box::new(
            (StatusCode::BAD_REQUEST, format!("{label} too long")).into_response(),
        ));
    }
    Ok(trimmed.to_string())
}

/// Map a [`FolderMoveError`] to its HTTP status: a cycle or over-deep nesting is a client
/// error (409 / 422), a missing folder is 404, and a backend failure is a logged 500.
fn folder_move_error(e: FolderMoveError) -> Response {
    match e {
        FolderMoveError::Cycle => (
            StatusCode::CONFLICT,
            "a folder cannot be moved into itself or one of its descendants",
        )
            .into_response(),
        FolderMoveError::TooDeep => {
            (StatusCode::UNPROCESSABLE_ENTITY, "folder nesting too deep").into_response()
        }
        FolderMoveError::NotFound => (StatusCode::NOT_FOUND, "folder not found").into_response(),
        FolderMoveError::Backend(e) => {
            error!(error = &*e as &dyn std::error::Error, "folder store error");
            internal_error()
        }
    }
}

/// Distinguish "field absent" from "field present and null" for a JSON `PATCH`: serde maps
/// an absent field to `None` (via `default`) and a present `null` to `Some(None)`. Shared
/// with [`super::scenes`]'s move handler.
pub(crate) fn double_option<'de, D, T>(de: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    serde::Deserialize::deserialize(de).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ext_routes::test_support::{current_user, FaultyStore};
    use oxydraw_core::store::FolderStore;
    use oxydraw_storage::MemoryStore;

    /// Seed a root folder owned by `org` and return it.
    fn folder_in(org: &str) -> Folder {
        let now = Timestamp::now();
        Folder {
            id: "f1".to_string(),
            name: "Inbox".to_string(),
            parent_id: None,
            org_id: org.to_string(),
            owner_user_id: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn require_folder_access_returns_the_folder_for_a_same_org_id() {
        let store = MemoryStore::new();
        store.create_folder(folder_in("myorg")).await.unwrap();
        let current = current_user("myorg");
        let folder = require_folder_access(&store, &current, "f1", Permission::Viewer)
            .await
            .expect("same-org folder is accessible");
        assert_eq!(folder.id, "f1");
        assert_eq!(folder.org_id, "myorg");
    }

    #[tokio::test]
    async fn require_folder_access_404_for_a_missing_id() {
        let store = MemoryStore::new();
        let current = current_user("myorg");
        let err = require_folder_access(&store, &current, "missing", Permission::Viewer)
            .await
            .expect_err("a missing folder is rejected");
        assert_eq!(err.status(), StatusCode::NOT_FOUND);
    }

    /// The non-disclosure invariant: another org's folder is indistinguishable from a
    /// missing one (404, never 403).
    #[tokio::test]
    async fn require_folder_access_404_for_an_other_org_id() {
        let store = MemoryStore::new();
        store.create_folder(folder_in("otherorg")).await.unwrap();
        let current = current_user("myorg");
        let err = require_folder_access(&store, &current, "f1", Permission::Viewer)
            .await
            .expect_err("a cross-org folder is rejected");
        assert_eq!(err.status(), StatusCode::NOT_FOUND);
    }

    /// A backend failure must surface as 500 — never 401/403, which would leak the check as
    /// an authorization decision.
    #[tokio::test]
    async fn require_folder_access_500_on_a_backend_error() {
        let current = current_user("myorg");
        let err = require_folder_access(&FaultyStore, &current, "f1", Permission::Viewer)
            .await
            .expect_err("a backend error is rejected");
        assert_eq!(err.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn folder_move_error_maps_each_variant_to_its_status() {
        assert_eq!(
            folder_move_error(FolderMoveError::Cycle).status(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            folder_move_error(FolderMoveError::TooDeep).status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            folder_move_error(FolderMoveError::NotFound).status(),
            StatusCode::NOT_FOUND
        );
        let backend = FolderMoveError::Backend(Box::new(std::io::Error::other("boom")));
        assert_eq!(
            folder_move_error(backend).status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }
}
