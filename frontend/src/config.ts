// Backend endpoints. Default to same-origin against the OxyDraw Rust binary; overridable via
// Vite env for the dev loop (point the dev server at a separately running `cargo run`).
//
// Unlike the old vendored app — whose Firebase URLs were string-rewritten server-side at
// serve time — this frontend is built knowing its own contract, so endpoints are plain
// config and no serve-time rewriting is needed.

const env = import.meta.env;

/** POST encrypted share blob → `{ id }`. */
export const SHARE_POST_URL: string = env.VITE_SHARE_POST_URL ?? "/api/v2/post/";

/** GET `${SHARE_GET_URL}${id}` → encrypted share blob. */
export const SHARE_GET_URL: string = env.VITE_SHARE_GET_URL ?? "/api/v2/";

/** PUT/GET `${FILE_URL}${id}` → opaque file blob (scene images). */
export const FILE_URL: string = env.VITE_FILE_URL ?? "/api/files/";

/** PUT/GET `${SCENE_URL}${roomId}/scene` → opaque collab scene snapshot. */
export const SCENE_URL: string = env.VITE_SCENE_URL ?? "/api/rooms/";

/** Socket.IO collaboration relay. Empty = same origin (the OxyDraw binary's built-in relay). */
export const WS_URL: string = env.VITE_WS_URL ?? "";
