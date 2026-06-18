//! Serves the embedded frontend — the OxyDraw SPA built from `frontend/`.
//!
//! Plain static-asset serving with a single-page-app fallback: an unknown extension-less path
//! resolves to `index.html` so client-side routes work. Unlike the previous vendored-app
//! integration, the SPA is built knowing its own backend contract (same-origin `/api/*`), so
//! there is no serve-time host/scheme rewriting — assets are served verbatim.

use std::borrow::Cow;

use axum::body::Bytes;
use axum::http::{header, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use rust_embed::RustEmbed;

/// The built frontend, embedded at compile time from `assets/`. Populated by the build
/// (`make frontend` or the Docker frontend stage). When `assets/` holds only its
/// `.gitkeep`, the server falls back to [`PLACEHOLDER_HTML`].
#[derive(RustEmbed)]
#[folder = "assets/"]
struct Assets;

/// Fallback handler: serve a static asset, or fall back to `index.html` for client-side
/// routes (paths without a file extension).
pub async fn serve(uri: Uri) -> Response {
    let raw = uri.path().trim_start_matches('/');
    let path = if raw.is_empty() { "index.html" } else { raw };

    let (data, name) = match Assets::get(path) {
        Some(file) => (file.data, path),
        // SPA fallback: client-side routes (and the root) serve index.html, or the built-in
        // placeholder when no real frontend has been embedded yet.
        None if !path.contains('.') => match Assets::get("index.html") {
            Some(file) => (file.data, "index.html"),
            None => return placeholder_response(),
        },
        None => return (StatusCode::NOT_FOUND, "not found").into_response(),
    };

    let mime = mime_guess::from_path(name).first_or_octet_stream();
    // Served straight from the embedded data — no copy, no rewrite.
    ([(header::CONTENT_TYPE, mime.as_ref())], cow_bytes(data)).into_response()
}

/// Embedded-asset body without copying: compile-time-embedded data (`Cow::Borrowed`,
/// release builds) is served as a `'static` slice.
fn cow_bytes(data: Cow<'static, [u8]>) -> Bytes {
    match data {
        Cow::Borrowed(bytes) => Bytes::from_static(bytes),
        Cow::Owned(bytes) => Bytes::from(bytes),
    }
}

/// Served when no real frontend has been embedded (the `assets/` dir holds only `.gitkeep`).
/// Build the real UI with `make frontend` (which builds the `frontend/` SPA) or use the Docker image.
const PLACEHOLDER_HTML: &str = concat!(
    "<!doctype html>\n",
    "<meta charset=\"utf-8\">\n",
    "<title>oxydraw</title>\n",
    "<h1>oxydraw backend is running</h1>\n",
    "<p>No frontend is embedded in this build. Build it with ",
    "<code>make frontend</code> (or use the Docker image), which bundles the ",
    "<code>frontend/</code> SPA into <code>backend/crates/server/assets/</code>.</p>\n",
);

fn placeholder_response() -> Response {
    (
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        PLACEHOLDER_HTML,
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cow_bytes_preserves_borrowed_and_owned() {
        assert_eq!(&cow_bytes(Cow::Borrowed(b"static"))[..], b"static");
        assert_eq!(&cow_bytes(Cow::Owned(b"owned".to_vec()))[..], b"owned");
    }
}
