//! `/api/ext/*` routes: the scene library and its login —
//! password fallback plus OAuth/OIDC sign-in (Google, GitHub).
//!
//! Split by concern: [`auth`] owns login (password + OAuth) and the session gate;
//! [`scenes`] owns the scene-library CRUD and `/me`. This root keeps only what both
//! share — the default-org bootstrap and the [`CurrentUser`] principal — plus the thin
//! [`router`] assembly.

mod auth;
mod folders;
mod scenes;
#[cfg(test)]
mod test_support;

use axum::http::StatusCode;
use axum::middleware;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use oxydraw_core::model::{Folder, Org, Timestamp};
use oxydraw_core::store::{FolderId, Permission, Store, StoreError};
use tracing::error;

use crate::routes::{internal_error, log_err};
use crate::AppState;

use auth::{auth_callback, auth_providers, auth_start, login, logout, require_session};
use folders::{create_folder, delete_folder, list_folders, update_folder};
use scenes::{create_scene, delete_scene, list_scenes, me, update_scene};

pub(crate) use auth::LoginThrottle;

/// The single organization every user joins today. Matches the legacy scene `owner`
/// value, so pre-auth scene rows need no migration.
pub const DEFAULT_ORG: &str = "default";

/// Display name paired with [`DEFAULT_ORG`].
pub(crate) const DEFAULT_ORG_NAME: &str = "Default";

/// The authenticated principal, resolved by [`auth::require_session`] and read by the
/// scene-library handlers in [`scenes`].
#[derive(Clone)]
pub(crate) struct CurrentUser {
    /// `None` in open mode (no password and no OAuth provider configured).
    user_id: Option<String>,
    org_id: String,
    org_name: String,
}

/// Idempotently create the default org ([`DEFAULT_ORG`]); existing rows are untouched.
/// The single bootstrap shared by server startup and every login.
pub(crate) async fn ensure_default_org(store: &dyn Store) -> Result<(), StoreError> {
    store
        .ensure_org(Org {
            id: DEFAULT_ORG.to_string(),
            name: DEFAULT_ORG_NAME.to_string(),
            created_at: Timestamp::now(),
        })
        .await
}

/// Authorization checkpoint for a folder-scoped operation, called by every folder and
/// scene handler that touches a specific folder. Part 1 enforces only the org (tenant)
/// boundary: the folder must exist and belong to the caller's org. A missing *or*
/// cross-org folder is reported identically as 404 — never 403 — so the API does not
/// disclose the existence of another org's folders.
///
/// `required` documents the [`Permission`] the operation needs. Part 1 ignores it (all
/// org members share the library); part 2 will make the body call
/// [`effective_permission`](oxydraw_core::store::FolderStore::effective_permission) and
/// compare against `required`, **without changing any call site** — the single seam that
/// keeps per-folder permissions an additive change.
///
/// ## Access-helper return convention (shared with [`scenes::require_scene_access`])
///
/// Both access helpers return `Result<Resource, Response>`: the resolved row
/// ([`Folder`] / [`Scene`](oxydraw_core::model::Scene)) on success, or the error
/// [`Response`] the handler returns verbatim — a missing *or* cross-org row maps to 404
/// (no existence disclosure), a backend failure to a logged 500. Same shape on both, so a
/// handler never deref-boxes one path its sibling returns plainly.
pub(crate) async fn require_folder_access(
    store: &dyn Store,
    current: &CurrentUser,
    folder_id: &str,
    required: Permission,
) -> Result<Folder, Response> {
    // Part-1: org membership is the only grant; `required` is enforced in part 2.
    let _ = required;
    match store.find_folder(FolderId(folder_id)).await {
        Ok(folder) if folder.org_id == current.org_id => Ok(folder),
        Ok(_) | Err(StoreError::NotFound) => {
            Err((StatusCode::NOT_FOUND, "folder not found").into_response())
        }
        Err(e) => {
            error!(
                error = log_err(&e),
                "failed to resolve folder for access check"
            );
            Err(internal_error())
        }
    }
}

pub fn router(state: AppState) -> Router<AppState> {
    let guarded = Router::new()
        .route("/api/ext/scenes", get(list_scenes).post(create_scene))
        .route(
            "/api/ext/scenes/{id}",
            axum::routing::patch(update_scene).delete(delete_scene),
        )
        .route("/api/ext/folders", get(list_folders).post(create_folder))
        .route(
            "/api/ext/folders/{id}",
            axum::routing::patch(update_folder).delete(delete_folder),
        )
        .route("/api/ext/me", get(me))
        .route_layer(middleware::from_fn_with_state(state, require_session));
    Router::new()
        .merge(guarded)
        .route("/api/ext/login", post(login))
        .route("/api/ext/logout", post(logout))
        .route("/api/ext/auth/providers", get(auth_providers))
        .route("/api/ext/auth/callback/{provider}", get(auth_callback))
        .route("/api/ext/auth/{provider}", get(auth_start))
}
