//! Real-time collaboration relay over Socket.IO, ported from the Go `setupSocketIO`.
//!
//! This is the open-source Excalidraw `excalidraw-room` protocol: the server only relays
//! opaque, end-to-end-encrypted payloads between members of a room and tracks presence. It
//! never sees plaintext scene data.
//!
//! Events:
//! - on connect → emit `init-room`
//! - `join-room {roomId}` → join; if first, emit `first-in-room`, else broadcast `new-user`;
//!   then emit `room-user-change [socketIds]` to the room
//! - `server-broadcast {roomId, data, iv}` → relay as `client-broadcast` to the room (not sender)
//! - `server-volatile-broadcast {roomId, data, iv}` → same. Upstream uses Socket.IO
//!   volatile emits here (cursor traffic, dropped for slow consumers); socketioxide has no
//!   volatile emission, so this is intentionally degraded to reliable delivery — a slow
//!   consumer buffers stale cursor frames, bounded by its connection's send queue and
//!   cleared on disconnect.
//! - on disconnect → recompute membership, emit `room-user-change`

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use bytes::Bytes;
use indexmap::IndexSet;
use socketioxide::extract::{Data, SocketRef, State};
use socketioxide::layer::SocketIoLayer;
use socketioxide::SocketIo;
use tracing::{debug, warn};

/// Lock `mutex`, recovering from poisoning instead of panicking: a panic in one handler
/// while it holds the guard must not turn the whole relay into a permanent panic loop.
/// (Duplicated from `oxydraw_core::sync` — this crate intentionally stays free of
/// workspace dependencies, see DUP-1/DUP-9. The poison-recovery contract is pinned to the
/// core implementation by `recovers_a_usable_guard_from_a_poisoned_mutex` below.)
fn lock_unpoisoned<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Room membership state, kept behind a single lock so the two indexes never diverge.
#[derive(Default)]
struct RoomsInner {
    /// `roomId -> socket ids`, preserving join order (Excalidraw expects an ordered array
    /// in `room-user-change`).
    members: HashMap<String, IndexSet<String>>,
    /// Reverse index `sid -> roomIds`, so disconnect cleanup is O(rooms-for-this-sid)
    /// instead of a scan over every room.
    rooms_by_sid: HashMap<String, HashSet<String>>,
}

/// Tracks which socket ids are present in each room, preserving join order (Excalidraw
/// expects an ordered array in `room-user-change`). Membership is maintained here rather
/// than via socketioxide's room introspection so it is stable across library versions.
/// Deliberately not `pub`: `build()` is the crate's whole public API, and socketioxide's
/// `State` extractor only needs the type visible to this module (ARCH-9).
#[derive(Clone, Default)]
struct Rooms(Arc<Mutex<RoomsInner>>);

/// Caps guarding the unauthenticated relay against a single connection growing the
/// registry without bound (each accepted room id is stored ~3×: members key, the sid's
/// reverse index, and socketioxide's adapter). A legitimate Excalidraw client joins one
/// room with a short id, so these reject only abuse.
const MAX_ROOM_ID_BYTES: usize = 256;
const MAX_ROOMS_PER_SOCKET: usize = 32;

impl Rooms {
    /// Register `sid` in `room` and return the post-join member snapshot (join order),
    /// or `None` when the join violates [`MAX_ROOM_ID_BYTES`] / [`MAX_ROOMS_PER_SOCKET`].
    /// Re-joining an already-joined room never counts against the cap. The snapshot is
    /// taken under the same lock acquisition as the insert, so the `room-user-change`
    /// payload built from it is exactly the membership this join produced — a separate
    /// `members()` read could interleave with a concurrent join/disconnect.
    fn try_join(&self, room: &str, sid: &str) -> Option<Vec<String>> {
        if room.len() > MAX_ROOM_ID_BYTES {
            return None;
        }
        let mut guard = lock_unpoisoned(&self.0);
        let inner = &mut *guard;
        let joined = inner.rooms_by_sid.entry(sid.to_string()).or_default();
        if joined.len() >= MAX_ROOMS_PER_SOCKET && !joined.contains(room) {
            return None;
        }
        joined.insert(room.to_string());
        let set = inner.members.entry(room.to_string()).or_default();
        set.insert(sid.to_string());
        Some(set.iter().cloned().collect())
    }

