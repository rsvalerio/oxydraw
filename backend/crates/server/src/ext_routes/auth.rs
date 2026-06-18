//! Login for the scene library: the `EXT_PASSWORD` fallback (with brute-force
//! throttle), the OAuth/OIDC sign-in flow (Google, GitHub) and its host-poisoning and
//! browser-binding guards, and the [`require_session`] gate that resolves a request to a
//! [`CurrentUser`]. The scene-library CRUD that consumes that principal lives in
//! [`super::scenes`].

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Context;
use axum::extract::{Path, Query, Request, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::{AppendHeaders, IntoResponse, Redirect, Response};
use axum::Json;
use oxydraw_core::model::Timestamp;
use oxydraw_core::store::{IdentityProfile, OrgId, Role, StoreError, UserId};
use oxydraw_core::sync::lock_unpoisoned;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use tracing::{error, warn};

use crate::oauth::{AuthError, BeginFlow, Pending, RemoteUser};
use crate::routes::{internal_error, log_anyhow, log_err};
use crate::session::{self, SESSION_COOKIE};
use crate::AppState;

use super::{ensure_default_org, CurrentUser, DEFAULT_ORG, DEFAULT_ORG_NAME};

/// The built-in user behind the `EXT_PASSWORD` fallback login.
const LOCAL_PROVIDER: &str = "local";

/// Short-lived cookie binding an in-progress OAuth flow to the browser that started it.
/// Without it, an attacker could begin a flow, capture the callback URL (`state` + a
/// code for the *attacker's* identity), and deliver it to a victim — logging the
/// victim's browser into the attacker's account (login CSRF / session fixation).
/// `SameSite=Lax` on purpose: the cookie must survive the top-level redirect back from
/// the provider, which `Strict` would drop.
const OAUTH_STATE_COOKIE: &str = "ext_oauth_state";

/// How many failed password attempts are allowed within [`LOGIN_FAILURE_WINDOW`] before
/// further login attempts are rejected with 429.
const LOGIN_MAX_FAILURES: usize = 10;
const LOGIN_FAILURE_WINDOW: Duration = Duration::from_secs(60);

/// Fixed-window throttle for failed password logins, so `EXT_PASSWORD` cannot be brute
/// forced at network speed. Global rather than per-IP on purpose: the password is a
/// single shared secret (a global guess budget is what matters) and peer addresses are
/// unreliable behind proxies. The worst case for a legitimate user during an active
/// attack is waiting out one [`LOGIN_FAILURE_WINDOW`].
#[derive(Clone, Default)]
pub(crate) struct LoginThrottle(Arc<Mutex<VecDeque<Instant>>>);

impl LoginThrottle {
    /// Whether a login attempt at `now` may proceed; prunes failures older than the
    /// window. `now` is injected (rather than read internally) so the window-expiry
    /// recovery path — what stops the throttle becoming a permanent DoS on the legitimate
    /// user — is unit-testable without waiting out a real [`LOGIN_FAILURE_WINDOW`].
    fn allow(&self, now: Instant) -> bool {
        let mut failures = lock_unpoisoned(&self.0);
        while failures
            .front()
            .is_some_and(|t| now.duration_since(*t) >= LOGIN_FAILURE_WINDOW)
        {
            failures.pop_front();
        }
        failures.len() < LOGIN_MAX_FAILURES
    }

    /// Record a failed attempt at `now`. Only reachable after [`Self::allow`], so the
    /// queue is bounded at [`LOGIN_MAX_FAILURES`] entries.
    fn record_failure(&self, now: Instant) {
        lock_unpoisoned(&self.0).push_back(now);
    }
}

/// Is any login mechanism configured? With none, the library stays open (the rest of
/// this server — shares, collab — is unauthenticated by design anyway).
fn auth_enabled(state: &AppState) -> bool {
    state.config.ext_password.is_some() || !state.oauth.is_empty()
}

/// Gate for the scene library: resolves the session to a [`CurrentUser`] or 401s.
pub(crate) async fn require_session(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Response {
    let current = if auth_enabled(&state) {
        match current_user(&state, req.headers()).await {
            Ok(Some(current)) => current,
            Ok(None) => return (StatusCode::UNAUTHORIZED, "unauthorized").into_response(),
            Err(e) => {
                // A backend outage must surface as 500, not a spurious 401 that makes every
                // authenticated user look logged out (and leaves no trace) — see ERR-6.
                error!(error = log_err(&e), "session validation failed");
                return internal_error();
            }
        }
    } else {
        CurrentUser {
            user_id: None,
            org_id: DEFAULT_ORG.to_string(),
            org_name: DEFAULT_ORG_NAME.to_string(),
        }
    };
    req.extensions_mut().insert(current);
    next.run(req).await
}

/// Resolve the request's session to a user. `Ok(None)` means no/unknown/expired token
/// (a genuine 401); `Err` means the session store itself failed (a 500) — the two must
/// stay distinct so an outage isn't reported as "logged out".
async fn current_user(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<Option<CurrentUser>, StoreError> {
    let Some(token) = headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(session::session_token)
    else {
        return Ok(None);
    };
    let Some(user_id) = state.sessions.validate(token).await? else {
        return Ok(None);
    };
    let (org_id, org_name) = match state.store.org_for_user(UserId(&user_id)).await {
        Ok(org) => (org.id, org.name),
        // Memberships are written at login, so a missing one is unexpected — but a user
        // without one still belongs in the default org rather than locked out. Only
        // `NotFound` earns that fallback: any other failure is a storage outage and must
        // propagate (a 500 via `require_session`), not masquerade as default-org
        // membership — see ERR-6 and `Sessions::validate`.
        Err(StoreError::NotFound) => (DEFAULT_ORG.to_string(), DEFAULT_ORG_NAME.to_string()),
        Err(e) => return Err(e),
    };
    Ok(Some(CurrentUser {
        user_id: Some(user_id),
        org_id,
        org_name,
    }))
}

#[derive(Deserialize)]
pub(crate) struct Login {
    password: String,
}

pub(crate) async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<Login>,
) -> Response {
    let Some(expected) = state.config.ext_password.as_ref() else {
        // Password auth disabled: nothing to log into, but the sign-in UI shouldn't error out.
        return StatusCode::NO_CONTENT.into_response();
    };
    if !state.login_throttle.allow(Instant::now()) {
        warn!("login throttled: too many failed password attempts");
        return (
            StatusCode::TOO_MANY_REQUESTS,
            "too many failed login attempts, try again later",
        )
            .into_response();
    }
    // SEC-7: compare fixed-width digests, not raw bytes — `ct_eq` on slices
    // short-circuits on a length mismatch, which would leak the configured password's
    // length as a timing oracle. Hashing first makes both operands 32 bytes for any
    // input, so the comparison is genuinely constant-time in length and content.
    let matches: bool = Sha256::digest(expected.expose_secret().as_bytes())
        .ct_eq(&Sha256::digest(req.password.as_bytes()))
        .into();
    if !matches {
        state.login_throttle.record_failure(Instant::now());
        return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    }

    let profile = IdentityProfile {
        provider: LOCAL_PROVIDER.to_string(),
        provider_user_id: LOCAL_PROVIDER.to_string(),
        ..IdentityProfile::default()
    };
    let token = match establish_session(&state, &profile).await {
        Ok(token) => token,
        Err(e) => {
            error!(
                error = log_anyhow(&e),
                "failed to establish password session"
            );
            return internal_error();
        }
    };
    let cookie = session_cookie(&token, request_is_https(&state, &headers));
    (StatusCode::NO_CONTENT, [(header::SET_COOKIE, cookie)]).into_response()
}

pub(crate) async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Some(token) = headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(session::session_token)
    {
        if let Err(e) = state.sessions.revoke(token).await {
            // The cookie is cleared regardless, but a failed delete leaves the session
            // valid until TTL — operators need to see that, so don't swallow it (ERR-1).
            error!(error = log_err(&e), "failed to revoke session on logout");
        }
    }
    let cookie = format!("{SESSION_COOKIE}=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0");
    (StatusCode::NO_CONTENT, [(header::SET_COOKIE, cookie)]).into_response()
}

/// Upsert the user for `profile`, enroll them in the default org, and mint a session.
/// The shared tail of every login path (password and OAuth).
async fn establish_session(state: &AppState, profile: &IdentityProfile) -> anyhow::Result<String> {
    let user = state
        .store
        .upsert_user_for_identity(profile, Timestamp::now())
        .await
        .context("upserting user for identity")?;
    ensure_default_org(state.store.as_ref())
        .await
        .context("ensuring default org")?;
    state
        .store
        .add_member(OrgId(DEFAULT_ORG), UserId(&user.id), Role::Member)
        .await
        .context("enrolling user in default org")?;
    let token = state
        .sessions
        .mint(&user.id)
        .await
        .context("minting session")?;
    Ok(token)
}

/// Whether the deployment serves HTTPS — decides the cookies' `Secure` attribute.
///
/// `PUBLIC_URL`'s scheme is authoritative when set: it is deployment configuration,
/// immune to request-header spoofing, so an HTTPS deployment always gets `Secure`
/// cookies. Without it we fall back to `x-forwarded-proto`, which is only trustworthy
/// behind a reverse proxy that overwrites the header (see `docs/AUTH.md`); `Secure`
/// stays off for plain-http LAN setups so the cookie still works.
fn request_is_https(state: &AppState, headers: &HeaderMap) -> bool {
    if let Some(url) = &state.config.public_url {
        return url
            .trim_start()
            .to_ascii_lowercase()
            .starts_with("https://");
    }
    headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|proto| proto.eq_ignore_ascii_case("https"))
}

