//! Helpers shared by the integration-test binaries. Each binary compiles its own copy,
//! so not every helper is used everywhere.
#![allow(dead_code)]

use std::sync::Arc;

use axum::Router;
use oxydraw::{build_router, AppState};
use oxydraw_core::config::{Config, StorageType};
use oxydraw_core::store::Store;
use oxydraw_storage::MemoryStore;

/// A memory-store config bound to an ephemeral port — the base every server test starts
/// from. Tests layer their own fields with `..test_config()`.
pub fn test_config() -> Config {
    Config {
        listen: "127.0.0.1:0".to_string(),
        storage_type: StorageType::Memory,
        ..Config::default()
    }
}

/// Bind an ephemeral port, serve `router` on a background task, and return its address.
/// The one place the listener-bind / `axum::serve` block lives — fake third-party servers
/// route their custom routers through here too.
pub async fn serve(router: Router) -> std::net::SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    addr
}

/// Spawn the full app backed by `store` on an ephemeral port; returns its address.
pub async fn spawn_app_with_store(config: Config, store: Arc<dyn Store>) -> std::net::SocketAddr {
    let state = AppState::new(config, store).unwrap();
    serve(build_router(state)).await
}

/// Spawn the full app (fresh memory store) on an ephemeral port and return its address.
pub async fn spawn_app(config: Config) -> std::net::SocketAddr {
    spawn_app_with_store(config, Arc::new(MemoryStore::new())).await
}

/// A client that does not follow redirects, so tests can inspect them.
pub fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap()
}

/// The `Location` header of a redirect response.
pub fn location(r: &reqwest::Response) -> String {
    r.headers()
        .get("location")
        .and_then(|v| v.to_str().ok())
        .expect("redirect has a Location header")
        .to_string()
}

/// A single query parameter from a URL.
pub fn query_param(url: &str, name: &str) -> Option<String> {
    let query = url.split_once('?')?.1;
    query.split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        (k == name).then(|| v.to_string())
    })
}

/// The `name=value` pair of the `Set-Cookie` header for cookie `name`, ready to send back
/// on a follow-up request. Panics if the response set no such cookie.
pub fn cookie(r: &reqwest::Response, name: &str) -> String {
    let prefix = format!("{name}=");
    r.headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find(|c| c.starts_with(&prefix))
        .unwrap_or_else(|| panic!("response sets the `{name}` cookie"))
        .split(';')
        .next()
        .unwrap()
        .to_string()
}

/// The browser-binding state cookie set by `/api/ext/auth/{provider}`, as a
/// `name=value` pair to send back on the callback.
pub fn state_cookie(r: &reqwest::Response) -> String {
    cookie(r, "ext_oauth_state")
}