    /// Current member list (test-only introspection — production reads come from
    /// `try_join`'s atomic snapshot).
    #[cfg(test)]
    fn members(&self, room: &str) -> Vec<String> {
        lock_unpoisoned(&self.0)
            .members
            .get(room)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Remove `sid` from every room it joined; return each affected room with its remaining
    /// members. Rooms emptied by the removal are dropped from the registry entirely, so the
    /// map only ever holds active rooms (room ids are client-supplied and otherwise leak).
    fn leave_all(&self, sid: &str) -> Vec<(String, Vec<String>)> {
        let mut guard = lock_unpoisoned(&self.0);
        let Some(joined) = guard.rooms_by_sid.remove(sid) else {
            return Vec::new();
        };
        let mut affected = Vec::with_capacity(joined.len());
        for room in joined {
            if let Some(set) = guard.members.get_mut(&room) {
                set.shift_remove(sid);
                let remaining: Vec<String> = set.iter().cloned().collect();
                if remaining.is_empty() {
                    guard.members.remove(&room);
                }
                affected.push((room, remaining));
            }
        }
        affected
    }

    /// Whether `sid` has joined `room` — the broadcast path's access check.
    fn is_member(&self, room: &str, sid: &str) -> bool {
        lock_unpoisoned(&self.0)
            .rooms_by_sid
            .get(sid)
            .is_some_and(|joined| joined.contains(room))
    }

    /// Whether a room currently exists in the registry (test-only introspection).
    #[cfg(test)]
    fn contains_room(&self, room: &str) -> bool {
        lock_unpoisoned(&self.0).members.contains_key(room)
    }
}

/// Max Socket.IO payload, matching the Go backend's `SetMaxHttpBufferSize(5_000_000)`.
/// socketioxide defaults to 100 kB, which is too small for many Excalidraw scene broadcasts.
const MAX_PAYLOAD_BYTES: u64 = 5_000_000;

/// Build the Socket.IO Tower layer (mount it on the axum router) and its handle.
pub fn build() -> (SocketIoLayer, SocketIo) {
    let (layer, io) = SocketIo::builder()
        .max_payload(MAX_PAYLOAD_BYTES)
        .with_state(Rooms::default())
        .build_layer();
    io.ns("/", on_connect);
    (layer, io)
}

/// Log a failed emit with enough context (event, room, sid) to trace dropped relays —
/// silently discarding the `Result` would turn lost presence updates and broadcasts
/// into an undebuggable black box. Emit failures are rare (serialization errors, peers
/// gone mid-send), so warn-level is not noisy. The room id is client-supplied (any
/// bytes), so it is Debug-escaped — raw `Display` would let embedded newlines/ANSI
/// escapes forge log lines under the plain-text subscriber (CWE-117).
fn log_emit_failure<E: std::fmt::Display>(
    result: Result<(), E>,
    event: &str,
    room: Option<&str>,
    sid: impl std::fmt::Display,
) {
    if let Err(e) = result {
        warn!(sid = %sid, ?room, event, error = %e, "failed to emit");
    }
}

async fn on_connect(socket: SocketRef) {
    debug!(sid = %socket.id, "socket connected");
    log_emit_failure(socket.emit("init-room", &()), "init-room", None, socket.id);

    socket.on("join-room", on_join_room);
    socket.on("server-broadcast", on_server_broadcast);
    // Reliable on purpose: socketioxide has no volatile emit (see the module doc).
    socket.on("server-volatile-broadcast", on_server_broadcast);
    socket.on_disconnect(on_disconnect);
}

async fn on_join_room(socket: SocketRef, Data(room): Data<String>, State(rooms): State<Rooms>) {
    let sid = socket.id.to_string();
    // Enforce the caps before touching socketioxide's adapter state, so a rejected join
    // leaves no trace anywhere.
    let Some(members) = rooms.try_join(&room, &sid) else {
        warn!(
            %sid,
            room_id_len = room.len(),
            "join-room rejected: room-id length or rooms-per-socket cap exceeded"
        );
        return;
    };
    // SEC-38: socketioxide (0.18) spawns event handlers and the disconnect callback as
    // independent tasks with no ordering guarantee, so `on_disconnect`'s `leave_all` can
    // run *before* this handler reaches `try_join`, stranding a dead sid in the registry
    // forever. `Socket::close()` flips `connected` to false before spawning the
    // disconnect handler, so a single post-insert check closes the race: if the flag is
    // false here, cleanup either already ran (and missed the entry just inserted) or is
    // about to run (and will remove it too) — rolling back is correct and idempotent in
    // both interleavings.
    if !socket.connected() {
        rooms.leave_all(&sid);
        debug!(%sid, "join-room raced disconnect: rolled back registration");
        return;
    }
    socket.join(room.clone());
    // `?room`, not `%room`: the id is client-supplied — see `log_emit_failure`.
    debug!(%sid, ?room, count = members.len(), "join-room");

    if members.len() <= 1 {
        log_emit_failure(
            socket.emit("first-in-room", &()),
            "first-in-room",
            Some(&room),
            &sid,
        );
    } else {
        log_emit_failure(
            socket.to(room.clone()).emit("new-user", &sid).await,
            "new-user",
            Some(&room),
            &sid,
        );
    }

    // `members` is the snapshot taken atomically with this join's insert. Residual
    // caveat: the lock is long released by this await, so when joins/leaves race, the
    // *delivery order* of their `room-user-change` emissions is still unsynchronized —
    // each payload is internally consistent, but clients may apply them out of order
    // until the next change.
    log_emit_failure(
        socket
            .within(room.clone())
            .emit("room-user-change", &members)
            .await,
        "room-user-change",
        Some(&room),
        &sid,
    );
}

/// Relay an end-to-end-encrypted update to the rest of the room. The Excalidraw client emits
/// `(roomId, encryptedBuffer, iv)` where the latter two are **binary** (`ArrayBuffer` /
/// `Uint8Array`), so they must be typed as [`Bytes`] — `serde_json::Value` would silently
/// capture the binary-attachment placeholder instead of the bytes. We forward `data` and `iv`
/// verbatim as `client-broadcast`'s two arguments.
async fn on_server_broadcast(
    socket: SocketRef,
    Data((room, data, iv)): Data<(String, Bytes, Bytes)>,
    State(rooms): State<Rooms>,
) {
    let sid = socket.id.to_string();
    // The relay's only access control is knowledge of the room id, so a sender must have
    // gone through `join-room` (and its caps) before it may inject frames into a room —
    // otherwise any connected socket could spam arbitrary room ids while bypassing
    // MAX_ROOM_ID_BYTES / MAX_ROOMS_PER_SOCKET. Length first: it rejects without a lock,
    // and the registry can never contain an over-long id anyway.
    if room.len() > MAX_ROOM_ID_BYTES || !rooms.is_member(&room, &sid) {
        warn!(
            %sid,
            room_id_len = room.len(),
            "server-broadcast rejected: sender has not joined the room"
        );
        return;
    }
    log_emit_failure(
        socket
            .to(room.clone())
            .emit("client-broadcast", &(data, iv))
            .await,
        "client-broadcast",
        Some(&room),
        socket.id,
    );
}

async fn on_disconnect(socket: SocketRef, State(rooms): State<Rooms>) {
    let sid = socket.id.to_string();
    debug!(%sid, "socket disconnecting");
    for (room, members) in rooms.leave_all(&sid) {
        log_emit_failure(
            socket
                .to(room.clone())
                .emit("room-user-change", &members)
                .await,
            "room-user-change",
            Some(&room),
            &sid,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Pins this crate's duplicated `lock_unpoisoned` to the same poison-recovery contract as
    // `oxydraw_core::sync::lock_unpoisoned` (DUP-1): a poisoned mutex must still yield a usable
    // guard rather than panicking. If the two ever drift, one of these tests fails.
    #[test]
    fn recovers_a_usable_guard_from_a_poisoned_mutex() {
        let mutex = Mutex::new(7);
        // Poison the mutex: panic while holding the guard.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = mutex.lock().unwrap();
            panic!("poison the mutex");
        }));
        assert!(result.is_err());
        assert!(mutex.is_poisoned());
        assert_eq!(*lock_unpoisoned(&mutex), 7, "guard still usable");
    }