fn session_cookie(token: &str, https: bool) -> String {
    let secure = if https { "; Secure" } else { "" };
    format!("{SESSION_COOKIE}={token}; HttpOnly; SameSite=Strict; Path=/{secure}")
}

/// The [`OAUTH_STATE_COOKIE`] binding an OAuth flow to this browser; `max_age` 0 clears it.
fn oauth_state_cookie(value: &str, max_age: u64, https: bool) -> String {
    let secure = if https { "; Secure" } else { "" };
    format!(
        "{OAUTH_STATE_COOKIE}={value}; HttpOnly; SameSite=Lax; Path=/api/ext/auth; Max-Age={max_age}{secure}"
    )
}

#[derive(Serialize)]
pub(crate) struct ProvidersView {
    /// Configured OAuth providers, e.g. `["github", "google"]`.
    providers: Vec<&'static str>,
    /// Whether the password fallback login is available.
    password: bool,
}

pub(crate) async fn auth_providers(State(state): State<AppState>) -> Json<ProvidersView> {
    Json(ProvidersView {
        providers: state.oauth.names(),
        password: state.config.ext_password.is_some(),
    })
}

/// Where OAuth providers send the user back. Prefers `PUBLIC_URL`; without it the base
/// is derived from the request's `Host`, but **only for loopback hosts** (local
/// development). `None` for any other host: building the redirect_uri from an
/// attacker-controlled `Host` header is the OAuth analog of password-reset poisoning —
/// the provider would send the victim's browser, auth code attached, to the attacker's
/// origin. Non-loopback deployments must set `PUBLIC_URL`.
fn redirect_uri(state: &AppState, headers: &HeaderMap, provider: &str) -> Option<String> {
    let base = match &state.config.public_url {
        Some(url) => url.trim_end_matches('/').to_string(),
        None => {
            let scheme = if request_is_https(state, headers) {
                "https"
            } else {
                "http"
            };
            let host = headers
                .get(header::HOST)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("localhost");
            if !is_loopback_host(host) {
                return None;
            }
            format!("{scheme}://{host}")
        }
    };
    Some(format!("{base}/api/ext/auth/callback/{provider}"))
}

