//! Binary round-trip test for the crate's core path: `server-broadcast {roomId, data, iv}`
//! relayed to room peers as `client-broadcast`, byte-identical and never echoed to the
//! sender.
//!
//! `rust_socketio` cannot emit a mixed (text, binary, binary) argument list, so this test
//! speaks Engine.IO v4 long-polling directly: binary attachments travel as base64
//! `b`-prefixed packets, exactly as the Excalidraw client's polling fallback sends them.
//! That also guards the regression the handler documents — `data`/`iv` must be extracted
//! as real bytes, not the binary-attachment placeholder.

use std::time::Duration;

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;

mod common;
use common::spawn_server;

/// Engine.IO v4 payload framing: packets in one HTTP body are separated by U+001E.
const RECORD_SEPARATOR: char = '\u{1e}';

/// A minimal Engine.IO v4 long-polling Socket.IO client.
struct PollingClient {
    http: reqwest::Client,
    /// Session URL (`…?EIO=4&transport=polling&sid=<sid>`).
    url: String,
    /// The Socket.IO session id (from the namespace-connect ack) — the id the server
    /// uses in `room-user-change` member arrays.
    sio_sid: String,
}

impl PollingClient {
    /// Engine.IO open handshake + Socket.IO namespace connect.
    async fn connect(base: &str) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(6))
            .build()
            .unwrap();
        let body = http
            .get(format!("{base}/socket.io/?EIO=4&transport=polling"))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        // The open packet is `0{json}`; its sid keys every subsequent request.
        let open: serde_json::Value =
            serde_json::from_str(body.strip_prefix('0').expect("engine.io open packet"))
                .expect("open packet payload is JSON");
        let sid = open["sid"].as_str().expect("open packet carries a sid");
        let mut client = Self {
            http,
            url: format!("{base}/socket.io/?EIO=4&transport=polling&sid={sid}"),
            sio_sid: String::new(),
        };
        client.post("40").await; // Socket.IO connect to the default namespace
        let packets = client
            .wait_for(|p| p.starts_with("40"))
            .await
            .expect_match("socket.io connect ack");
        let ack = packets
            .iter()
            .find(|p| p.starts_with("40"))
            .expect("ack packet present");
        let ack: serde_json::Value =
            serde_json::from_str(&ack[2..]).expect("connect ack payload is JSON");
        client.sio_sid = ack["sid"]
            .as_str()
            .expect("connect ack carries the socket.io sid")
            .to_string();
        client
    }

    async fn post(&self, payload: &str) {
        let resp = self
            .http
            .post(&self.url)
            .body(payload.to_string())
            .send()
            .await
            .unwrap();
        assert!(resp.status().is_success(), "POST failed: {}", resp.status());
    }

    /// Long-poll until a packet matching `pred` arrives, for at most the bounded
    /// retries. The outcome distinguishes "never showed up" from "the transport broke"
    /// so negative-delivery proofs cannot pass vacuously when the server dies.
    async fn wait_for(&self, pred: impl Fn(&str) -> bool) -> PollOutcome {
        for _ in 0..10 {
            let resp = match self.http.get(&self.url).send().await {
                Ok(resp) => resp,
                // The 6s client timeout firing on an idle long-poll (the server's ping
                // interval is far longer) is the legitimate "no traffic" outcome the
                // bounded wait exists for; any other failure means the transport broke.
                Err(e) if e.is_timeout() => return PollOutcome::NoMatch,
                Err(e) => return PollOutcome::TransportError(e),
            };
            let body = match resp.text().await {
                Ok(body) => body,
                Err(e) if e.is_timeout() => return PollOutcome::NoMatch,
                Err(e) => return PollOutcome::TransportError(e),
            };
            let packets: Vec<String> = body.split(RECORD_SEPARATOR).map(str::to_string).collect();
            if packets.iter().any(|p| pred(p)) {
                return PollOutcome::Matched(packets);
            }
        }
        PollOutcome::NoMatch
    }
}

/// Outcome of one bounded [`PollingClient::wait_for`], so the negative-delivery proofs
/// can tell "the packet never arrived" (what they assert) from "the poll transport
/// failed" (a dead server proves nothing about dropped frames — TEST-11).
enum PollOutcome {
    /// A matching packet arrived; holds every packet of the payload that contained it
    /// (binary attachments follow their event packet).
    Matched(Vec<String>),
    /// The bounded polls drained (or the idle long-poll timed out) without a match.
    NoMatch,
    /// A poll request failed for a reason other than the idle timeout.
    TransportError(reqwest::Error),
}

