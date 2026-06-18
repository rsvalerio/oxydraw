//! oxydraw server: axum router + Socket.IO collaboration + embedded frontend.
//!
//! The router is exposed via [`build_router`] so integration tests can drive the full HTTP
//! surface in-process; [`run`] is the binary entry point.

mod bounded_map;
mod ext_routes;
pub mod files;
mod frontend;
mod host;
pub mod oauth;
pub mod rooms;
pub mod routes;
pub mod session;

use std::sync::Arc;

use anyhow::Context;
use axum::extract::DefaultBodyLimit;
use axum::http::{header, HeaderValue, Method};
use axum::Router;
use oxydraw_core::config::Config;
use oxydraw_core::store::Store;
use tower_http::cors::{Any, CorsLayer};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;
use tracing::{info, warn};

/// Shared application state handed to every handler.
#[derive(Clone)]
pub struct AppState {
    pub store: Arc<dyn Store>,
    pub config: Arc<Config>,
    /// In-memory per-room collab scene snapshots (`/api/rooms/{id}/scene`).
    pub scenes: rooms::SceneSnapshots,
    /// Store-backed login sessions for the scene library.
    pub sessions: session::Sessions,
    /// Configured OAuth/OIDC identity providers.
    pub oauth: Arc<oauth::Registry>,
    /// In-progress OAuth logins (CSRF state → PKCE verifier/nonce).
    pub flows: oauth::PendingFlows,
    /// Failed-password-login throttle, so `EXT_PASSWORD` cannot be brute forced.
    pub(crate) login_throttle: ext_routes::LoginThrottle,
}

impl AppState {
    /// Assemble state from config + store, building the OAuth registry and the
    /// store-backed session manager. Used by `run()` and integration tests.
    pub fn new(config: Config, store: Arc<dyn Store>) -> anyhow::Result<Self> {
        let oauth = oauth::Registry::from_config(&config)?;
        Ok(Self {
            sessions: session::Sessions::new(store.clone()),
            store,
            config: Arc::new(config),
            scenes: rooms::SceneSnapshots::default(),
            oauth: Arc::new(oauth),
            flows: oauth::PendingFlows::default(),
            login_throttle: ext_routes::LoginThrottle::default(),
        })
    }
}

/// Maximum accepted request-body size, matching the collab relay's 5 MB scene ceiling
/// (`MAX_PAYLOAD_BYTES`) — the largest legitimate canvas payload a client uploads.
const MAX_BODY_BYTES: usize = 5_000_000;

/// CORS policy from [`Config::cors_allowed_origins`]. Unset emits no CORS headers at
/// all — the bundled UI is same-origin, so by default a cross-origin page can neither
/// call the API nor read responses. A configured allowlist is scoped to the methods and
/// headers the API actually uses (GET/POST + JSON bodies; sessions ride a cookie, so no
/// `Authorization`). The literal `*` restores allow-everything for development.
fn cors_layer(config: &Config) -> CorsLayer {
    match config.cors_allowed_origins.as_deref().map(str::trim) {
        None | Some("") => CorsLayer::new(),
        Some("*") => CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any),
        Some(list) => {
            let origins: Vec<HeaderValue> = list
                .split(',')
                .map(str::trim)
                .filter(|origin| !origin.is_empty())
                .filter_map(|origin| match origin.parse() {
                    Ok(value) => Some(value),
                    Err(_) => {
                        warn!(origin, "ignoring unparseable CORS_ALLOWED_ORIGINS entry");
                        None
                    }
                })
                .collect();
            CorsLayer::new()
                .allow_origin(origins)
                .allow_methods([Method::GET, Method::POST])
                .allow_headers([header::CONTENT_TYPE])
        }
    }
}

