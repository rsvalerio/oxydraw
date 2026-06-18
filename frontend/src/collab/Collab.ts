// Real-time collaboration client.
//
// Speaks the OxyDraw relay's Socket.IO protocol (unchanged from upstream excalidraw-room) and
// keeps the canvas in sync end-to-end-encrypted: the room AES key lives only in the URL
// fragment, so the relay and the snapshot store see opaque bytes. Remote element updates are
// merged with the package's `reconcileElements`; presence/cursors flow through the editor's
// `collaborators` map.

import { io, type Socket } from "socket.io-client";
import {
  CaptureUpdateAction,
  getSceneVersion,
  reconcileElements,
  restoreElements,
} from "@excalidraw/excalidraw";
import type { OrderedExcalidrawElement } from "@excalidraw/excalidraw/element/types";
import type { RemoteExcalidrawElement } from "@excalidraw/excalidraw/data/reconcile";
import type {
  BinaryFileData,
  Collaborator,
  ExcalidrawImperativeAPI,
  SocketId,
} from "@excalidraw/excalidraw/types";

import { fetchFile, uploadFile } from "../files";
import { fetchWithTimeout } from "../http";

import {
  decrypt,
  decryptParts,
  encrypt,
  encryptParts,
  exportKey,
  generateKey,
  importKey,
} from "../crypto";
import { SCENE_URL, WS_URL } from "../config";
import { buildRoomFragment, parseBroadcastPayload, type BroadcastPayload } from "./protocol";

/** Throttle for outgoing cursor frames (~30fps), matching upstream. */
const CURSOR_SYNC_MS = 33;
/** Debounce for persisting the room snapshot to the backend. */
const SAVE_DEBOUNCE_MS = 1000;

const COLLAB_COLORS = [
  "#e64980",
  "#fab005",
  "#15aabf",
  "#7048e8",
  "#f76707",
  "#37b24d",
  "#1c7ed6",
  "#f03e3e",
];

export interface CollabCallbacks {
  /** Called when a room becomes active, with the shareable room URL. */
  onStarted: (roomUrl: string) => void;
  /** Called when collaboration stops (user ends it or the socket closes for good). */
  onStopped: () => void;
}

export class Collab {
  private readonly api: ExcalidrawImperativeAPI;
  private readonly callbacks: CollabCallbacks;
  private socket: Socket | null = null;
  private roomId = "";
  private key: CryptoKey | null = null;
  private collaborators = new Map<SocketId, Collaborator>();
  /** Ids of files already pushed to (or pulled from) the backend, so each transfers once. */
  private readonly knownFiles = new Set<string>();
  private readonly fetchingFiles = new Set<string>();
  private readonly username = `User-${Math.floor(Math.random() * 9000 + 1000)}`;
  private lastBroadcastVersion = -1;
  private lastCursorSentAt = 0;
  private saveTimer: ReturnType<typeof setTimeout> | undefined;

  constructor(api: ExcalidrawImperativeAPI, callbacks: CollabCallbacks) {
    this.api = api;
    this.callbacks = callbacks;
  }

  get isActive(): boolean {
    return this.socket !== null;
  }

  /** Create a fresh room from the current scene and connect. */
  async startNewRoom(): Promise<void> {
    const roomId = randomRoomId();
    const key = await generateKey();
    await this.connect(roomId, key);
    // Persist the current scene immediately so the room has state before peers arrive.
    await this.saveSnapshot();
    const url = new URL(window.location.href);
    url.hash = buildRoomFragment(roomId, await exportKey(key));
    window.history.replaceState(null, "", url.toString());
    this.callbacks.onStarted(url.toString());
  }

  /** Join an existing room referenced by a share fragment. */
  async joinRoom(roomId: string, encodedKey: string): Promise<void> {
    const key = await importKey(encodedKey);
    await this.connect(roomId, key);
    this.callbacks.onStarted(window.location.href);
  }

  stop(): void {
    clearTimeout(this.saveTimer);
    this.socket?.disconnect();
    this.socket = null;
    this.key = null;
    this.collaborators.clear();
    this.api.updateScene({ collaborators: new Map() });
    this.callbacks.onStopped();
  }

  private async connect(roomId: string, key: CryptoKey): Promise<void> {
    this.roomId = roomId;
    this.key = key;
    // Empty WS_URL → same origin (the OxyDraw binary's built-in relay), default /socket.io path.
    const socket = WS_URL ? io(WS_URL) : io();
    this.socket = socket;

    socket.on("init-room", () => {
      socket.emit("join-room", this.roomId);
    });
    socket.on("first-in-room", () => {
      // Nobody else is here: restore the last persisted snapshot, if any.
      void this.loadSnapshot();
    });
    socket.on("new-user", () => {
      // A peer joined: hand them the full current scene.
      void this.broadcastScene("SCENE_INIT");
    });
    socket.on("room-user-change", (sids: string[]) => {
      this.updatePresence(sids);
    });
    socket.on("client-broadcast", (data: ArrayBuffer, iv: ArrayBuffer) => {
      void this.onClientBroadcast(data, iv);
    });
  }

