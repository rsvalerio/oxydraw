//! Auth around the scene library: `EXT_PASSWORD` + session cookie.

use std::sync::Arc;

use oxydraw_core::config::Config;

mod common;
use common::{spawn_app, spawn_app_with_store};

fn test_config(ext_password: Option<&str>) -> Config {
    Config {
        ext_password: ext_password.map(Into::into),
        ..common::test_config()
    }
}

async fn spawn(ext_password: Option<&str>) -> std::net::SocketAddr {
    spawn_app(test_config(ext_password)).await
}

async fn login(client: &reqwest::Client, base: &str, password: &str) -> reqwest::Response {
    client
        .post(format!("{base}/api/ext/login"))
        .header("content-type", "application/json")
        .body(serde_json::json!({ "password": password }).to_string())
        .send()
        .await
        .unwrap()
}

/// Independent of any session (TEST-2): with a password configured, the scene library
/// rejects requests carrying no cookie and requests carrying a bogus token alike.
#[tokio::test]
async fn unauthenticated_scene_requests_are_rejected() {
    let addr = spawn(Some("hunter2")).await;
    let base = format!("http://{addr}");
    let client = reqwest::Client::new();

    // No cookie → 401.
    let r = client
        .get(format!("{base}/api/ext/scenes"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 401);

    // A bogus token → 401.
    let r = client
        .get(format!("{base}/api/ext/scenes"))
        .header("cookie", "ext_session=forged")
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 401);
}

/// A wrong password is rejected with 401 and never mints a cookie.
#[tokio::test]
async fn wrong_password_is_rejected_without_a_cookie() {
    let addr = spawn(Some("hunter2")).await;
    let base = format!("http://{addr}");
    let client = reqwest::Client::new();

    let r = login(&client, &base, "wrong").await;
    assert_eq!(r.status(), 401);
    assert!(r.headers().get("set-cookie").is_none());
}

/// The genuinely sequential happy path: log in, use the cookie to reach the library, then
/// log out and confirm the session is revoked.
#[tokio::test]
async fn password_login_grants_access_then_logout_revokes() {
    let addr = spawn(Some("hunter2")).await;
    let base = format!("http://{addr}");
    let client = reqwest::Client::new();

    // Correct password → 204 + session cookie.
    let r = login(&client, &base, "hunter2").await;
    assert_eq!(r.status(), 204);
    let cookie = r
        .headers()
        .get("set-cookie")
        .and_then(|v| v.to_str().ok())
        .expect("login sets a session cookie")
        .to_string();
    assert!(cookie.starts_with("ext_session="));
    assert!(cookie.contains("HttpOnly"));
    // Plain-http request must not get a Secure cookie (it would be dropped).
    assert!(!cookie.contains("Secure"));

    // Replaying the cookie grants access.
    let session = common::cookie(&r, "ext_session");
    let r = client
        .get(format!("{base}/api/ext/scenes"))
        .header("cookie", &session)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);

    // Logout revokes the session.
    let r = client
        .post(format!("{base}/api/ext/logout"))
        .header("cookie", &session)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 204);
    let r = client
        .get(format!("{base}/api/ext/scenes"))
        .header("cookie", &session)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 401);
}

/// Brute-force protection: after the failure budget is spent, login attempts — even
/// with the correct password — are rejected with 429 until the window passes.
#[tokio::test]
async fn login_throttles_rapid_wrong_passwords() {
    let addr = spawn(Some("hunter2")).await;
    let base = format!("http://{addr}");
    let client = reqwest::Client::new();

    // Burn through the failure budget at network speed.
    for _ in 0..10 {
        let r = login(&client, &base, "wrong").await;
        assert_eq!(r.status(), 401);
    }

    // Further guesses are throttled rather than compared.
    let r = login(&client, &base, "wrong").await;
    assert_eq!(r.status(), 429, "attempts beyond the budget are throttled");

    // The lockout applies to the correct password too (it expires with the window).
    let r = login(&client, &base, "hunter2").await;
    assert_eq!(r.status(), 429);
}

