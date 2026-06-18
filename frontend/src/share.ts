// Share-link flow: serialize → encrypt → POST to the backend, and the inverse on load.
//
// Mirrors Excalidraw's capability-URL semantics: the backend stores opaque ciphertext under
// an unguessable id (the `/api/v2` routes the Rust binary already serves), and the AES key
// travels only in the URL fragment (`#json=<id>,<key>`), never to the server.

import { serializeAsJSON } from "@excalidraw/excalidraw";
import type {
  ExcalidrawElement,
  NonDeletedExcalidrawElement,
} from "@excalidraw/excalidraw/element/types";
import type { AppState, BinaryFiles } from "@excalidraw/excalidraw/types";

import { SHARE_GET_URL, SHARE_POST_URL } from "./config";
import { decrypt, encrypt, exportKey, generateKey, importKey } from "./crypto";
import { SHARE_FRAGMENT_PREFIX } from "./fragment";
import { fetchWithTimeout } from "./http";

// The capability-URL parser lives in the editor-free `fragment.ts` so it stays independently
// unit-testable; re-exported here to keep `parseShareFragment` part of the share module's API.
export { parseShareFragment } from "./fragment";

export interface RestoredScene {
  elements: readonly ExcalidrawElement[];
  appState: Partial<AppState>;
  files: BinaryFiles;
}

/**
 * Encrypt the current scene (images embedded), store it, and return the backend document id
 * plus the encoded decryption key. The backend never sees scene contents. Shared by the
 * share-link flow ({@link exportToShareLink}) and the scene library (which records the
 * `{id, key}` against a name).
 */
export async function uploadScene(
  elements: readonly NonDeletedExcalidrawElement[],
  appState: Partial<AppState>,
  files: BinaryFiles,
): Promise<{ id: string; key: string }> {
  const json = serializeAsJSON(elements, appState, files, "database");
  const key = await generateKey();
  const blob = await encrypt(key, new TextEncoder().encode(json));

  const response = await fetchWithTimeout(SHARE_POST_URL, { method: "POST", body: blob });
  if (!response.ok) {
    throw new Error(`share upload failed: ${response.status}`);
  }
  const body = (await response.json()) as unknown;
  if (typeof body !== "object" || body === null || typeof (body as { id?: unknown }).id !== "string") {
    throw new Error("share upload: malformed response (expected { id: string })");
  }
  const { id } = body as { id: string };
  return { id, key: await exportKey(key) };
}

/**
 * Encrypt the current scene, store it, and return a shareable URL whose fragment carries the
 * decryption key. The returned URL is safe to copy: the backend cannot read the scene.
 */
export async function exportToShareLink(
  elements: readonly NonDeletedExcalidrawElement[],
  appState: Partial<AppState>,
  files: BinaryFiles,
): Promise<string> {
  const { id, key } = await uploadScene(elements, appState, files);
  const url = new URL(window.location.href);
  url.hash = `${SHARE_FRAGMENT_PREFIX.slice(1)}${id},${key}`;
  return url.toString();
}

/** Fetch and decrypt a shared scene referenced by a fragment from `parseShareFragment`. */
export async function importFromShareLink(
  id: string,
  encodedKey: string,
): Promise<RestoredScene> {
  const response = await fetchWithTimeout(`${SHARE_GET_URL}${encodeURIComponent(id)}`);
  if (!response.ok) {
    throw new Error(`share download failed: ${response.status}`);
  }
  const blob = new Uint8Array(await response.arrayBuffer());
  const key = await importKey(encodedKey);
  const json = new TextDecoder().decode(await decrypt(key, blob));
  const parsed = JSON.parse(json) as {
    elements?: readonly ExcalidrawElement[];
    appState?: Partial<AppState>;
    files?: BinaryFiles;
  };
  return {
    elements: parsed.elements ?? [],
    appState: parsed.appState ?? {},
    files: parsed.files ?? {},
  };
}
