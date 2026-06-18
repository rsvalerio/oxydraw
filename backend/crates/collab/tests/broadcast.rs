//! End-to-end delivery test for the collaboration relay, driving a real Socket.IO client
//! (`rust_socketio`) against the server.

use std::time::Duration;

use futures_util::FutureExt;
use rust_socketio::asynchronous::ClientBuilder;
use rust_socketio::Payload;
use tokio::sync::mpsc;

mod common;
use common::spawn_server;

/// Emit a single JSON-string argument (e.g. a room id), matching `Data<String>` server-side.
fn room_payload(room: &str) -> Payload {
    Payload::Text(vec![serde_json::Value::String(room.to_string())])
}

fn record(
    tx: &mpsc::UnboundedSender<String>,
    name: &'static str,
) -> impl Fn(
    Payload,
    rust_socketio::asynchronous::Client,
) -> futures_util::future::BoxFuture<'static, ()>
       + Send
       + Sync {
    let tx = tx.clone();
    move |payload: Payload, _client| {
        let tx = tx.clone();
        async move {
            let _ = tx.send(format!("{name}:{payload:?}"));
        }
        .boxed()
    }
}

/// Drain events (bounded wait per event) until one labeled with `prefix` arrives;
/// everything drained lands in `seen` for diagnostics. Returns whether it arrived.
async fn wait_for(
    rx: &mut mpsc::UnboundedReceiver<String>,
    prefix: &str,
    seen: &mut Vec<String>,
) -> bool {
    loop {
        match tokio::time::timeout(Duration::from_secs(6), rx.recv()).await {
            Ok(Some(ev)) => {
                let hit = ev.starts_with(prefix);
                seen.push(ev);
                if hit {
                    return true;
                }
            }
            _ => return false,
        }
    }
}

#[tokio::test]
async fn join_room_notifies_existing_members() {
    let url = spawn_server().await;
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let (tx_b, mut rx_b) = mpsc::unbounded_channel::<String>();
    let mut seen = Vec::new();
    let mut seen_b = Vec::new();

    // Setup ordering is event-driven, never slept on: the server emits `init-room` from
    // its connect handler, so receiving it proves the handler ran and the server-side
    // event listeners are registered before we emit `join-room`.
    let a = ClientBuilder::new(url.clone())
        .on("init-room", record(&tx, "init-room"))
        .on("first-in-room", record(&tx, "first-in-room"))
        .on("new-user", record(&tx, "new-user"))
        .on("room-user-change", record(&tx, "room-user-change"))
        .connect()
        .await
        .expect("client A connects");
    assert!(
        wait_for(&mut rx, "init-room", &mut seen).await,
        "A never received init-room: {seen:?}"
    );
    a.emit("join-room", room_payload("room1"))
        .await
        .expect("A joins room");
    // A is alone, so the server answers `first-in-room` — A's join is fully processed
    // before B enters the picture.
    assert!(
        wait_for(&mut rx, "first-in-room", &mut seen).await,
        "A never received first-in-room: {seen:?}"
    );

    let b = ClientBuilder::new(url)
        .on("init-room", record(&tx_b, "init-room"))
        .connect()
        .await
        .expect("client B connects");
    assert!(
        wait_for(&mut rx_b, "init-room", &mut seen_b).await,
        "B never received init-room: {seen_b:?}"
    );
    b.emit("join-room", room_payload("room1"))
        .await
        .expect("B joins room");

    let got_new_user = wait_for(&mut rx, "new-user", &mut seen).await;

    let _ = a.disconnect().await;
    let _ = b.disconnect().await;

    assert!(
        got_new_user,
        "client A should receive `new-user` after B joins. Events A saw: {seen:?}"
    );
}