/// The callback URL the provider must redirect back to, or a login-error response when
/// it can't be derived (a non-loopback `Host` without `PUBLIC_URL` — the host-poisoning
/// guard, SEC-29). Shared by `auth_start` and `auth_callback` so the warning and the
/// user-facing message stay in lockstep across both (DUP-1).
/// The error is boxed for the same reason [`validated_callback`]'s is: `Response` is
/// large and clippy flags oversized `Err` variants.
fn required_redirect_uri(
    state: &AppState,
    headers: &HeaderMap,
    provider: &str,
) -> Result<String, Box<Response>> {
    redirect_uri(state, headers, provider).ok_or_else(|| {
        warn!(
            provider = %provider,
            "refusing to derive OAuth redirect_uri from a non-loopback Host; set PUBLIC_URL"
        );
        Box::new(login_failed("sign-in is not configured for this host"))
    })
}

/// Whether a `Host` header value (optionally `host:port` / `[v6]:port`) is loopback.
fn is_loopback_host(host: &str) -> bool {
    let name = crate::host::host_name(host);
    name.eq_ignore_ascii_case("localhost") || name == "127.0.0.1" || name == "::1"
}

/// Percent-encode `value` for a query-string component (`application/x-www-form-urlencoded`
/// rules: unreserved bytes verbatim, space as `+`, everything else `%XX`). Builds into a
/// single buffer rather than allocating a `String` per byte.
fn percent_encode_query(value: &str) -> String {
    use std::fmt::Write;
    value
        .bytes()
        .fold(String::with_capacity(value.len()), |mut out, b| {
            match b {
                b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'.' | b'_' => {
                    out.push(char::from(b))
                }
                b' ' => out.push('+'),
                // Writing to a String is infallible.
                other => {
                    let _ = write!(out, "%{other:02X}");
                }
            }
            out
        })
}