    #[test]
    fn room_membership_tracks_join_order_and_leave() {
        let rooms = Rooms::default();
        assert_eq!(rooms.try_join("r1", "a"), Some(vec!["a".to_string()]));
        assert_eq!(
            rooms.try_join("r1", "b"),
            Some(vec!["a".to_string(), "b".to_string()])
        );
        // Idempotent re-join: same snapshot, join order preserved.
        assert_eq!(
            rooms.try_join("r1", "a"),
            Some(vec!["a".to_string(), "b".to_string()])
        );
        assert_eq!(rooms.members("r1"), vec!["a".to_string(), "b".to_string()]);

        let affected = rooms.leave_all("a");
        assert_eq!(affected, vec![("r1".to_string(), vec!["b".to_string()])]);
        assert_eq!(rooms.members("r1"), vec!["b".to_string()]);
    }

    #[test]
    fn emptied_room_is_dropped_from_registry() {
        let rooms = Rooms::default();
        assert!(rooms.try_join("r1", "a").is_some());
        assert!(rooms.try_join("r2", "a").is_some());
        assert!(rooms.try_join("r2", "b").is_some());

        let mut affected = rooms.leave_all("a");
        affected.sort();
        assert_eq!(
            affected,
            vec![
                ("r1".to_string(), vec![]),
                ("r2".to_string(), vec!["b".to_string()]),
            ]
        );
        // r1 was emptied → its entry is removed, not retained as an empty set.
        assert!(!rooms.contains_room("r1"));
        assert!(rooms.contains_room("r2"));

        rooms.leave_all("b");
        assert!(!rooms.contains_room("r2"));
    }

