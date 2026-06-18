//! Framework smoke test, kept on purpose: it does not guard our room logic (that's
//! `broadcast.rs`) but proves `build()` mounts socketioxide under axum speaking the
//! Engine.IO protocol an Excalidraw client expects — the cheap canary for socketioxide
//! upgrades breaking the mount or the handshake. The one project-specific behavior it
//! asserts is the negotiated `maxPayload`, which `build()` raises to 5 MB.

use std::time::Duration;

mod common;
use common::spawn_server;

#[tokio::test]
async fn engineio_handshake_responds_with_sid() {
    let base = spawn_server().await;

    // Engine.IO v4 polling handshake: the first GET returns the open packet, which carries
    // the session id ("sid") and negotiated parameters.
    let url = format!("{base}/socket.io/?EIO=4&transport=polling");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    let resp = client.get(&url).send().await.unwrap();
    assert!(
        resp.status().is_success(),
        "handshake HTTP status: {}",
        resp.status()
    );
    let body = resp.text().await.unwrap();
    assert!(
        body.contains("\"sid\""),
        "expected an Engine.IO open packet containing a sid, got: {body}"
    );
    // Project configuration, not framework default (100 kB): build() raises the payload
    // ceiling to 5 MB so large scene broadcasts fit.
    assert!(
        body.contains("\"maxPayload\":5000000"),
        "expected the 5 MB maxPayload configured by build(), got: {body}"
    );
}