/// Send the user to their browser's login error — the scene-library panel shows it after redirect.
fn login_failed(message: &str) -> Response {
    let encoded = percent_encode_query(message);
    Redirect::temporary(&format!("/?ext_auth_error={encoded}")).into_response()
}

pub(crate) async fn auth_start(
    State(state): State<AppState>,
    Path(provider_name): Path<String>,
    headers: HeaderMap,
) -> Response {
    let Some(provider) = state.oauth.get(&provider_name) else {
        return (StatusCode::NOT_FOUND, "unknown provider").into_response();
    };
    let redirect = match required_redirect_uri(&state, &headers, &provider_name) {
        Ok(redirect) => redirect,
        Err(response) => return *response,
    };
    match provider.begin(&state.oauth.http, &redirect).await {
        Ok(BeginFlow {
            authorize_url,
            state: flow_state,
            pending,
        }) => {
            let state_cookie = oauth_state_cookie(
                &flow_state,
                crate::oauth::FLOW_TTL.as_secs(),
                request_is_https(&state, &headers),
            );
            state.flows.insert(flow_state, pending);
            (
                [(header::SET_COOKIE, state_cookie)],
                Redirect::temporary(&authorize_url),
            )
                .into_response()
        }
        Err(e) => {
            error!(
                provider = %provider_name,
                error = log_err(&e),
                "failed to start OAuth flow"
            );
            login_failed("could not reach the identity provider")
        }
    }
}

#[derive(Deserialize)]
pub(crate) struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    /// Set instead of `code` when the user denies consent.
    error: Option<String>,
}

/// Validate everything an OAuth callback must prove *before* the code exchange:
/// consent granted, `code` + `state` present, the flow bound to this browser via the
/// state cookie, a live (unexpired, unused) pending flow, and the provider matching the
/// one that started it. The guard order is load-bearing — in particular the browser
/// binding is checked before the pending flow is consumed, so probing requests cannot
/// burn a victim's in-flight login.
///
/// The error is boxed: `Response` is large and clippy flags oversized `Err` variants.
fn validated_callback(
    state: &AppState,
    provider_name: &str,
    headers: &HeaderMap,
    query: CallbackQuery,
) -> Result<(String, Pending), Box<Response>> {
    if let Some(denied) = query.error {
        warn!(provider = %provider_name, error = %denied, "OAuth consent denied");
        return Err(Box::new(login_failed("sign-in was cancelled")));
    }
    let (Some(code), Some(flow_state)) = (query.code, query.state) else {
        return Err(Box::new(login_failed("sign-in failed")));
    };
    // The flow must come back on the browser that started it: the state cookie set by
    // `auth_start` has to match the returned `state` (see [`OAUTH_STATE_COOKIE`]).
    let bound_state = headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|c| session::cookie_value(c, OAUTH_STATE_COOKIE));
    if bound_state != Some(flow_state.as_str()) {
        warn!(
            provider = %provider_name,
            "OAuth callback not bound to this browser (missing or mismatched state cookie)"
        );
        return Err(Box::new(login_failed("sign-in failed")));
    }
    let Some(pending) = state.flows.take(&flow_state) else {
        warn!(provider = %provider_name, "OAuth callback with unknown or expired state");
        return Err(Box::new(login_failed("sign-in expired, please try again")));
    };
    if pending.provider != provider_name {
        warn!(
            expected = %pending.provider,
            got = %provider_name,
            "OAuth callback provider mismatch"
        );
        return Err(Box::new(login_failed("sign-in failed")));
    }
    Ok((code, pending))
}

