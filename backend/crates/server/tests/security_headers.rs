//! HTTP perimeter defaults: baseline security headers and the CORS allowlist.

use oxydraw_core::config::Config;

mod common;
use common::{spawn_app as spawn, test_config};

fn header<'a>(r: &'a reqwest::Response, name: &str) -> Option<&'a str> {
    r.headers().get(name).and_then(|v| v.to_str().ok())
}

/// nosniff / frame control / referrer policy ride a router-wide layer, so they must be
/// present on every route: the app shell, the scene-library API, and the share API.
#[tokio::test]
async fn baseline_security_headers_on_all_routes() {
    let addr = spawn(test_config()).await;
    let base = format!("http://{addr}");
    let client = reqwest::Client::new();

    for path in ["/", "/api/ext/auth/providers", "/api/v2/does-not-exist"] {
        let r = client.get(format!("{base}{path}")).send().await.unwrap();
        assert_eq!(
            header(&r, "x-content-type-options"),
            Some("nosniff"),
            "{path}"
        );
        assert_eq!(header(&r, "x-frame-options"), Some("SAMEORIGIN"), "{path}");
        assert_eq!(
            header(&r, "referrer-policy"),
            Some("strict-origin-when-cross-origin"),
            "{path}"
        );
    }
}

/// Default: no CORS headers at all — a cross-origin page cannot read responses.
#[tokio::test]
async fn cors_is_same_origin_only_by_default() {
    let addr = spawn(test_config()).await;
    let client = reqwest::Client::new();

    let r = client
        .get(format!("http://{addr}/api/v2/does-not-exist"))
        .header("origin", "https://evil.example")
        .send()
        .await
        .unwrap();
    assert!(
        header(&r, "access-control-allow-origin").is_none(),
        "no origin may be granted by default"
    );
}

/// The literal `*` is the development escape hatch: any origin is granted via the
/// wildcard `access-control-allow-origin: *` response header.
#[tokio::test]
async fn cors_wildcard_grants_any_origin() {
    let config = Config {
        cors_allowed_origins: Some("*".to_string()),
        ..test_config()
    };
    let addr = spawn(config).await;
    let client = reqwest::Client::new();

    let r = client
        .get(format!("http://{addr}/api/v2/does-not-exist"))
        .header("origin", "https://anywhere.example")
        .send()
        .await
        .unwrap();
    assert_eq!(header(&r, "access-control-allow-origin"), Some("*"));
}

/// An unparseable allowlist entry is skipped (warn + drop), not fatal: the server still
/// starts, the surviving valid origins are granted, and no other origin sneaks in.
#[tokio::test]
async fn cors_allowlist_skips_unparseable_entries_and_keeps_valid_ones() {
    let config = Config {
        // The middle entry contains a DEL byte, which is not a valid header value.
        cors_allowed_origins: Some(
            "https://ok.example, bad\u{7f}value, https://second.example".to_string(),
        ),
        ..test_config()
    };
    let addr = spawn(config).await;
    let client = reqwest::Client::new();
    let url = format!("http://{addr}/api/v2/does-not-exist");

    for origin in ["https://ok.example", "https://second.example"] {
        let r = client
            .get(&url)
            .header("origin", origin)
            .send()
            .await
            .unwrap();
        assert_eq!(
            header(&r, "access-control-allow-origin"),
            Some(origin),
            "valid entries around the malformed one must survive"
        );
    }

    let r = client
        .get(&url)
        .header("origin", "https://evil.example")
        .send()
        .await
        .unwrap();
    assert!(header(&r, "access-control-allow-origin").is_none());
}

/// `CORS_ALLOWED_ORIGINS` grants exactly the listed origins, nothing else.
#[tokio::test]
async fn cors_allowlist_grants_only_configured_origins() {
    let config = Config {
        cors_allowed_origins: Some("https://app.example.com".to_string()),
        ..test_config()
    };
    let addr = spawn(config).await;
    let client = reqwest::Client::new();
    let url = format!("http://{addr}/api/v2/does-not-exist");

    let r = client
        .get(&url)
        .header("origin", "https://app.example.com")
        .send()
        .await
        .unwrap();
    assert_eq!(
        header(&r, "access-control-allow-origin"),
        Some("https://app.example.com")
    );

    let r = client
        .get(&url)
        .header("origin", "https://evil.example")
        .send()
        .await
        .unwrap();
    assert!(header(&r, "access-control-allow-origin").is_none());
}
