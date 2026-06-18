//! Clean shared-scene snapshot API for collaboration persistence, replacing the Firestore-REST
//! emulator.
//!
//! Stores one opaque, end-to-end-encrypted snapshot per room so a client joining an otherwise
//! empty room can restore the last state. While peers are connected they re-broadcast the scene
//! to each other directly (see the relay in `oxydraw-collab`), so this snapshot only matters for
//! the first joiner after a room empties. State is in-process and bounded (SEC-33): a live room
//! re-saves on change, so an evicted or restart-lost snapshot self-heals on the next save.

use std::sync::{Arc, RwLock};

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use oxydraw_core::sync::{read_unpoisoned, write_unpoisoned};
use tracing::warn;

use crate::bounded_map::BoundedMap;
use crate::AppState;

/// Bounds for the in-memory snapshot store. Unauthenticated, so a ceiling stops unbounded
/// growth (SEC-33); LRU eviction is safe because live rooms re-save on change.
const SCENE_MAX_ENTRIES: usize = 1024;
const SCENE_MAX_BYTES: usize = 256 * 1024 * 1024;

/// SEC-33: upper bound on a room id. Legitimate ids are short; the bound only rejects
/// pathological input that would otherwise mint unbounded map keys.
const MAX_ROOM_ID_BYTES: usize = 256;

/// In-memory per-room scene snapshots, bounded and keyed by room id.
#[derive(Clone)]
pub struct SceneSnapshots(Arc<RwLock<BoundedMap<Bytes>>>);

impl Default for SceneSnapshots {
    fn default() -> Self {
        Self(Arc::new(RwLock::new(BoundedMap::new(
            SCENE_MAX_ENTRIES,
            SCENE_MAX_BYTES,
        ))))
    }
}

impl SceneSnapshots {
    fn put(&self, room: String, data: Bytes) {
        let bytes = data.len();
        write_unpoisoned(&self.0).insert(room, data, bytes);
    }

    fn get(&self, room: &str) -> Option<Bytes> {
        // Shared read lock: concurrent late-joiner snapshot loads don't serialize (CONC-7).
        read_unpoisoned(&self.0).get(room).cloned()
    }
}

/// `PUT /api/rooms/{room_id}/scene` — store the room's latest opaque encrypted snapshot. The
/// body size is bounded by the router's global request-body limit; the bounded map evicts past
/// its caps. Always overwrites (last save wins).
pub async fn put_scene(
    State(state): State<AppState>,
    Path(room): Path<String>,
    body: Bytes,
) -> Response {
    if room.len() > MAX_ROOM_ID_BYTES {
        warn!(room_id_len = room.len(), "scene put: room id too long");
        return (StatusCode::BAD_REQUEST, "room id too long").into_response();
    }
    state.scenes.put(room, body);
    StatusCode::NO_CONTENT.into_response()
}

/// `GET /api/rooms/{room_id}/scene` — return the room's snapshot, or 404 when none is stored
/// (an empty room with no prior state).
pub async fn get_scene(State(state): State<AppState>, Path(room): Path<String>) -> Response {
    match state.scenes.get(&room) {
        Some(data) => ([(header::CONTENT_TYPE, "application/octet-stream")], data).into_response(),
        None => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}