pub(crate) async fn auth_callback(
    State(state): State<AppState>,
    Path(provider_name): Path<String>,
    headers: HeaderMap,
    Query(query): Query<CallbackQuery>,
) -> Response {
    let Some(provider) = state.oauth.get(&provider_name) else {
        return (StatusCode::NOT_FOUND, "unknown provider").into_response();
    };
    let (code, pending) = match validated_callback(&state, &provider_name, &headers, query) {
        Ok(validated) => validated,
        Err(response) => return *response,
    };

    let redirect = match required_redirect_uri(&state, &headers, &provider_name) {
        Ok(redirect) => redirect,
        Err(response) => return *response,
    };
    let remote = match map_exchange_result(
        &provider_name,
        provider
            .exchange(&state.oauth.http, &redirect, &code, pending)
            .await,
    ) {
        Ok(remote) => remote,
        Err(response) => return *response,
    };

    // SEC-18: gate OAuth identities on EXT_ALLOWED_EMAILS before any user/membership row
    // is created, so an unconfigured allowlist is the only way a stranger's account gets in.
    if !email_allowed(
        state.config.ext_allowed_emails.as_deref(),
        remote.email.as_deref(),
    ) {
        warn!(
            provider = %provider_name,
            "OAuth identity not in EXT_ALLOWED_EMAILS; rejecting sign-in"
        );
        return login_failed("this account is not allowed to sign in");
    }

    let token = match establish_session(&state, &profile_from_remote(&remote)).await {
        Ok(token) => token,
        Err(e) => {
            error!(provider = %provider_name, error = log_anyhow(&e), "failed to establish OAuth session");
            return login_failed("sign-in failed");
        }
    };
    signed_in_response(&state, &headers, &token)
}

/// Collapse the three [`AuthError`] arms of an OAuth exchange to a single `login_failed`
/// response, distinguishing only the server-side log line: a `Provider` stage error is
/// logged with its full source chain at `error!`, anything else at `warn!`. Extracted
/// from `auth_callback` (FN-1) so the handler stays one abstraction level — orchestration
/// — while this owns the error mapping. The error is boxed for the same reason
/// [`validated_callback`]'s is: `Response` is large and clippy flags oversized `Err`
/// variants.
fn map_exchange_result(
    provider_name: &str,
    result: Result<RemoteUser, AuthError>,
) -> Result<RemoteUser, Box<Response>> {
    match result {
        Ok(remote) => Ok(remote),
        Err(e @ AuthError::Provider { .. }) => {
            // Recording as `dyn Error` lets tracing render the full source chain.
            error!(
                provider = %provider_name,
                error = log_err(&e),
                "OAuth exchange failed"
            );
            Err(Box::new(login_failed("sign-in failed")))
        }
        Err(e) => {
            warn!(provider = %provider_name, error = %e, "OAuth exchange rejected");
            Err(Box::new(login_failed("sign-in failed")))
        }
    }
}

/// The successful sign-in response: set the session cookie, expire the flow's
/// browser-binding cookie, and send the user back to the app.
fn signed_in_response(state: &AppState, headers: &HeaderMap, token: &str) -> Response {
    let https = request_is_https(state, headers);
    let cookie = session_cookie(token, https);
    // The flow is complete; expire its browser-binding cookie alongside the session.
    // `AppendHeaders` (not a plain header array, which would replace) so both
    // `Set-Cookie` headers survive.
    let clear_state = oauth_state_cookie("", 0, https);
    (
        AppendHeaders([
            (header::SET_COOKIE, cookie),
            (header::SET_COOKIE, clear_state),
        ]),
        Redirect::temporary("/"),
    )
        .into_response()
}