#[tokio::test]
async fn https_login_marks_cookie_secure() {
    let addr = spawn(Some("hunter2")).await;
    let base = format!("http://{addr}");
    let client = reqwest::Client::new();

    let r = client
        .post(format!("{base}/api/ext/login"))
        .header("content-type", "application/json")
        .header("x-forwarded-proto", "https")
        .body(serde_json::json!({ "password": "hunter2" }).to_string())
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 204);
    let cookie = r
        .headers()
        .get("set-cookie")
        .and_then(|v| v.to_str().ok())
        .unwrap();
    assert!(cookie.contains("Secure"));
}

/// With an https PUBLIC_URL the Secure attribute is deployment config: it is set even
/// when the request carries no x-forwarded-proto header (which a direct client could
/// spoof — or omit — at will).
#[tokio::test]
async fn https_public_url_forces_secure_cookie_without_headers() {
    let config = Config {
        public_url: Some("https://draw.example.com".to_string()),
        ..test_config(Some("hunter2"))
    };
    let addr = spawn_app(config).await;

    let client = reqwest::Client::new();
    let r = login(&client, &format!("http://{addr}"), "hunter2").await;
    assert_eq!(r.status(), 204);
    let cookie = r
        .headers()
        .get("set-cookie")
        .and_then(|v| v.to_str().ok())
        .unwrap();
    assert!(cookie.contains("Secure"), "PUBLIC_URL=https forces Secure");
}

#[tokio::test]
async fn password_login_acts_as_local_user() {
    let addr = spawn(Some("hunter2")).await;
    let base = format!("http://{addr}");
    let client = reqwest::Client::new();

    let r = login(&client, &base, "hunter2").await;
    let session = common::cookie(&r, "ext_session");

    let me: serde_json::Value = client
        .get(format!("{base}/api/ext/me"))
        .header("cookie", &session)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    // The built-in local user has no profile, but belongs to the default org.
    assert_eq!(me["user"]["email"], serde_json::Value::Null);
    assert_eq!(me["org"]["id"], "default");

    // The providers endpoint advertises password-only auth.
    let providers: serde_json::Value = client
        .get(format!("{base}/api/ext/auth/providers"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(providers["providers"], serde_json::json!([]));
    assert_eq!(providers["password"], serde_json::json!(true));
}

/// Sessions are persisted: a server restart over the same SQLite file keeps logins.
#[tokio::test]
async fn sessions_survive_restart_with_sqlite() {
    let db = tempfile::NamedTempFile::new().unwrap();
    let url = format!("sqlite://{}?mode=rwc", db.path().display());

    let spawn_sqlite = |url: String| async move {
        let store = oxydraw_storage::SqliteStore::connect(&url).await.unwrap();
        spawn_app_with_store(test_config(Some("hunter2")), Arc::new(store)).await
    };

    let addr = spawn_sqlite(url.clone()).await;
    let client = reqwest::Client::new();
    let r = login(&client, &format!("http://{addr}"), "hunter2").await;
    let session = common::cookie(&r, "ext_session");

    // "Restart": a fresh app over the same database file.
    let addr2 = spawn_sqlite(url).await;
    let r = client
        .get(format!("http://{addr2}/api/ext/scenes"))
        .header("cookie", &session)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200, "session must survive the restart");

    // Logout on the new instance revokes it durably.
    client
        .post(format!("http://{addr2}/api/ext/logout"))
        .header("cookie", &session)
        .send()
        .await
        .unwrap();
    let r = client
        .get(format!("http://{addr2}/api/ext/scenes"))
        .header("cookie", &session)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 401);
}

#[tokio::test]
async fn scene_library_stays_open_without_password() {
    let addr = spawn(None).await;
    let base = format!("http://{addr}");
    let client = reqwest::Client::new();

    let r = client
        .get(format!("{base}/api/ext/scenes"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);

    // Login is a no-op rather than an error, so the sign-in UI never wedges.
    let r = login(&client, &base, "anything").await;
    assert_eq!(r.status(), 204);
}
