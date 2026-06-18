// Collaboration wire protocol shared between peers.
//
// The OxyDraw Rust relay (backend/crates/collab) is protocol-agnostic: it forwards opaque encrypted
// frames and tracks room membership. These are the *decrypted* payload shapes our own client
// puts on the wire — the same `SCENE_*` / `MOUSE_LOCATION` scheme upstream Excalidraw uses, so
// the relay's event names (`join-room`, `server-broadcast`, `client-broadcast`,
// `room-user-change`, `new-user`, `first-in-room`, `init-room`) line up unchanged.

import type { OrderedExcalidrawElement } from "@excalidraw/excalidraw/element/types";
import type { AppState, SocketId } from "@excalidraw/excalidraw/types";

import { parseFragment } from "../fragment";

export type CollabPointer = { x: number; y: number; tool: "pointer" | "laser" };

/** A decrypted broadcast frame. */
export type BroadcastPayload =
  | {
      type: "SCENE_INIT" | "SCENE_UPDATE";
      payload: { elements: readonly OrderedExcalidrawElement[] };
    }
  | {
      type: "MOUSE_LOCATION";
      payload: {
        socketId: SocketId;
        pointer: CollabPointer;
        button: "up" | "down";
        selectedElementIds: AppState["selectedElementIds"];
        username: string;
      };
    };

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

/**
 * Validate a decrypted broadcast frame's shape before it is applied to scene/presence state.
 * Frames are AES-GCM authenticated, so a sender already holds the room key — but the room link is
 * the only capability, so any peer is implicitly trusted. A buggy or hostile peer can still send a
 * frame whose `type` is valid but whose `payload` is malformed; reject those instead of writing
 * them into the collaborators map or scene. `SCENE_*` element *contents* are sanitized downstream by
 * `restoreElements`, so we only assert `elements` is an array here.
 */
export function parseBroadcastPayload(value: unknown): BroadcastPayload | null {
  if (!isRecord(value) || !isRecord(value.payload)) {
    return null;
  }
  const { payload } = value;
  switch (value.type) {
    case "SCENE_INIT":
    case "SCENE_UPDATE":
      if (!Array.isArray(payload.elements)) {
        return null;
      }
      return {
        type: value.type,
        payload: { elements: payload.elements as readonly OrderedExcalidrawElement[] },
      };
    case "MOUSE_LOCATION":
      if (
        typeof payload.socketId !== "string" ||
        typeof payload.username !== "string" ||
        (payload.button !== "up" && payload.button !== "down") ||
        !isPointer(payload.pointer) ||
        !isRecord(payload.selectedElementIds)
      ) {
        return null;
      }
      return {
        type: "MOUSE_LOCATION",
        payload: {
          socketId: payload.socketId as SocketId,
          pointer: payload.pointer,
          button: payload.button,
          selectedElementIds: payload.selectedElementIds as AppState["selectedElementIds"],
          username: payload.username,
        },
      };
    default:
      return null;
  }
}

function isPointer(value: unknown): value is CollabPointer {
  return (
    isRecord(value) &&
    typeof value.x === "number" &&
    typeof value.y === "number" &&
    (value.tool === "pointer" || value.tool === "laser")
  );
}

const ROOM_PREFIX = "#room=";

/** Parse `#room=<roomId>,<key>` from the current location, or `null` if absent/malformed. */
export function parseRoomFragment(
  hash: string = window.location.hash,
): { roomId: string; key: string } | null {
  const parts = parseFragment(ROOM_PREFIX, hash);
  return parts ? { roomId: parts[0], key: parts[1] } : null;
}

/** Build the fragment body (without the leading `#`) for a room URL. */
export function buildRoomFragment(roomId: string, key: string): string {
  return `room=${roomId},${key}`;
}
