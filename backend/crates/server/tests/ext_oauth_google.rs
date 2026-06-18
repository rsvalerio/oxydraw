//! Google (OIDC) sign-in, end to end against a mock OIDC provider that serves
//! discovery + JWKS and signs real RS256 ID tokens with a fixed test key.

use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use oxydraw_core::config::{Config, StorageType};

mod common;
use common::{client, cookie, location, query_param, serve, state_cookie};

/// Test-only RSA key (checked in, never used outside tests) and its precomputed JWK
/// modulus (base64url, no padding). The public exponent is the standard 65537.
const TEST_KEY_PEM: &str = include_str!("data/oidc_test_key.pem");
const TEST_KEY_N: &str = include_str!("data/oidc_test_key.n.txt");
const KID: &str = "test-key";
const CLIENT_ID: &str = "google-client-id";

#[derive(Clone)]
struct MockOidc {
    /// Issuer URL (the mock's own base URL, set after binding).
    issuer: Arc<Mutex<String>>,
    /// Nonce to embed in the next ID token (the test copies it from the auth URL).
    nonce: Arc<Mutex<String>>,
}

async fn token_endpoint(State(mock): State<MockOidc>) -> Json<serde_json::Value> {
    let issuer = mock.issuer.lock().unwrap().clone();
    let nonce = mock.nonce.lock().unwrap().clone();
    let now = chrono::Utc::now().timestamp();
    let claims = serde_json::json!({
        "iss": issuer,
        "sub": "108417629577929095412",
        "aud": CLIENT_ID,
        "exp": now + 3600,
        "iat": now,
        "nonce": nonce,
        "email": "alice@example.com",
        "email_verified": true,
        "name": "Alice Example",
        "picture": "https://example.com/alice.png",
    });
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(KID.to_string());
    let key = EncodingKey::from_rsa_pem(TEST_KEY_PEM.as_bytes()).unwrap();
    let id_token = encode(&header, &claims, &key).unwrap();
    Json(serde_json::json!({
        "access_token": "mock-access-token",
        "token_type": "Bearer",
        "expires_in": 3600,
        "id_token": id_token,
    }))
}

async fn spawn_mock_oidc(mock: MockOidc) -> std::net::SocketAddr {
    let app = Router::new()
        .route(
            "/.well-known/openid-configuration",
            get(|State(mock): State<MockOidc>| async move {
                let issuer = mock.issuer.lock().unwrap().clone();
                Json(serde_json::json!({
                    "issuer": issuer,
                    "authorization_endpoint": format!("{issuer}/authorize"),
                    "token_endpoint": format!("{issuer}/token"),
                    "jwks_uri": format!("{issuer}/jwks"),
                    "response_types_supported": ["code"],
                    "subject_types_supported": ["public"],
                    "id_token_signing_alg_values_supported": ["RS256"],
                }))
            }),
        )
        .route(
            "/jwks",
            get(|| async {
                Json(serde_json::json!({
                    "keys": [{
                        "kty": "RSA", "use": "sig", "alg": "RS256", "kid": KID,
                        "n": TEST_KEY_N, "e": "AQAB",
                    }]
                }))
            }),
        )
        .route("/token", post(token_endpoint))
        .with_state(mock.clone());
    let addr = serve(app).await;
    // The issuer is read per request from the shared Mutex, so setting it right after the
    // server is up (before the test makes any call) is fine.
    *mock.issuer.lock().unwrap() = format!("http://{addr}");
    addr
}

async fn spawn_app(oidc: std::net::SocketAddr) -> std::net::SocketAddr {
    spawn_app_with(oidc, Config::default()).await
}

/// Like [`spawn_app`] but lets a test layer extra config (e.g. an allowlist) over the
/// Google provider settings.
async fn spawn_app_with(oidc: std::net::SocketAddr, extra: Config) -> std::net::SocketAddr {
    common::spawn_app(Config {
        listen: "127.0.0.1:0".to_string(),
        storage_type: StorageType::Memory,
        google_client_id: Some(CLIENT_ID.to_string()),
        google_client_secret: Some("google-client-secret".into()),
        google_issuer_url: Some(format!("http://{oidc}")),
        ..extra
    })
    .await
}

