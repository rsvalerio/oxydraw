//! Core API routes: anonymous document sharing ("export to link"), scene image files, and
//! per-room collab scene snapshots. (Handlers for files/scenes live in [`crate::files`] /
//! [`crate::rooms`]; this module owns the share endpoints and assembles the router.)

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use oxydraw_core::model::Document;
use oxydraw_core::store::StoreError;
use tracing::{error, warn};

use crate::AppState;

/// `/api/v2/*` routes.
pub fn api_router() -> Router<AppState> {
    Router::new()
        // Anonymous, shareable documents.
        .route("/api/v2/post/", post(create_document))
        .route("/api/v2/{id}", get(get_document))
        // Scene image files: client-addressed opaque blobs (collab + shared scenes).
        .route(
            "/api/files/{id}",
            put(crate::files::put_file).get(crate::files::get_file),
        )
        // Per-room collab scene snapshot (opaque, end-to-end-encrypted).
        .route(
            "/api/rooms/{room_id}/scene",
            put(crate::rooms::put_scene).get(crate::rooms::get_scene),
        )
}

// ---- anonymous documents ----------------------------------------------------------------

async fn create_document(State(state): State<AppState>, body: Bytes) -> Response {
    // Anonymous shares are quota-bound so a client loop cannot fill the disk. The
    // check-then-insert is racy, but the overshoot is bounded by one body per in-flight
    // request.
    let total = match state.store.documents_total_bytes().await {
        Ok(total) => total,
        Err(e) => {
            error!(error = log_err(&e), "failed to read document quota");
            return internal_error();
        }
    };
    // Lossless on 64-bit targets (usize == u64); on the impossible-overflow path saturate to
    // u64::MAX so the quota gate rejects rather than wrapping to a small value (SEC-15).
    let incoming = u64::try_from(body.len()).unwrap_or(u64::MAX);
    if let Err(response) = enforce_byte_quota(
        total,
        incoming,
        state.config.max_documents_bytes,
        "document",
    ) {
        return *response;
    }

    match state
        .store
        .create(Document {
            data: body.to_vec(),
        })
        .await
    {
        Ok(id) => Json(serde_json::json!({ "id": id })).into_response(),
        Err(e) => {
            error!(error = log_err(&e), "failed to create document");
            internal_error()
        }
    }
}

/// Anonymous access is intentional (SEC-19): documents are capability URLs — anyone who
/// knows the unguessable UUID may read the (end-to-end-encrypted) payload, matching
/// Excalidraw's share-link semantics. Do not add per-user auth here; it would break
/// share links. The no-auth contract is pinned by an integration test.
async fn get_document(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    match state.store.find_id(&id).await {
        Ok(doc) => ([(header::CONTENT_TYPE, "application/json")], doc.data).into_response(),
        Err(StoreError::NotFound) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => {
            error!(error = log_err(&e), "failed to read document");
            internal_error()
        }
    }
}

// ---- helpers ----------------------------------------------------------------------------

/// The canonical 500 response, shared by every handler module.
pub(crate) fn internal_error() -> Response {
    (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response()
}

/// Shared byte-quota gate for the anonymous write paths (DUP-2). Given the resource's
/// current stored `total` (already read by the caller — the read's error handling differs
/// per store method), reject with `507 Insufficient Storage` when adding `incoming` bytes
/// would exceed `limit`. `what` names the resource for the log line. Check-then-insert is
/// racy, but the overshoot is bounded by one body per in-flight request.
pub(crate) fn enforce_byte_quota(
    total: u64,
    incoming: u64,
    limit: u64,
    what: &str,
) -> Result<(), Box<Response>> {
    if total.saturating_add(incoming) > limit {
        warn!(
            total,
            what, "byte quota exhausted, rejecting anonymous write"
        );
        return Err(Box::new(
            (StatusCode::INSUFFICIENT_STORAGE, "storage quota exhausted").into_response(),
        ));
    }
    Ok(())
}

/// The `&dyn Error` coercion tracing's `Value` impl needs to record an error with its
/// source chain — shared so call sites don't repeat the cast (DUP-3).
pub(crate) fn log_err<E: std::error::Error + 'static>(e: &E) -> &(dyn std::error::Error + 'static) {
    e
}

/// [`log_err`] for `anyhow::Error`, which exposes the chain as a `dyn Error` deref
/// rather than implementing the trait itself.
pub(crate) fn log_anyhow(e: &anyhow::Error) -> &(dyn std::error::Error + 'static) {
    &**e as &(dyn std::error::Error + 'static)
}
