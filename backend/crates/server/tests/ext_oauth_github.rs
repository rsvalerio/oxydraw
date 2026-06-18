//! GitHub OAuth sign-in, end to end against a fake GitHub (token + user API).

use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use oxydraw_core::config::{Config, StorageType};

mod common;
use common::{client, cookie, location, query_param, serve, state_cookie};

/// What the fake GitHub should serve, plus what it has seen.
#[derive(Clone, Default)]
struct FakeGithub {
    /// `/user` payload.
    user: Arc<Mutex<serde_json::Value>>,
    /// Body of the last token-exchange request (to assert PKCE made it through).
    token_request: Arc<Mutex<Option<String>>>,
}

async fn spawn_fake_github(fake: FakeGithub) -> std::net::SocketAddr {
    let app = Router::new()
        .route(
            "/login/oauth/access_token",
            post(|State(fake): State<FakeGithub>, body: String| async move {
                *fake.token_request.lock().unwrap() = Some(body);
                Json(serde_json::json!({
                    "access_token": "gho_test_token",
                    "token_type": "bearer",
                    "scope": "read:user,user:email",
                }))
            }),
        )
        .route(
            "/user",
            get(|State(fake): State<FakeGithub>| async move {
                let user = fake.user.lock().unwrap().clone();
                Json(user)
            }),
        )
        .route(
            "/user/emails",
            get(|| async {
                Json(serde_json::json!([
                    { "email": "ci@example.com", "primary": false, "verified": true },
                    // The round-trip fixture's profile email, here reported verified so it is
                    // honored as the gating email (SEC-11): only addresses /user/emails
                    // confirms verified may gate.
                    { "email": "octocat@github.com", "primary": false, "verified": true },
                    { "email": "octo@example.com", "primary": true, "verified": true },
                ]))
            }),
        )
        .with_state(fake);
    serve(app).await
}

async fn spawn_app(github: std::net::SocketAddr) -> std::net::SocketAddr {
    common::spawn_app(Config {
        listen: "127.0.0.1:0".to_string(),
        storage_type: StorageType::Memory,
        github_client_id: Some("test-client-id".to_string()),
        github_client_secret: Some("test-client-secret".into()),
        github_base_url: Some(format!("http://{github}")),
        github_api_url: Some(format!("http://{github}")),
        ..Config::default()
    })
    .await
}