#[tokio::test]
async fn google_sign_in_round_trip() {
    let mock = MockOidc {
        issuer: Arc::new(Mutex::new(String::new())),
        nonce: Arc::new(Mutex::new(String::new())),
    };
    let oidc = spawn_mock_oidc(mock.clone()).await;
    let addr = spawn_app(oidc).await;
    let base = format!("http://{addr}");
    let client = client();

    // Start: redirect to the provider with PKCE + nonce + openid scope.
    let r = client
        .get(format!("{base}/api/ext/auth/google"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 307);
    let auth_url = location(&r);
    assert!(auth_url.starts_with(&format!("http://{oidc}/authorize")));
    assert!(query_param(&auth_url, "scope").unwrap().contains("openid"));
    assert_eq!(
        query_param(&auth_url, "code_challenge_method").as_deref(),
        Some("S256")
    );
    let state = query_param(&auth_url, "state").unwrap();
    let flow_cookie = state_cookie(&r);
    let nonce = query_param(&auth_url, "nonce").expect("OIDC auth URL carries a nonce");
    *mock.nonce.lock().unwrap() = nonce;

    // Callback: exchanges the code, verifies the ID token, mints a session.
    let r = client
        .get(format!(
            "{base}/api/ext/auth/callback/google?code=mock-code&state={state}"
        ))
        .header("cookie", &flow_cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 307);
    assert_eq!(location(&r), "/");
    let session = cookie(&r, "ext_session");

    let me: serde_json::Value = client
        .get(format!("{base}/api/ext/me"))
        .header("cookie", &session)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(me["user"]["email"], "alice@example.com");
    assert_eq!(me["user"]["name"], "Alice Example");
    assert_eq!(me["user"]["avatar_url"], "https://example.com/alice.png");
    assert_eq!(me["org"]["id"], "default");
}

#[tokio::test]
async fn google_rejects_identity_outside_allowlist() {
    let mock = MockOidc {
        issuer: Arc::new(Mutex::new(String::new())),
        nonce: Arc::new(Mutex::new(String::new())),
    };
    let oidc = spawn_mock_oidc(mock.clone()).await;
    // The mock signs tokens for alice@example.com; the allowlist names someone else.
    let addr = spawn_app_with(
        oidc,
        Config {
            ext_allowed_emails: Some("only-bob@example.com".to_string()),
            ..Config::default()
        },
    )
    .await;
    let base = format!("http://{addr}");
    let client = client();

    let r = client
        .get(format!("{base}/api/ext/auth/google"))
        .send()
        .await
        .unwrap();
    let auth_url = location(&r);
    let state = query_param(&auth_url, "state").unwrap();
    let flow_cookie = state_cookie(&r);
    let nonce = query_param(&auth_url, "nonce").expect("OIDC auth URL carries a nonce");
    *mock.nonce.lock().unwrap() = nonce;

    // The token exchange and ID-token verification succeed, but the allowlist gate
    // rejects the identity: no session cookie, bounce to the login error.
    let r = client
        .get(format!(
            "{base}/api/ext/auth/callback/google?code=mock-code&state={state}"
        ))
        .header("cookie", &flow_cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 307);
    assert!(location(&r).starts_with("/?ext_auth_error="));
    assert!(
        r.headers()
            .get_all("set-cookie")
            .iter()
            .filter_map(|v| v.to_str().ok())
            .all(|c| !c.starts_with("ext_session=")),
        "a non-allowlisted identity must not receive a session cookie"
    );
}

#[tokio::test]
async fn google_rejects_id_token_with_wrong_nonce() {
    let mock = MockOidc {
        issuer: Arc::new(Mutex::new(String::new())),
        nonce: Arc::new(Mutex::new("attacker-controlled".to_string())),
    };
    let oidc = spawn_mock_oidc(mock.clone()).await;
    let addr = spawn_app(oidc).await;
    let base = format!("http://{addr}");
    let client = client();

    let r = client
        .get(format!("{base}/api/ext/auth/google"))
        .send()
        .await
        .unwrap();
    let state = query_param(&location(&r), "state").unwrap();
    let flow_cookie = state_cookie(&r);
    // Note: the mock signs the ID token with a nonce that does NOT match the flow's.

    let r = client
        .get(format!(
            "{base}/api/ext/auth/callback/google?code=mock-code&state={state}"
        ))
        .header("cookie", &flow_cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 307);
    assert!(location(&r).starts_with("/?ext_auth_error="));
    assert!(r.headers().get("set-cookie").is_none());
}
