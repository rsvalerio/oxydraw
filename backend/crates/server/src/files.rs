//! Clean file-blob API for scene attachments (images), replacing the old Firebase Storage
//! emulator.
//!
//! The editor / collaboration client uploads each file under its content-addressed id
//! (Excalidraw's `BinaryFileData.id`, a hash) with `PUT /api/files/{id}`, and peers fetch it
//! back with `GET /api/files/{id}`. Because the id is a hash, the upload is idempotent and two
//! peers holding the same image converge on the same key with no coordination.
//!
//! The server stores opaque, end-to-end-encrypted bytes — it never sees plaintext image data.
//! All files are durable (via [`Store`](oxydraw_core::store::Store)); a row cap and a total-byte
//! quota bound the unauthenticated write path (SEC-33). Unlike the old emulator there is no
//! ephemeral in-memory tier: collab images simply persist, which is simpler and restart-safe.

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use oxydraw_core::model::StoredFile;
use oxydraw_core::store::StoreError;
use tracing::{error, warn};

use crate::routes::{enforce_byte_quota, internal_error, log_err};
use crate::AppState;

/// SEC-33: upper bound on a file id's length. Ids are short content hashes (Excalidraw's are
/// 40 hex chars), so this only rejects pathological input — the byte quota charges payload
/// bytes only, so without it an unbounded id would be free storage.
const MAX_FILE_ID_BYTES: usize = 256;

/// Content types safe to echo back on download. Uploads are unauthenticated, so an
/// attacker-chosen type like `text/html` would otherwise turn a download into same-origin
/// stored XSS; anything outside this inert allow-list is served as `application/octet-stream`.
/// `image/svg+xml` is deliberately absent — SVG can carry script when navigated to as a
/// document.
const INERT_CONTENT_TYPES: &[&str] = &[
    "application/octet-stream",
    "image/png",
    "image/jpeg",
    "image/gif",
    "image/webp",
    "image/avif",
];

/// `PUT /api/files/{id}` — store an opaque file blob under a client-chosen, content-addressed
/// id. Idempotent: re-putting the same id overwrites (peers computing the same hash converge).
/// The `Content-Type` header is recorded verbatim and validated on download.
pub async fn put_file(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if id.len() > MAX_FILE_ID_BYTES {
        warn!(id_len = id.len(), "file upload: id too long");
        return (StatusCode::BAD_REQUEST, "file id too long").into_response();
    }

    // Row cap first (SEC-33): bounds the table even when payloads are tiny and the byte quota
    // below never trips.
    let count = match state.store.count_files().await {
        Ok(count) => count,
        Err(e) => {
            error!(
                error = log_err(&e),
                "file upload: reading file count failed"
            );
            return internal_error();
        }
    };
    if count >= state.config.max_files_count {
        warn!(count, "file upload: durable file row cap reached");
        return (StatusCode::INSUFFICIENT_STORAGE, "storage quota exhausted").into_response();
    }

    let total = match state.store.files_total_bytes().await {
        Ok(total) => total,
        Err(e) => {
            error!(
                error = log_err(&e),
                "file upload: reading file quota failed"
            );
            return internal_error();
        }
    };
    // Lossless on 64-bit targets (usize == u64); on the impossible-overflow path saturate to
    // u64::MAX so the quota gate rejects rather than wrapping to a small value (SEC-15).
    let incoming = u64::try_from(body.len()).unwrap_or(u64::MAX);
    if let Err(response) = enforce_byte_quota(total, incoming, state.config.max_files_bytes, "file")
    {
        return *response;
    }

    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();

    let file = StoredFile {
        content_type,
        data: body.to_vec(),
    };
    if let Err(e) = state.store.put_file(&id, file).await {
        error!(error = log_err(&e), "file upload: persisting file failed");
        return internal_error();
    }
    Json(serde_json::json!({ "id": id })).into_response()
}

/// `GET /api/files/{id}` — return a stored file's bytes. The stored content type is echoed only
/// when inert; otherwise `application/octet-stream`, always with
/// `Content-Disposition: attachment`, so an attacker-chosen type cannot become on-origin XSS.
/// Clients consume these via `fetch()` and decrypt client-side, so neither header affects the
/// legitimate path.
pub async fn get_file(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    match state.store.get_file(&id).await {
        Ok(file) => {
            let content_type =
                if INERT_CONTENT_TYPES.contains(&file.content_type.to_ascii_lowercase().trim()) {
                    file.content_type
                } else {
                    "application/octet-stream".to_string()
                };
            (
                [
                    (header::CONTENT_TYPE, content_type),
                    (header::CONTENT_DISPOSITION, "attachment".to_string()),
                ],
                file.data,
            )
                .into_response()
        }
        Err(StoreError::NotFound) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => {
            error!(error = log_err(&e), "file download: reading file failed");
            internal_error()
        }
    }
}