/// Assemble the HTTP routes (without the Socket.IO layer, which the caller applies).
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .merge(routes::api_router())
        .merge(ext_routes::router(state.clone()))
        .fallback(frontend::serve)
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(cors_layer(&state.config))
        // Baseline security headers on every response: stored blobs must not be
        // MIME-sniffed into something executable, the UI must not be framed by other
        // origins (clickjacking), and outbound navigation must not leak full URLs.
        .layer(SetResponseHeaderLayer::if_not_present(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static("SAMEORIGIN"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::REFERRER_POLICY,
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        ))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Whether the `LISTEN` bind address is loopback-only. A parse failure (e.g. a hostname)
/// or an unspecified address like `0.0.0.0` counts as non-loopback — the conservative
/// choice, since both can expose the server beyond the local host.
fn listen_is_loopback(listen: &str) -> bool {
    listen
        .parse::<std::net::SocketAddr>()
        .map(|addr| addr.ip().is_loopback())
        .unwrap_or(false)
}

/// Binary entry point: load config, select storage, and serve.
pub async fn run() -> anyhow::Result<()> {
    let config = Config::from_env().context("loading configuration")?;
    let store = oxydraw_storage::select_store(&config)
        .await
        .context("selecting storage backend")?;
    info!(storage = %config.storage_type, "storage backend selected");

    let listen = config.listen.clone();
    let state = AppState::new(config, store).context("building application state")?;

    if !state.oauth.is_empty() {
        info!(providers = ?state.oauth.names(), "OAuth sign-in enabled");
        // SEC-29: without PUBLIC_URL the OAuth redirect_uri is derived from the request
        // Host header (loopback-gated). Behind a misconfigured reverse proxy that gate
        // can be fooled, so production deployments must pin PUBLIC_URL — warn loudly at
        // startup instead of letting the misconfiguration surface as a redirect to an
        // attacker-influenced URI.
        if state.config.public_url.is_none() {
            warn!(
                "OAuth is enabled but PUBLIC_URL is not set: redirect URIs will be derived \
                 from the request Host header (localhost only). Set PUBLIC_URL for \
                 production deployments — see docs/AUTH.md"
            );
        }
        // SEC-18: without an allowlist, any account the provider authenticates can sign in.
        if state.config.ext_allowed_emails.is_none() {
            warn!(
                "OAuth is enabled but EXT_ALLOWED_EMAILS is not set: any Google/GitHub \
                 account can sign in and reach the scene library. Set EXT_ALLOWED_EMAILS \
                 to restrict access — see docs/AUTH.md"
            );
        }
    }
    if state.config.ext_password.is_none() && state.oauth.is_empty() {
        // SEC-29: open-by-default is fine on a loopback bind (the documented trusted-LAN /
        // dev case), but a non-loopback bind (e.g. the default 0.0.0.0) with no auth exposes
        // a writable scene library and unauthenticated storage to anyone who can reach the
        // socket — that warrants a loud warning, not a quiet info notice.
        if listen_is_loopback(&state.config.listen) {
            info!("no EXT_PASSWORD or OAuth provider set: the scene library is open to anyone who can reach this server");
        } else {
            warn!(
                listen = %state.config.listen,
                "no EXT_PASSWORD or OAuth provider set while bound to a non-loopback address: \
                 the scene library and storage endpoints are open to anyone who can reach this \
                 socket. Set EXT_PASSWORD or an OAuth provider, or bind LISTEN to 127.0.0.1 — \
                 see docs/DEPLOYMENT.md"
            );
        }
    }

    // The single org every signed-in user joins (see ext_routes::DEFAULT_ORG).
    ext_routes::ensure_default_org(state.store.as_ref())
        .await
        .context("creating default org")?;

    let (collab_layer, _io) = oxydraw_collab::build();
    let app = build_router(state).layer(collab_layer);

    let listener = tokio::net::TcpListener::bind(&listen)
        .await
        .with_context(|| format!("binding {listen}"))?;
    let local_addr = listener.local_addr().context("reading local addr")?;
    let url_host = if local_addr.ip().is_unspecified() {
        "127.0.0.1".to_string()
    } else {
        local_addr.ip().to_string()
    };
    info!(
        "starting http server http://{}:{} with binding addr {}",
        url_host,
        local_addr.port(),
        local_addr
    );
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server error")?;
    Ok(())
}

/// Resolve when the process is asked to stop: Ctrl-C (SIGINT) everywhere, plus SIGTERM
/// on unix — what systemd (`systemctl stop`/`restart`, package upgrades) and
/// `docker stop` send. Without the SIGTERM arm, the graceful-shutdown path wired into
/// `axum::serve` never runs on those deployments and in-flight requests are dropped.
async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            // Registration failed; don't shut down for it — wait for the other signal.
            warn!(error = %e, "failed to install Ctrl-C handler");
            std::future::pending::<()>().await;
        }
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{signal, SignalKind};
        match signal(SignalKind::terminate()) {
            Ok(mut sigterm) => {
                sigterm.recv().await;
            }
            Err(e) => {
                warn!(error = %e, "failed to install SIGTERM handler");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    info!("shutting down");
}