  /** Forward a local scene change: broadcast a delta and schedule a snapshot save. */
  onLocalChange(elements: readonly OrderedExcalidrawElement[]): void {
    if (!this.isActive) {
      return;
    }
    const version = getSceneVersion(elements);
    if (version !== this.lastBroadcastVersion) {
      this.lastBroadcastVersion = version;
      void this.syncLocalFiles();
      void this.broadcast({ type: "SCENE_UPDATE", payload: { elements } }, false);
      this.scheduleSave();
    }
  }

  /** Forward the local pointer position as a (throttled, volatile) cursor frame. */
  onPointerUpdate(pointer: { x: number; y: number; tool: "pointer" | "laser" }, button: "up" | "down"): void {
    if (!this.isActive || !this.socket) {
      return;
    }
    const now = Date.now();
    if (now - this.lastCursorSentAt < CURSOR_SYNC_MS) {
      return;
    }
    this.lastCursorSentAt = now;
    void this.broadcast(
      {
        type: "MOUSE_LOCATION",
        payload: {
          socketId: (this.socket.id ?? "") as SocketId,
          pointer,
          button,
          selectedElementIds: this.api.getAppState().selectedElementIds,
          username: this.username,
        },
      },
      true,
    );
  }

  private async broadcastScene(type: "SCENE_INIT" | "SCENE_UPDATE"): Promise<void> {
    const elements = this.api.getSceneElementsIncludingDeleted();
    this.lastBroadcastVersion = getSceneVersion(elements);
    await this.broadcast({ type, payload: { elements } }, false);
  }

  private async broadcast(payload: BroadcastPayload, volatile: boolean): Promise<void> {
    if (!this.socket || !this.key) {
      return;
    }
    const data = new TextEncoder().encode(JSON.stringify(payload));
    const { iv, ciphertext } = await encryptParts(this.key, data);
    const event = volatile ? "server-volatile-broadcast" : "server-broadcast";
    this.socket.emit(event, this.roomId, ciphertext, iv);
  }

  private async onClientBroadcast(data: ArrayBuffer, iv: ArrayBuffer): Promise<void> {
    if (!this.key) {
      return;
    }
    let message: BroadcastPayload | null;
    try {
      const plaintext = await decryptParts(this.key, new Uint8Array(iv), new Uint8Array(data));
      // Validate the decoded frame before dispatch: a peer that holds the room key can still emit a
      // malformed payload. Never log the error/plaintext (a JSON.parse SyntaxError can echo input).
      message = parseBroadcastPayload(JSON.parse(new TextDecoder().decode(plaintext)));
    } catch {
      console.warn("dropping undecodable broadcast frame");
      return;
    }
    if (!message) {
      console.warn("dropping malformed broadcast frame");
      return;
    }
    switch (message.type) {
      case "SCENE_INIT":
      case "SCENE_UPDATE":
        this.applyRemoteElements(message.payload.elements);
        break;
      case "MOUSE_LOCATION":
        this.applyPointer(message.payload);
        break;
    }
  }

  private applyRemoteElements(elements: readonly OrderedExcalidrawElement[]): void {
    const local = this.api.getSceneElementsIncludingDeleted();
    // `restoreElements` returns plain ordered elements; reconcile expects the branded remote
    // type, so cast through `unknown` (the brand is a compile-time marker only).
    const remote = restoreElements(elements, null) as unknown as readonly RemoteExcalidrawElement[];
    const reconciled = reconcileElements(local, remote, this.api.getAppState());
    this.api.updateScene({ elements: reconciled, captureUpdate: CaptureUpdateAction.NEVER });
    // Match our broadcast watermark to the merged scene so the resulting onChange does not
    // bounce the same update straight back out.
    this.lastBroadcastVersion = getSceneVersion(reconciled);
    void this.fetchMissingFiles(reconciled);
  }

  /** Upload any local scene files not yet pushed, encrypted with the room key. */
  private async syncLocalFiles(): Promise<void> {
    if (!this.key) {
      return;
    }
    for (const [id, file] of Object.entries(this.api.getFiles())) {
      if (this.knownFiles.has(id)) {
        continue;
      }
      this.knownFiles.add(id);
      try {
        const blob = await encrypt(this.key, new TextEncoder().encode(file.dataURL));
        await uploadFile(id, "application/octet-stream", blob);
      } catch (error) {
        this.knownFiles.delete(id); // allow a retry on the next change
        console.error("collab file upload failed", error);
      }
    }
  }