impl PollOutcome {
    /// Positive-wait accessor: the matched payload, or a panic carrying `msg` — both a
    /// missing packet and a broken transport fail the test here.
    #[track_caller]
    fn expect_match(self, msg: &str) -> Vec<String> {
        match self {
            Self::Matched(packets) => packets,
            Self::NoMatch => panic!("{msg}: no matching packet within the bounded polls"),
            Self::TransportError(e) => panic!("{msg}: poll transport failed: {e}"),
        }
    }

    /// Negative-delivery accessor: passes only on a genuine no-match drain. A transport
    /// error fails the test instead of being misread as "frame correctly dropped".
    #[track_caller]
    fn expect_no_match(self, msg: &str) {
        match self {
            Self::NoMatch => {}
            Self::Matched(packets) => panic!("{msg}: unexpectedly matched {packets:?}"),
            Self::TransportError(e) => panic!("{msg}: poll transport failed: {e}"),
        }
    }
}

/// A `server-broadcast` emit with two binary attachments, in Engine.IO polling framing.
fn binary_broadcast(room: &str, data: &[u8], iv: &[u8]) -> String {
    format!(
        "452-[\"server-broadcast\",\"{room}\",\
         {{\"_placeholder\":true,\"num\":0}},{{\"_placeholder\":true,\"num\":1}}]\
         {sep}b{data}{sep}b{iv}",
        sep = RECORD_SEPARATOR,
        data = B64.encode(data),
        iv = B64.encode(iv),
    )
}

/// Pull the two base64 attachments following the `client-broadcast` event packet.
fn broadcast_attachments(packets: &[String]) -> (Vec<u8>, Vec<u8>) {
    let idx = packets
        .iter()
        .position(|p| p.contains("client-broadcast"))
        .expect("payload holds a client-broadcast event");
    assert!(
        packets[idx].starts_with("452-"),
        "client-broadcast carries two binary attachments: {}",
        packets[idx]
    );
    let att: Vec<&String> = packets[idx + 1..]
        .iter()
        .filter(|p| p.starts_with('b'))
        .take(2)
        .collect();
    assert_eq!(att.len(), 2, "attachments follow the event: {packets:?}");
    (
        B64.decode(&att[0][1..]).expect("data attachment is base64"),
        B64.decode(&att[1][1..]).expect("iv attachment is base64"),
    )
}