/// Drive `/api/ext/auth/github` and the callback; returns the session cookie.
async fn sign_in(client: &reqwest::Client, base: &str) -> String {
    let r = client
        .get(format!("{base}/api/ext/auth/github"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 307);
    let auth_url = location(&r);
    assert!(auth_url.contains("/login/oauth/authorize"));
    assert_eq!(
        query_param(&auth_url, "code_challenge_method").as_deref(),
        Some("S256")
    );
    assert!(query_param(&auth_url, "code_challenge").is_some());
    let state = query_param(&auth_url, "state").expect("auth URL carries state");
    let flow_cookie = state_cookie(&r);
    let redirect_uri = query_param(&auth_url, "redirect_uri").unwrap();
    assert!(redirect_uri.contains("callback%2Fgithub") || redirect_uri.contains("callback/github"));

    let r = client
        .get(format!(
            "{base}/api/ext/auth/callback/github?code=test-code&state={state}"
        ))
        .header("cookie", &flow_cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 307);
    assert_eq!(location(&r), "/");
    // The session cookie's HttpOnly/Secure attributes come from the shared `session_cookie`
    // builder and are pinned once at that level by ext_auth.rs; here we just need the value
    // to drive follow-up requests.
    cookie(&r, "ext_session")
}

#[tokio::test]
async fn github_sign_in_round_trip() {
    let fake = FakeGithub::default();
    *fake.user.lock().unwrap() = serde_json::json!({
        "id": 583231, "login": "octocat", "name": "The Octocat",
        "email": "octocat@github.com", "avatar_url": "https://example.com/a.png",
    });
    let github = spawn_fake_github(fake.clone()).await;
    let addr = spawn_app(github).await;
    let base = format!("http://{addr}");
    let client = client();

    // With only GitHub configured, the providers endpoint says so.
    let r = client
        .get(format!("{base}/api/ext/auth/providers"))
        .send()
        .await
        .unwrap();
    let providers: serde_json::Value = r.json().await.unwrap();
    assert_eq!(providers["providers"], serde_json::json!(["github"]));
    assert_eq!(providers["password"], serde_json::json!(false));

    // The library is gated before sign-in.
    let r = client
        .get(format!("{base}/api/ext/scenes"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 401);

    let session = sign_in(&client, &base).await;

    // The token exchange carried the PKCE verifier.
    let token_body = fake.token_request.lock().unwrap().clone().unwrap();
    assert!(token_body.contains("code_verifier="));
    assert!(token_body.contains("code=test-code"));

    // The session works: /me shows the GitHub profile, scenes are accessible.
    let r = client
        .get(format!("{base}/api/ext/me"))
        .header("cookie", &session)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let me: serde_json::Value = r.json().await.unwrap();
    assert_eq!(me["user"]["name"], "The Octocat");
    assert_eq!(me["user"]["email"], "octocat@github.com");
    assert_eq!(me["org"]["id"], "default");

    let r = client
        .get(format!("{base}/api/ext/scenes"))
        .header("cookie", &session)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);

    // Without the cookie, /me stays 401.
    let r = client
        .get(format!("{base}/api/ext/me"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 401);
}

#[tokio::test]
async fn github_private_email_falls_back_to_emails_endpoint() {
    let fake = FakeGithub::default();
    *fake.user.lock().unwrap() = serde_json::json!({
        "id": 99, "login": "shy", "name": null, "email": null, "avatar_url": null,
    });
    let github = spawn_fake_github(fake).await;
    let addr = spawn_app(github).await;
    let base = format!("http://{addr}");
    let client = client();

    let session = sign_in(&client, &base).await;
    let me: serde_json::Value = client
        .get(format!("{base}/api/ext/me"))
        .header("cookie", &session)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    // Primary verified email from /user/emails; login as the name fallback.
    assert_eq!(me["user"]["email"], "octo@example.com");
    assert_eq!(me["user"]["name"], "shy");
}

/// Spawn the app behind a default fake GitHub; returns its base URL and a client. For the
/// callback-guard tests that bail out before any GitHub call, so the user fixture is moot.
async fn spawn_default_app() -> (String, reqwest::Client) {
    let github = spawn_fake_github(FakeGithub::default()).await;
    let addr = spawn_app(github).await;
    (format!("http://{addr}"), client())
}

/// A callback bearing a state the server never issued is rejected with an error redirect
/// and mints no session. Independent of any flow (TEST-2).
#[tokio::test]
async fn callback_with_forged_state_is_rejected() {
    let (base, client) = spawn_default_app().await;
    let r = client
        .get(format!(
            "{base}/api/ext/auth/callback/github?code=c&state=forged"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 307);
    assert!(location(&r).starts_with("/?ext_auth_error="));
    assert!(r.headers().get("set-cookie").is_none());
}

/// A callback reporting denied consent redirects to the login error, no session.
#[tokio::test]
async fn callback_consent_denied_redirects_to_error() {
    let (base, client) = spawn_default_app().await;
    let r = client
        .get(format!(
            "{base}/api/ext/auth/callback/github?error=access_denied&state=whatever"
        ))
        .send()
        .await
        .unwrap();
    assert!(location(&r).starts_with("/?ext_auth_error="));
    assert!(r.headers().get("set-cookie").is_none());
}

/// Starting a flow for an unconfigured provider is a 404.
#[tokio::test]
async fn auth_start_for_unknown_provider_is_404() {
    let (base, client) = spawn_default_app().await;
    let r = client
        .get(format!("{base}/api/ext/auth/facebook"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 404);
}

/// Genuinely sequential (TEST-2): a flow's state is single-use — once a callback consumes
/// it, replaying the same (state, code) is rejected.
#[tokio::test]
async fn callback_state_is_single_use() {
    let fake = FakeGithub::default();
    *fake.user.lock().unwrap() = serde_json::json!({
        "id": 1, "login": "x", "name": null, "email": "x@example.com", "avatar_url": null,
    });
    let github = spawn_fake_github(fake).await;
    let addr = spawn_app(github).await;
    let base = format!("http://{addr}");
    let client = client();

    let r = client
        .get(format!("{base}/api/ext/auth/github"))
        .send()
        .await
        .unwrap();
    let state = query_param(&location(&r), "state").unwrap();
    let flow_cookie = state_cookie(&r);
    let callback = format!("{base}/api/ext/auth/callback/github?code=test-code&state={state}");

    // First completion succeeds.
    let r = client
        .get(&callback)
        .header("cookie", &flow_cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(location(&r), "/");

    // Replaying the now-consumed state fails.
    let r = client
        .get(&callback)
        .header("cookie", &flow_cookie)
        .send()
        .await
        .unwrap();
    assert!(location(&r).starts_with("/?ext_auth_error="));
}

/// Login-CSRF binding: a callback with a state that is valid server-side is still
/// rejected unless the browser presents the state cookie set by `auth_start`.
#[tokio::test]
async fn callback_requires_browser_bound_state_cookie() {
    let fake = FakeGithub::default();
    *fake.user.lock().unwrap() = serde_json::json!({
        "id": 7, "login": "victim", "name": null, "email": "v@example.com", "avatar_url": null,
    });
    let github = spawn_fake_github(fake).await;
    let addr = spawn_app(github).await;
    let base = format!("http://{addr}");
    let client = client();

    // Attacker starts a flow and obtains a server-side-valid (state, code) pair…
    let r = client
        .get(format!("{base}/api/ext/auth/github"))
        .send()
        .await
        .unwrap();
    let state = query_param(&location(&r), "state").unwrap();
    let flow_cookie = state_cookie(&r);
    let callback = format!("{base}/api/ext/auth/callback/github?code=test-code&state={state}");

    // …but a victim browser without the state cookie is rejected…
    let r = client.get(&callback).send().await.unwrap();
    assert!(location(&r).starts_with("/?ext_auth_error="));
    assert!(r.headers().get("set-cookie").is_none(), "no session minted");

    // …as is one carrying a different flow's cookie.
    let r = client
        .get(&callback)
        .header("cookie", "ext_oauth_state=some-other-flow")
        .send()
        .await
        .unwrap();
    assert!(location(&r).starts_with("/?ext_auth_error="));
    assert!(r.headers().get("set-cookie").is_none(), "no session minted");

    // The initiating browser (correct cookie) still completes the flow.
    let r = client
        .get(&callback)
        .header("cookie", &flow_cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(location(&r), "/");
}

/// Host-header poisoning: with PUBLIC_URL unset, the redirect_uri may only be derived
/// from a loopback Host. A forged Host on /api/ext/auth/{provider} is refused instead
/// of being handed to the provider as the callback target.
#[tokio::test]
async fn auth_start_refuses_forged_host_without_public_url() {
    let fake = FakeGithub::default();
    let github = spawn_fake_github(fake).await;
    let addr = spawn_app(github).await;
    let base = format!("http://{addr}");
    let client = client();

    // Forged Host → error redirect, never a provider redirect carrying redirect_uri.
    let r = client
        .get(format!("{base}/api/ext/auth/github"))
        .header("host", "attacker.example")
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 307);
    assert!(
        location(&r).starts_with("/?ext_auth_error="),
        "forged Host must not start a flow: {}",
        location(&r)
    );

    // The loopback Host the tests use keeps working.
    let r = client
        .get(format!("{base}/api/ext/auth/github"))
        .send()
        .await
        .unwrap();
    assert!(location(&r).contains("/login/oauth/authorize"));
}