  /** Pull binary data for image elements whose files we don't have locally. */
  private async fetchMissingFiles(
    elements: readonly OrderedExcalidrawElement[],
  ): Promise<void> {
    if (!this.key) {
      return;
    }
    const have = this.api.getFiles();
    const wanted = new Set<string>();
    for (const element of elements) {
      if (element.type === "image" && element.fileId && !element.isDeleted) {
        wanted.add(element.fileId);
      }
    }
    for (const id of wanted) {
      if (have[id] || this.fetchingFiles.has(id)) {
        continue;
      }
      this.fetchingFiles.add(id);
      try {
        const { data } = await fetchFile(id);
        const dataURL = new TextDecoder().decode(await decrypt(this.key, data));
        const mimeType = dataURL.slice(5, dataURL.indexOf(";")) || "image/png";
        // All four required fields are present; only the branded string/union types
        // (`FileId`, `DataURL`, the mimeType union) need a cast — they are compile-time
        // markers our plain strings can't carry. Casting per-field keeps the object's
        // shape compiler-checked (a missing/renamed field would still error).
        const file: BinaryFileData = {
          id: id as BinaryFileData["id"],
          dataURL: dataURL as BinaryFileData["dataURL"],
          mimeType: mimeType as BinaryFileData["mimeType"],
          created: Date.now(),
        };
        this.api.addFiles([file]);
        this.knownFiles.add(id); // we have it now; no need to re-upload
      } catch (error) {
        console.error("collab file fetch failed", error);
      } finally {
        this.fetchingFiles.delete(id);
      }
    }
  }

  private applyPointer(payload: Extract<BroadcastPayload, { type: "MOUSE_LOCATION" }>["payload"]): void {
    const { socketId } = payload;
    const existing = this.collaborators.get(socketId) ?? {};
    this.collaborators.set(socketId, {
      ...existing,
      id: socketId,
      socketId,
      pointer: payload.pointer,
      button: payload.button,
      selectedElementIds: payload.selectedElementIds,
      username: payload.username,
      color: colorFor(socketId),
    });
    this.api.updateScene({ collaborators: new Map(this.collaborators) });
  }

  private updatePresence(sids: string[]): void {
    const ownId = this.socket?.id;
    const next = new Map<SocketId, Collaborator>();
    for (const sid of sids) {
      if (sid === ownId) {
        continue;
      }
      const id = sid as SocketId;
      next.set(id, this.collaborators.get(id) ?? { id: sid, socketId: id, color: colorFor(sid) });
    }
    this.collaborators = next;
    this.api.updateScene({ collaborators: new Map(next) });
  }

  private scheduleSave(): void {
    clearTimeout(this.saveTimer);
    this.saveTimer = setTimeout(() => void this.saveSnapshot(), SAVE_DEBOUNCE_MS);
  }

  private async saveSnapshot(): Promise<void> {
    if (!this.key) {
      return;
    }
    const elements = this.api.getSceneElementsIncludingDeleted();
    const json = JSON.stringify({ elements });
    const blob = await encrypt(this.key, new TextEncoder().encode(json));
    try {
      const response = await fetchWithTimeout(`${SCENE_URL}${encodeURIComponent(this.roomId)}/scene`, {
        method: "PUT",
        body: blob,
      });
      if (!response.ok) {
        // A 4xx/5xx (quota, lapsed auth, server error) resolves the promise normally; without
        // this check a failed persist looks like success and the durable snapshot silently
        // stops updating, so a later first-in-room peer restores stale state.
        console.error(`failed to persist room snapshot: ${response.status}`);
      }
    } catch (error) {
      console.error("failed to persist room snapshot", error);
    }
  }

  private async loadSnapshot(): Promise<void> {
    if (!this.key) {
      return;
    }
    try {
      const response = await fetchWithTimeout(`${SCENE_URL}${encodeURIComponent(this.roomId)}/scene`);
      if (!response.ok) {
        return; // 404: empty room with no prior state.
      }
      const blob = new Uint8Array(await response.arrayBuffer());
      const json = new TextDecoder().decode(await decrypt(this.key, blob));
      const parsed = JSON.parse(json) as { elements?: readonly OrderedExcalidrawElement[] };
      if (parsed.elements) {
        this.applyRemoteElements(parsed.elements);
      }
    } catch (error) {
      console.error("failed to load room snapshot", error);
    }
  }
}

function randomRoomId(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(10));
  return Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
}

/** Deterministic per-peer cursor color so a peer keeps the same color across frames. */
function colorFor(socketId: string): { background: string; stroke: string } {
  let hash = 0;
  for (let i = 0; i < socketId.length; i++) {
    hash = (hash * 31 + socketId.charCodeAt(i)) | 0;
  }
  const color = COLLAB_COLORS[Math.abs(hash) % COLLAB_COLORS.length];
  return { background: color, stroke: color };
}