    /// Run `body` under a tracing subscriber whose formatted output is captured, and
    /// return everything it logged. Shared by the log-assertion tests (DUP-1).
    fn capture_logs(body: impl FnOnce()) -> String {
        use std::io::Write;
        use tracing_subscriber::fmt::MakeWriter;

        #[derive(Clone, Default)]
        struct Capture(Arc<Mutex<Vec<u8>>>);

        impl Write for Capture {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.0.lock().expect("capture lock poisoned").write(buf)
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        impl<'a> MakeWriter<'a> for Capture {
            type Writer = Capture;
            fn make_writer(&'a self) -> Capture {
                self.clone()
            }
        }

        let capture = Capture::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(capture.clone())
            .with_ansi(false)
            .finish();
        tracing::subscriber::with_default(subscriber, body);
        let bytes = capture.0.lock().expect("capture lock poisoned").clone();
        String::from_utf8(bytes).expect("log output is utf-8")
    }

    #[test]
    fn failed_emit_is_logged_with_context() {
        let out = capture_logs(|| {
            log_emit_failure(
                Err::<(), _>("socket closed"),
                "client-broadcast",
                Some("room-1"),
                "sid-1",
            );
            log_emit_failure(
                Ok::<(), &str>(()),
                "client-broadcast",
                Some("room-1"),
                "sid-1",
            );
        });
        assert_eq!(
            out.matches("failed to emit").count(),
            1,
            "Err logs once, Ok logs nothing: {out}"
        );
        for context in ["client-broadcast", "room-1", "sid-1", "socket closed"] {
            assert!(out.contains(context), "log line carries {context}: {out}");
        }
    }

    /// SEC-11 / CWE-117: a client-supplied room id containing a newline must not be able
    /// to forge a second log line — the id is Debug-escaped, never `Display`ed raw.
    #[test]
    fn room_id_with_newline_cannot_forge_a_log_line() {
        let out = capture_logs(|| {
            log_emit_failure(
                Err::<(), _>("socket closed"),
                "client-broadcast",
                Some("room-1\nFORGED admin login ok"),
                "sid-1",
            );
        });
        assert_eq!(
            out.trim_end().lines().count(),
            1,
            "the embedded newline must stay escaped inside one log line: {out}"
        );
        assert!(
            out.contains(r"room-1\nFORGED"),
            "the room id is Debug-escaped: {out}"
        );
    }

    #[test]
    fn join_rejects_oversized_room_ids() {
        let rooms = Rooms::default();
        let long = "x".repeat(MAX_ROOM_ID_BYTES + 1);
        assert_eq!(rooms.try_join(&long, "a"), None);
        assert!(
            !rooms.contains_room(&long),
            "rejected join must not register"
        );
        // At the bound the id is still accepted.
        let max = "x".repeat(MAX_ROOM_ID_BYTES);
        assert_eq!(rooms.try_join(&max, "a"), Some(vec!["a".to_string()]));
    }

    #[test]
    fn join_caps_rooms_per_socket() {
        let rooms = Rooms::default();
        for i in 0..MAX_ROOMS_PER_SOCKET {
            assert_eq!(
                rooms.try_join(&format!("r{i}"), "a"),
                Some(vec!["a".to_string()])
            );
        }
        assert_eq!(rooms.try_join("overflow", "a"), None);
        assert!(
            !rooms.contains_room("overflow"),
            "join past the cap must not grow the registry"
        );
        // Re-joining an already-joined room is still fine at the cap, and another
        // socket is unaffected by this one's usage.
        assert_eq!(rooms.try_join("r0", "a"), Some(vec!["a".to_string()]));
        assert_eq!(rooms.try_join("overflow", "b"), Some(vec!["b".to_string()]));
    }

    /// SEC-38: the join-racing-disconnect interleaving — disconnect cleanup runs first,
    /// the in-flight join registers a dead sid, and the handler's post-insert
    /// `connected()` check rolls it back — must leave no residual registry entry.
    #[test]
    fn join_racing_disconnect_rolls_back_cleanly() {
        let rooms = Rooms::default();
        // on_disconnect's leave_all ran before the join handler got to try_join.
        assert!(rooms.leave_all("a").is_empty());
        // The late join then registers the already-dead sid …
        assert_eq!(rooms.try_join("r1", "a"), Some(vec!["a".to_string()]));
        // … and the handler, observing connected() == false, rolls back.
        assert_eq!(rooms.leave_all("a").len(), 1);
        assert!(
            !rooms.contains_room("r1"),
            "rolled-back join must not strand a registry entry"
        );
    }

    #[test]
    fn leave_all_is_scoped_to_joined_rooms() {
        let rooms = Rooms::default();
        assert!(rooms.try_join("r1", "a").is_some());
        // A sid that never joined anything touches nothing.
        assert!(rooms.leave_all("ghost").is_empty());
        assert!(rooms.contains_room("r1"));
        // Leaving twice is a no-op the second time.
        assert_eq!(rooms.leave_all("a").len(), 1);
        assert!(rooms.leave_all("a").is_empty());
    }
}