/// Whether an OAuth identity with `email` may sign in, given the `EXT_ALLOWED_EMAILS`
/// allowlist. Unset allowlist → every identity is allowed (the historical behavior, fine
/// on a trusted LAN). When set, only the listed addresses pass — the gate that stops
/// "Sign in with Google/GitHub" from admitting every account the provider authenticates
/// (SEC-18). An allowlist with no matching email (or a provider that returned no email)
/// is a rejection. Matching is case-insensitive and ignores surrounding whitespace.
fn email_allowed(allowlist: Option<&str>, email: Option<&str>) -> bool {
    let Some(list) = allowlist else {
        return true;
    };
    let Some(email) = email.map(str::trim) else {
        return false;
    };
    list.split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .any(|allowed| allowed.eq_ignore_ascii_case(email))
}

fn profile_from_remote(remote: &RemoteUser) -> IdentityProfile {
    IdentityProfile {
        provider: remote.provider.to_string(),
        provider_user_id: remote.provider_user_id.clone(),
        email: remote.email.clone(),
        name: remote.name.clone(),
        avatar_url: remote.avatar_url.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The OAuth-redirect gate: only loopback hosts may derive a redirect_uri without
    /// `PUBLIC_URL` (see [`redirect_uri`]).
    #[test]
    fn loopback_hosts() {
        for host in [
            "localhost",
            "localhost:3002",
            "LOCALHOST:3002",
            "127.0.0.1",
            "127.0.0.1:3002",
            "[::1]",
            "[::1]:3002",
        ] {
            assert!(is_loopback_host(host), "{host} should be loopback");
        }
        for host in [
            "evil.example",
            "evil.example:3002",
            "localhost.evil.example",
            "10.0.0.7", // private LAN is trusted for asset rewrites, never for OAuth
            "192.168.1.5:3002",
            "8.8.8.8",
            "[2001:db8::1]:443",
            "::1", // unbracketed v6 in a Host header is malformed; rsplit mangles it
            "",
        ] {
            assert!(!is_loopback_host(host), "{host} must not be loopback");
        }
    }

    /// The throttle blocks once the failure budget is spent, then recovers once the
    /// window passes — the path that keeps it from permanently locking out a legitimate
    /// user (TEST-6). Driven by an injected clock so no real 60s wait is needed.
    #[test]
    fn login_throttle_recovers_after_window() {
        let throttle = LoginThrottle::default();
        let start = Instant::now();

        // Spend the whole failure budget at t0.
        for _ in 0..LOGIN_MAX_FAILURES {
            assert!(throttle.allow(start));
            throttle.record_failure(start);
        }
        // Budget exhausted: blocked at t0 and right up to the window's edge.
        assert!(
            !throttle.allow(start),
            "budget exhausted blocks further attempts"
        );
        assert!(!throttle.allow(start + LOGIN_FAILURE_WINDOW - Duration::from_millis(1)));
        // Once the window elapses, the old failures prune and logins recover.
        assert!(
            throttle.allow(start + LOGIN_FAILURE_WINDOW),
            "window expiry restores access"
        );
    }

    /// The OAuth allowlist gate ([`email_allowed`], SEC-18).
    #[test]
    fn email_allowlist_gate() {
        // Unset allowlist: every identity passes (historical open behavior).
        assert!(email_allowed(None, Some("anyone@example.com")));
        assert!(email_allowed(None, None));

        // Set allowlist: only listed addresses pass, case-insensitively and trimming
        // both the configured entries and the candidate.
        let list = Some(" Alice@example.com , bob@example.com ");
        assert!(email_allowed(list, Some("alice@example.com")));
        assert!(email_allowed(list, Some("BOB@EXAMPLE.COM")));
        assert!(email_allowed(list, Some("  bob@example.com  ")));

        // Non-matching identities — and identities with no email — are rejected.
        assert!(!email_allowed(list, Some("mallory@evil.example")));
        assert!(!email_allowed(list, None));
        assert!(!email_allowed(Some(""), Some("alice@example.com")));
    }
}