/// TEST-6: a member dropping its connection must push an updated `room-user-change` —
/// without the departed sid — to the remaining members. This is the presence feature
/// Excalidraw relies on to drop departed collaborators, and the handler wiring
/// (`on_disconnect` registration, emit-after-leave) is invisible to the unit tests.
#[tokio::test]
async fn disconnect_pushes_updated_member_list_to_remaining_members() {
    let url = spawn_server().await;
    let stayer = PollingClient::connect(&url).await;
    let leaver = PollingClient::connect(&url).await;

    stayer.post(r#"42["join-room","presence-room"]"#).await;
    stayer
        .wait_for(|p| p.contains("first-in-room"))
        .await
        .expect_match("stayer is first in the room");
    leaver.post(r#"42["join-room","presence-room"]"#).await;
    // The stayer sees the two-member roster before the leaver departs.
    stayer
        .wait_for(|p| p.contains("room-user-change") && p.contains(&leaver.sio_sid))
        .await
        .expect_match("stayer sees the leaver join");

    // Engine.IO close packet: the leaver drops its connection.
    leaver.post("1").await;

    let packets = stayer
        .wait_for(|p| p.contains("room-user-change") && !p.contains(&leaver.sio_sid))
        .await
        .expect_match("stayer receives a roster without the departed sid");
    let roster = packets
        .iter()
        .find(|p| p.contains("room-user-change") && !p.contains(&leaver.sio_sid))
        .expect("matched packet present");
    assert!(
        roster.contains(&stayer.sio_sid),
        "the remaining member is still in the roster: {roster}"
    );
}

/// SEC-18: a socket that never joined a room must not be able to inject
/// `client-broadcast` frames into it — the membership check happens on the broadcast
/// path, not just at join time.
#[tokio::test]
async fn server_broadcast_from_non_member_is_dropped() {
    let url = spawn_server().await;
    let member_a = PollingClient::connect(&url).await;
    let member_b = PollingClient::connect(&url).await;
    let outsider = PollingClient::connect(&url).await;

    member_a.post(r#"42["join-room","guarded-room"]"#).await;
    member_a
        .wait_for(|p| p.contains("first-in-room"))
        .await
        .expect_match("A is first in the room");
    member_b.post(r#"42["join-room","guarded-room"]"#).await;
    member_b
        .wait_for(|p| p.contains("room-user-change"))
        .await
        .expect_match("B joined the room");

    // The outsider connects but never joins, then tries to broadcast into the room.
    let garbage = vec![0xAAu8; 8];
    let garbage_iv = vec![0xBBu8; 4];
    outsider
        .post(&binary_broadcast("guarded-room", &garbage, &garbage_iv))
        .await;

    // Event-driven non-delivery proof: B then sends a sentinel. Server-side the
    // outsider's frame was handled first, so if it had been relayed, A would see it
    // before (or alongside) the sentinel — A's first client-broadcast must be B's.
    let sentinel = vec![1u8; 4];
    let sentinel_iv = vec![2u8; 3];
    member_b
        .post(&binary_broadcast("guarded-room", &sentinel, &sentinel_iv))
        .await;
    let packets = member_a
        .wait_for(|p| p.contains("client-broadcast"))
        .await
        .expect_match("A receives B's sentinel broadcast");
    let (got_data, got_iv) = broadcast_attachments(&packets);
    assert_eq!(
        got_data, sentinel,
        "A's first client-broadcast is B's sentinel — the outsider's frame was dropped"
    );
    assert_eq!(got_iv, sentinel_iv);

    // Close the false-pass window (TEST-21): socketioxide runs event handlers as tasks, so
    // the sentinel arriving first doesn't by itself prove the outsider's frame won't be
    // relayed late if handler scheduling inverts. Drain one more bounded poll — no further
    // client-broadcast may reach A, and a transport failure fails the test rather than
    // passing as "dropped".
    member_a
        .wait_for(|p| p.contains("client-broadcast"))
        .await
        .expect_no_match(
            "no further client-broadcast reaches A after the sentinel — the outsider frame stayed dropped",
        );
}

#[tokio::test]
async fn server_broadcast_relays_binary_payload_to_peers_only() {
    let url = spawn_server().await;
    let a = PollingClient::connect(&url).await;
    let b = PollingClient::connect(&url).await;

    a.post(r#"42["join-room","relay-room"]"#).await;
    a.wait_for(|p| p.contains("first-in-room"))
        .await
        .expect_match("A is first in the room");
    b.post(r#"42["join-room","relay-room"]"#).await;
    b.wait_for(|p| p.contains("room-user-change"))
        .await
        .expect_match("B joined the room");

    // Every byte value, so a placeholder/UTF-8 mangling regression cannot slip through.
    let data: Vec<u8> = (0u8..=255).collect();
    let iv: Vec<u8> = (0u8..16).collect();
    a.post(&binary_broadcast("relay-room", &data, &iv)).await;

    let packets = b
        .wait_for(|p| p.contains("client-broadcast"))
        .await
        .expect_match("B receives the relayed client-broadcast");
    let (got_data, got_iv) = broadcast_attachments(&packets);
    assert_eq!(got_data, data, "data relayed byte-identical");
    assert_eq!(got_iv, iv, "iv relayed byte-identical");

    // Sender non-delivery, event-driven: B answers with a distinct sentinel broadcast.
    // Per-socket delivery is ordered, so if A's own broadcast had been echoed back it
    // would arrive before the sentinel — A's *first* client-broadcast must be B's.
    let sentinel_data = vec![9u8; 4];
    let sentinel_iv = vec![7u8; 3];
    b.post(&binary_broadcast(
        "relay-room",
        &sentinel_data,
        &sentinel_iv,
    ))
    .await;
    let packets = a
        .wait_for(|p| p.contains("client-broadcast"))
        .await
        .expect_match("A receives B's broadcast");
    let (got_data, got_iv) = broadcast_attachments(&packets);
    assert_eq!(
        got_data, sentinel_data,
        "A's first client-broadcast is B's sentinel, not its own echo"
    );
    assert_eq!(got_iv, sentinel_iv);

    // Close the false-pass window (TEST-21): handlers run as tasks, so the sentinel
    // arriving first doesn't prove A's own broadcast won't be echoed back late. Drain one
    // more bounded poll — no further client-broadcast may reach A, and a transport
    // failure fails the test rather than passing as "not echoed".
    a.wait_for(|p| p.contains("client-broadcast"))
        .await
        .expect_no_match(
        "no further client-broadcast reaches A after the sentinel — A's own frame was not echoed",
    );
}
